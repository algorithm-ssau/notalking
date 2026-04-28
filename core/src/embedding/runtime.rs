use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::config::{CoreConfig, EmbeddingRegenerationMode};
use crate::embedding::{HttpEmbeddingProvider, QdrantVectorStore};
use crate::note::block_repo::BlockRepository;
use crate::note::blocks::LoadedNoteBlocks;
use crate::persist::SqlNoteStore;
use editor::content::Content;

/// Coordinates async embedding regeneration (SPEC 4.5).
pub struct EmbeddingRuntime {
    inner: Arc<Inner>,
}

struct Inner {
    provider: HttpEmbeddingProvider,
    store: QdrantVectorStore,
    notes: Arc<SqlNoteStore>,
    mode: EmbeddingRegenerationMode,
    idle_ms: u64,
    debounce: Mutex<HashMap<Uuid, tokio::task::JoinHandle<()>>>,
}

impl EmbeddingRuntime {
    pub async fn try_new(config: &CoreConfig, notes: Arc<SqlNoteStore>) -> Result<Option<Self>, String> {
        let q_url = match &config.qdrant_url {
            Some(u) if !u.is_empty() => u.clone(),
            _ => return Ok(None),
        };
        let emb_base = match &config.embedding_provider_url {
            Some(u) if !u.is_empty() => u.clone(),
            _ => return Ok(None),
        };

        let store = QdrantVectorStore::new(&q_url, &config.qdrant_collection, config.embedding_vector_dimensions).await?;
        let api_key = std::env::var("OPENAI_API_KEY").ok();
        let provider = HttpEmbeddingProvider::new(
            emb_base,
            config.embedding_model.clone(),
            api_key,
            config.embedding_query_prefix.clone(),
            config.embedding_document_prefix.clone(),
        );

        let idle_ms = match config.embedding_regeneration {
            EmbeddingRegenerationMode::AfterQuietPeriodSinceLastPatch { idle_ms } => idle_ms,
            EmbeddingRegenerationMode::OnEachBlockPatch => 0,
        };

        Ok(Some(Self {
            inner: Arc::new(Inner {
                provider,
                store,
                notes,
                mode: config.embedding_regeneration,
                idle_ms,
                debounce: Mutex::new(HashMap::new()),
            }),
        }))
    }

    pub fn notify_blocks_changed(&self, user_id: Uuid, note_id: Uuid) {
        let inner = self.inner.clone();
        match inner.mode {
            EmbeddingRegenerationMode::OnEachBlockPatch => {
                tokio::spawn(async move {
                    if let Err(e) = inner.reindex_note(user_id, note_id).await {
                        tracing::warn!(error = %e, ?note_id, "embedding reindex failed");
                    }
                });
            }
            EmbeddingRegenerationMode::AfterQuietPeriodSinceLastPatch { .. } => {
                tokio::spawn(async move {
                    let mut g = inner.debounce.lock().await;
                    if let Some(h) = g.remove(&note_id) {
                        h.abort();
                    }
                    let inner2 = inner.clone();
                    let delay = std::time::Duration::from_millis(inner2.idle_ms.max(1));
                    let h = tokio::spawn(async move {
                        tokio::time::sleep(delay).await;
                        if let Err(e) = inner2.reindex_note(user_id, note_id).await {
                            tracing::warn!(error = %e, ?note_id, "embedding reindex failed");
                        }
                    });
                    g.insert(note_id, h);
                });
            }
        }
    }

    pub async fn semantic_search(
        &self,
        user_id: Uuid,
        query: &str,
        limit: u64,
    ) -> Result<Vec<(Uuid, Uuid, f32)>, String> {
        let vec = self.inner.provider.embed_query(query).await?;
        self.inner.store.search(user_id, vec, limit).await
    }
}

impl Inner {
    async fn reindex_note(&self, user_id: Uuid, note_id: Uuid) -> Result<(), String> {
        let raw = self
            .notes
            .load_document(note_id)
            .await
            .map_err(|_| "note load failed".to_owned())?
            .ok_or_else(|| "missing document".to_owned())?;
        let loaded = LoadedNoteBlocks::from_raw(raw).map_err(|_| "corrupt blocks".to_owned())?;

        let mut block_texts: Vec<(Uuid, String)> = Vec::new();
        for block in loaded.blocks.values() {
            let plain = match &block.content {
                Content::Text(tb) => tb.to_plain_text(),
                Content::OrderedListItem(tb, _) | Content::UnorderedListItem(tb, _) => tb.to_plain_text(),
                Content::Image(_) | Content::Video(_) => continue,
            };
            if plain.trim().is_empty() {
                continue;
            }
            block_texts.push((block.id, plain));
        }

        let mut vectors = Vec::with_capacity(block_texts.len());
        for (_, text) in &block_texts {
            vectors.push(self.provider.embed_document(text).await?);
        }

        self.store
            .reindex_note_blocks(user_id, note_id, &block_texts, &vectors)
            .await?;
        if block_texts.is_empty() {
            tracing::info!(%note_id, "qdrant note cleared (no non-empty text blocks)");
        } else {
            tracing::info!(%note_id, blocks = block_texts.len(), "qdrant note vectors upserted");
        }
        Ok(())
    }
}
