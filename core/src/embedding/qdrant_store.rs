use std::sync::Arc;

use qdrant_client::Payload;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointStruct,
    SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
};
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
pub struct QdrantVectorStore {
    client: Arc<Qdrant>,
    collection: String,
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
        };
        if !client
            .collection_exists(collection)
            .await
            .map_err(|e| e.to_string())?
        {
            client
                .create_collection(
                    CreateCollectionBuilder::new(collection)
                        .vectors_config(VectorParamsBuilder::new(vector_dim as u64, Distance::Cosine)),
                )
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(s)
    }

    async fn delete_by_note_id(&self, note_id: Uuid) -> Result<(), String> {
        let filter = Filter::must([Condition::matches("note_id", note_id.to_string())]);
        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.collection)
                    .points(filter)
                    .wait(true),
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn reindex_note_blocks(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        blocks: &[(Uuid, String)],
        vectors: &[Vec<f32>],
    ) -> Result<(), String> {
        self.delete_by_note_id(note_id).await?;
        if blocks.is_empty() {
            return Ok(());
        }
        if blocks.len() != vectors.len() {
            return Err("embedding count mismatch".to_owned());
        }
        let mut points = Vec::new();
        for ((block_id, plain), vec) in blocks.iter().zip(vectors.iter()) {
            let payload = Payload::try_from(json!({
                "user_id": user_id.to_string(),
                "note_id": note_id.to_string(),
                "block_id": block_id.to_string(),
                "text": plain,
            }))
            .map_err(|e| e.to_string())?;
            let id = format!("{note_id}:{block_id}");
            points.push(PointStruct::new(id, vec.clone(), payload));
        }
        self.client
            .upsert_points(
                UpsertPointsBuilder::new(&self.collection, points).wait(true),
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn search(
        &self,
        user_id: Uuid,
        vector: Vec<f32>,
        limit: u64,
    ) -> Result<Vec<(Uuid, Uuid, f32)>, String> {
        let filter = Filter::must([Condition::matches(
            "user_id",
            user_id.to_string(),
        )]);
        let result = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection, vector, limit).filter(filter),
            )
            .await
            .map_err(|e| e.to_string())?;
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
        Ok(out)
    }
}
