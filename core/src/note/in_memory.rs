#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::note::{Note, NoteError};

use super::block_repo::{BlockRepository, RawBlockDocument};
use super::repo::Repo;

struct Inner {
    notes: HashMap<Uuid, Note>,
    block_docs: HashMap<Uuid, RawBlockDocument>,
}

#[derive(Clone)]
pub struct InMemoryNoteStore {
    inner: Arc<RwLock<Inner>>,
}

impl InMemoryNoteStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                notes: HashMap::new(),
                block_docs: HashMap::new(),
            })),
        }
    }
}

impl Repo for InMemoryNoteStore {
    async fn create(&self, note: Note) -> Result<Note, NoteError> {
        let mut g = self.inner.write().await;
        g.notes.insert(note.id, note.clone());
        Ok(note)
    }

    async fn update(&self, note: Note) -> Result<(), NoteError> {
        let mut g = self.inner.write().await;
        if !g.notes.contains_key(&note.id) {
            return Err(NoteError::NotFound);
        }
        g.notes.insert(note.id, note);
        Ok(())
    }

    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Note>, NoteError> {
        let g = self.inner.read().await;
        let mut list: Vec<Note> = g
            .notes
            .values()
            .filter(|note| note.user_id == user_id)
            .cloned()
            .collect();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(list)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Note>, NoteError> {
        let g = self.inner.read().await;
        Ok(g.notes.get(&id).cloned())
    }

    async fn delete_by_id(&self, id: Uuid) -> Result<(), NoteError> {
        let mut g = self.inner.write().await;
        if g.notes.remove(&id).is_none() {
            return Err(NoteError::NotFound);
        }
        g.block_docs.remove(&id);
        Ok(())
    }
}

impl BlockRepository for InMemoryNoteStore {
    async fn load_document(&self, note_id: Uuid) -> Result<Option<RawBlockDocument>, NoteError> {
        let g = self.inner.read().await;
        Ok(g.block_docs.get(&note_id).cloned())
    }

    async fn save_document(&self, note_id: Uuid, doc: &RawBlockDocument) -> Result<(), NoteError> {
        let mut g = self.inner.write().await;
        if !g.notes.contains_key(&note_id) {
            return Err(NoteError::NotFound);
        }
        g.block_docs.insert(note_id, doc.clone());
        Ok(())
    }

    async fn delete_document(&self, note_id: Uuid) -> Result<(), NoteError> {
        let mut g = self.inner.write().await;
        g.block_docs.remove(&note_id);
        Ok(())
    }
}
