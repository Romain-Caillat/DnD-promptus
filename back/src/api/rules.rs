//! The campaign's rules, for a seated player or spectator
//! (`player/read-the-rules`), behind `require_player`
//! (`app::player_routes`). Built by `campaigns::projection::rules`.
//!
//! - `GET  /api/play/{campaign}/rules` → the rules page: the rules as
//!   data, my own numbers when my sheet has a class, and what changed
//!   since the version I last read;
//! - `POST /api/play/{campaign}/rules/seen` → I read this version: the
//!   changes are gone. Answers the page.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::auth::player::CurrentPlayer;
use crate::campaigns::projection::rules::project_rules;
use crate::campaigns::{self, CampaignRow, rules_seen};
use crate::error::AppError;
use crate::players;
use crate::state::AppState;

async fn page(
    state: &AppState,
    p: &CurrentPlayer,
    row: &CampaignRow,
) -> Result<Response, AppError> {
    let current = &row.story.rules;
    let system = row.rules().ok_or(AppError::NotFound("RULES_NOT_FOUND"))?;
    let character = players::character_of(&state.pool, &p.0).await?;
    let seen = rules_seen::seen(&state.pool, p.0.id).await?;
    let changes = rules_seen::changes(&state.pool, row.id, seen.as_ref(), current, system).await?;
    let view = project_rules(system, character.as_ref().map(|c| &c.sheet), changes);
    Ok(Json(json!({ "data": view })).into_response())
}

async fn campaign_of(state: &AppState, p: &CurrentPlayer) -> Result<CampaignRow, AppError> {
    campaigns::find(&state.pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))
}

/// `GET /api/play/{campaign}/rules`
///
/// # Errors
///
/// 401 `NOT_JOINED` (from the guard); 404 `RULES_NOT_FOUND` when the
/// server does not have the campaign's rule system; a database error.
pub async fn rules(State(state): State<AppState>, p: CurrentPlayer) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    page(&state, &p, &row).await
}

/// `POST /api/play/{campaign}/rules/seen`
///
/// # Errors
///
/// As [`rules`].
pub async fn mark_seen(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    if row.rules().is_none() {
        return Err(AppError::NotFound("RULES_NOT_FOUND"));
    }
    rules_seen::mark_seen(&state.pool, p.0.id, &row.story.rules).await?;
    page(&state, &p, &row).await
}
