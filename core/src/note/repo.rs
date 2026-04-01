#![allow(dead_code)]

use uuid::Uuid;

use crate::note::{Note, NoteError};

pub trait Repo {
    async fn create(&self, note: Note) -> Result<Note, NoteError>;
    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Note>, NoteError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Note>, NoteError>;
    async fn delete_by_id(&self, id: Uuid) -> Result<(), NoteError>;
}
