use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    auth::{
        AuthError, AuthUsecase, AuthorizedSession, CloseOtherSessionsInput, CloseSessionInput,
        ListSessionsInput, LogoutInput, ManagedSessionView, PasswordService, Session, SessionRepo,
        SessionView,
    },
    user::repo::{CreateUser, Repo as UserRepo, UserRepoError},
};

use super::RegisterInput;

#[derive(Clone)]
pub struct SimplePasswordService;

impl PasswordService for SimplePasswordService {
    fn hash_password(&self, plain: &str) -> Result<String, AuthError> {
        Ok(plain.to_owned())
    }

    fn verify_password(&self, plain: &str, hash: &str) -> Result<bool, AuthError> {
        Ok(plain == hash)
    }
}

#[derive(Clone)]
pub struct AuthService<U, S, P> {
    user_repo: Arc<U>,
    session_repo: Arc<S>,
    password_service: Arc<P>,
    session_ttl_hours: i64,
}

impl<U, S, P> AuthService<U, S, P> {
    pub fn new(user_repo: Arc<U>, session_repo: Arc<S>, password_service: Arc<P>) -> Self {
        Self {
            user_repo,
            session_repo,
            password_service,
            session_ttl_hours: 24,
        }
    }

    fn map_user_repo_error(error: UserRepoError) -> AuthError {
        match error {
            UserRepoError::LoginAlreadyTaken => AuthError::LoginAlreadyTaken,
            UserRepoError::NotFound => AuthError::UserNotFound,
            UserRepoError::Internal => AuthError::Internal,
        }
    }

    fn ensure_active_session(session: &Session) -> Result<(), AuthError> {
        if session.revoked_at.is_some() {
            return Err(AuthError::SessionAlreadyRevoked);
        }
        if session.expires_at <= Utc::now() {
            return Err(AuthError::SessionExpired);
        }
        Ok(())
    }

    async fn resolve_current_session(&self, session_id: Uuid) -> Result<Session, AuthError>
    where
        S: SessionRepo + Send + Sync + 'static,
    {
        let session = self
            .session_repo
            .find_session(session_id)
            .await?
            .ok_or(AuthError::SessionNotFound)?;
        Self::ensure_active_session(&session)?;
        Ok(session)
    }
}

impl<U, S, P> AuthUsecase for AuthService<U, S, P>
where
    U: UserRepo + Send + Sync + 'static,
    S: SessionRepo + Send + Sync + 'static,
    P: PasswordService + Send + Sync + 'static,
{
    async fn register(&self, input: RegisterInput) -> Result<SessionView, AuthError> {
        if input.login.trim().is_empty() || input.password.is_empty() {
            return Err(AuthError::InvalidCredentials);
        }

        if self
            .user_repo
            .find_by_login(&input.login)
            .await
            .map_err(Self::map_user_repo_error)?
            .is_some()
        {
            return Err(AuthError::LoginAlreadyTaken);
        }

        let password_hash = self.password_service.hash_password(&input.password)?;
        let user = self
            .user_repo
            .create(CreateUser {
                login: input.login,
                password_hash,
            })
            .await
            .map_err(Self::map_user_repo_error)?;

        let now = Utc::now();
        let expires_at = now + Duration::hours(self.session_ttl_hours);
        let session = self
            .session_repo
            .create_session(user.id, expires_at)
            .await?;

        Ok(SessionView {
            session_id: session.id,
            user_id: user.id,
            issued_at: session.issued_at,
            expires_at: session.expires_at,
        })
    }

    async fn login(&self, input: super::LoginInput) -> Result<SessionView, AuthError> {
        if input.login.trim().is_empty() || input.password.is_empty() {
            return Err(AuthError::InvalidCredentials);
        }

        let user = self
            .user_repo
            .find_by_login(&input.login)
            .await
            .map_err(Self::map_user_repo_error)?
            .ok_or(AuthError::InvalidCredentials)?;

        let password_ok = self
            .password_service
            .verify_password(&input.password, &user.password_hash)?;
        if !password_ok {
            return Err(AuthError::InvalidCredentials);
        }

        let now = Utc::now();
        let expires_at = now + Duration::hours(self.session_ttl_hours);
        let session = self
            .session_repo
            .create_session(user.id, expires_at)
            .await?;

        Ok(SessionView {
            session_id: session.id,
            user_id: user.id,
            issued_at: session.issued_at,
            expires_at: session.expires_at,
        })
    }

    async fn logout(&self, input: LogoutInput) -> Result<(), AuthError> {
        let session = self
            .session_repo
            .find_session(input.session_id)
            .await?
            .ok_or(AuthError::SessionNotFound)?;

        if session.revoked_at.is_some() {
            return Err(AuthError::SessionAlreadyRevoked);
        }

        self.session_repo
            .revoke_session(input.session_id, Utc::now())
            .await
    }

    async fn list_sessions(
        &self,
        input: ListSessionsInput,
    ) -> Result<Vec<ManagedSessionView>, AuthError> {
        let current_session = self
            .resolve_current_session(input.current_session_id)
            .await?;
        let mut sessions = self
            .session_repo
            .list_sessions_by_user(current_session.user_id)
            .await?;

        sessions.sort_by(|a, b| b.issued_at.cmp(&a.issued_at));

        Ok(sessions
            .into_iter()
            .map(|session| ManagedSessionView {
                session_id: session.id,
                issued_at: session.issued_at,
                expires_at: session.expires_at,
                revoked_at: session.revoked_at,
                is_current: session.id == current_session.id,
            })
            .collect())
    }

    async fn close_session(&self, input: CloseSessionInput) -> Result<(), AuthError> {
        let current_session = self
            .resolve_current_session(input.current_session_id)
            .await?;
        if input.target_session_id == input.current_session_id {
            return Err(AuthError::CurrentSessionUseLogout);
        }

        self.session_repo
            .revoke_session_for_user(current_session.user_id, input.target_session_id, Utc::now())
            .await
    }

    async fn close_other_sessions(&self, input: CloseOtherSessionsInput) -> Result<u64, AuthError> {
        let current_session = self
            .resolve_current_session(input.current_session_id)
            .await?;
        self.session_repo
            .revoke_sessions_for_user_except(
                current_session.user_id,
                current_session.id,
                Utc::now(),
            )
            .await
    }

    async fn authorize_session(&self, session_id: Uuid) -> Result<AuthorizedSession, AuthError> {
        let current_session = self.resolve_current_session(session_id).await?;
        Ok(AuthorizedSession {
            session_id: current_session.id,
            user_id: current_session.user_id,
        })
    }
}
