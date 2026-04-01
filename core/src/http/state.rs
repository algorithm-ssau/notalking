use std::sync::Arc;
use std::time::Duration;

use crate::{
    auth::service::{AuthService, SimplePasswordService},
    http::rate_limit::InMemoryRateLimiter,
    note,
    session,
    user,
};

#[derive(Clone)]
pub struct AppState {
    pub auth: AuthService<user::in_memory::InMemoryRepo, session::in_memory::InMemoryRepo, SimplePasswordService>,
    pub note: note::service::NoteService<note::in_memory::InMemoryRepo>,
    pub rate_limiter: InMemoryRateLimiter,
}

impl AppState {
    pub fn new() -> Self {
        let user_repo = Arc::new(user::in_memory::InMemoryRepo::new());
        let session_repo = Arc::new(session::in_memory::InMemoryRepo::new());
        let note_repo = Arc::new(note::in_memory::InMemoryRepo::new());
        let password_service = Arc::new(SimplePasswordService);

        Self {
            auth: AuthService::new(user_repo, session_repo, password_service),
            note: note::service::NoteService::new(note_repo),
            rate_limiter: InMemoryRateLimiter::new(20, Duration::from_secs(60)),
        }
    }
}
