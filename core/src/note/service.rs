use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::note::{CreateNoteInput, DeleteNoteInput, Note, NoteError, NoteUsecase};

use super::repo::Repo as NoteRepo;

#[derive(Clone)]
pub struct NoteService<R> {
    repo: Arc<R>,
}

impl<R> NoteService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

impl<R> NoteUsecase for NoteService<R>
where
    R: NoteRepo + Send + Sync + 'static,
{
    async fn create_note(&self, input: CreateNoteInput) -> Result<Note, NoteError> {
        if input.title.trim().is_empty() {
            return Err(NoteError::InvalidInput);
        }

        let now = Utc::now();
        let note = Note {
            id: Uuid::new_v4(),
            user_id: input.user_id,
            title: input.title,
            body: input.body,
            created_at: now,
            updated_at: now,
        };

        self.repo.create(note).await
    }

    async fn list_notes(&self, user_id: Uuid) -> Result<Vec<Note>, NoteError> {
        self.repo.list_by_user(user_id).await
    }

    async fn delete_note(&self, input: DeleteNoteInput) -> Result<(), NoteError> {
        let note = self
            .repo
            .find_by_id(input.note_id)
            .await?
            .ok_or(NoteError::NotFound)?;

        if note.user_id != input.user_id {
            return Err(NoteError::Forbidden);
        }

        self.repo.delete_by_id(input.note_id).await
    }
}
