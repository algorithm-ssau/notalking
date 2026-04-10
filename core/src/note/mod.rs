#![allow(dead_code)]

pub mod block_repo;
pub mod blocks;
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CreateNoteInput {
    pub user_id: Uuid,
    pub title: String,
    /// Initial plain text for the first text block (empty string yields one empty text block).
    pub initial_text: String,
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
    BlockNotFound,
    CorruptBlocks,
    InvalidOperation,
    Internal,
}

pub trait NoteUsecase {
    async fn create_note(&self, input: CreateNoteInput) -> Result<Note, NoteError>;
    async fn list_notes(&self, user_id: Uuid) -> Result<Vec<Note>, NoteError>;
    async fn delete_note(&self, input: DeleteNoteInput) -> Result<(), NoteError>;
    async fn list_blocks(
        &self,
        user_id: Uuid,
        note_id: Uuid,
    ) -> Result<Vec<editor::block::Block<()>>, NoteError>;
    async fn create_block(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        after_id: Option<Uuid>,
        content: editor::content::Content,
    ) -> Result<editor::block::Block<()>, NoteError>;
    async fn delete_block(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        block_id: Uuid,
    ) -> Result<(), NoteError>;
    async fn move_block(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        block_id: Uuid,
        after_id: Option<Uuid>,
        before_id: Option<Uuid>,
    ) -> Result<(), NoteError>;
    async fn apply_text_patch(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        block_id: Uuid,
        patch: TextPatch,
    ) -> Result<(), NoteError>;
}

/// High-level text operations on a single block (backed by `editor::text::TextBlock`).
#[derive(Debug, Clone)]
pub enum TextPatch {
    InsertText {
        position: usize,
        text: String,
        style: editor::text::Style,
    },
    DeleteRange {
        start: usize,
        end: usize,
    },
    DeleteAt {
        position: usize,
        direction: editor::text::DeleteDirection,
    },
    EnableFormatting {
        start: usize,
        end: usize,
        style: editor::text::Style,
    },
    DisableFormatting {
        start: usize,
        end: usize,
        style: editor::text::Style,
    },
}
