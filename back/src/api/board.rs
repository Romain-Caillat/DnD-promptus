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
//! - `POST /api/campaigns/{id}/fight/loot` → `{ gives: [{ index, character }] }`;
//! - `POST /api/campaigns/{id}/battle` → `{ node }`: open its ship battle;
//! - `POST /api/campaigns/{id}/battle/command` → `board::battle::GmCommand`.
//!
//! Player (behind `require_player`, built by
//! `campaigns::projection::board`):
//! - `GET  /api/play/{campaign}/board` → the grid as I may see it;
//! - `POST /api/play/{campaign}/board/walk` → `{ path }`;
//! - `POST /api/play/{campaign}/fight` → `board::fight::Command`;
//! - `GET  /api/play/{campaign}/battle` → the ship battle as I may see it
//!   (`campaigns::projection::battle`);
//! - `POST /api/play/{campaign}/battle` → `board::battle::CrewCommand`.

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
use crate::board::battle::{self, CrewCommand, GmCommand as BattleCommand};
use crate::board::fight::{self, Command, GmCommand};
use crate::board::rewards::{self, Give};
use crate::board::{self, Edit};
use crate::campaigns::projection::battle::{options, project_battle};
use crate::campaigns::projection::board::{Looker, project_board};
use crate::campaigns::{self, CampaignRow};
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
        // A ship battle opens from its own panel; its deck fight only by
        // boarding.
        .filter(|n| n.encounter.as_ref().is_some_and(|e| e.vehicles.is_none()))
        .map(|n| json!({ "node": n.id, "title": n.title, "map": n.map }))
        .collect();
    let battles: Vec<_> = row
        .story
        .nodes
        .iter()
        .filter(|n| n.encounter.as_ref().is_some_and(|e| e.vehicles.is_some()))
        .map(|n| json!({ "node": n.id, "title": n.title }))
        .collect();
    Ok(json!({
        "board": b,
        "maps": board::choices(&state.pool, row).await?,
        "encounters": encounters,
        "encounter": fight_json,
        "battles": battles,
        "battle": battle_json(state, row, rules).await?,
        "conditions": rules.map(|r| r.conditions.iter().map(|c| json!({ "id": c.id, "name": c.name })).collect::<Vec<_>>()),
    }))
}

/// The ship battle from the GM's side: the whole state, every ship's
/// gauges, the co-GM's proposal, what each crew member can do (to play
/// the ship's holder), where the active enemy ships may go and what
/// they can fire at.
async fn battle_json(
    state: &AppState,
    row: &CampaignRow,
    rules: Option<&promptus_shared::rules::RuleSystem>,
) -> Result<serde_json::Value, AppError> {
    let Some(b) = battle::latest(&state.pool, row.id).await? else {
        return Ok(serde_json::Value::Null);
    };
    let events = battle::events(&state.pool, b.id, EVENTS).await?;
    let view = project_battle(&b, &[], rules, None);
    let crew_options: serde_json::Map<String, serde_json::Value> = match rules {
        Some(r) => b
            .battle
            .crew
            .iter()
            .map(|c| (c.id.clone(), json!(options(r, &b.battle, &c.id))))
            .collect(),
        None => serde_json::Map::new(),
    };
    let enemy_reach: serde_json::Map<String, serde_json::Value> = rules
        .map(|r| battle::enemy_reach(r, &b.battle))
        .unwrap_or_default()
        .into_iter()
        .map(|(ship, cells)| {
            let cells: Vec<_> = cells.into_iter().map(|(at, _)| at).collect();
            (ship, json!(cells))
        })
        .collect();
    let enemy_fire = rules.map(|r| battle::enemy_fire_options(r, &b.battle));
    Ok(json!({
        "id": b.id,
        "node": b.node,
        "status": b.status,
        "version": b.version,
        "battle": b.battle,
        "view": view,
        "proposal": b.proposal.as_ref().map(|p| json!({
            "unit": p.unit,
            "version": p.version,
            "orders": p.orders,
            "events": p.events,
        })),
        "events": events,
        "crewOptions": crew_options,
        "enemyReach": enemy_reach,
        "enemyFire": enemy_fire,
        "boardingEncounter": b.boarding_encounter,
        "startedAt": b.started_at,
        "endedAt": b.ended_at,
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
            rules: row.rules(),
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

/// `POST /api/campaigns/{id}/battle` — answers the GM board (201).
///
/// # Errors
///
/// The codes of `board::battle::start`.
pub async fn start_battle(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(body): Body<StartBody>,
) -> Result<Response, AppError> {
    battle::start(&state.pool, &gm, parse_id(&id)?, &body.node).await?;
    let row = owned_row(&state, &gm, &id).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": gm_json(&state, &row).await? })),
    )
        .into_response())
}

/// `POST /api/campaigns/{id}/battle/command` — answers the GM board.
///
/// # Errors
///
/// The codes of `board::battle::gm_command`.
pub async fn battle_command(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(cmd): Body<BattleCommand>,
) -> Result<Response, AppError> {
    battle::gm_command(&state.pool, &gm, parse_id(&id)?, &cmd).await?;
    let row = owned_row(&state, &gm, &id).await?;
    Ok(Json(json!({ "data": gm_json(&state, &row).await? })).into_response())
}

/// The ship battle as player `p` may see it (`null` when none was
/// fought).
async fn player_battle_json(
    state: &AppState,
    p: &CurrentPlayer,
) -> Result<serde_json::Value, AppError> {
    let pool = &state.pool;
    let row = campaigns::find(pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))?;
    let Some(b) = battle::latest(pool, row.id).await? else {
        return Ok(serde_json::Value::Null);
    };
    let me = players::character_of(pool, &p.0)
        .await?
        .filter(|c| c.status == CharacterStatus::Validated && p.0.role == players::Role::Player)
        .map(|c| board::character_token(c.id));
    let events = battle::events(pool, b.id, EVENTS).await?;
    let rules = fight::rules_of(&row).ok();
    let view = project_battle(&b, &events, rules.as_ref(), me.as_deref());
    serde_json::to_value(view).map_err(|e| AppError::internal("battle view", e))
}

/// `GET /api/play/{campaign}/battle`
///
/// # Errors
///
/// 401 `NOT_JOINED`.
pub async fn player_battle(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    Ok(Json(json!({ "data": player_battle_json(&state, &p).await? })).into_response())
}

/// `POST /api/play/{campaign}/battle` — answers the battle.
///
/// # Errors
///
/// The codes of `board::battle::command`.
pub async fn crew_command(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(cmd): Body<CrewCommand>,
) -> Result<Response, AppError> {
    battle::command(&state.pool, &p.0, &cmd).await?;
    Ok(Json(json!({ "data": player_battle_json(&state, &p).await? })).into_response())
}
