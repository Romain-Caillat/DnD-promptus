//! The journey on the world map over HTTP (maps/travel-hex-world).
//!
//! GM (behind `require_gm`):
//! - `GET  /api/campaigns/{id}/travel` → the world map on the table, the
//!   party in full (the events drawn for the GM included), its places,
//!   the party members, the guide's portions, watches and supplies, the
//!   rules' abilities and difficulties for a group check; `null` when no
//!   world map is shown;
//! - `POST /api/campaigns/{id}/travel` → `travel::GmCommand`.
//!
//! Player (behind `require_player`, built by
//! `campaigns::projection::travel`):
//! - `GET  /api/play/{campaign}/travel` → the journey as I may see it;
//! - `POST /api/play/{campaign}/travel` → `travel::PlayerCommand`.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::auth::player::CurrentPlayer;
use crate::campaigns::projection::travel::project_travel;
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::players::Role;
use crate::state::AppState;
use crate::travel::{self, GmCommand, PlayerCommand};

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

async fn owned_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)
}

async fn gm_json(state: &AppState, row: &CampaignRow) -> Result<serde_json::Value, AppError> {
    let Some((map, guide, t)) = travel::on_table(&state.pool, row.id).await? else {
        return Ok(serde_json::Value::Null);
    };
    let members = travel::members(&state.pool, row.id).await?;
    let rules = row.rules();
    Ok(json!({
        "mapId": map.id,
        "mapName": map.name,
        "travel": t,
        "places": travel::places(&map),
        "members": members,
        "guide": {
            "portions": guide.portions,
            "watches": guide.watches,
            "supplies": guide.supplies.name,
            "perPersonPerDay": guide.supplies.per_person_per_day,
            "stepsPerPortion": guide.steps_per_portion,
        },
        "abilities": rules.map(|r| r.abilities.iter().map(|a| json!({ "id": a.id, "name": a.name })).collect::<Vec<_>>()),
        "difficulties": rules.map(|r| r.difficulties.iter().map(|d| json!({ "id": d.id, "name": d.name, "value": d.value })).collect::<Vec<_>>()),
    }))
}

/// `GET /api/campaigns/{id}/travel`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn gm_travel(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    Ok(Json(json!({ "data": gm_json(&state, &row).await? })).into_response())
}

/// `POST /api/campaigns/{id}/travel` — answers the GM view.
///
/// # Errors
///
/// The codes of `travel::gm`.
pub async fn gm_command(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(cmd): Body<GmCommand>,
) -> Result<Response, AppError> {
    travel::gm(&state.pool, &gm, parse_id(&id)?, &cmd).await?;
    let row = owned_row(&state, &gm, &id).await?;
    Ok(Json(json!({ "data": gm_json(&state, &row).await? })).into_response())
}

/// The journey as player `p` may see it (`null` when no world map is
/// shown, or the party has not been there).
async fn player_json(state: &AppState, p: &CurrentPlayer) -> Result<serde_json::Value, AppError> {
    let campaign = p.0.campaign_id;
    let Some((map, guide, Some(t))) = travel::on_table(&state.pool, campaign).await? else {
        return Ok(serde_json::Value::Null);
    };
    let members = travel::members(&state.pool, campaign).await?;
    let me = (p.0.role == Role::Player).then_some(p.0.id);
    let view = project_travel(&t, &map, &guide, &members, me);
    serde_json::to_value(view).map_err(|e| AppError::internal("travel view", e))
}

/// `GET /api/play/{campaign}/travel`
///
/// # Errors
///
/// 401 `NOT_JOINED`.
pub async fn player_travel(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    Ok(Json(json!({ "data": player_json(&state, &p).await? })).into_response())
}

/// `POST /api/play/{campaign}/travel` — answers the journey.
///
/// # Errors
///
/// The codes of `travel::player`.
pub async fn player_command(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(cmd): Body<PlayerCommand>,
) -> Result<Response, AppError> {
    travel::player(&state.pool, &p.0, &cmd).await?;
    Ok(Json(json!({ "data": player_json(&state, &p).await? })).into_response())
}
