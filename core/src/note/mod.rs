#![allow(dead_code)]

pub mod in_memory;
pub mod repo;
pub mod service;

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct Note {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CreateNoteInput {
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
}

pub struct DeleteNoteInput {
    pub user_id: Uuid,
    pub note_id: Uuid,
}

#[derive(Debug, Clone, Copy)]
pub enum NoteError {
    InvalidInput,
    NotFound,
    Forbidden,
    Internal,
}

pub trait NoteUsecase {
    async fn create_note(&self, input: CreateNoteInput) -> Result<Note, NoteError>;
    async fn list_notes(&self, user_id: Uuid) -> Result<Vec<Note>, NoteError>;
    async fn delete_note(&self, input: DeleteNoteInput) -> Result<(), NoteError>;
}
