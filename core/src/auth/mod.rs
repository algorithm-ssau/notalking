#![allow(dead_code)]

pub mod service;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::user::User;

pub struct RegisterInput {
    pub login: String,
    pub password: String,
}

pub struct LoginInput {
    pub login: String,
    pub password: String,
}

pub struct LogoutInput {
    pub session_id: Uuid,
}

pub struct ListSessionsInput {
    pub current_session_id: Uuid,
}

pub struct CloseSessionInput {
    pub current_session_id: Uuid,
    pub target_session_id: Uuid,
}

pub struct CloseOtherSessionsInput {
    pub current_session_id: Uuid,
}

#[derive(Clone)]
pub struct AuthorizedSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Clone)]
pub struct SessionView {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct ManagedSessionView {
    pub session_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum AuthError {
    LoginAlreadyTaken,
    UserNotFound,
    InvalidCredentials,
    SessionNotFound,
    SessionAlreadyRevoked,
    SessionExpired,
    CurrentSessionUseLogout,
    Forbidden,
    Internal,
}

pub trait AuthUserReader {
    async fn find_by_login(&self, login: &str) -> Result<Option<User>, AuthError>;
}

pub trait SessionRepo {
    async fn create_session(
        &self,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> Result<Session, AuthError>;
    async fn find_session(&self, session_id: Uuid) -> Result<Option<Session>, AuthError>;
    async fn revoke_session(
        &self,
        session_id: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<(), AuthError>;
    async fn list_sessions_by_user(&self, user_id: Uuid) -> Result<Vec<Session>, AuthError>;
    async fn revoke_session_for_user(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<(), AuthError>;
    async fn revoke_sessions_for_user_except(
        &self,
        user_id: Uuid,
        keep_session_id: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<u64, AuthError>;
}

pub trait PasswordService {
    fn hash_password(&self, plain: &str) -> Result<String, AuthError>;
    fn verify_password(&self, plain: &str, hash: &str) -> Result<bool, AuthError>;
}

pub trait AuthUsecase {
    async fn register(&self, input: RegisterInput) -> Result<SessionView, AuthError>;
    async fn login(&self, input: LoginInput) -> Result<SessionView, AuthError>;
    async fn logout(&self, input: LogoutInput) -> Result<(), AuthError>;
    async fn list_sessions(&self, input: ListSessionsInput) -> Result<Vec<ManagedSessionView>, AuthError>;
    async fn close_session(&self, input: CloseSessionInput) -> Result<(), AuthError>;
    async fn close_other_sessions(&self, input: CloseOtherSessionsInput) -> Result<u64, AuthError>;
    async fn authorize_session(&self, session_id: Uuid) -> Result<AuthorizedSession, AuthError>;
}

pub enum AuthTraceStep {
    ValidateInput,
    FindUserByLogin,
    CheckLoginIsFree,
    HashPassword,
    CreateUser,
    VerifyPassword,
    CreateSession,
    FindSession,
    RevokeSession,
    ReturnSession,
    ReturnOk,
}

pub fn register_trace() -> [AuthTraceStep; 7] {
    [
        AuthTraceStep::ValidateInput,
        AuthTraceStep::FindUserByLogin,
        AuthTraceStep::CheckLoginIsFree,
        AuthTraceStep::HashPassword,
        AuthTraceStep::CreateUser,
        AuthTraceStep::CreateSession,
        AuthTraceStep::ReturnSession,
    ]
}

pub fn login_trace() -> [AuthTraceStep; 5] {
    [
        AuthTraceStep::ValidateInput,
        AuthTraceStep::FindUserByLogin,
        AuthTraceStep::VerifyPassword,
        AuthTraceStep::CreateSession,
        AuthTraceStep::ReturnSession,
    ]
}

pub fn logout_trace() -> [AuthTraceStep; 3] {
    [
        AuthTraceStep::FindSession,
        AuthTraceStep::RevokeSession,
        AuthTraceStep::ReturnOk,
    ]
}
