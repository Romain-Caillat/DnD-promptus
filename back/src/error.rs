use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Every error a handler can return. The wire shape is
/// `{ "error": { "code": "...", "message": "..." } }`: the front
/// dispatches on `code` and shows its own translated text — `message`
/// is for logs and humans reading the network tab, never for the UI.
///
/// Each variant carries its machine-readable code.
#[derive(Debug)]
pub enum AppError {
    /// 400 — the request itself is malformed or a field is invalid.
    BadRequest(&'static str),
    /// 400 — like `BadRequest`, with a detail the person must read to
    /// fix their input (the line and column of a YAML error).
    Invalid {
        code: &'static str,
        detail: String,
    },
    /// 401 — no valid GM session (missing, unknown, expired or signed
    /// out), or a passkey answer that does not verify.
    Unauthorized(&'static str),
    /// 403 — the caller is known (or holds a code) but is not allowed
    /// to do this, whatever the resource.
    Forbidden(&'static str),
    /// 404 — the resource does not exist **or belongs to another GM**:
    /// the two are indistinguishable on purpose (see `auth::guard`).
    NotFound(&'static str),
    /// 409 — the resource is not in a state that allows this (a sheet
    /// already sent to the GM, not waiting for review, or changed since
    /// it was read).
    Conflict(&'static str),
    /// A dependency the request needs (today: the database) is down, or
    /// the server refuses new work for a moment.
    ServiceUnavailable(&'static str),
    Internal(String),
}

impl AppError {
    pub fn internal(context: &str, e: impl std::fmt::Display) -> Self {
        Self::Internal(format!("{context}: {e}"))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(code) => (StatusCode::BAD_REQUEST, code, "invalid request".into()),
            Self::Invalid { code, detail } => (StatusCode::BAD_REQUEST, code, detail),
            Self::Unauthorized(code) => (StatusCode::UNAUTHORIZED, code, "not signed in".into()),
            Self::Forbidden(code) => (StatusCode::FORBIDDEN, code, "not allowed".into()),
            Self::NotFound(code) => (StatusCode::NOT_FOUND, code, "not found".into()),
            Self::Conflict(code) => (StatusCode::CONFLICT, code, "conflict".into()),
            Self::ServiceUnavailable(code) => (
                StatusCode::SERVICE_UNAVAILABLE,
                code,
                "a dependency is unavailable".into(),
            ),
            Self::Internal(detail) => {
                tracing::error!(%detail, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "internal error".to_string(),
                )
            }
        };
        (
            status,
            Json(json!({ "error": { "code": code, "message": message } })),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e.to_string())
    }
}
