//! GM routes that are about the account itself (all behind `require_gm`):
//! - `GET    /api/me` → the signed-in GM;
//! - `GET    /api/gm-invites` → their pending invitations;
//! - `POST   /api/gm-invites` → a new invitation, code included (201);
//! - `DELETE /api/gm-invites/{id}` → revoke one of theirs (204).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use uuid::Uuid;

use crate::auth::guard::CurrentGm;
use crate::auth::invites;
use crate::error::AppError;
use crate::state::AppState;

/// `GET /api/me`
pub async fn me(gm: CurrentGm) -> Response {
    Json(json!({ "data": gm })).into_response()
}

/// `GET /api/gm-invites`
///
/// # Errors
///
/// Fails on a database error.
pub async fn list_invites(
    State(state): State<AppState>,
    gm: CurrentGm,
) -> Result<Response, AppError> {
    let list = invites::list_pending(&state.pool, &gm).await?;
    Ok(Json(json!({ "data": list })).into_response())
}

/// `POST /api/gm-invites`
///
/// # Errors
///
/// Fails on a database error.
pub async fn create_invite(
    State(state): State<AppState>,
    gm: CurrentGm,
) -> Result<Response, AppError> {
    let minted = invites::create(&state.pool, &gm).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": minted }))).into_response())
}

/// `DELETE /api/gm-invites/{id}`
///
/// # Errors
///
/// 404 `NOT_FOUND` when the invitation is not this GM's pending one
/// (a malformed id included).
pub async fn revoke_invite(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError::NotFound("NOT_FOUND"))?;
    invites::revoke(&state.pool, &gm, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
