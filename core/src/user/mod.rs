#![allow(dead_code)]

use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod repo;
pub mod in_memory;
pub mod trace;
pub mod sqlite;

#[derive(Clone)]
pub struct User {
    pub id: Uuid,
    pub login: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
