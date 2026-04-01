#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::auth::{AuthError, Session, SessionRepo};

#[derive(Clone, Default)]
pub struct InMemoryRepo {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
}

impl InMemoryRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SessionRepo for InMemoryRepo {
    async fn create_session(&self, user_id: Uuid, expires_at: DateTime<Utc>) -> Result<Session, AuthError> {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();

        let session = Session {
            id: Uuid::new_v4(),
            user_id,
            issued_at: now,
            expires_at,
            revoked_at: None,
        };
        sessions.insert(session.id, session.clone());

        Ok(session)
    }

    async fn find_session(&self, session_id: Uuid) -> Result<Option<Session>, AuthError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(&session_id).cloned())
    }

    async fn revoke_session(&self, session_id: Uuid, revoked_at: DateTime<Utc>) -> Result<(), AuthError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or(AuthError::SessionNotFound)?;

        if session.revoked_at.is_some() {
            return Err(AuthError::SessionAlreadyRevoked);
        }

        session.revoked_at = Some(revoked_at);
        Ok(())
    }

    async fn list_sessions_by_user(&self, user_id: Uuid) -> Result<Vec<Session>, AuthError> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .values()
            .filter(|session| session.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn revoke_session_for_user(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<(), AuthError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or(AuthError::SessionNotFound)?;

        if session.user_id != user_id {
            return Err(AuthError::Forbidden);
        }
        if session.revoked_at.is_some() {
            return Err(AuthError::SessionAlreadyRevoked);
        }

        session.revoked_at = Some(revoked_at);
        Ok(())
    }

    async fn revoke_sessions_for_user_except(
        &self,
        user_id: Uuid,
        keep_session_id: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<u64, AuthError> {
        let mut sessions = self.sessions.write().await;
        let mut revoked_count = 0_u64;

        for session in sessions.values_mut() {
            if session.user_id == user_id
                && session.id != keep_session_id
                && session.revoked_at.is_none()
            {
                session.revoked_at = Some(revoked_at);
                revoked_count += 1;
            }
        }

        Ok(revoked_count)
    }
}
