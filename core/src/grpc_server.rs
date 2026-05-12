use std::cmp::Ordering;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use editor::block::Block;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::embedding::EmbeddingRuntime;
use crate::note::block_repo::BlockRepository;
use crate::note::blocks::LoadedNoteBlocks;
use crate::note::repo::Repo;
use crate::note::service::NoteService;
use crate::note::{CreateNoteInput, Note, NoteBodyUpdate, NoteError, NoteUsecase, UpdateNoteInput};
use crate::persist::SqlNoteStore;

pub mod notalking {
    pub mod v1 {
        tonic::include_proto!("notalking.v1");
    }
}

use notalking::v1::core_bridge_server::{CoreBridge, CoreBridgeServer};
use notalking::v1::{
    CreateNoteRequest, CreateNoteResponse, GetNoteContentRequest, GetNoteContentResponse,
    GetNoteContextRequest, GetNoteContextResponse, HealthCheckRequest, HealthCheckResponse,
    NoteBlockContext, NoteSearchHit, SearchNotesRequest, SearchNotesResponse, UpdateNoteMode,
    UpdateNoteRequest, UpdateNoteResponse,
};

pub struct CoreGrpcService {
    pub notes: Arc<SqlNoteStore>,
    pub note_service: NoteService<SqlNoteStore, SqlNoteStore>,
    pub embedding: Option<Arc<EmbeddingRuntime>>,
}

#[derive(Clone)]
struct SearchHitDraft {
    note_id: String,
    title: String,
    matched_by: String,
    score: f32,
    excerpt: String,
    block_id: String,
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, Status> {
    Uuid::parse_str(value).map_err(|_| Status::invalid_argument(field))
}

fn bounded_limit(value: u32, default: usize, max: usize) -> usize {
    if value == 0 {
        default
    } else {
        (value as usize).min(max)
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_owned();
    }

    let take = max_chars.saturating_sub(3);
    let mut out: String = trimmed.chars().take(take).collect();
    out.push_str("...");
    out
}

fn search_terms(query: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "a", "an", "and", "are", "about", "did", "do", "does", "find", "for", "from", "had", "has",
        "have", "i", "in", "is", "me", "my", "note", "notes", "of", "on", "please", "search",
        "show", "tell", "that", "the", "this", "to", "what", "when", "where", "which", "with",
        "you",
    ];

    let raw: Vec<String> = query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| term.chars().count() >= 2)
        .map(str::to_owned)
        .collect();
    let filtered: Vec<String> = raw
        .iter()
        .filter(|term| !STOP_WORDS.contains(&term.as_str()))
        .cloned()
        .collect();
    if filtered.is_empty() { raw } else { filtered }
}

fn term_hits(haystack_lower: &str, terms: &[String]) -> usize {
    terms
        .iter()
        .filter(|term| haystack_lower.contains(term.as_str()))
        .count()
}

fn block_plain_text(block: &Block<()>) -> String {
    block.content.to_plain_text()
}

fn first_non_empty_excerpt(blocks: &[Block<()>]) -> String {
    blocks
        .iter()
        .map(block_plain_text)
        .find(|text| !text.trim().is_empty())
        .map(|text| truncate_chars(&text, 320))
        .unwrap_or_default()
}

fn keep_best_hit(hits: &mut HashMap<String, SearchHitDraft>, hit: SearchHitDraft) {
    match hits.get(&hit.note_id) {
        Some(existing) if existing.score >= hit.score => {}
        _ => {
            hits.insert(hit.note_id.clone(), hit);
        }
    }
}

fn note_error_status(error: NoteError) -> Status {
    match error {
        NoteError::InvalidInput => Status::invalid_argument("invalid note input"),
        NoteError::NotFound => Status::not_found("note not found"),
        NoteError::Forbidden => Status::permission_denied("forbidden"),
        NoteError::BlockNotFound => Status::not_found("block not found"),
        NoteError::CorruptBlocks => Status::internal("note document is corrupt"),
        NoteError::InvalidOperation => Status::invalid_argument("invalid note operation"),
        NoteError::Internal => Status::internal("database error"),
    }
}

impl CoreGrpcService {
    async fn note_for_user(&self, user_id: Uuid, note_id: Uuid) -> Result<Note, Status> {
        let note = self
            .notes
            .find_by_id(note_id)
            .await
            .map_err(|_| Status::internal("database error"))?
            .ok_or_else(|| Status::not_found("note not found"))?;

        if note.user_id != user_id {
            return Err(Status::permission_denied("note not owned by user"));
        }

        Ok(note)
    }

    async fn ordered_blocks(&self, note_id: Uuid) -> Result<Vec<Block<()>>, Status> {
        let raw = self
            .notes
            .load_document(note_id)
            .await
            .map_err(|_| Status::internal("database error"))?
            .ok_or_else(|| Status::internal("note document is missing"))?;
        let loaded = LoadedNoteBlocks::from_raw(raw)
            .map_err(|_| Status::internal("note document is corrupt"))?;
        Ok(loaded.ordered_blocks().into_iter().cloned().collect())
    }

    async fn add_semantic_hits(
        &self,
        user_id: Uuid,
        query: &str,
        limit: usize,
        hits: &mut HashMap<String, SearchHitDraft>,
    ) {
        let Some(embedding) = self.embedding.as_ref() else {
            return;
        };
        let Ok(rows) = embedding
            .semantic_search(user_id, query, limit as u64)
            .await
        else {
            tracing::debug!(%user_id, "CoreBridge note semantic search failed; falling back to lexical search");
            return;
        };

        for (note_id, block_id, score) in rows {
            let Ok(note) = self.note_for_user(user_id, note_id).await else {
                continue;
            };
            let blocks = self.ordered_blocks(note_id).await.unwrap_or_default();
            let excerpt = blocks
                .iter()
                .find(|block| block.id == block_id)
                .map(block_plain_text)
                .filter(|text| !text.trim().is_empty())
                .map(|text| truncate_chars(&text, 320))
                .unwrap_or_else(|| first_non_empty_excerpt(&blocks));

            keep_best_hit(
                hits,
                SearchHitDraft {
                    note_id: note.id.to_string(),
                    title: note.title,
                    matched_by: "semantic".to_owned(),
                    score: 90.0 + score,
                    excerpt,
                    block_id: block_id.to_string(),
                },
            );
        }
    }

    async fn plain_text_for_note(&self, note_id: Uuid) -> Result<String, Status> {
        Ok(self
            .ordered_blocks(note_id)
            .await?
            .into_iter()
            .map(|block| block_plain_text(&block))
            .collect::<Vec<_>>()
            .join("\n\n"))
    }
}

#[tonic::async_trait]
impl CoreBridge for CoreGrpcService {
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: "ok".to_owned(),
        }))
    }

    async fn get_note_context(
        &self,
        request: Request<GetNoteContextRequest>,
    ) -> Result<Response<GetNoteContextResponse>, Status> {
        let inner = request.into_inner();
        let user_id =
            Uuid::parse_str(&inner.user_id).map_err(|_| Status::invalid_argument("user_id"))?;
        let note_id =
            Uuid::parse_str(&inner.note_id).map_err(|_| Status::invalid_argument("note_id"))?;

        let note = self
            .notes
            .find_by_id(note_id)
            .await
            .map_err(|_| Status::internal("database error"))?
            .ok_or_else(|| Status::not_found("note not found"))?;

        if note.user_id != user_id {
            return Err(Status::permission_denied("note not owned by user"));
        }

        Ok(Response::new(GetNoteContextResponse {
            title: note.title,
            head_block_id: note.head_id.map(|u| u.to_string()).unwrap_or_default(),
        }))
    }

    async fn search_notes(
        &self,
        request: Request<SearchNotesRequest>,
    ) -> Result<Response<SearchNotesResponse>, Status> {
        let inner = request.into_inner();
        let user_id = parse_uuid(&inner.user_id, "user_id")?;
        let query = inner.query.trim();
        let query_lower = query.to_lowercase();
        let terms = search_terms(query);
        let limit = bounded_limit(inner.limit, 8, 50);

        let notes = self
            .notes
            .list_by_user(user_id)
            .await
            .map_err(|_| Status::internal("database error"))?;
        let mut hits: HashMap<String, SearchHitDraft> = HashMap::new();

        if !query.is_empty() {
            self.add_semantic_hits(user_id, query, limit, &mut hits)
                .await;
        }

        for (idx, note) in notes.into_iter().enumerate() {
            let blocks = match self.ordered_blocks(note.id).await {
                Ok(blocks) => blocks,
                Err(error) => {
                    tracing::warn!(note_id = %note.id, error = %error, "CoreBridge search skipped unreadable note blocks");
                    Vec::new()
                }
            };

            if query.is_empty() {
                keep_best_hit(
                    &mut hits,
                    SearchHitDraft {
                        note_id: note.id.to_string(),
                        title: note.title,
                        matched_by: "recent".to_owned(),
                        score: 1.0 / ((idx + 1) as f32),
                        excerpt: first_non_empty_excerpt(&blocks),
                        block_id: String::new(),
                    },
                );
                continue;
            }

            let title_lower = note.title.to_lowercase();
            let title_term_hits = term_hits(&title_lower, &terms);
            if title_lower.contains(&query_lower) || title_term_hits > 0 {
                let score = if title_lower.contains(&query_lower) {
                    100.0
                } else {
                    50.0 + (title_term_hits as f32 * 10.0)
                };
                keep_best_hit(
                    &mut hits,
                    SearchHitDraft {
                        note_id: note.id.to_string(),
                        title: note.title.clone(),
                        matched_by: "title".to_owned(),
                        score,
                        excerpt: first_non_empty_excerpt(&blocks),
                        block_id: String::new(),
                    },
                );
            }

            let mut best_block: Option<(Uuid, String, f32)> = None;
            for block in &blocks {
                let plain = block_plain_text(block);
                if plain.trim().is_empty() {
                    continue;
                }
                let plain_lower = plain.to_lowercase();
                let block_term_hits = term_hits(&plain_lower, &terms);
                let exact = plain_lower.contains(&query_lower);
                if !exact && block_term_hits == 0 {
                    continue;
                }

                let score = if exact {
                    80.0
                } else {
                    30.0 + (block_term_hits as f32 * 8.0) + (title_term_hits as f32 * 3.0)
                };
                match &best_block {
                    Some((_, _, existing_score)) if *existing_score >= score => {}
                    _ => best_block = Some((block.id, plain, score)),
                }
            }

            if let Some((block_id, plain, score)) = best_block {
                keep_best_hit(
                    &mut hits,
                    SearchHitDraft {
                        note_id: note.id.to_string(),
                        title: note.title,
                        matched_by: "block".to_owned(),
                        score,
                        excerpt: truncate_chars(&plain, 320),
                        block_id: block_id.to_string(),
                    },
                );
            }
        }

        let mut hits: Vec<SearchHitDraft> = hits.into_values().collect();
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        hits.truncate(limit);

        Ok(Response::new(SearchNotesResponse {
            hits: hits
                .into_iter()
                .map(|hit| NoteSearchHit {
                    note_id: hit.note_id,
                    title: hit.title,
                    matched_by: hit.matched_by,
                    score: hit.score,
                    excerpt: hit.excerpt,
                    block_id: hit.block_id,
                })
                .collect(),
        }))
    }

    async fn get_note_content(
        &self,
        request: Request<GetNoteContentRequest>,
    ) -> Result<Response<GetNoteContentResponse>, Status> {
        let inner = request.into_inner();
        let user_id = parse_uuid(&inner.user_id, "user_id")?;
        let note_id = parse_uuid(&inner.note_id, "note_id")?;
        let max_blocks = bounded_limit(inner.max_blocks, 80, 200);
        let max_chars_per_block = bounded_limit(inner.max_chars_per_block, 4_000, 12_000);

        let note = self.note_for_user(user_id, note_id).await?;
        let blocks = self.ordered_blocks(note_id).await?;

        Ok(Response::new(GetNoteContentResponse {
            note_id: note.id.to_string(),
            title: note.title,
            head_block_id: note.head_id.map(|u| u.to_string()).unwrap_or_default(),
            blocks: blocks
                .into_iter()
                .take(max_blocks)
                .enumerate()
                .map(|(idx, block)| NoteBlockContext {
                    block_id: block.id.to_string(),
                    order: idx as u32,
                    plain_text: truncate_chars(&block_plain_text(&block), max_chars_per_block),
                })
                .collect(),
        }))
    }

    async fn create_note(
        &self,
        request: Request<CreateNoteRequest>,
    ) -> Result<Response<CreateNoteResponse>, Status> {
        let inner = request.into_inner();
        let user_id = parse_uuid(&inner.user_id, "user_id")?;
        let title = inner.title.trim().to_owned();
        let note = self
            .note_service
            .create_note(CreateNoteInput {
                user_id,
                title,
                initial_text: inner.initial_text,
            })
            .await
            .map_err(note_error_status)?;

        if let Some(embedding) = self.embedding.as_ref() {
            embedding.notify_blocks_changed(user_id, note.id);
        }

        Ok(Response::new(CreateNoteResponse {
            note_id: note.id.to_string(),
            title: note.title,
            head_block_id: note.head_id.map(|u| u.to_string()).unwrap_or_default(),
        }))
    }

    async fn update_note(
        &self,
        request: Request<UpdateNoteRequest>,
    ) -> Result<Response<UpdateNoteResponse>, Status> {
        let inner = request.into_inner();
        let user_id = parse_uuid(&inner.user_id, "user_id")?;
        let note_id = parse_uuid(&inner.note_id, "note_id")?;

        let (title, body) = match UpdateNoteMode::try_from(inner.mode) {
            Ok(UpdateNoteMode::UpdateTitle) => {
                let title = inner.title.trim().to_owned();
                if title.is_empty() {
                    return Err(Status::invalid_argument("title"));
                }
                (Some(title), None)
            }
            Ok(UpdateNoteMode::ReplaceBody) => (
                if inner.title.trim().is_empty() {
                    None
                } else {
                    Some(inner.title.trim().to_owned())
                },
                Some(NoteBodyUpdate::ReplacePlainText { text: inner.body }),
            ),
            Ok(UpdateNoteMode::AppendBody) => (
                if inner.title.trim().is_empty() {
                    None
                } else {
                    Some(inner.title.trim().to_owned())
                },
                Some(NoteBodyUpdate::AppendPlainText { text: inner.body }),
            ),
            Ok(UpdateNoteMode::Unspecified) | Err(_) => {
                return Err(Status::invalid_argument("mode"));
            }
        };

        let note = self
            .note_service
            .update_note(UpdateNoteInput {
                user_id,
                note_id,
                title,
                body,
            })
            .await
            .map_err(note_error_status)?;

        let plain_text = self.plain_text_for_note(note.id).await?;

        if let Some(embedding) = self.embedding.as_ref() {
            embedding.notify_blocks_changed(user_id, note.id);
        }

        Ok(Response::new(UpdateNoteResponse {
            note_id: note.id.to_string(),
            title: note.title,
            head_block_id: note.head_id.map(|u| u.to_string()).unwrap_or_default(),
            plain_text,
        }))
    }
}

pub async fn serve_grpc(
    bind: SocketAddr,
    notes: Arc<SqlNoteStore>,
    note_service: NoteService<SqlNoteStore, SqlNoteStore>,
    embedding: Option<Arc<EmbeddingRuntime>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = CoreGrpcService {
        notes,
        note_service,
        embedding,
    };
    let server = CoreBridgeServer::new(service);
    tonic::transport::Server::builder()
        .add_service(server)
        .serve(bind)
        .await?;
    Ok(())
}
