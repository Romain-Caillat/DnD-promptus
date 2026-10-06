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
//! - `POST /api/play/{campaign}/character/submit` → send it to the GM.

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
        content::rule_system(&row.story.rules),
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
    let view = projection::project_home(
        &row.story,
        &gm_name,
        &p.0,
        character.as_ref(),
        content::rule_system(&row.story.rules),
    );
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
        content::rule_system(&row.story.rules),
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
    let rules = content::rule_system(&row.story.rules);
    let sheet = sheet.cleaned(rules, content::packs(), &content::pack_for(&row.story).id)?;
    let character = players::save_sheet(&state.pool, &p.0, &sheet).await?;
    let view = projection::project_character(rules, &character);
    Ok(Json(json!({ "data": view })).into_response())
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
    let rules = content::rule_system(&row.story.rules);
    let character = players::submit_character(&state.pool, &p.0, rules).await?;
    let view = projection::project_character(rules, &character);
    Ok(Json(json!({ "data": view })).into_response())
}
