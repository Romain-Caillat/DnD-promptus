use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Every error a handler can return. The wire shape is
/// `{ "error": { "code": "...", "message": "..." } }`: the front
/// dispatches on `code` and shows its own translated text — `message`
/// is for logs and humans reading the network tab, never for the UI.
#[derive(Debug)]
pub enum AppError {
    /// A dependency the request needs (today: the database) is down.
    /// The body string is the machine-readable code.
    ServiceUnavailable(&'static str),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::ServiceUnavailable(code) => (
                StatusCode::SERVICE_UNAVAILABLE,
                code,
                "a dependency is unavailable".to_string(),
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
