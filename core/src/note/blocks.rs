use std::collections::HashMap;

use chrono::{DateTime, Utc};
use editor::block::Block;
use editor::content::Content;
use editor::text::TextBlock;
use uuid::Uuid;

use super::NoteError;
use super::block_repo::RawBlockDocument;

/// In-memory view of a note's blocks after deserialization.
#[derive(Debug, Clone)]
pub struct LoadedNoteBlocks {
    pub head_id: Option<Uuid>,
    pub blocks: HashMap<Uuid, Block<()>>,
}

impl LoadedNoteBlocks {
    pub fn empty() -> Self {
        Self {
            head_id: None,
            blocks: HashMap::new(),
        }
    }

    pub fn single_text_block(text: &str, now: DateTime<Utc>) -> Self {
        let id = Uuid::new_v4();
        let mut blocks = HashMap::new();
        blocks.insert(
            id,
            Block {
                id,
                prev_id: None,
                next_id: None,
                content: Content::Text(TextBlock::from_text(text)),
                metadata: (),
                created_at: now,
                updated_at: now,
            },
        );
        Self {
            head_id: Some(id),
            blocks,
        }
    }

    pub fn from_raw(raw: RawBlockDocument) -> Result<Self, NoteError> {
        let mut blocks = HashMap::with_capacity(raw.blocks.len());
        for (id, bytes) in raw.blocks {
            let block: Block<()> = serde_json::from_slice(&bytes).map_err(|_| NoteError::CorruptBlocks)?;
            if block.id != id {
                return Err(NoteError::CorruptBlocks);
            }
            blocks.insert(id, block);
        }
        Ok(Self {
            head_id: raw.head_id,
            blocks,
        })
    }

    pub fn into_raw(self) -> Result<RawBlockDocument, NoteError> {
        let mut out = RawBlockDocument {
            head_id: self.head_id,
            blocks: HashMap::with_capacity(self.blocks.len()),
        };
        for (id, block) in self.blocks {
            let bytes = serde_json::to_vec(&block).map_err(|_| NoteError::Internal)?;
            out.blocks.insert(id, bytes);
        }
        Ok(out)
    }

    pub fn ordered_blocks(&self) -> Vec<&Block<()>> {
        let mut out = Vec::new();
        let mut cur = self.head_id;
        let mut guard = 0usize;
        let max = self.blocks.len().saturating_add(1);
        while let Some(id) = cur {
            guard += 1;
            if guard > max {
                break;
            }
            let Some(b) = self.blocks.get(&id) else { break };
            out.push(b);
            cur = b.next_id;
        }
        out
    }

    pub fn find_tail(&self) -> Option<Uuid> {
        let mut cur = self.head_id?;
        loop {
            let b = self.blocks.get(&cur)?;
            match b.next_id {
                None => return Some(cur),
                Some(n) => cur = n,
            }
        }
    }

    pub fn insert_block_after(
        &mut self,
        after_id: Option<Uuid>,
        content: Content,
        now: DateTime<Utc>,
    ) -> Result<Uuid, NoteError> {
        let id = Uuid::new_v4();
        let mut new_block = Block {
            id,
            prev_id: None,
            next_id: None,
            content,
            metadata: (),
            created_at: now,
            updated_at: now,
        };

        match after_id {
            None => {
                if let Some(tail_id) = self.find_tail() {
                    new_block.prev_id = Some(tail_id);
                    if let Some(tail) = self.blocks.get_mut(&tail_id) {
                        tail.next_id = Some(id);
                        tail.updated_at = now;
                    }
                } else {
                    self.head_id = Some(id);
                }
                self.blocks.insert(id, new_block);
            }
            Some(after) => {
                let after_block = self.blocks.get(&after).ok_or(NoteError::BlockNotFound)?;
                let after_next = after_block.next_id;
                new_block.prev_id = Some(after);
                new_block.next_id = after_next;
                if let Some(nid) = after_next {
                    if let Some(n) = self.blocks.get_mut(&nid) {
                        n.prev_id = Some(id);
                        n.updated_at = now;
                    }
                }
                if let Some(a) = self.blocks.get_mut(&after) {
                    a.next_id = Some(id);
                    a.updated_at = now;
                }
                self.blocks.insert(id, new_block);
            }
        }
        Ok(id)
    }

    pub fn delete_block(&mut self, block_id: Uuid, now: DateTime<Utc>) -> Result<(), NoteError> {
        let block = self.blocks.get(&block_id).ok_or(NoteError::BlockNotFound)?.clone();
        let prev = block.prev_id;
        let next = block.next_id;

        if Some(block_id) == self.head_id {
            self.head_id = next;
        }
        if let Some(pid) = prev {
            if let Some(p) = self.blocks.get_mut(&pid) {
                p.next_id = next;
                p.updated_at = now;
            }
        }
        if let Some(nid) = next {
            if let Some(n) = self.blocks.get_mut(&nid) {
                n.prev_id = prev;
                n.updated_at = now;
            }
        }
        self.blocks.remove(&block_id);
        Ok(())
    }

    /// Moves `block_id` so it sits immediately after `after_id`.
    /// `after_id: None` appends to the tail.
    pub fn move_block_after(
        &mut self,
        block_id: Uuid,
        after_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), NoteError> {
        if !self.blocks.contains_key(&block_id) {
            return Err(NoteError::BlockNotFound);
        }

        if let Some(after) = after_id {
            if after == block_id {
                return Err(NoteError::InvalidOperation);
            }
            if !self.blocks.contains_key(&after) {
                return Err(NoteError::BlockNotFound);
            }
        }

        // Unlink from current position
        let (prev, next) = {
            let b = self.blocks.get(&block_id).ok_or(NoteError::BlockNotFound)?;
            (b.prev_id, b.next_id)
        };

        if Some(block_id) == self.head_id {
            self.head_id = next;
        }
        if let Some(pid) = prev {
            if let Some(p) = self.blocks.get_mut(&pid) {
                p.next_id = next;
                p.updated_at = now;
            }
        }
        if let Some(nid) = next {
            if let Some(n) = self.blocks.get_mut(&nid) {
                n.prev_id = prev;
                n.updated_at = now;
            }
        }

        {
            let b = self.blocks.get_mut(&block_id).ok_or(NoteError::BlockNotFound)?;
            b.prev_id = None;
            b.next_id = None;
            b.updated_at = now;
        }

        match after_id {
            None => {
                if let Some(tail_id) = self.find_tail() {
                    if tail_id == block_id {
                        return Ok(());
                    }
                    let b = self.blocks.get_mut(&block_id).ok_or(NoteError::BlockNotFound)?;
                    b.prev_id = Some(tail_id);
                    b.updated_at = now;
                    if let Some(t) = self.blocks.get_mut(&tail_id) {
                        t.next_id = Some(block_id);
                        t.updated_at = now;
                    }
                } else {
                    self.head_id = Some(block_id);
                }
            }
            Some(after) => {
                let after_next = self.blocks.get(&after).ok_or(NoteError::BlockNotFound)?.next_id;
                if after_next == Some(block_id) {
                    return Ok(());
                }
                {
                    let b = self.blocks.get_mut(&block_id).ok_or(NoteError::BlockNotFound)?;
                    b.prev_id = Some(after);
                    b.next_id = after_next;
                    b.updated_at = now;
                }
                if let Some(nid) = after_next {
                    if let Some(n) = self.blocks.get_mut(&nid) {
                        n.prev_id = Some(block_id);
                        n.updated_at = now;
                    }
                }
                if let Some(a) = self.blocks.get_mut(&after) {
                    a.next_id = Some(block_id);
                    a.updated_at = now;
                }
            }
        }
        Ok(())
    }

    pub fn text_mut(&mut self, block_id: Uuid, now: DateTime<Utc>) -> Result<&mut TextBlock, NoteError> {
        let block = self.blocks.get_mut(&block_id).ok_or(NoteError::BlockNotFound)?;
        block.updated_at = now;
        match &mut block.content {
            Content::Text(tb) => Ok(tb),
        }
    }
}
