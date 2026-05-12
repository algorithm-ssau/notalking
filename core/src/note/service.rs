use std::sync::Arc;

use chrono::Utc;
use editor::block::Block;
use editor::content::Content;
use uuid::Uuid;

use crate::note::{
    CreateNoteInput, DeleteNoteInput, Note, NoteBodyUpdate, NoteError, NoteUsecase, TextPatch,
    UpdateNoteInput, blocks::LoadedNoteBlocks,
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

    async fn persist_loaded(
        &self,
        mut note: Note,
        loaded: LoadedNoteBlocks,
    ) -> Result<(), NoteError> {
        note.head_id = loaded.head_id;
        note.updated_at = Utc::now();
        let raw = loaded.into_raw()?;
        self.blocks.save_document(note.id, &raw).await?;
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
        let loaded = LoadedNoteBlocks::single_text_block(&input.initial_text, now);
        let head_id = loaded.head_id;
        let note = Note {
            id: Uuid::new_v4(),
            user_id: input.user_id,
            title: input.title,
            head_id,
            created_at: now,
            updated_at: now,
        };

        let created = self.repo.create(note).await?;
        self.persist_loaded(created.clone(), loaded).await?;
        Ok(created)
    }

    async fn list_notes(&self, user_id: Uuid) -> Result<Vec<Note>, NoteError> {
        self.repo.list_by_user(user_id).await
    }

    async fn update_note(&self, input: UpdateNoteInput) -> Result<Note, NoteError> {
        let mut note = self.ensure_note_owner(input.user_id, input.note_id).await?;

        if input.title.is_none() && input.body.is_none() {
            return Err(NoteError::InvalidInput);
        }

        if let Some(title) = input.title {
            let normalized = title.trim().to_owned();
            if normalized.is_empty() {
                return Err(NoteError::InvalidInput);
            }
            note.title = normalized;
        }

        if let Some(body) = input.body {
            let now = Utc::now();
            let text = match body {
                NoteBodyUpdate::ReplacePlainText { text } => text,
                NoteBodyUpdate::AppendPlainText { text } => {
                    let current = self.load_blocks(input.note_id).await?.plain_text();
                    if current.trim().is_empty() {
                        text
                    } else if text.trim().is_empty() {
                        current
                    } else {
                        format!("{current}\n\n{text}")
                    }
                }
            };
            let loaded = LoadedNoteBlocks::single_text_block(&text, now);
            note.head_id = loaded.head_id;
            note.updated_at = now;
            let raw = loaded.into_raw()?;
            self.blocks.save_document(note.id, &raw).await?;
            self.repo.update(note.clone()).await?;
            return Ok(note);
        }

        note.updated_at = Utc::now();
        self.repo.update(note.clone()).await?;
        Ok(note)
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

    async fn list_blocks(&self, user_id: Uuid, note_id: Uuid) -> Result<Vec<Block<()>>, NoteError> {
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
        let created = loaded.blocks.get(&id).cloned().ok_or(NoteError::Internal)?;
        self.persist_loaded(note, loaded).await?;
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
        self.persist_loaded(note, loaded).await?;
        Ok(())
    }

    async fn move_block(
        &self,
        user_id: Uuid,
        note_id: Uuid,
        block_id: Uuid,
        after_id: Option<Uuid>,
        before_id: Option<Uuid>,
    ) -> Result<(), NoteError> {
        let note = self.ensure_note_owner(user_id, note_id).await?;
        let now = Utc::now();
        let mut loaded = self.load_blocks(note_id).await?;
        match (after_id, before_id) {
            (Some(_), Some(_)) => return Err(NoteError::InvalidInput),
            (_, Some(before)) => loaded.move_block_before(block_id, before, now)?,
            (after, None) => loaded.move_block_after(block_id, after, now)?,
        }
        self.persist_loaded(note, loaded).await?;
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
            TextPatch::EnableFormatting { start, end, style } => {
                tb.enable_formatting(start, end, style)
            }
            TextPatch::DisableFormatting { start, end, style } => {
                tb.disable_formatting(start, end, style)
            }
        }
        self.persist_loaded(note, loaded).await?;
        Ok(())
    }
}
