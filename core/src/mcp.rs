use std::sync::Arc;

use axum::http::header::COOKIE;
use axum::http::request::Parts;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{
        tool::{Extension, ToolRouter},
        wrapper::Parameters,
    },
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        session::local::LocalSessionManager,
        tower::{StreamableHttpServerConfig, StreamableHttpService},
    },
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{AuthUsecase, LogoutInput};
use crate::note::{CreateNoteInput, DeleteNoteInput, NoteUsecase};
use crate::http::state::AppState;

#[derive(Clone)]
pub struct NotalkingMcp {
    state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
}

impl NotalkingMcp {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    async fn user_id(&self, parts: &Parts) -> Result<Uuid, McpError> {
        let sid = session_id_from_parts(parts)?;
        let (auth, _) = self
            .state
            .auth
            .authorize_session(sid)
            .await
            .map_err(|e| McpError::invalid_params(format!("{e:?}"), None))?;
        Ok(auth.user_id)
    }
}

fn session_id_from_parts(parts: &Parts) -> Result<Uuid, McpError> {
    for raw in parts.headers.get_all(COOKIE) {
        let Ok(cookie_header) = raw.to_str() else {
            continue;
        };
        for item in cookie_header.split(';') {
            let item = item.trim();
            if let Some(value) = item.strip_prefix("session_id=") {
                return Uuid::parse_str(value).map_err(|_| {
                    McpError::invalid_params("invalid session_id cookie", None)
                });
            }
        }
    }
    Err(McpError::invalid_params(
        "missing session_id cookie; authenticate via HTTP first",
        None,
    ))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TitleQuery {
    #[schemars(description = "Substring to match against note titles (case-sensitive).")]
    pub title_contains: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct NoteIdParam {
    #[schemars(description = "Note UUID.")]
    pub note_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    #[schemars(description = "Natural-language query for semantic search.")]
    pub query: String,
    #[serde(default)]
    #[schemars(description = "Maximum hits (1–50).")]
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateNoteParams {
    #[schemars(description = "Title for the new note.")]
    pub title: String,
    #[serde(default)]
    #[schemars(description = "Initial body text for the first block.")]
    pub initial_text: String,
}

#[tool_router]
impl NotalkingMcp {
    #[tool(description = "List the signed-in user's notes (id and title).")]
    async fn list_notes(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<String, McpError> {
        let uid = self.user_id(&parts).await?;
        let notes = self
            .state
            .note
            .list_notes(uid)
            .await
            .map_err(|e| McpError::invalid_params(format!("{e:?}"), None))?;
        let v: Vec<_> = notes
            .into_iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id.to_string(),
                    "title": n.title,
                    "head_id": n.head_id.map(|h| h.to_string()),
                })
            })
            .collect();
        serde_json::to_string_pretty(&v).map_err(|e| McpError::invalid_params(e.to_string(), None))
    }

    #[tool(description = "Find notes whose title contains the given substring.")]
    async fn find_note_by_title(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(TitleQuery { title_contains }): Parameters<TitleQuery>,
    ) -> Result<String, McpError> {
        let uid = self.user_id(&parts).await?;
        let notes = self
            .state
            .note
            .list_notes(uid)
            .await
            .map_err(|e| McpError::invalid_params(format!("{e:?}"), None))?;
        let hits: Vec<_> = notes
            .into_iter()
            .filter(|n| n.title.contains(&title_contains))
            .map(|n| {
                serde_json::json!({
                    "id": n.id.to_string(),
                    "title": n.title,
                })
            })
            .collect();
        serde_json::to_string_pretty(&hits)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))
    }

    #[tool(description = "Load all blocks for a note as JSON (same shape as the REST API).")]
    async fn get_note_content(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(NoteIdParam { note_id }): Parameters<NoteIdParam>,
    ) -> Result<String, McpError> {
        let uid = self.user_id(&parts).await?;
        let note_id = Uuid::parse_str(&note_id)
            .map_err(|_| McpError::invalid_params("invalid note_id", None))?;
        let blocks = self
            .state
            .note
            .list_blocks(uid, note_id)
            .await
            .map_err(|e| McpError::invalid_params(format!("{e:?}"), None))?;
        let json_blocks: Vec<serde_json::Value> = blocks
            .into_iter()
            .map(|b| serde_json::to_value(b).unwrap_or(serde_json::Value::Null))
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({ "blocks": json_blocks }))
            .map_err(|e| McpError::invalid_params(e.to_string(), None))
    }

    #[tool(description = "Semantic search over the user's notes (requires embeddings).")]
    async fn semantic_search(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(SearchParams { query, limit }): Parameters<SearchParams>,
    ) -> Result<String, McpError> {
        let uid = self.user_id(&parts).await?;
        let emb = self.state.embedding.as_ref().ok_or_else(|| {
            McpError::invalid_params("semantic search is not configured on this server", None)
        })?;
        let limit = limit.unwrap_or(10).min(50);
        let hits = emb
            .semantic_search(uid, &query, limit)
            .await
            .map_err(|e| McpError::invalid_params(e, None))?;
        let rows: Vec<_> = hits
            .into_iter()
            .map(|(n, b, s)| {
                serde_json::json!({
                    "note_id": n.to_string(),
                    "block_id": b.to_string(),
                    "score": s,
                })
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({ "hits": rows }))
            .map_err(|e| McpError::invalid_params(e.to_string(), None))
    }

    #[tool(description = "Create a note with an initial text block.")]
    async fn create_note(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(CreateNoteParams {
            title,
            initial_text,
        }): Parameters<CreateNoteParams>,
    ) -> Result<String, McpError> {
        let uid = self.user_id(&parts).await?;
        let note = self
            .state
            .note
            .create_note(CreateNoteInput {
                user_id: uid,
                title,
                initial_text,
            })
            .await
            .map_err(|e| McpError::invalid_params(format!("{e:?}"), None))?;
        self.state.notify_embedding(uid, note.id);
        serde_json::to_string_pretty(&serde_json::json!({
            "id": note.id.to_string(),
            "title": note.title,
            "head_id": note.head_id.map(|h| h.to_string()),
        }))
        .map_err(|e| McpError::invalid_params(e.to_string(), None))
    }

    #[tool(description = "Delete a note owned by the signed-in user.")]
    async fn delete_note(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(NoteIdParam { note_id }): Parameters<NoteIdParam>,
    ) -> Result<String, McpError> {
        let uid = self.user_id(&parts).await?;
        let note_id = Uuid::parse_str(&note_id)
            .map_err(|_| McpError::invalid_params("invalid note_id", None))?;
        self.state
            .note
            .delete_note(DeleteNoteInput {
                user_id: uid,
                note_id,
            })
            .await
            .map_err(|e| McpError::invalid_params(format!("{e:?}"), None))?;
        Ok(r#"{"ok":true}"#.to_owned())
    }

    #[tool(
        name = "session_logout",
        description = "Experimental: revoke the current HTTP session (same as POST /auth/logout)."
    )]
    async fn session_logout(&self, Extension(parts): Extension<Parts>) -> Result<String, McpError> {
        let sid = session_id_from_parts(&parts)?;
        self.state
            .auth
            .logout(LogoutInput { session_id: sid })
            .await
            .map_err(|e| McpError::invalid_params(format!("{e:?}"), None))?;
        Ok(r#"{"revoked":true}"#.to_owned())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for NotalkingMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Notalking Core MCP: send the same session_id cookie as browser REST calls."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub fn streamable_http_service(
    state: Arc<AppState>,
) -> StreamableHttpService<NotalkingMcp, LocalSessionManager> {
    let session_manager = Arc::new(LocalSessionManager::default());
    let state_for_factory = state.clone();
    StreamableHttpService::new(
        move || Ok(NotalkingMcp::new(state_for_factory.clone())),
        session_manager,
        StreamableHttpServerConfig::default(),
    )
}
