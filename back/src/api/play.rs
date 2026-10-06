//! What players (and spectators) call. Every answer is built by
//! `campaigns::projection`, the single projection point (`MEMORY.md` §3).
//!
//! Public, with an invitation code (`app::invitation_routes`):
//! - `GET  /api/join/{code}` → the campaign the link opens (title, hook,
//!   GM's name);
//! - `POST /api/join/{code}` → `{ nickname, role }`: take a seat (201),
//!   the device token in the campaign's player cookie.
//!
//! Player, behind `require_player` (`app::player_routes`):
//! - `GET /api/play/{campaign}/me` → who I am at this table, the
//!   campaign's header and my character;
//! - `GET /api/play/{campaign}/view` → the campaign as players see it now;
//! - `GET /api/play/{campaign}/creation` → what the character creator
//!   offers: the sprite pack, a starting look, what the rules ask;
//! - `PUT /api/play/{campaign}/character` → save my draft sheet;
//! - `POST /api/play/{campaign}/character/backstory` → `{ origin, loss,
//!   quest }`: the co-GM's paragraph from my three answers;
//! - `POST /api/play/{campaign}/character/submit` → send it to the GM;
//! - `POST /api/play/{campaign}/character/equip` → `{ entry, equipped }`:
//!   carry a bag line on the character, or put it back in the bag, once
//!   in play (`players::play::equip`);
//! - `POST /api/play/{campaign}/character/level-up` → `{ level, choice }`:
//!   take a reached level's hit points, rolled or at the average;
//! - `POST /api/play/{campaign}/character/upgrade` → `{ ability }`: spend
//!   an upgrade point (`engine/level-up`).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use super::body::Body;
use crate::auth::player::{self, CurrentPlayer};
use crate::campaigns::{self, CampaignRow, projection};
use crate::content;
use crate::error::AppError;
use crate::players::{self, CharacterSheet, Role};
use crate::state::AppState;
use promptus_shared::rules::progression::HitPointChoice;

fn invite_not_found() -> AppError {
    AppError::NotFound("INVITE_NOT_FOUND")
}

/// The campaign behind a usable invitation code.
async fn invited_campaign(state: &AppState, code: &str) -> Result<CampaignRow, AppError> {
    let id = players::campaign_by_invite(&state.pool, code)
        .await?
        .ok_or_else(invite_not_found)?;
    campaigns::find(&state.pool, id)
        .await?
        .ok_or_else(invite_not_found)
}

/// The campaign of the player behind the request. It exists: deleting a
/// campaign deletes its players.
async fn campaign_of(state: &AppState, p: &CurrentPlayer) -> Result<CampaignRow, AppError> {
    campaigns::find(&state.pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))
}

/// `GET /api/join/{code}`
///
/// # Errors
///
/// 404 `INVITE_NOT_FOUND` for a wrong, revoked or expired code.
pub async fn invitation(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Response, AppError> {
    let row = invited_campaign(&state, &code).await?;
    let gm_name = campaigns::gm_name(&state.pool, &row).await?;
    let view = projection::project_invitation(row.id, &row.story, &gm_name);
    Ok(Json(json!({ "data": view })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinBody {
    nickname: String,
    role: Role,
}

/// `POST /api/join/{code}`
///
/// # Errors
///
/// 404 `INVITE_NOT_FOUND`; 400 `INVALID_NICKNAME`, `NICKNAME_TAKEN`,
/// `INVALID_BODY`.
pub async fn join(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Body(body): Body<JoinBody>,
) -> Result<Response, AppError> {
    let row = invited_campaign(&state, &code).await?;
    let (joined, token) = players::join(&state.pool, row.id, &body.nickname, body.role).await?;
    let character = players::character_of(&state.pool, &joined).await?;
    let gm_name = campaigns::gm_name(&state.pool, &row).await?;
    let view = projection::project_home(
        &row.story,
        &gm_name,
        &joined,
        character.as_ref(),
        row.rules(),
    );
    Ok((
        StatusCode::CREATED,
        [(
            header::SET_COOKIE,
            player::set_cookie(row.id, &token, state.auth.secure_cookie),
        )],
        Json(json!({ "data": view })),
    )
        .into_response())
}

/// `GET /api/play/{campaign}/me`
///
/// # Errors
///
/// 401 `NOT_JOINED` (from the guard); a database error.
pub async fn me(State(state): State<AppState>, p: CurrentPlayer) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let gm_name = campaigns::gm_name(&state.pool, &row).await?;
    let character = players::character_of(&state.pool, &p.0).await?;
    let view =
        projection::project_home(&row.story, &gm_name, &p.0, character.as_ref(), row.rules());
    Ok(Json(json!({ "data": view })).into_response())
}

/// `GET /api/play/{campaign}/view`
///
/// # Errors
///
/// 401 `NOT_JOINED` (from the guard); a database error.
pub async fn view(State(state): State<AppState>, p: CurrentPlayer) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let view = projection::project_for_players(&row.story, &row.world);
    Ok(Json(json!({ "data": view })).into_response())
}

/// `GET /api/play/{campaign}/creation`
///
/// # Errors
///
/// 401 `NOT_JOINED` (from the guard); a database error.
pub async fn creation(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let view = projection::project_creation(
        &content::pack_for(&row.story).id,
        content::start_look(&row.story),
        row.rules(),
    );
    Ok(Json(json!({ "data": view })).into_response())
}

/// `PUT /api/play/{campaign}/character` → the whole sheet: save the
/// draft. Answers the character as `me` shows it.
///
/// # Errors
///
/// 401 `NOT_JOINED`; 404 `NO_CHARACTER` for a spectator; 409
/// `CHARACTER_LOCKED` once sent to the GM; 400 `INVALID_BODY` or the
/// code of [`CharacterSheet::cleaned`].
pub async fn save_character(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(sheet): Body<CharacterSheet>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let rules = row.rules();
    let sheet = sheet.cleaned(rules, content::packs(), &content::pack_for(&row.story).id)?;
    let character = players::save_sheet(&state.pool, &p.0, &sheet).await?;
    let view = projection::project_character(rules, &character);
    Ok(Json(json!({ "data": view })).into_response())
}

/// `POST /api/play/{campaign}/character/backstory` — the co-GM writes
/// the player's three answers up as a paragraph (`{ text }`), which the
/// player keeps, edits and saves in their draft.
///
/// # Errors
///
/// See [`players::assist::write_backstory`].
pub async fn write_backstory(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(answers): Body<players::assist::Answers>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let text =
        players::assist::write_backstory(&state.pool, &state.ai, &row, &p.0, &answers).await?;
    Ok(Json(json!({ "data": { "text": text } })).into_response())
}

/// `POST /api/play/{campaign}/character/submit` — send the character to
/// the GM. Answers the character as `me` shows it.
///
/// # Errors
///
/// 401 `NOT_JOINED`; 404 `NO_CHARACTER`; 409 `CHARACTER_LOCKED`; 400
/// `CHARACTER_INCOMPLETE`.
pub async fn submit_character(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let rules = row.rules();
    let character = players::submit_character(&state.pool, &p.0, rules).await?;
    let view = projection::project_character(rules, &character);
    Ok(Json(json!({ "data": view })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipBody {
    /// The bag line's `key`.
    entry: String,
    equipped: bool,
}

/// `POST /api/play/{campaign}/character/equip` — answers the character
/// as `me` shows it.
///
/// # Errors
///
/// 401 `NOT_JOINED`; 404 `NO_CHARACTER`, `NO_SUCH_ENTRY`; 409
/// `CHARACTER_NOT_VALIDATED`, `RULES_UNKNOWN`; 400 `INVALID_BODY`.
pub async fn equip(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(body): Body<EquipBody>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let rules = row.rules();
    let Some(system) = rules else {
        // Without the rules nothing is in play; a spectator still learns
        // they have no character.
        players::character_of(&state.pool, &p.0)
            .await?
            .ok_or(AppError::NotFound("NO_CHARACTER"))?;
        return Err(AppError::Conflict("RULES_UNKNOWN"));
    };
    players::play::equip(&state.pool, &p.0, system, &body.entry, body.equipped).await?;
    let character = players::character_of(&state.pool, &p.0)
        .await?
        .ok_or(AppError::NotFound("NO_CHARACTER"))?;
    let view = projection::project_character(rules, &character);
    Ok(Json(json!({ "data": view })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelUpBody {
    level: u32,
    choice: HitPointChoice,
}

/// `POST /api/play/{campaign}/character/level-up` — `{ level, choice:
/// roll | average }`: takes a reached level's hit points, the server
/// rolling the die. Answers `{ taken, character }`: the roll, the
/// maximum before and after, and the character as `me` shows it.
///
/// # Errors
///
/// 401 `NOT_JOINED`; 404 `NO_CHARACTER`; 409 `CHARACTER_NOT_VALIDATED`,
/// `RULES_UNKNOWN`, `NO_LEVEL_HIT_POINTS`, `LEVEL_NOT_REACHED`,
/// `LEVEL_ALREADY_TAKEN`; 400 `INVALID_BODY`.
pub async fn level_up(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(body): Body<LevelUpBody>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let rules = row.rules();
    let system = in_play_rules(&state, &p, rules).await?;
    let taken = players::play::level_up(&state.pool, &p.0, system, body.level, body.choice).await?;
    let character = players::character_of(&state.pool, &p.0)
        .await?
        .ok_or(AppError::NotFound("NO_CHARACTER"))?;
    let view = projection::project_character(rules, &character);
    Ok(Json(json!({ "data": { "taken": {
        "level": taken.level,
        "die": taken.die,
        "faces": taken.roll.as_ref().map(|r| r.faces.clone()),
        "maxBefore": taken.max_before,
        "maxAfter": taken.max_after,
    }, "character": view } }))
    .into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeBody {
    ability: String,
}

/// `POST /api/play/{campaign}/character/upgrade` — `{ ability }`: one
/// upgrade point on an ability. Answers the character as `me` shows it.
///
/// # Errors
///
/// 401 `NOT_JOINED`; 404 `NO_CHARACTER`; 409 `CHARACTER_NOT_VALIDATED`,
/// `RULES_UNKNOWN`, `NO_UPGRADE_POINT`; 400 `UNKNOWN_ABILITY`,
/// `INVALID_BODY`.
pub async fn upgrade(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(body): Body<UpgradeBody>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let rules = row.rules();
    let system = in_play_rules(&state, &p, rules).await?;
    players::play::upgrade(&state.pool, &p.0, system, &body.ability).await?;
    let character = players::character_of(&state.pool, &p.0)
        .await?
        .ok_or(AppError::NotFound("NO_CHARACTER"))?;
    let view = projection::project_character(rules, &character);
    Ok(Json(json!({ "data": view })).into_response())
}

/// The rules a character plays by; without them nothing is in play, and
/// a spectator still learns they have no character.
async fn in_play_rules<'r>(
    state: &AppState,
    p: &CurrentPlayer,
    rules: Option<&'r promptus_shared::rules::RuleSystem>,
) -> Result<&'r promptus_shared::rules::RuleSystem, AppError> {
    if let Some(system) = rules {
        return Ok(system);
    }
    players::character_of(&state.pool, &p.0)
        .await?
        .ok_or(AppError::NotFound("NO_CHARACTER"))?;
    Err(AppError::Conflict("RULES_UNKNOWN"))
}
