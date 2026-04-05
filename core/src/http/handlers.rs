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
use uuid::Uuid;

use crate::auth::{
    AuthError, AuthUsecase, CloseOtherSessionsInput, CloseSessionInput, ListSessionsInput,
    LoginInput, LogoutInput, ManagedSessionView, RegisterInput,
};
use crate::note::{CreateNoteInput, DeleteNoteInput, NoteError, NoteUsecase, TextPatch};
use editor::content::Content;
use editor::text::Style;

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
pub struct ErrorResponse {
    pub error: String,
    pub hint: Option<String>,
}

#[derive(Serialize)]
pub struct ManagedSessionResponse {
    pub session_id: String,
    pub issued_at: String,
    pub expires_at: String,
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

fn map_auth_error(error: AuthError) -> (StatusCode, Json<ErrorResponse>) {
    let (status, message, hint) = match error {
        AuthError::LoginAlreadyTaken => (StatusCode::CONFLICT, "login already taken", None),
        AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid credentials", None),
        AuthError::UserNotFound => (StatusCode::NOT_FOUND, "user not found", None),
        AuthError::SessionNotFound => (StatusCode::NOT_FOUND, "session not found", None),
        AuthError::SessionAlreadyRevoked => (StatusCode::CONFLICT, "session already revoked", None),
        AuthError::SessionExpired => (StatusCode::UNAUTHORIZED, "session expired", None),
        AuthError::CurrentSessionUseLogout => (
            StatusCode::CONFLICT,
            "cannot close current session with this endpoint",
            Some("use POST /auth/logout for current session"),
        ),
        AuthError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", None),
        AuthError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error", None),
    };

    (
        status,
        Json(ErrorResponse {
            error: message.to_owned(),
            hint: hint.map(str::to_owned),
        }),
    )
}

fn map_note_error(error: NoteError) -> (StatusCode, Json<ErrorResponse>) {
    let (status, message) = match error {
        NoteError::InvalidInput => (StatusCode::BAD_REQUEST, "invalid note input"),
        NoteError::NotFound => (StatusCode::NOT_FOUND, "note not found"),
        NoteError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
        NoteError::BlockNotFound => (StatusCode::NOT_FOUND, "block not found"),
        NoteError::CorruptBlocks => (StatusCode::INTERNAL_SERVER_ERROR, "note blocks are missing or invalid"),
        NoteError::InvalidOperation => (StatusCode::BAD_REQUEST, "invalid block operation"),
        NoteError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
    };

    (
        status,
        Json(ErrorResponse {
            error: message.to_owned(),
            hint: None,
        }),
    )
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
        issued_at: view.issued_at.to_rfc3339(),
        expires_at: view.expires_at.to_rfc3339(),
        revoked_at: view.revoked_at.map(|value| value.to_rfc3339()),
        is_current: view.is_current,
    }
}

fn into_note_response(note: crate::note::Note) -> NoteResponse {
    NoteResponse {
        id: note.id.to_string(),
        title: note.title,
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

/// Optional `bold` / `italic` / `color` at the same JSON level as `op` (in addition to nested `style`).
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

fn build_session_cookie(session_id: Uuid) -> String {
    format!(
        "{SESSION_COOKIE_NAME}={session_id}; HttpOnly; Path=/; SameSite=Lax; Max-Age=86400"
    )
}

fn build_clear_session_cookie() -> String {
    format!("{SESSION_COOKIE_NAME}=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0")
}

fn attach_cookie(
    mut response: Response,
    cookie_value: String,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let header_value = HeaderValue::from_str(&cookie_value).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "failed to write session cookie".to_owned(),
                hint: None,
            }),
        )
    })?;
    response.headers_mut().append(SET_COOKIE, header_value);
    Ok(response)
}

fn session_id_from_cookie(headers: &HeaderMap) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    let cookie_headers = headers.get_all(COOKIE);
    for raw in cookie_headers {
        if let Ok(cookie_header) = raw.to_str() {
            for part in cookie_header.split(';') {
                let item = part.trim();
                if let Some(value) = item.strip_prefix(&format!("{SESSION_COOKIE_NAME}=")) {
                    return Uuid::parse_str(value).map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            Json(ErrorResponse {
                                error: "invalid session cookie".to_owned(),
                                hint: None,
                            }),
                        )
                    });
                }
            }
        }
    }

    Err((
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "session cookie is missing".to_owned(),
            hint: None,
        }),
    ))
}

pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<CredentialsRequest>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let session = state
        .auth
        .register(RegisterInput {
            login: payload.login,
            password: payload.password,
        })
        .await
        .map_err(map_auth_error)?;

    let response = (StatusCode::CREATED, Json(into_session_response(session.clone()))).into_response();
    attach_cookie(response, build_session_cookie(session.session_id))
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<CredentialsRequest>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let session = state
        .auth
        .login(LoginInput {
            login: payload.login,
            password: payload.password,
        })
        .await
        .map_err(map_auth_error)?;

    let response = Json(into_session_response(session.clone())).into_response();
    attach_cookie(response, build_session_cookie(session.session_id))
}

pub async fn logout_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let session_id = session_id_from_cookie(&headers)?;

    state
        .auth
        .logout(LogoutInput { session_id })
        .await
        .map_err(map_auth_error)?;

    let response = StatusCode::NO_CONTENT.into_response();
    attach_cookie(response, build_clear_session_cookie())
}

pub async fn list_sessions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let current_session_id = session_id_from_cookie(&headers)?;
    let sessions = state
        .auth
        .list_sessions(ListSessionsInput { current_session_id })
        .await
        .map_err(map_auth_error)?;

    Ok(Json(SessionsListResponse {
        sessions: sessions
            .into_iter()
            .map(into_managed_session_response)
            .collect(),
    }))
}

pub async fn close_session_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_session_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let current_session_id = session_id_from_cookie(&headers)?;
    let target_session_id = Uuid::parse_str(&target_session_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid target session_id".to_owned(),
                hint: None,
            }),
        )
    })?;

    state
        .auth
        .close_session(CloseSessionInput {
            current_session_id,
            target_session_id,
        })
        .await
        .map_err(map_auth_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn close_other_sessions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CloseOtherSessionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let current_session_id = session_id_from_cookie(&headers)?;
    let closed_count = state
        .auth
        .close_other_sessions(CloseOtherSessionsInput { current_session_id })
        .await
        .map_err(map_auth_error)?;

    Ok(Json(CloseOtherSessionsResponse { closed_count }))
}

pub async fn create_note_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateNoteRequest>,
) -> Result<(StatusCode, Json<NoteResponse>), (StatusCode, Json<ErrorResponse>)> {
    let session_id = session_id_from_cookie(&headers)?;
    let session = state
        .auth
        .authorize_session(session_id)
        .await
        .map_err(map_auth_error)?;

    let note = state
        .note
        .create_note(CreateNoteInput {
            user_id: session.user_id,
            title: payload.title,
            initial_text: payload.body,
        })
        .await
        .map_err(map_note_error)?;

    Ok((StatusCode::CREATED, Json(into_note_response(note))))
}

pub async fn list_notes_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<NotesQuery>,
) -> Result<Json<NotesListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session_id = session_id_from_cookie(&headers)?;
    let session = state
        .auth
        .authorize_session(session_id)
        .await
        .map_err(map_auth_error)?;

    let notes = state
        .note
        .list_notes(session.user_id)
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

    Ok(Json(NotesListResponse {
        notes: paged_notes.into_iter().map(into_note_response).collect(),
        page,
        per_page,
        total,
        total_pages,
    }))
}

pub async fn delete_note_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(note_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let session_id = session_id_from_cookie(&headers)?;
    let session = state
        .auth
        .authorize_session(session_id)
        .await
        .map_err(map_auth_error)?;
    let note_id = Uuid::parse_str(&note_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid note id".to_owned(),
                hint: None,
            }),
        )
    })?;

    state
        .note
        .delete_note(DeleteNoteInput {
            user_id: session.user_id,
            note_id,
        })
        .await
        .map_err(map_note_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_note_blocks_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(note_id): Path<String>,
) -> Result<Json<NoteBlocksResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session_id = session_id_from_cookie(&headers)?;
    let session = state
        .auth
        .authorize_session(session_id)
        .await
        .map_err(map_auth_error)?;
    let note_id = Uuid::parse_str(&note_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid note id".to_owned(),
                hint: None,
            }),
        )
    })?;

    let blocks = state
        .note
        .list_blocks(session.user_id, note_id)
        .await
        .map_err(map_note_error)?;

    let json_blocks: Vec<serde_json::Value> = blocks
        .into_iter()
        .map(|b| serde_json::to_value(b).unwrap_or(serde_json::Value::Null))
        .collect();

    Ok(Json(NoteBlocksResponse { blocks: json_blocks }))
}

pub async fn create_note_block_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(note_id): Path<String>,
    Json(payload): Json<CreateBlockRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let session_id = session_id_from_cookie(&headers)?;
    let session = state
        .auth
        .authorize_session(session_id)
        .await
        .map_err(map_auth_error)?;
    let note_id = Uuid::parse_str(&note_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid note id".to_owned(),
                hint: None,
            }),
        )
    })?;

    let after_id = match &payload.after_id {
        None => None,
        Some(s) => Some(Uuid::parse_str(s).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid after_id".to_owned(),
                    hint: None,
                }),
            )
        })?),
    };

    let content = match payload.content {
        CreateBlockContent::Text { text } => Content::Text(editor::text::TextBlock::from_text(&text)),
    };

    let block = state
        .note
        .create_block(session.user_id, note_id, after_id, content)
        .await
        .map_err(map_note_error)?;

    let body = serde_json::to_value(block).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "failed to serialize block".to_owned(),
                hint: None,
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(body)))
}

pub async fn delete_note_block_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((note_id, block_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let session_id = session_id_from_cookie(&headers)?;
    let session = state
        .auth
        .authorize_session(session_id)
        .await
        .map_err(map_auth_error)?;
    let note_id = Uuid::parse_str(&note_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid note id".to_owned(),
                hint: None,
            }),
        )
    })?;
    let block_id = Uuid::parse_str(&block_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid block id".to_owned(),
                hint: None,
            }),
        )
    })?;

    state
        .note
        .delete_block(session.user_id, note_id, block_id)
        .await
        .map_err(map_note_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn patch_note_block_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((note_id, block_id)): Path<(String, String)>,
    Json(payload): Json<BlockPatchBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let session_id = session_id_from_cookie(&headers)?;
    let session = state
        .auth
        .authorize_session(session_id)
        .await
        .map_err(map_auth_error)?;
    let note_id = Uuid::parse_str(&note_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid note id".to_owned(),
                hint: None,
            }),
        )
    })?;
    let block_id = Uuid::parse_str(&block_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid block id".to_owned(),
                hint: None,
            }),
        )
    })?;

    match payload {
        BlockPatchBody::Move { after_id } => {
            let after_uuid = match after_id {
                None => None,
                Some(s) => Some(Uuid::parse_str(&s).map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: "invalid after_id".to_owned(),
                            hint: None,
                        }),
                    )
                })?),
            };
            state
                .note
                .move_block(session.user_id, note_id, block_id, after_uuid)
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
                    session.user_id,
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
                    session.user_id,
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
                    session.user_id,
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
                    session.user_id,
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
                    session.user_id,
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

    Ok(StatusCode::NO_CONTENT)
}
