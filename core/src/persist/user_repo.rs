use std::sync::Arc;

use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

use crate::db::Db;
use crate::user::User;
use crate::user::repo::{CreateUser, Repo, UpdateUser, UserRepoError};

#[derive(Clone)]
pub struct SqlUserRepo {
    db: Arc<Db>,
}

impl SqlUserRepo {
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    fn map_err(e: sqlx::Error) -> UserRepoError {
        if let sqlx::Error::Database(ref db) = e {
            let msg = db.message();
            if msg.contains("unique") || msg.contains("UNIQUE") || msg.contains("duplicate key") {
                return UserRepoError::LoginAlreadyTaken;
            }
        }
        UserRepoError::Internal
    }

    fn row_to_user_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<User, UserRepoError> {
        let id_s: String = row.try_get("id").map_err(|_| UserRepoError::Internal)?;
        let id = Uuid::parse_str(&id_s).map_err(|_| UserRepoError::Internal)?;
        Ok(User {
            id,
            login: row.try_get("login").map_err(|_| UserRepoError::Internal)?,
            password_hash: row
                .try_get("password_hash")
                .map_err(|_| UserRepoError::Internal)?,
            created_at: row
                .try_get("created_at")
                .map_err(|_| UserRepoError::Internal)?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|_| UserRepoError::Internal)?,
        })
    }

    fn row_to_user_pg(row: &sqlx::postgres::PgRow) -> Result<User, UserRepoError> {
        Ok(User {
            id: row.try_get("id").map_err(|_| UserRepoError::Internal)?,
            login: row.try_get("login").map_err(|_| UserRepoError::Internal)?,
            password_hash: row
                .try_get("password_hash")
                .map_err(|_| UserRepoError::Internal)?,
            created_at: row
                .try_get("created_at")
                .map_err(|_| UserRepoError::Internal)?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|_| UserRepoError::Internal)?,
        })
    }
}

impl Repo for SqlUserRepo {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserRepoError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let row = sqlx::query("SELECT id, login, password_hash, created_at, updated_at FROM users WHERE id = ?")
                    .bind(id.to_string())
                    .fetch_optional(pool)
                    .await
                    .map_err(Self::map_err)?;
                row.map(|r| Self::row_to_user_sqlite(&r)).transpose()
            }
            Db::Postgres(pool) => {
                let row = sqlx::query("SELECT id, login, password_hash, created_at, updated_at FROM users WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(Self::map_err)?;
                row.map(|r| Self::row_to_user_pg(&r)).transpose()
            }
        }
    }

    async fn find_by_login(&self, login: &str) -> Result<Option<User>, UserRepoError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, login, password_hash, created_at, updated_at FROM users WHERE login = ?",
                )
                .bind(login)
                .fetch_optional(pool)
                .await
                .map_err(Self::map_err)?;
                row.map(|r| Self::row_to_user_sqlite(&r)).transpose()
            }
            Db::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, login, password_hash, created_at, updated_at FROM users WHERE login = $1",
                )
                .bind(login)
                .fetch_optional(pool)
                .await
                .map_err(Self::map_err)?;
                row.map(|r| Self::row_to_user_pg(&r)).transpose()
            }
        }
    }

    async fn create(&self, user: CreateUser) -> Result<User, UserRepoError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let row_user = User {
            id,
            login: user.login.clone(),
            password_hash: user.password_hash,
            created_at: now,
            updated_at: now,
        };
        match &*self.db {
            Db::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO users (id, login, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                )
                .bind(id.to_string())
                .bind(&row_user.login)
                .bind(&row_user.password_hash)
                .bind(row_user.created_at)
                .bind(row_user.updated_at)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
            Db::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO users (id, login, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(id)
                .bind(&row_user.login)
                .bind(&row_user.password_hash)
                .bind(row_user.created_at)
                .bind(row_user.updated_at)
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
        }
        Ok(row_user)
    }

    async fn update(&self, id: Uuid, patch: UpdateUser) -> Result<User, UserRepoError> {
        let existing = self.find_by_id(id).await?.ok_or(UserRepoError::NotFound)?;
        let login = patch.login.unwrap_or(existing.login);
        let password_hash = patch.password_hash.unwrap_or(existing.password_hash);
        let updated_at = patch.updated_at;
        match &*self.db {
            Db::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE users SET login = ?, password_hash = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&login)
                .bind(&password_hash)
                .bind(updated_at)
                .bind(id.to_string())
                .execute(pool)
                .await
                .map_err(Self::map_err)?;
            }
            Db::Postgres(pool) => {
                sqlx::query("UPDATE users SET login = $1, password_hash = $2, updated_at = $3 WHERE id = $4")
                    .bind(&login)
                    .bind(&password_hash)
                    .bind(updated_at)
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(Self::map_err)?;
            }
        }
        Ok(User {
            id,
            login,
            password_hash,
            created_at: existing.created_at,
            updated_at,
        })
    }

    async fn delete(&self, id: Uuid) -> Result<(), UserRepoError> {
        match &*self.db {
            Db::Sqlite(pool) => {
                let r = sqlx::query("DELETE FROM users WHERE id = ?")
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(Self::map_err)?;
                if r.rows_affected() == 0 {
                    return Err(UserRepoError::NotFound);
                }
            }
            Db::Postgres(pool) => {
                let r = sqlx::query("DELETE FROM users WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(Self::map_err)?;
                if r.rows_affected() == 0 {
                    return Err(UserRepoError::NotFound);
                }
            }
        }
        Ok(())
    }
}
