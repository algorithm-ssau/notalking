use std::collections::HashMap;

use base64::Engine;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::NoteError;

/// Serialized block payloads keyed by block id, plus the list head.
/// Each value is typically JSON bytes for `editor::Block<()>`, so a database
/// implementation can store one row per block or a single JSON document.
#[derive(Clone, Debug, Default)]
pub struct RawBlockDocument {
    pub head_id: Option<Uuid>,
    pub blocks: HashMap<Uuid, Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
struct RawBlockDocumentSerde {
    head_id: Option<Uuid>,
    blocks: std::collections::HashMap<Uuid, String>,
}

impl RawBlockDocument {
    pub fn to_json_string(&self) -> Result<String, NoteError> {
        let engine = base64::engine::general_purpose::STANDARD;
        let mut blocks = std::collections::HashMap::new();
        for (id, bytes) in &self.blocks {
            blocks.insert(*id, engine.encode(bytes));
        }
        let dto = RawBlockDocumentSerde {
            head_id: self.head_id,
            blocks,
        };
        serde_json::to_string(&dto).map_err(|_| NoteError::Internal)
    }

    pub fn from_json_str(s: &str) -> Result<Self, NoteError> {
        let engine = base64::engine::general_purpose::STANDARD;
        let dto: RawBlockDocumentSerde =
            serde_json::from_str(s).map_err(|_| NoteError::CorruptBlocks)?;
        let mut blocks = HashMap::new();
        for (id, b64) in dto.blocks {
            let bytes = engine.decode(b64.as_bytes()).map_err(|_| NoteError::CorruptBlocks)?;
            blocks.insert(id, bytes);
        }
        Ok(RawBlockDocument {
            head_id: dto.head_id,
            blocks,
        })
    }
}

pub trait BlockRepository: Send + Sync {
    async fn load_document(&self, note_id: Uuid) -> Result<Option<RawBlockDocument>, NoteError>;
    async fn save_document(&self, note_id: Uuid, doc: &RawBlockDocument) -> Result<(), NoteError>;
    async fn delete_document(&self, note_id: Uuid) -> Result<(), NoteError>;
}
