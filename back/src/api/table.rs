//! The GM's side of a campaign's table (all behind `require_gm`; another
//! GM's campaign answers 404 like a missing one):
//! - `GET    /api/campaigns/{id}/invite` → the active link's dates, or
//!   `null` (never the code);
//! - `POST   /api/campaigns/{id}/invite` → a new link, code included
//!   (201); the previous one stops working;
//! - `DELETE /api/campaigns/{id}/invite` → no link at all (204);
//! - `GET    /api/campaigns/{id}/players` → who joined, and where their
//!   character stands;
//! - `DELETE /api/campaigns/{id}/players/{player}` → remove someone
//!   from the table (204).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;
use crate::players;
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

/// The id of `raw`, a campaign `gm` owns.
async fn owned_campaign(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<Uuid, AppError> {
    let row = owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)?;
    Ok(row.id)
}

/// `GET /api/campaigns/{id}/invite`
///
/// # Errors
///
/// 404 when missing or another GM's; a database error.
pub async fn invite(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let id = owned_campaign(&state, &gm, &id).await?;
    let status = players::invite_status(&state.pool, id).await?;
    Ok(Json(json!({ "data": status })).into_response())
}

/// `POST /api/campaigns/{id}/invite`
///
/// # Errors
///
/// 404 when missing or another GM's; a database error.
pub async fn mint_invite(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let id = owned_campaign(&state, &gm, &id).await?;
    let minted = players::mint_invite(&state.pool, id).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": minted }))).into_response())
}

/// `DELETE /api/campaigns/{id}/invite`
///
/// # Errors
///
/// 404 when missing or another GM's; a database error.
pub async fn revoke_invite(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let id = owned_campaign(&state, &gm, &id).await?;
    players::revoke_invite(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `GET /api/campaigns/{id}/players`
///
/// # Errors
///
/// 404 when missing or another GM's; a database error.
pub async fn seats(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let id = owned_campaign(&state, &gm, &id).await?;
    let seats = players::seats(&state.pool, id).await?;
    Ok(Json(json!({ "data": seats })).into_response())
}

/// `DELETE /api/campaigns/{id}/players/{player}`
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, or the player does
/// not sit at its table; a database error.
pub async fn remove_player(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, player)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let id = owned_campaign(&state, &gm, &id).await?;
    players::remove(&state.pool, id, parse_id(&player)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
