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
use crate::note::{CreateNoteInput, DeleteNoteInput, NoteError, NoteUsecase};

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
    pub body: String,
}

#[derive(Serialize)]
pub struct NoteResponse {
    pub id: String,
    pub title: String,
    pub body: String,
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
        body: note.body,
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
    }
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
            body: payload.body,
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
