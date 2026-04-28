use axum::http::{
    HeaderMap, HeaderValue, StatusCode,
    header::{COOKIE, SET_COOKIE},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::auth::{
    AuthError, AuthUsecase, CloseOtherSessionsInput, ListSessionsInput,
    LoginInput, LogoutInput, ManagedSessionView, RegisterInput, SessionView,
};
use crate::note::{CreateNoteInput, DeleteNoteInput, NoteError, NoteUsecase, TextPatch};
use editor::content::Content;
use editor::text::Style;

use super::errors::ApiError;
use super::state::AppState;

pub async fn health_handler() -> StatusCode {
    StatusCode::OK
}

#[derive(Deserialize)]
pub struct CredentialsRequest {
    pub login: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub user_id: String,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Serialize)]
pub struct ManagedSessionResponse {
    pub session_id: String,
    pub device: String,
    pub location: String,
    pub issued_at: String,
    pub expires_at: String,
    pub updated_at: String,
    pub revoked_at: Option<String>,
    pub is_current: bool,
}

#[derive(Serialize)]
pub struct SessionsListResponse {
    pub sessions: Vec<ManagedSessionResponse>,
}

#[derive(Serialize)]
pub struct CloseOtherSessionsResponse {
    pub closed_count: u64,
}

#[derive(Deserialize)]
pub struct CreateNoteRequest {
    pub title: String,
    #[serde(default)]
    pub body: String,
}

#[derive(Serialize)]
pub struct NoteResponse {
    pub id: String,
    pub title: String,
    pub head_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct NotesListResponse {
    pub notes: Vec<NoteResponse>,
    pub page: u64,
    pub per_page: u64,
    pub total: u64,
    pub total_pages: u64,
}

#[derive(Deserialize)]
pub struct NotesQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Deserialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    pub limit: Option<u64>,
}

#[derive(Serialize)]
pub struct SemanticHitResponse {
    pub note_id: String,
    pub block_id: String,
    pub score: f32,
}

fn map_auth_error(error: AuthError) -> ApiError {
    let (status, code, message) = match error {
        AuthError::LoginAlreadyTaken => (StatusCode::CONFLICT, "login_taken", "login already taken"),
        AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials", "invalid credentials"),
        AuthError::UserNotFound => (StatusCode::NOT_FOUND, "user_not_found", "user not found"),
        AuthError::SessionNotFound => (StatusCode::NOT_FOUND, "session_not_found", "session not found"),
        AuthError::SessionAlreadyRevoked => (StatusCode::CONFLICT, "session_revoked", "session already revoked"),
        AuthError::SessionExpired => (StatusCode::UNAUTHORIZED, "session_expired", "session expired"),
        AuthError::CurrentSessionUseLogout => (
            StatusCode::CONFLICT,
            "use_logout",
            "cannot close current session with this endpoint",
        ),
        AuthError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "forbidden"),
        AuthError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "internal error"),
    };
    ApiError {
        status,
        code,
        message: message.to_owned(),
        details: None,
    }
}

fn map_note_error(error: NoteError) -> ApiError {
    let (status, code, message) = match error {
        NoteError::InvalidInput => (StatusCode::BAD_REQUEST, "invalid_input", "invalid note input"),
        NoteError::NotFound => (StatusCode::NOT_FOUND, "not_found", "note not found"),
        NoteError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "forbidden"),
        NoteError::BlockNotFound => (StatusCode::NOT_FOUND, "block_not_found", "block not found"),
        NoteError::CorruptBlocks => (StatusCode::INTERNAL_SERVER_ERROR, "corrupt_blocks", "note blocks are missing or invalid"),
        NoteError::InvalidOperation => (StatusCode::BAD_REQUEST, "invalid_operation", "invalid block operation"),
        NoteError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "internal error"),
    };
    ApiError {
        status,
        code,
        message: message.to_owned(),
        details: None,
    }
}

fn into_session_response(view: crate::auth::SessionView) -> SessionResponse {
    SessionResponse {
        session_id: view.session_id.to_string(),
        user_id: view.user_id.to_string(),
        issued_at: view.issued_at.to_rfc3339(),
        expires_at: view.expires_at.to_rfc3339(),
    }
}

fn into_managed_session_response(view: ManagedSessionView) -> ManagedSessionResponse {
    ManagedSessionResponse {
        session_id: view.session_id.to_string(),
        device: view.device,
        location: view.location,
        issued_at: view.issued_at.to_rfc3339(),
        expires_at: view.expires_at.to_rfc3339(),
        updated_at: view.updated_at.to_rfc3339(),
        revoked_at: view.revoked_at.map(|value| value.to_rfc3339()),
        is_current: view.is_current,
    }
}

fn into_note_response(note: crate::note::Note) -> NoteResponse {
    NoteResponse {
        id: note.id.to_string(),
        title: note.title,
        head_id: note.head_id.map(|h| h.to_string()),
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
    }
}

#[derive(Serialize)]
pub struct NoteBlocksResponse {
    pub blocks: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct CreateBlockRequest {
    pub after_id: Option<String>,
    #[serde(flatten)]
    pub content: CreateBlockContent,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CreateBlockContent {
    Text {
        #[serde(default)]
        text: String,
    },
}

#[derive(Deserialize, Default)]
pub struct StyleDto {
    bold: Option<bool>,
    italic: Option<bool>,
    color: Option<String>,
}

impl From<StyleDto> for Style {
    fn from(d: StyleDto) -> Self {
        Style {
            bold: d.bold,
            italic: d.italic,
            color: d.color,
        }
    }
}

#[derive(Deserialize, Default)]
pub struct LooseStyleFields {
    #[serde(default)]
    bold: Option<bool>,
    #[serde(default)]
    italic: Option<bool>,
    #[serde(default)]
    color: Option<String>,
}

fn style_from_patch(nested: Option<StyleDto>, loose: LooseStyleFields) -> Style {
    let n = nested.unwrap_or_default();
    Style {
        bold: loose.bold.or(n.bold),
        italic: loose.italic.or(n.italic),
        color: loose.color.or(n.color),
    }
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BlockPatchBody {
    Move {
        after_id: Option<String>,
        #[serde(default)]
        before_id: Option<String>,
    },
    InsertText {
        position: usize,
        text: String,
        #[serde(default)]
        style: Option<StyleDto>,
        #[serde(flatten)]
        loose: LooseStyleFields,
    },
    DeleteRange {
        start: usize,
        end: usize,
    },
    DeleteAt {
        position: usize,
        direction: editor::text::DeleteDirection,
    },
    EnableFormatting {
        start: usize,
        end: usize,
        #[serde(default)]
        style: Option<StyleDto>,
        #[serde(flatten)]
        loose: LooseStyleFields,
    },
    DisableFormatting {
        start: usize,
        end: usize,
        #[serde(default)]
        style: Option<StyleDto>,
        #[serde(flatten)]
        loose: LooseStyleFields,
    },
}

const SESSION_COOKIE_NAME: &str = "session_id";

fn client_meta(headers: &HeaderMap) -> (String, String) {
    let device = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    let location = headers
        .get("cf-ipcountry")
        .or_else(|| headers.get("x-geo-country"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    (device, location)
}

fn session_cookie_value(session_id: Uuid, secure: bool) -> String {
    let mut s = format!(
        "{SESSION_COOKIE_NAME}={session_id}; HttpOnly; Path=/; SameSite=Lax; Max-Age=604800"
    );
    if secure {
        s.push_str("; Secure");
    }
    s
}

fn clear_session_cookie(secure: bool) -> String {
    let mut s = format!("{SESSION_COOKIE_NAME}=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0");
    if secure {
        s.push_str("; Secure");
    }
    s
}

fn attach_cookie(
    mut response: Response,
    cookie_value: String,
) -> Result<Response, ApiError> {
    let header_value = HeaderValue::from_str(&cookie_value).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "cookie_write_failed",
        message: "failed to write session cookie".to_owned(),
        details: None,
    })?;
    response.headers_mut().append(SET_COOKIE, header_value);
    Ok(response)
}

fn merge_session_refresh(
    response: Response,
    refresh: SessionView,
    secure: bool,
) -> Result<Response, ApiError> {
    attach_cookie(response, session_cookie_value(refresh.session_id, secure))
}

fn session_id_from_cookie(headers: &HeaderMap) -> Result<Uuid, ApiError> {
    let cookie_headers = headers.get_all(COOKIE);
    for raw in cookie_headers {
        if let Ok(cookie_header) = raw.to_str() {
            for part in cookie_header.split(';') {
                let item = part.trim();
                if let Some(value) = item.strip_prefix(&format!("{SESSION_COOKIE_NAME}=")) {
                    return Uuid::parse_str(value).map_err(|_| ApiError {
                        status: StatusCode::BAD_REQUEST,
                        code: "invalid_session_cookie",
                        message: "invalid session cookie".to_owned(),
                        details: None,
                    });
                }
            }
        }
    }

    Err(ApiError {
        status: StatusCode::UNAUTHORIZED,
        code: "missing_session",
        message: "session cookie is missing".to_owned(),
        details: None,
    })
}

pub async fn register_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CredentialsRequest>,
) -> Result<Response, ApiError> {
    let (device, location) = client_meta(&headers);
    let session = state
        .auth
        .register(RegisterInput {
            login: payload.login,
            password: payload.password,
            device,
            location,
        })
        .await
        .map_err(map_auth_error)?;

    let response = (StatusCode::CREATED, Json(into_session_response(session.clone()))).into_response();
    attach_cookie(response, session_cookie_value(session.session_id, state.config.cookie_secure))
}

pub async fn login_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CredentialsRequest>,
) -> Result<Response, ApiError> {
    let (device, location) = client_meta(&headers);
    let session = state
        .auth
        .login(LoginInput {
            login: payload.login,
            password: payload.password,
            device,
            location,
        })
        .await
        .map_err(map_auth_error)?;

    let response = Json(into_session_response(session.clone())).into_response();
    attach_cookie(response, session_cookie_value(session.session_id, state.config.cookie_secure))
}

pub async fn logout_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let session_id = session_id_from_cookie(&headers)?;

    state
        .auth
        .logout(LogoutInput { session_id })
        .await
        .map_err(map_auth_error)?;

    let response = StatusCode::NO_CONTENT.into_response();
    attach_cookie(response, clear_session_cookie(state.config.cookie_secure))
}

pub async fn list_sessions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let session_id = session_id_from_cookie(&headers)?;
    let sessions = state
        .auth
        .list_sessions(ListSessionsInput { current_session_id: session_id })
        .await
        .map_err(map_auth_error)?;

    let refresh = state
        .auth
        .authorize_session(session_id)
        .await
        .map_err(map_auth_error)?
        .1;

    let body = Json(SessionsListResponse {
        sessions: sessions
            .into_iter()
            .map(into_managed_session_response)
            .collect(),
    })
    .into_response();
    merge_session_refresh(body, refresh, state.config.cookie_secure)
}

pub async fn close_session_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_session_id): Path<String>,
) -> Result<Response, ApiError> {
    let current_session_id = session_id_from_cookie(&headers)?;
    let target_session_id = Uuid::parse_str(&target_session_id).map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "invalid_session_id",
        message: "invalid target session_id".to_owned(),
        details: None,
    })?;

    state
        .auth
        .close_session(crate::auth::CloseSessionInput {
            current_session_id,
            target_session_id,
        })
        .await
        .map_err(map_auth_error)?;

    let refresh = state
        .auth
        .authorize_session(current_session_id)
        .await
        .map_err(map_auth_error)?
        .1;

    let body = StatusCode::NO_CONTENT.into_response();
    merge_session_refresh(body, refresh, state.config.cookie_secure)
}

pub async fn close_other_sessions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let current_session_id = session_id_from_cookie(&headers)?;
    let closed_count = state
        .auth
        .close_other_sessions(CloseOtherSessionsInput { current_session_id })
        .await
        .map_err(map_auth_error)?;

    let refresh = state
        .auth
        .authorize_session(current_session_id)
        .await
        .map_err(map_auth_error)?
        .1;

    let body = Json(CloseOtherSessionsResponse { closed_count }).into_response();
    merge_session_refresh(body, refresh, state.config.cookie_secure)
}

pub async fn create_note_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateNoteRequest>,
) -> Result<Response, ApiError> {
    let session_id = session_id_from_cookie(&headers)?;
    let (auth, refresh) = state.auth.authorize_session(session_id).await.map_err(map_auth_error)?;

    let note = state
        .note
        .create_note(CreateNoteInput {
            user_id: auth.user_id,
            title: payload.title,
            initial_text: payload.body,
        })
        .await
        .map_err(map_note_error)?;

    state.notify_embedding(auth.user_id, note.id);

    let body = (StatusCode::CREATED, Json(into_note_response(note))).into_response();
    merge_session_refresh(body, refresh, state.config.cookie_secure)
}

pub async fn list_notes_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<NotesQuery>,
) -> Result<Response, ApiError> {
    let session_id = session_id_from_cookie(&headers)?;
    let (auth, refresh) = state.auth.authorize_session(session_id).await.map_err(map_auth_error)?;

    let notes = state
        .note
        .list_notes(auth.user_id)
        .await
        .map_err(map_note_error)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let total = notes.len() as u64;
    let total_pages = if total == 0 {
        0
    } else {
        ((total - 1) / per_page) + 1
    };

    let start = ((page - 1) * per_page) as usize;
    let end = (start + per_page as usize).min(notes.len());
    let paged_notes = if start >= notes.len() {
        Vec::new()
    } else {
        notes[start..end].to_vec()
    };

    let body = Json(NotesListResponse {
        notes: paged_notes.into_iter().map(into_note_response).collect(),
        page,
        per_page,
        total,
        total_pages,
    })
    .into_response();
    merge_session_refresh(body, refresh, state.config.cookie_secure)
}

pub async fn delete_note_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(note_id): Path<String>,
) -> Result<Response, ApiError> {
    let session_id = session_id_from_cookie(&headers)?;
    let (auth, refresh) = state.auth.authorize_session(session_id).await.map_err(map_auth_error)?;
    let note_id = Uuid::parse_str(&note_id).map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "invalid_note_id",
        message: "invalid note id".to_owned(),
        details: None,
    })?;

    state
        .note
        .delete_note(DeleteNoteInput {
            user_id: auth.user_id,
            note_id,
        })
        .await
        .map_err(map_note_error)?;

    let body = StatusCode::NO_CONTENT.into_response();
    merge_session_refresh(body, refresh, state.config.cookie_secure)
}

pub async fn list_note_blocks_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(note_id): Path<String>,
) -> Result<Response, ApiError> {
    let session_id = session_id_from_cookie(&headers)?;
    let (auth, refresh) = state.auth.authorize_session(session_id).await.map_err(map_auth_error)?;
    let note_id = Uuid::parse_str(&note_id).map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "invalid_note_id",
        message: "invalid note id".to_owned(),
        details: None,
    })?;

    let blocks = state
        .note
        .list_blocks(auth.user_id, note_id)
        .await
        .map_err(map_note_error)?;

    let json_blocks: Vec<serde_json::Value> = blocks
        .into_iter()
        .map(|b| serde_json::to_value(b).unwrap_or(serde_json::Value::Null))
        .collect();

    let body = Json(NoteBlocksResponse { blocks: json_blocks }).into_response();
    merge_session_refresh(body, refresh, state.config.cookie_secure)
}

pub async fn create_note_block_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(note_id): Path<String>,
    Json(payload): Json<CreateBlockRequest>,
) -> Result<Response, ApiError> {
    let session_id = session_id_from_cookie(&headers)?;
    let (auth, refresh) = state.auth.authorize_session(session_id).await.map_err(map_auth_error)?;
    let note_id = Uuid::parse_str(&note_id).map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "invalid_note_id",
        message: "invalid note id".to_owned(),
        details: None,
    })?;

    let after_id = match &payload.after_id {
        None => None,
        Some(s) => Some(Uuid::parse_str(s).map_err(|_| ApiError {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_after_id",
            message: "invalid after_id".to_owned(),
            details: None,
        })?),
    };

    let content = match payload.content {
        CreateBlockContent::Text { text } => Content::Text(editor::text::TextBlock::from_text(&text)),
    };

    let block = state
        .note
        .create_block(auth.user_id, note_id, after_id, content)
        .await
        .map_err(map_note_error)?;

    state.notify_embedding(auth.user_id, note_id);

    let body = serde_json::to_value(&block).map_err(|_| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "serialize_failed",
        message: "failed to serialize block".to_owned(),
        details: None,
    })?;
    let resp = (StatusCode::CREATED, Json(body)).into_response();
    merge_session_refresh(resp, refresh, state.config.cookie_secure)
}

pub async fn delete_note_block_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((note_id, block_id)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let session_id = session_id_from_cookie(&headers)?;
    let (auth, refresh) = state.auth.authorize_session(session_id).await.map_err(map_auth_error)?;
    let note_id = Uuid::parse_str(&note_id).map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "invalid_note_id",
        message: "invalid note id".to_owned(),
        details: None,
    })?;
    let block_id = Uuid::parse_str(&block_id).map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "invalid_block_id",
        message: "invalid block id".to_owned(),
        details: None,
    })?;

    state
        .note
        .delete_block(auth.user_id, note_id, block_id)
        .await
        .map_err(map_note_error)?;

    state.notify_embedding(auth.user_id, note_id);

    let body = StatusCode::NO_CONTENT.into_response();
    merge_session_refresh(body, refresh, state.config.cookie_secure)
}

pub async fn patch_note_block_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((note_id, block_id)): Path<(String, String)>,
    Json(payload): Json<BlockPatchBody>,
) -> Result<Response, ApiError> {
    let session_id = session_id_from_cookie(&headers)?;
    let (auth, refresh) = state.auth.authorize_session(session_id).await.map_err(map_auth_error)?;
    let note_id = Uuid::parse_str(&note_id).map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "invalid_note_id",
        message: "invalid note id".to_owned(),
        details: None,
    })?;
    let block_id = Uuid::parse_str(&block_id).map_err(|_| ApiError {
        status: StatusCode::BAD_REQUEST,
        code: "invalid_block_id",
        message: "invalid block id".to_owned(),
        details: None,
    })?;

    match payload {
        BlockPatchBody::Move {
            after_id,
            before_id,
        } => {
            let after_uuid = match &after_id {
                None => None,
                Some(s) => Some(Uuid::parse_str(s).map_err(|_| ApiError {
                    status: StatusCode::BAD_REQUEST,
                    code: "invalid_after_id",
                    message: "invalid after_id".to_owned(),
                    details: None,
                })?),
            };
            let before_uuid = match &before_id {
                None => None,
                Some(s) => Some(Uuid::parse_str(s).map_err(|_| ApiError {
                    status: StatusCode::BAD_REQUEST,
                    code: "invalid_before_id",
                    message: "invalid before_id".to_owned(),
                    details: None,
                })?),
            };
            state
                .note
                .move_block(
                    auth.user_id,
                    note_id,
                    block_id,
                    after_uuid,
                    before_uuid,
                )
                .await
                .map_err(map_note_error)?;
        }
        BlockPatchBody::InsertText {
            position,
            text,
            style,
            loose,
        } => {
            state
                .note
                .apply_text_patch(
                    auth.user_id,
                    note_id,
                    block_id,
                    TextPatch::InsertText {
                        position,
                        text,
                        style: style_from_patch(style, loose),
                    },
                )
                .await
                .map_err(map_note_error)?;
        }
        BlockPatchBody::DeleteRange { start, end } => {
            state
                .note
                .apply_text_patch(
                    auth.user_id,
                    note_id,
                    block_id,
                    TextPatch::DeleteRange { start, end },
                )
                .await
                .map_err(map_note_error)?;
        }
        BlockPatchBody::DeleteAt {
            position,
            direction,
        } => {
            state
                .note
                .apply_text_patch(
                    auth.user_id,
                    note_id,
                    block_id,
                    TextPatch::DeleteAt {
                        position,
                        direction,
                    },
                )
                .await
                .map_err(map_note_error)?;
        }
        BlockPatchBody::EnableFormatting {
            start,
            end,
            style,
            loose,
        } => {
            state
                .note
                .apply_text_patch(
                    auth.user_id,
                    note_id,
                    block_id,
                    TextPatch::EnableFormatting {
                        start,
                        end,
                        style: style_from_patch(style, loose),
                    },
                )
                .await
                .map_err(map_note_error)?;
        }
        BlockPatchBody::DisableFormatting {
            start,
            end,
            style,
            loose,
        } => {
            state
                .note
                .apply_text_patch(
                    auth.user_id,
                    note_id,
                    block_id,
                    TextPatch::DisableFormatting {
                        start,
                        end,
                        style: style_from_patch(style, loose),
                    },
                )
                .await
                .map_err(map_note_error)?;
        }
    }

    state.notify_embedding(auth.user_id, note_id);

    let body = StatusCode::NO_CONTENT.into_response();
    merge_session_refresh(body, refresh, state.config.cookie_secure)
}

pub async fn semantic_search_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SemanticSearchRequest>,
) -> Result<Response, ApiError> {
    let session_id = session_id_from_cookie(&headers)?;
    let (auth, refresh) = state.auth.authorize_session(session_id).await.map_err(map_auth_error)?;

    let emb = state.embedding.as_ref().ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "embeddings_disabled",
        message: "semantic search is not configured".to_owned(),
        details: None,
    })?;

    let limit = body.limit.unwrap_or(10).min(50);
    let hits = emb
        .semantic_search(auth.user_id, &body.query, limit)
        .await
        .map_err(|e| ApiError {
            status: StatusCode::BAD_GATEWAY,
            code: "search_failed",
            message: e,
            details: None,
        })?;

    let hits_json: Vec<SemanticHitResponse> = hits
        .into_iter()
        .map(|(n, b, s)| SemanticHitResponse {
            note_id: n.to_string(),
            block_id: b.to_string(),
            score: s,
        })
        .collect();

    let resp = Json(json!({ "hits": hits_json })).into_response();
    merge_session_refresh(resp, refresh, state.config.cookie_secure)
}
