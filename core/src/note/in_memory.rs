#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::note::{Note, NoteError};

use super::repo::Repo;

#[derive(Clone, Default)]
pub struct InMemoryRepo {
    notes: Arc<RwLock<HashMap<Uuid, Note>>>,
}

impl InMemoryRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Repo for InMemoryRepo {
    async fn create(&self, note: Note) -> Result<Note, NoteError> {
        let mut notes = self.notes.write().await;
        notes.insert(note.id, note.clone());
        Ok(note)
    }

    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Note>, NoteError> {
        let notes = self.notes.read().await;
        let mut list: Vec<Note> = notes
            .values()
            .filter(|note| note.user_id == user_id)
            .cloned()
            .collect();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(list)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Note>, NoteError> {
        let notes = self.notes.read().await;
        Ok(notes.get(&id).cloned())
    }

    async fn delete_by_id(&self, id: Uuid) -> Result<(), NoteError> {
        let mut notes = self.notes.write().await;
        if notes.remove(&id).is_none() {
            return Err(NoteError::NotFound);
        }
        Ok(())
    }
}
