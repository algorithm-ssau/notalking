use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::Db;
use crate::note::block_repo::{BlockRepository, RawBlockDocument};
use crate::note::repo::Repo;
use crate::note::{Note, NoteError};

#[derive(Clone)]
pub struct SqlNoteStore {
    db: Arc<Db>,
}

impl SqlNoteStore {
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    fn map_err(_e: sqlx::Error) -> NoteError {
        NoteError::Internal
    }

    fn row_note_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<Note, NoteError> {
        let id_s: String = row.try_get("id").map_err(|_| NoteError::Internal)?;
        let uid_s: String = row.try_get("user_id").map_err(|_| NoteError::Internal)?;
        let head: Option<String> = row.try_get("head_id").ok();
        Ok(Note {
            id: Uuid::parse_str(&id_s).map_err(|_| NoteError::Internal)?,
            user_id: Uuid::parse_str(&uid_s).map_err(|_| NoteError::Internal)?,
            title: row.try_get("title").map_err(|_| NoteError::Internal)?,
            head_id: head.and_then(|s| Uuid::parse_str(&s).ok()),
            created_at: row.try_get("created_at").map_err(|_| NoteError::Internal)?,
            updated_at: row.try_get("updated_at").map_err(|_| NoteError::Internal)?,
        })
    }

    fn row_note_pg(row: &sqlx::postgres::PgRow) -> Result<Note, NoteError> {
        Ok(Note {
            id: row.try_get("id").map_err(|_| NoteError::Internal)?,
            user_id: row.try_get("user_id").map_err(|_| NoteError::Internal)?,
            title: row.try_get("title").map_err(|_| NoteError::Internal)?,
            head_id: row.try_get("head_id").ok(),
            created_at: row.try_get("created_at").map_err(|_| NoteError::Internal)?,
            updated_at: row.try_get("updated_at").map_err(|_| NoteError::Internal)?,
        })
    }
}

impl Repo for SqlNoteStore {
    async fn create(&self, note: Note) -> Result<Note, NoteError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO notes (id, user_id, title, head_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(note.id.to_string())
                .bind(note.user_id.to_string())
                .bind(&note.title)
                .bind(note.head_id.map(|h| h.to_string()))
                .bind(note.created_at)
                .bind(note.updated_at)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
            Db::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO notes (id, user_id, title, head_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(note.id)
                .bind(note.user_id)
                .bind(&note.title)
                .bind(note.head_id)
                .bind(note.created_at)
                .bind(note.updated_at)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
        }
        Ok(note)
    }

    async fn update(&self, note: Note) -> Result<(), NoteError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let r = sqlx::query(
                    "UPDATE notes SET title = ?, head_id = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&note.title)
                .bind(note.head_id.map(|h| h.to_string()))
                .bind(note.updated_at)
                .bind(note.id.to_string())
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
                if r.rows_affected() == 0 {
                    return Err(NoteError::NotFound);
                }
            }
            Db::Postgres(pool) => {
                let r = sqlx::query(
                    "UPDATE notes SET title = $1, head_id = $2, updated_at = $3 WHERE id = $4",
                )
                .bind(&note.title)
                .bind(note.head_id)
                .bind(note.updated_at)
                .bind(note.id)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
                if r.rows_affected() == 0 {
                    return Err(NoteError::NotFound);
                }
            }
        }
        Ok(())
    }

    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Note>, NoteError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, user_id, title, head_id, created_at, updated_at FROM notes WHERE user_id = ? ORDER BY updated_at DESC",
                )
                .bind(user_id.to_string())
                .fetch_all(pool)
                .await
                .map_err(Self::map_err)?;
                rows.iter().map(|r| Self::row_note_sqlite(r)).collect()
            }
            Db::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, user_id, title, head_id, created_at, updated_at FROM notes WHERE user_id = $1 ORDER BY updated_at DESC",
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
                .map_err(Self::map_err)?;
                rows.iter().map(|r| Self::row_note_pg(r)).collect()
            }
        }
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Note>, NoteError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, user_id, title, head_id, created_at, updated_at FROM notes WHERE id = ?",
                )
                .bind(id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(Self::map_err)?;
                row.map(|r| Self::row_note_sqlite(&r)).transpose()
            }
            Db::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, user_id, title, head_id, created_at, updated_at FROM notes WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(Self::map_err)?;
                row.map(|r| Self::row_note_pg(&r)).transpose()
            }
        }
    }

    async fn delete_by_id(&self, id: Uuid) -> Result<(), NoteError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let r = sqlx::query("DELETE FROM notes WHERE id = ?")
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(Self::map_err)?;
                if r.rows_affected() == 0 {
                    return Err(NoteError::NotFound);
                }
            }
            Db::Postgres(pool) => {
                let r = sqlx::query("DELETE FROM notes WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(Self::map_err)?;
                if r.rows_affected() == 0 {
                    return Err(NoteError::NotFound);
                }
            }
        }
        Ok(())
    }
}

impl BlockRepository for SqlNoteStore {
    async fn load_document(&self, note_id: Uuid) -> Result<Option<RawBlockDocument>, NoteError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let row: Option<(String,)> =
                    sqlx::query_as("SELECT payload FROM note_documents WHERE note_id = ?")
                        .bind(note_id.to_string())
                        .fetch_optional(pool)
                        .await
                        .map_err(Self::map_err)?;
                row.map(|(payload,)| RawBlockDocument::from_json_str(&payload))
                    .transpose()
            }
            Db::Postgres(pool) => {
                let row: Option<(String,)> =
                    sqlx::query_as("SELECT payload FROM note_documents WHERE note_id = $1")
                        .bind(note_id)
                        .fetch_optional(pool)
                        .await
                        .map_err(Self::map_err)?;
                row.map(|(payload,)| RawBlockDocument::from_json_str(&payload))
                    .transpose()
            }
        }
    }

    async fn save_document(&self, note_id: Uuid, doc: &RawBlockDocument) -> Result<(), NoteError> {
        let payload = doc.to_json_string()?;
        match &*self.db {
            Db::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO note_documents (note_id, payload) VALUES (?, ?) ON CONFLICT(note_id) DO UPDATE SET payload = excluded.payload",
                )
                .bind(note_id.to_string())
                .bind(&payload)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
            Db::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO note_documents (note_id, payload) VALUES ($1, $2) ON CONFLICT(note_id) DO UPDATE SET payload = EXCLUDED.payload",
                )
                .bind(note_id)
                .bind(&payload)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
        }
        Ok(())
    }

    async fn delete_document(&self, note_id: Uuid) -> Result<(), NoteError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                sqlx::query("DELETE FROM note_documents WHERE note_id = ?")
                    .bind(note_id.to_string())
                    .execute(pool)
                    .await
                    .map_err(Self::map_err)?;
            }
            Db::Postgres(pool) => {
                sqlx::query("DELETE FROM note_documents WHERE note_id = $1")
                    .bind(note_id)
                    .execute(pool)
                    .await
                    .map_err(Self::map_err)?;
            }
        }
        Ok(())
    }
}
