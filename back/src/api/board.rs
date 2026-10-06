//! The grid and the fights over HTTP.
//!
//! GM (behind `require_gm`):
//! - `GET  /api/campaigns/{id}/board` → the map in full, the fog, every
//!   token, the maps to show, the fight with the co-GM's proposal, the
//!   loot and the events;
//! - `POST /api/campaigns/{id}/board` → `{ map }`: show a map;
//! - `POST /api/campaigns/{id}/board/edit` → `board::Edit`;
//! - `POST /api/campaigns/{id}/fight` → `{ node }`: open its encounter;
//! - `POST /api/campaigns/{id}/fight/command` → `board::fight::GmCommand`;
//! - `POST /api/campaigns/{id}/fight/loot` → `{ gives: [{ index, character }] }`.
//!
//! Player (behind `require_player`, built by
//! `campaigns::projection::board`):
//! - `GET  /api/play/{campaign}/board` → the grid as I may see it;
//! - `POST /api/play/{campaign}/board/walk` → `{ path }`;
//! - `POST /api/play/{campaign}/fight` → `board::fight::Command`.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use promptus_shared::maps::Cell;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::auth::player::CurrentPlayer;
use crate::board::fight::{self, Command, GmCommand};
use crate::board::rewards::{self, Give};
use crate::board::{self, Edit};
use crate::campaigns::projection::board::{Looker, project_board};
use crate::campaigns::{self, CampaignRow};
use crate::content;
use crate::error::AppError;
use crate::players::{self, CharacterStatus};
use crate::state::AppState;

/// How many events of a fight a screen receives.
const EVENTS: i64 = 60;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

async fn owned_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)
}

async fn gm_json(state: &AppState, row: &CampaignRow) -> Result<serde_json::Value, AppError> {
    let pool = &state.pool;
    let owned = fight::rules_of(row).ok();
    let rules = owned.as_ref();
    let b = board::current(pool, row.id).await?;
    let enc = fight::latest(pool, row.id).await?;
    let fight_json = match &enc {
        Some(e) => {
            let events = fight::events(pool, e.id, EVENTS).await?;
            let active = e.fight.active().map(str::to_string);
            let reach = match (rules, &active, e.live) {
                (Some(r), Some(a), true) => fight::reachable(r, &e.fight, a)
                    .into_iter()
                    .map(|(at, cost)| json!({ "at": at, "cost": cost }))
                    .collect(),
                _ => Vec::new(),
            };
            let max_hp: serde_json::Map<String, serde_json::Value> = e
                .fight
                .scene
                .combatants
                .iter()
                .filter_map(|(id, c)| {
                    let max = rules.and_then(|r| c.max_hit_points(r).ok())?;
                    Some((id.clone(), json!(max)))
                })
                .collect();
            json!({
                "id": e.id,
                "node": e.node,
                "live": e.live,
                "version": e.version,
                "fight": e.fight,
                "maxHitPoints": max_hp,
                "proposal": e.proposal,
                "loot": e.loot,
                "events": events,
                "reachable": reach,
                "startedAt": e.started_at,
                "endedAt": e.ended_at,
            })
        }
        None => serde_json::Value::Null,
    };
    let encounters: Vec<_> = row
        .story
        .nodes
        .iter()
        .filter(|n| n.encounter.is_some())
        .map(|n| json!({ "node": n.id, "title": n.title, "map": n.map }))
        .collect();
    Ok(json!({
        "board": b,
        "maps": board::choices(row),
        "encounters": encounters,
        "encounter": fight_json,
        "conditions": rules.map(|r| r.conditions.iter().map(|c| json!({ "id": c.id, "name": c.name })).collect::<Vec<_>>()),
    }))
}

/// `GET /api/campaigns/{id}/board`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn gm_board(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    Ok(Json(json!({ "data": gm_json(&state, &row).await? })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowBody {
    map: String,
}

/// `POST /api/campaigns/{id}/board` — answers the GM board.
///
/// # Errors
///
/// The codes of `board::show`.
pub async fn show(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(body): Body<ShowBody>,
) -> Result<Response, AppError> {
    let campaign = parse_id(&id)?;
    board::show(&state.pool, &gm, campaign, &body.map).await?;
    let row = owned_row(&state, &gm, &id).await?;
    Ok(Json(json!({ "data": gm_json(&state, &row).await? })).into_response())
}

/// `POST /api/campaigns/{id}/board/edit` — answers the GM board.
///
/// # Errors
///
/// The codes of `board::edit`.
pub async fn edit(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(e): Body<Edit>,
) -> Result<Response, AppError> {
    board::edit(&state.pool, &gm, parse_id(&id)?, &e).await?;
    let row = owned_row(&state, &gm, &id).await?;
    Ok(Json(json!({ "data": gm_json(&state, &row).await? })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartBody {
    node: String,
}

/// `POST /api/campaigns/{id}/fight` — answers the GM board (201).
///
/// # Errors
///
/// The codes of `board::fight::start`.
pub async fn start_fight(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(body): Body<StartBody>,
) -> Result<Response, AppError> {
    fight::start(&state.pool, &gm, parse_id(&id)?, &body.node).await?;
    let row = owned_row(&state, &gm, &id).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": gm_json(&state, &row).await? })),
    )
        .into_response())
}

/// `POST /api/campaigns/{id}/fight/command` — answers the GM board.
///
/// # Errors
///
/// The codes of `board::fight::gm_command`.
pub async fn gm_command(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(cmd): Body<GmCommand>,
) -> Result<Response, AppError> {
    fight::gm_command(&state.pool, &gm, parse_id(&id)?, &cmd).await?;
    let row = owned_row(&state, &gm, &id).await?;
    Ok(Json(json!({ "data": gm_json(&state, &row).await? })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootBody {
    gives: Vec<Give>,
}

/// `POST /api/campaigns/{id}/fight/loot` — answers the GM board.
///
/// # Errors
///
/// The codes of `board::rewards::give`.
pub async fn give_loot(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(body): Body<LootBody>,
) -> Result<Response, AppError> {
    rewards::give(&state.pool, &gm, parse_id(&id)?, &body.gives).await?;
    let row = owned_row(&state, &gm, &id).await?;
    Ok(Json(json!({ "data": gm_json(&state, &row).await? })).into_response())
}

/// The grid as player `p` may see it (`null` when no map is shown).
async fn player_json(state: &AppState, p: &CurrentPlayer) -> Result<serde_json::Value, AppError> {
    let pool = &state.pool;
    let row = campaigns::find(pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))?;
    let Some(b) = board::current(pool, row.id).await? else {
        return Ok(serde_json::Value::Null);
    };
    let character = players::character_of(pool, &p.0)
        .await?
        .filter(|c| c.status == CharacterStatus::Validated)
        .map(|c| c.id);
    let enc = fight::latest(pool, row.id).await?;
    let events = match &enc {
        Some(e) => fight::events(pool, e.id, EVENTS).await?,
        None => Vec::new(),
    };
    let view = project_board(
        &b,
        enc.as_ref().map(|e| (e, events.as_slice())),
        &Looker {
            character,
            rules: content::rule_system(&row.story.rules),
        },
    );
    serde_json::to_value(view).map_err(|e| AppError::internal("board view", e))
}

/// `GET /api/play/{campaign}/board`
///
/// # Errors
///
/// 401 `NOT_JOINED`.
pub async fn player_board(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    Ok(Json(json!({ "data": player_json(&state, &p).await? })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalkBody {
    path: Vec<Cell>,
}

/// `POST /api/play/{campaign}/board/walk` — answers the grid.
///
/// # Errors
///
/// The codes of `board::walk`.
pub async fn walk(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(body): Body<WalkBody>,
) -> Result<Response, AppError> {
    board::walk(&state.pool, &p.0, &body.path).await?;
    Ok(Json(json!({ "data": player_json(&state, &p).await? })).into_response())
}

/// `POST /api/play/{campaign}/fight` — answers the grid.
///
/// # Errors
///
/// The codes of `board::fight::command`.
pub async fn command(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(cmd): Body<Command>,
) -> Result<Response, AppError> {
    fight::command(&state.pool, &p.0, &cmd).await?;
    Ok(Json(json!({ "data": player_json(&state, &p).await? })).into_response())
}
