//! GM routes that are about the account itself (all behind `require_gm`):
//! - `GET    /api/me` → the signed-in GM;
//! - `GET    /api/gm-tokens` → their live personal access tokens;
//! - `POST   /api/gm-tokens` → a new named token, secret included (201);
//! - `DELETE /api/gm-tokens/{id}` → revoke one of theirs (204).
//!
//! The token routes take [`SessionOnly`]: a token never mints a token.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::api_tokens;
use crate::auth::guard::{CurrentGm, SessionOnly};
use crate::error::AppError;
use crate::state::AppState;

/// `GET /api/me`
pub async fn me(gm: CurrentGm) -> Response {
    Json(json!({ "data": gm })).into_response()
}

/// `GET /api/gm-tokens`
///
/// # Errors
///
/// 403 `TOKEN_NOT_ALLOWED` for a token; a database error.
pub async fn list_tokens(
    State(state): State<AppState>,
    gm: CurrentGm,
    _session: SessionOnly,
) -> Result<Response, AppError> {
    let list = api_tokens::list(&state.pool, &gm).await?;
    Ok(Json(json!({ "data": list })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewToken {
    name: String,
}

/// `POST /api/gm-tokens`
///
/// # Errors
///
/// As `api_tokens::create`; 403 `TOKEN_NOT_ALLOWED` for a token.
pub async fn create_token(
    State(state): State<AppState>,
    gm: CurrentGm,
    _session: SessionOnly,
    Body(input): Body<NewToken>,
) -> Result<Response, AppError> {
    let minted = api_tokens::create(&state.pool, &gm, &input.name).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": minted }))).into_response())
}

/// `DELETE /api/gm-tokens/{id}`
///
/// # Errors
///
/// 404 `NOT_FOUND` when the token is not this GM's live one (a
/// malformed id included); 403 `TOKEN_NOT_ALLOWED` for a token.
pub async fn revoke_token(
    State(state): State<AppState>,
    gm: CurrentGm,
    _session: SessionOnly,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError::NotFound("NOT_FOUND"))?;
    api_tokens::revoke(&state.pool, &gm, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
