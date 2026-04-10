use axum::{
    Router,
    middleware,
    routing::{delete, get, post},
};

use super::{
    handlers::{
        close_other_sessions_handler, close_session_handler, create_note_block_handler, create_note_handler,
        delete_note_block_handler, delete_note_handler, health_handler, list_note_blocks_handler,
        list_notes_handler, list_sessions_handler, login_handler, logout_handler, patch_note_block_handler,
        register_handler,
    },
    logging::request_logging_middleware,
    rate_limit::auth_rate_limit,
    state::AppState,
};

fn create_auth_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .route("/logout", post(logout_handler))
        .route("/sessions", get(list_sessions_handler))
        .route("/sessions/others", delete(close_other_sessions_handler))
        .route("/sessions/{session_id}", delete(close_session_handler))
        .route_layer(middleware::from_fn_with_state(state, auth_rate_limit))
}

pub fn create_http_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/notes", get(list_notes_handler))
        .route("/notes", post(create_note_handler))
        .route("/notes/{note_id}", delete(delete_note_handler))
        .route(
            "/notes/{note_id}/blocks",
            get(list_note_blocks_handler).post(create_note_block_handler),
        )
        .route(
            "/notes/{note_id}/blocks/{block_id}",
            delete(delete_note_block_handler).patch(patch_note_block_handler),
        )
        .nest("/auth", create_auth_router(state.clone()))
        .layer(middleware::from_fn(request_logging_middleware))
        .with_state(state)
}

#[allow(dead_code)]
pub fn create_editor_router() -> Router {
    Router::new()
}
