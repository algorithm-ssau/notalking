use axum::{
    Router,
    http::{HeaderValue, Method},
    middleware,
    routing::{delete, get, post},
};
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::{
    handlers::{
        close_other_sessions_handler, close_session_handler, create_note_block_handler,
        create_note_handler, delete_note_block_handler, delete_note_handler, health_handler,
        list_note_blocks_handler, list_notes_handler, list_sessions_handler, login_handler,
        logout_handler, me_handler, patch_note_block_handler, register_handler,
        semantic_search_handler, update_note_handler,
    },
    logging::request_logging_middleware,
    rate_limit::{auth_rate_limit, global_rate_limit},
    state::AppState,
};

fn create_auth_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .route("/me", get(me_handler))
        .route("/logout", post(logout_handler))
        .route("/sessions", get(list_sessions_handler))
        .route("/sessions/others", delete(close_other_sessions_handler))
        .route("/sessions/{session_id}", delete(close_session_handler))
        .route_layer(middleware::from_fn_with_state(state, auth_rate_limit))
}

fn protected_api_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/notes", get(list_notes_handler))
        .route("/notes", post(create_note_handler))
        .route(
            "/notes/{note_id}",
            delete(delete_note_handler).patch(update_note_handler),
        )
        .route(
            "/notes/{note_id}/blocks",
            get(list_note_blocks_handler).post(create_note_block_handler),
        )
        .route(
            "/notes/{note_id}/blocks/{block_id}",
            delete(delete_note_block_handler).patch(patch_note_block_handler),
        )
        .route("/search/semantic", post(semantic_search_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            global_rate_limit,
        ))
}

pub fn create_http_router(state: AppState) -> Router {
    let auth = create_auth_router(state.clone());
    let api = protected_api_router(state.clone());

    let mut router = Router::new()
        .route("/health", get(health_handler))
        .merge(api)
        .nest("/auth", auth)
        .layer(middleware::from_fn(request_logging_middleware));

    if !state.config.cors_origins.is_empty() {
        let origins: Vec<HeaderValue> = state
            .config
            .cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        let cors = CorsLayer::new()
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(tower_http::cors::Any)
            .allow_credentials(true)
            .allow_origin(AllowOrigin::list(origins));
        router = router.layer(cors);
    }

    if state.config.mcp_enabled {
        let path = state.config.mcp_http_path.clone();
        let mcp_state = std::sync::Arc::new(state.clone());
        router = router.nest_service(&path, crate::mcp::streamable_http_service(mcp_state));
    }

    router.with_state(state)
}
