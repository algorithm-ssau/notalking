use std::collections::HashMap;

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

pub trait BlockRepository: Send + Sync {
    async fn load_document(&self, note_id: Uuid) -> Result<Option<RawBlockDocument>, NoteError>;
    async fn save_document(&self, note_id: Uuid, doc: &RawBlockDocument) -> Result<(), NoteError>;
    async fn delete_document(&self, note_id: Uuid) -> Result<(), NoteError>;
}
