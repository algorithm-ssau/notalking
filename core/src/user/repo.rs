#![allow(dead_code)]

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::user::User;

pub struct CreateUser {
    pub login: String,
    pub password_hash: String,
}

pub struct UpdateUser {
    pub login: Option<String>,
    pub password_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

pub enum UserRepoError {
    NotFound,
    LoginAlreadyTaken,
    Internal,
}

pub trait Repo {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserRepoError>;
    async fn find_by_login(&self, login: &str) -> Result<Option<User>, UserRepoError>;
    async fn create(&self, user: CreateUser) -> Result<User, UserRepoError>;
    async fn update(&self, id: Uuid, patch: UpdateUser) -> Result<User, UserRepoError>;
    async fn delete(&self, id: Uuid) -> Result<(), UserRepoError>;
}
