use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::{AuthError, Session, SessionRepo};

use crate::db::Db;

#[derive(Clone)]
pub struct SqlSessionRepo {
    db: Arc<Db>,
}

impl SqlSessionRepo {
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    fn map_err(_e: sqlx::Error) -> AuthError {
        AuthError::Internal
    }

    fn row_session_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<Session, AuthError> {
        let id_s: String = row.try_get("id").map_err(|_| AuthError::Internal)?;
        let uid_s: String = row.try_get("user_id").map_err(|_| AuthError::Internal)?;
        Ok(Session {
            id: Uuid::parse_str(&id_s).map_err(|_| AuthError::Internal)?,
            user_id: Uuid::parse_str(&uid_s).map_err(|_| AuthError::Internal)?,
            device: row.try_get("device").unwrap_or_default(),
            location: row.try_get("location").unwrap_or_default(),
            issued_at: row.try_get("issued_at").map_err(|_| AuthError::Internal)?,
            expires_at: row.try_get("expires_at").map_err(|_| AuthError::Internal)?,
            updated_at: row.try_get("updated_at").map_err(|_| AuthError::Internal)?,
            revoked_at: row.try_get("revoked_at").ok(),
        })
    }

    fn row_session_pg(row: &sqlx::postgres::PgRow) -> Result<Session, AuthError> {
        Ok(Session {
            id: row.try_get("id").map_err(|_| AuthError::Internal)?,
            user_id: row.try_get("user_id").map_err(|_| AuthError::Internal)?,
            device: row.try_get("device").unwrap_or_default(),
            location: row.try_get("location").unwrap_or_default(),
            issued_at: row.try_get("issued_at").map_err(|_| AuthError::Internal)?,
            expires_at: row.try_get("expires_at").map_err(|_| AuthError::Internal)?,
            updated_at: row.try_get("updated_at").map_err(|_| AuthError::Internal)?,
            revoked_at: row.try_get("revoked_at").ok(),
        })
    }
}

impl SessionRepo for SqlSessionRepo {
    async fn create_session(
        &self,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
        device: &str,
        location: &str,
    ) -> Result<Session, AuthError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        match &*self.db {
            Db::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO sessions (id, user_id, device, location, issued_at, expires_at, updated_at, revoked_at) VALUES (?, ?, ?, ?, ?, ?, ?, NULL)",
                )
                .bind(id.to_string())
                .bind(user_id.to_string())
                .bind(device)
                .bind(location)
                .bind(now)
                .bind(expires_at)
                .bind(now)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
            Db::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO sessions (id, user_id, device, location, issued_at, expires_at, updated_at, revoked_at) VALUES ($1, $2, $3, $4, $5, $6, $7, NULL)",
                )
                .bind(id)
                .bind(user_id)
                .bind(device)
                .bind(location)
                .bind(now)
                .bind(expires_at)
                .bind(now)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
        }
        Ok(Session {
            id,
            user_id,
            device: device.to_owned(),
            location: location.to_owned(),
            issued_at: now,
            expires_at,
            updated_at: now,
            revoked_at: None,
        })
    }

    async fn find_session(&self, session_id: Uuid) -> Result<Option<Session>, AuthError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, user_id, device, location, issued_at, expires_at, updated_at, revoked_at FROM sessions WHERE id = ?",
                )
                .bind(session_id.to_string())
                .fetch_optional(pool)
                .await
                .map_err(Self::map_err)?;
                row.map(|r| Self::row_session_sqlite(&r)).transpose()
            }
            Db::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, user_id, device, location, issued_at, expires_at, updated_at, revoked_at FROM sessions WHERE id = $1",
                )
                .bind(session_id)
                .fetch_optional(pool)
                .await
                .map_err(Self::map_err)?;
                row.map(|r| Self::row_session_pg(&r)).transpose()
            }
        }
    }

    async fn extend_session(
        &self,
        session_id: Uuid,
        new_expires_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<(), AuthError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE sessions SET expires_at = ?, updated_at = ? WHERE id = ? AND revoked_at IS NULL",
                )
                .bind(new_expires_at)
                .bind(updated_at)
                .bind(session_id.to_string())
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
            Db::Postgres(pool) => {
                sqlx::query(
                    "UPDATE sessions SET expires_at = $1, updated_at = $2 WHERE id = $3 AND revoked_at IS NULL",
                )
                .bind(new_expires_at)
                .bind(updated_at)
                .bind(session_id)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
        }
        Ok(())
    }

    async fn revoke_session(&self, session_id: Uuid, revoked_at: DateTime<Utc>) -> Result<(), AuthError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let r = sqlx::query("UPDATE sessions SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
                    .bind(revoked_at)
                    .bind(session_id.to_string())
                    .execute(pool)
                    .await
                    .map_err(Self::map_err)?;
                if r.rows_affected() == 0 {
                    return Err(AuthError::SessionNotFound);
                }
            }
            Db::Postgres(pool) => {
                let r = sqlx::query("UPDATE sessions SET revoked_at = $1 WHERE id = $2 AND revoked_at IS NULL")
                    .bind(revoked_at)
                    .bind(session_id)
                    .execute(pool)
                    .await
                    .map_err(Self::map_err)?;
                if r.rows_affected() == 0 {
                    return Err(AuthError::SessionNotFound);
                }
            }
        }
        Ok(())
    }

    async fn list_sessions_by_user(&self, user_id: Uuid) -> Result<Vec<Session>, AuthError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, user_id, device, location, issued_at, expires_at, updated_at, revoked_at FROM sessions WHERE user_id = ? ORDER BY issued_at DESC",
                )
                .bind(user_id.to_string())
                .fetch_all(pool)
                .await
                .map_err(Self::map_err)?;
                rows.iter().map(|r| Self::row_session_sqlite(r)).collect()
            }
            Db::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, user_id, device, location, issued_at, expires_at, updated_at, revoked_at FROM sessions WHERE user_id = $1 ORDER BY issued_at DESC",
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
                .map_err(Self::map_err)?;
                rows.iter().map(|r| Self::row_session_pg(r)).collect()
            }
        }
    }

    async fn revoke_session_for_user(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<(), AuthError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let r = sqlx::query(
                    "UPDATE sessions SET revoked_at = ? WHERE id = ? AND user_id = ? AND revoked_at IS NULL",
                )
                .bind(revoked_at)
                .bind(session_id.to_string())
                .bind(user_id.to_string())
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
                if r.rows_affected() == 0 {
                    return Err(AuthError::SessionNotFound);
                }
            }
            Db::Postgres(pool) => {
                let r = sqlx::query(
                    "UPDATE sessions SET revoked_at = $1 WHERE id = $2 AND user_id = $3 AND revoked_at IS NULL",
                )
                .bind(revoked_at)
                .bind(session_id)
                .bind(user_id)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
                if r.rows_affected() == 0 {
                    return Err(AuthError::SessionNotFound);
                }
            }
        }
        Ok(())
    }

    async fn revoke_sessions_for_user_except(
        &self,
        user_id: Uuid,
        keep_session_id: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<u64, AuthError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let r = sqlx::query(
                    "UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND id != ? AND revoked_at IS NULL",
                )
                .bind(revoked_at)
                .bind(user_id.to_string())
                .bind(keep_session_id.to_string())
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
                Ok(r.rows_affected())
            }
            Db::Postgres(pool) => {
                let r = sqlx::query(
                    "UPDATE sessions SET revoked_at = $1 WHERE user_id = $2 AND id != $3 AND revoked_at IS NULL",
                )
                .bind(revoked_at)
                .bind(user_id)
                .bind(keep_session_id)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
                Ok(r.rows_affected())
            }
        }
    }
}
