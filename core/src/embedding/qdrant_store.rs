use std::sync::Arc;

use qdrant_client::Payload;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CollectionInfo, Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter,
    PointStruct, SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder, vectors_config,
};
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
pub struct QdrantVectorStore {
    client: Arc<Qdrant>,
    collection: String,
    vector_dim: usize,
}

impl QdrantVectorStore {
    pub async fn new(url: &str, collection: &str, vector_dim: usize) -> Result<Self, String> {
        let client = Qdrant::from_url(url)
            .skip_compatibility_check()
            .build()
            .map_err(|e| e.to_string())?;
        let client = Arc::new(client);
        let s = Self {
            client: client.clone(),
            collection: collection.to_owned(),
            vector_dim,
        };
        if !client
            .collection_exists(collection)
            .await
            .map_err(|e| e.to_string())?
        {
            tracing::info!(%collection, configured_vector_dim = vector_dim, "creating qdrant collection");
            client
                .create_collection(CreateCollectionBuilder::new(collection).vectors_config(
                    VectorParamsBuilder::new(vector_dim as u64, Distance::Cosine),
                ))
                .await
                .map_err(|e| e.to_string())?;
        } else {
            let info = client
                .collection_info(collection)
                .await
                .map_err(|e| e.to_string())?;
            let actual_dim = info.result.as_ref().and_then(collection_vector_dim);
            match actual_dim {
                Some(actual) if actual != vector_dim as u64 => {
                    return Err(format!(
                        "qdrant collection {collection} vector dimension mismatch: expected {vector_dim}, got {actual}; use a new QDRANT_COLLECTION or recreate the collection"
                    ));
                }
                Some(actual) => {
                    tracing::info!(
                        %collection,
                        configured_vector_dim = vector_dim,
                        actual_vector_dim = actual,
                        "using existing qdrant collection"
                    );
                }
                None => {
                    tracing::warn!(
                        %collection,
                        configured_vector_dim = vector_dim,
                        "using existing qdrant collection with unknown vector dimension"
                    );
                }
            }
        }
        Ok(s)
    }

    async fn delete_by_note_id(&self, note_id: Uuid) -> Result<(), String> {
        let filter = Filter::must([Condition::matches("note_id", note_id.to_string())]);
        tracing::debug!(collection = %self.collection, %note_id, "deleting qdrant points for note");
        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.collection)
                    .points(filter)
                    .wait(true),
            )
            .await
            .map_err(|e| e.to_string())?;
        tracing::debug!(collection = %self.collection, %note_id, "qdrant points deleted for note");
        Ok(())
    }

    pub async fn reindex_note_blocks(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        blocks: &[(Uuid, String)],
        vectors: &[Vec<f32>],
    ) -> Result<(), String> {
        if blocks.is_empty() {
            self.delete_by_note_id(note_id).await?;
            return Ok(());
        }
        if blocks.len() != vectors.len() {
            return Err("embedding count mismatch".to_owned());
        }
        let mut points = Vec::new();
        for ((block_id, plain), vec) in blocks.iter().zip(vectors.iter()) {
            if vec.len() != self.vector_dim {
                return Err(format!(
                    "embedding vector dimension mismatch for block {block_id}: expected {}, got {}",
                    self.vector_dim,
                    vec.len(),
                ));
            }
            let payload = Payload::try_from(json!({
                "user_id": user_id.to_string(),
                "note_id": note_id.to_string(),
                "block_id": block_id.to_string(),
                "text": plain,
            }))
            .map_err(|e| e.to_string())?;
            let id = block_id.to_string();
            points.push(PointStruct::new(id, vec.clone(), payload));
        }
        self.delete_by_note_id(note_id).await?;
        tracing::debug!(
            collection = %self.collection,
            %note_id,
            points = points.len(),
            vector_dim = self.vector_dim,
            "upserting qdrant points"
        );
        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection, points).wait(true))
            .await
            .map_err(|e| e.to_string())?;
        tracing::debug!(collection = %self.collection, %note_id, "qdrant points upserted");
        Ok(())
    }

    pub async fn search(
        &self,
        user_id: Uuid,
        vector: Vec<f32>,
        limit: u64,
    ) -> Result<Vec<(Uuid, Uuid, f32)>, String> {
        let filter = Filter::must([Condition::matches("user_id", user_id.to_string())]);
        tracing::debug!(
            collection = %self.collection,
            %user_id,
            limit,
            vector_dim = vector.len(),
            "qdrant semantic search"
        );
        let result = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection, vector, limit)
                    .filter(filter)
                    .with_payload(true),
            )
            .await
            .map_err(|e| e.to_string())?;
        let raw_hits = result.result.len();
        let mut out = Vec::new();
        for hit in result.result {
            let score = hit.score;
            let note = hit.payload.get("note_id").and_then(|v| match &v.kind {
                Some(qdrant_client::qdrant::value::Kind::StringValue(s)) => Uuid::parse_str(s).ok(),
                _ => None,
            });
            let block = hit.payload.get("block_id").and_then(|v| match &v.kind {
                Some(qdrant_client::qdrant::value::Kind::StringValue(s)) => Uuid::parse_str(s).ok(),
                _ => None,
            });
            if let (Some(n), Some(b)) = (note, block) {
                out.push((n, b, score));
            }
        }
        tracing::debug!(
            collection = %self.collection,
            raw_hits,
            hits = out.len(),
            "qdrant semantic search complete"
        );
        Ok(out)
    }
}

fn collection_vector_dim(info: &CollectionInfo) -> Option<u64> {
    let config = info.config.as_ref()?;
    let params = config.params.as_ref()?;
    let vectors = params.vectors_config.as_ref()?;
    match vectors.config.as_ref()? {
        vectors_config::Config::Params(params) => Some(params.size),
        vectors_config::Config::ParamsMap(map) if map.map.len() == 1 => {
            map.map.values().next().map(|params| params.size)
        }
        vectors_config::Config::ParamsMap(_) => None,
    }
}
