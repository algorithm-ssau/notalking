use std::sync::Arc;

use chrono::Utc;
use editor::block::Block;
use editor::content::Content;
use uuid::Uuid;

use crate::note::{
    CreateNoteInput, DeleteNoteInput, Note, NoteError, NoteUsecase, TextPatch, blocks::LoadedNoteBlocks,
};

use super::block_repo::BlockRepository;
use super::repo::Repo as NoteRepoTrait;

#[derive(Clone)]
pub struct NoteService<R, B> {
    repo: Arc<R>,
    blocks: Arc<B>,
}

impl<R, B> NoteService<R, B> {
    pub fn new(repo: Arc<R>, blocks: Arc<B>) -> Self {
        Self { repo, blocks }
    }
}

impl<R, B> NoteService<R, B>
where
    R: NoteRepoTrait + Send + Sync,
    B: BlockRepository + Send + Sync,
{
    async fn ensure_note_owner(&self, user_id: Uuid, note_id: Uuid) -> Result<Note, NoteError> {
        let note = self
            .repo
            .find_by_id(note_id)
            .await?
            .ok_or(NoteError::NotFound)?;
        if note.user_id != user_id {
            return Err(NoteError::Forbidden);
        }
        Ok(note)
    }

    async fn load_blocks(&self, note_id: Uuid) -> Result<LoadedNoteBlocks, NoteError> {
        let raw = self
            .blocks
            .load_document(note_id)
            .await?
            .ok_or(NoteError::CorruptBlocks)?;
        LoadedNoteBlocks::from_raw(raw)
    }

    async fn save_blocks(&self, note_id: Uuid, doc: LoadedNoteBlocks) -> Result<(), NoteError> {
        let raw = doc.into_raw()?;
        self.blocks.save_document(note_id, &raw).await
    }

    async fn touch_note(&self, mut note: Note) -> Result<(), NoteError> {
        note.updated_at = Utc::now();
        self.repo.update(note).await
    }
}

impl<R, B> NoteUsecase for NoteService<R, B>
where
    R: NoteRepoTrait + Send + Sync + 'static,
    B: BlockRepository + Send + Sync + 'static,
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
            created_at: now,
            updated_at: now,
        };

        let created = self.repo.create(note).await?;
        let loaded = LoadedNoteBlocks::single_text_block(&input.initial_text, now);
        let raw = loaded.into_raw()?;
        self.blocks.save_document(created.id, &raw).await?;
        Ok(created)
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

    async fn list_blocks(
        &self,
        user_id: Uuid,
        note_id: Uuid,
    ) -> Result<Vec<Block<()>>, NoteError> {
        self.ensure_note_owner(user_id, note_id).await?;
        let loaded = self.load_blocks(note_id).await?;
        Ok(loaded.ordered_blocks().into_iter().cloned().collect())
    }

    async fn create_block(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        after_id: Option<Uuid>,
        content: Content,
    ) -> Result<Block<()>, NoteError> {
        let note = self.ensure_note_owner(user_id, note_id).await?;
        let now = Utc::now();
        let mut loaded = self.load_blocks(note_id).await?;
        let id = loaded.insert_block_after(after_id, content, now)?;
        let created = loaded
            .blocks
            .get(&id)
            .cloned()
            .ok_or(NoteError::Internal)?;
        self.save_blocks(note_id, loaded).await?;
        self.touch_note(note).await?;
        Ok(created)
    }

    async fn delete_block(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        block_id: Uuid,
    ) -> Result<(), NoteError> {
        let note = self.ensure_note_owner(user_id, note_id).await?;
        let now = Utc::now();
        let mut loaded = self.load_blocks(note_id).await?;
        loaded.delete_block(block_id, now)?;
        self.save_blocks(note_id, loaded).await?;
        self.touch_note(note).await?;
        Ok(())
    }

    async fn move_block(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        block_id: Uuid,
        after_id: Option<Uuid>,
    ) -> Result<(), NoteError> {
        let note = self.ensure_note_owner(user_id, note_id).await?;
        let now = Utc::now();
        let mut loaded = self.load_blocks(note_id).await?;
        loaded.move_block_after(block_id, after_id, now)?;
        self.save_blocks(note_id, loaded).await?;
        self.touch_note(note).await?;
        Ok(())
    }

    async fn apply_text_patch(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        block_id: Uuid,
        patch: TextPatch,
    ) -> Result<(), NoteError> {
        let note = self.ensure_note_owner(user_id, note_id).await?;
        let now = Utc::now();
        let mut loaded = self.load_blocks(note_id).await?;
        let tb = loaded.text_mut(block_id, now)?;
        match patch {
            TextPatch::InsertText {
                position,
                text,
                style,
            } => tb.insert_text(position, &text, style),
            TextPatch::DeleteRange { start, end } => tb.delete_range(start, end),
            TextPatch::DeleteAt {
                position,
                direction,
            } => tb.delete_at(position, direction),
            TextPatch::EnableFormatting {
                start,
                end,
                style,
            } => tb.enable_formatting(start, end, style),
            TextPatch::DisableFormatting {
                start,
                end,
                style,
            } => tb.disable_formatting(start, end, style),
        }
        self.save_blocks(note_id, loaded).await?;
        self.touch_note(note).await?;
        Ok(())
    }
}
