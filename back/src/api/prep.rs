//! Preparing a campaign (`campaign/review-story-graph`), behind
//! `require_gm`:
//! - `POST /api/campaigns/{id}/story/edits` → the GM's edits by id
//!   (`story::edit::Edit`), applied whole; answers the campaign and
//!   what changed;
//! - `GET  /api/campaigns/{id}/readiness` → one gauge per act;
//! - `POST /api/campaigns/{id}/story/validate` → declare it playable;
//! - `GET  /api/campaigns/{id}/workshop` → the co-GM's proposals;
//! - `POST /api/campaigns/{id}/workshop` → ask the co-GM (201);
//! - `POST /api/campaigns/{id}/workshop/{proposal}/accept|reject`.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use promptus_shared::story::edit::Edit;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use super::campaigns::detail;
use crate::auth::guard::CurrentGm;
use crate::error::AppError;
use crate::prep::{self, workshop};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditsBody {
    edits: Vec<Edit>,
}

/// `POST /api/campaigns/{id}/story/edits`
///
/// # Errors
///
/// As `prep::apply`.
pub async fn edits(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(input): Body<EditsBody>,
) -> Result<Response, AppError> {
    let (row, changes) = prep::apply(&state.pool, &gm, parse_id(&id)?, &input.edits).await?;
    Ok(Json(json!({ "data": { "campaign": detail(row), "changes": changes } })).into_response())
}

/// `POST /api/campaigns/{id}/story/validate`
///
/// # Errors
///
/// As `prep::validate_campaign`.
pub async fn validate(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = prep::validate_campaign(&state.pool, &gm, parse_id(&id)?).await?;
    Ok(Json(json!({ "data": detail(row) })).into_response())
}

/// `GET /api/campaigns/{id}/readiness`
///
/// # Errors
///
/// As `prep::act_readiness`.
pub async fn readiness(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let acts = prep::act_readiness(&state.pool, &gm, parse_id(&id)?).await?;
    Ok(Json(json!({ "data": acts })).into_response())
}

/// `GET /api/campaigns/{id}/workshop`
///
/// # Errors
///
/// As `workshop::list`.
pub async fn proposals(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let list = workshop::list(&state.pool, &gm, parse_id(&id)?).await?;
    Ok(Json(json!({ "data": list })).into_response())
}

/// `POST /api/campaigns/{id}/workshop`
///
/// # Errors
///
/// As `workshop::ask`.
pub async fn ask(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(input): Body<workshop::Ask>,
) -> Result<Response, AppError> {
    let p = workshop::ask(&state.pool, &state.ai, &gm, parse_id(&id)?, &input).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": p }))).into_response())
}

async fn decide(
    state: &AppState,
    gm: &CurrentGm,
    id: &str,
    proposal: &str,
    accept: bool,
) -> Result<Response, AppError> {
    let proposal = Uuid::parse_str(proposal).map_err(|_| AppError::NotFound("NO_SUCH_PROPOSAL"))?;
    let (row, p) = workshop::decide(&state.pool, gm, parse_id(id)?, proposal, accept).await?;
    Ok(Json(json!({ "data": { "campaign": detail(row), "proposal": p } })).into_response())
}

/// `POST /api/campaigns/{id}/workshop/{proposal}/accept`
///
/// # Errors
///
/// As `workshop::decide`.
pub async fn accept(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, proposal)): Path<(String, String)>,
) -> Result<Response, AppError> {
    decide(&state, &gm, &id, &proposal, true).await
}

/// `POST /api/campaigns/{id}/workshop/{proposal}/reject`
///
/// # Errors
///
/// As `workshop::decide`.
pub async fn reject(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, proposal)): Path<(String, String)>,
) -> Result<Response, AppError> {
    decide(&state, &gm, &id, &proposal, false).await
}
