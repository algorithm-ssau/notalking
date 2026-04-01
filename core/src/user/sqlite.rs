use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::user::User;
use crate::user::repo::{CreateUser, Repo, UpdateUser, UserRepoError};

pub struct SqliteRepo {
    pool: SqlitePool,
}

impl SqliteRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn map_sqlx_error(error: sqlx::Error) -> UserRepoError {
        if let sqlx::Error::Database(db_error) = &error {
            let message = db_error.message();
            if message.contains("UNIQUE constraint failed: users.login") {
                return UserRepoError::LoginAlreadyTaken;
            }
        }

        UserRepoError::Internal
    }

    fn map_row_to_user(row: sqlx::sqlite::SqliteRow) -> Result<User, UserRepoError> {
        let id_raw: String = row.try_get("id").map_err(|_| UserRepoError::Internal)?;
        let id = Uuid::parse_str(&id_raw).map_err(|_| UserRepoError::Internal)?;
        let login: String = row.try_get("login").map_err(|_| UserRepoError::Internal)?;
        let password_hash: String = row
            .try_get("password_hash")
            .map_err(|_| UserRepoError::Internal)?;
        let created_at: DateTime<Utc> = row
            .try_get("created_at")
            .map_err(|_| UserRepoError::Internal)?;
        let updated_at: DateTime<Utc> = row
            .try_get("updated_at")
            .map_err(|_| UserRepoError::Internal)?;

        Ok(User {
            id,
            login,
            password_hash,
            created_at,
            updated_at,
        })
    }
}

impl Repo for SqliteRepo {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserRepoError> {
        let row = sqlx::query(
            "SELECT id, login, password_hash, created_at, updated_at
             FROM users
             WHERE id = ?
             LIMIT 1",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(Self::map_row_to_user).transpose()
    }

    async fn find_by_login(&self, login: &str) -> Result<Option<User>, UserRepoError> {
        let row = sqlx::query(
            "SELECT id, login, password_hash, created_at, updated_at
             FROM users
             WHERE login = ?
             LIMIT 1",
        )
        .bind(login)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(Self::map_row_to_user).transpose()
    }

    async fn create(&self, user: CreateUser) -> Result<User, UserRepoError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let mut tx = self.pool.begin().await.map_err(Self::map_sqlx_error)?;

        sqlx::query(
            "INSERT INTO users (id, login, password_hash, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(&user.login)
        .bind(&user.password_hash)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        tx.commit().await.map_err(Self::map_sqlx_error)?;

        Ok(User {
            id,
            login: user.login,
            password_hash: user.password_hash,
            created_at: now,
            updated_at: now,
        })
    }

    async fn update(&self, id: Uuid, patch: UpdateUser) -> Result<User, UserRepoError> {
        let mut tx = self.pool.begin().await.map_err(Self::map_sqlx_error)?;

        let result = sqlx::query(
            "UPDATE users
             SET login = COALESCE(?, login),
                 password_hash = COALESCE(?, password_hash),
                 updated_at = ?
             WHERE id = ?",
        )
        .bind(&patch.login)
        .bind(&patch.password_hash)
        .bind(patch.updated_at)
        .bind(id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(UserRepoError::NotFound);
        }

        let row = sqlx::query(
            "SELECT id, login, password_hash, created_at, updated_at
             FROM users
             WHERE id = ?
             LIMIT 1",
        )
        .bind(id.to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        tx.commit().await.map_err(Self::map_sqlx_error)?;

        Self::map_row_to_user(row)
    }

    async fn delete(&self, id: Uuid) -> Result<(), UserRepoError> {
        let mut tx = self.pool.begin().await.map_err(Self::map_sqlx_error)?;

        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(UserRepoError::NotFound);
        }

        tx.commit().await.map_err(Self::map_sqlx_error)?;
        Ok(())
    }
}

impl crate::auth::AuthUserReader for SqliteRepo {
    async fn find_by_login(&self, login: &str) -> Result<Option<User>, crate::auth::AuthError> {
        Repo::find_by_login(self, login)
            .await
            .map_err(|error| match error {
                UserRepoError::Internal => crate::auth::AuthError::Internal,
                UserRepoError::NotFound | UserRepoError::LoginAlreadyTaken => {
                    crate::auth::AuthError::UserNotFound
                }
            })
    }
}
