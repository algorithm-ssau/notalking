#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::user::User;

use super::repo::{CreateUser, Repo, UpdateUser, UserRepoError};

#[derive(Clone, Default)]
pub struct InMemoryRepo {
    users: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl InMemoryRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Repo for InMemoryRepo {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserRepoError> {
        let users = self.users.read().await;
        Ok(users.get(&id).cloned())
    }

    async fn find_by_login(&self, login: &str) -> Result<Option<User>, UserRepoError> {
        let users = self.users.read().await;
        Ok(users.values().find(|user| user.login == login).cloned())
    }

    async fn create(&self, user: CreateUser) -> Result<User, UserRepoError> {
        let mut users = self.users.write().await;

        if users.values().any(|u| u.login == user.login) {
            return Err(UserRepoError::LoginAlreadyTaken);
        }

        let now = chrono::Utc::now();
        let created = User {
            id: Uuid::new_v4(),
            login: user.login,
            password_hash: user.password_hash,
            created_at: now,
            updated_at: now,
        };

        users.insert(created.id, created.clone());
        Ok(created)
    }

    async fn update(&self, id: Uuid, patch: UpdateUser) -> Result<User, UserRepoError> {
        let mut users = self.users.write().await;

        if let Some(ref next_login) = patch.login {
            let taken_by_other = users
                .values()
                .any(|user| user.id != id && user.login == *next_login);
            if taken_by_other {
                return Err(UserRepoError::LoginAlreadyTaken);
            }
        }

        let user = users.get_mut(&id).ok_or(UserRepoError::NotFound)?;

        if let Some(login) = patch.login {
            user.login = login;
        }
        if let Some(password_hash) = patch.password_hash {
            user.password_hash = password_hash;
        }
        user.updated_at = patch.updated_at;

        Ok(user.clone())
    }

    async fn delete(&self, id: Uuid) -> Result<(), UserRepoError> {
        let mut users = self.users.write().await;
        if users.remove(&id).is_none() {
            return Err(UserRepoError::NotFound);
        }
        Ok(())
    }
}
