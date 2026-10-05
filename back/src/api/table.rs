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
//!
//! The review of the sheets (`session/validate-characters`,
//! `players::review`):
//! - `GET  /api/campaigns/{id}/characters/{character}` → the sheet with
//!   the rules' checks, and what changed since the last review;
//! - `POST /api/campaigns/{id}/characters/{character}/validate` →
//!   `{ seen }` (the `updatedAt` the GM read): validated (204);
//! - `POST /api/campaigns/{id}/characters/{character}/return` →
//!   `{ seen, note }`: back to the player with a word (204);
//! - `GET  /api/campaigns/{id}/hooks` → the secret hooks, and the nodes
//!   and fronts they can tie into;
//! - `POST /api/campaigns/{id}/hooks` → `{ characterId, title, body,
//!   links }` (201);
//! - `PUT  /api/campaigns/{id}/hooks/{hook}` → `{ title, body, links }`;
//! - `DELETE /api/campaigns/{id}/hooks/{hook}` (204).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::players::{self, review};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

/// The campaign `raw`, which `gm` owns.
async fn owned_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)
}

/// The id of `raw`, a campaign `gm` owns.
async fn owned_campaign(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<Uuid, AppError> {
    Ok(owned_row(state, gm, raw).await?.id)
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
    let row = owned_row(&state, &gm, &id).await?;
    let mut seats = players::seats(&state.pool, row.id).await?;
    for c in seats.iter_mut().filter_map(|s| s.character.as_mut()) {
        c.class_name = review::class_name(&row.story, c.class_id.as_deref());
    }
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

/// `GET /api/campaigns/{id}/characters/{character}`
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, or the character
/// does not sit at its table; a database error.
pub async fn character(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, character)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let review = review::review(&state.pool, &row, parse_id(&character)?).await?;
    Ok(Json(json!({ "data": review })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidateBody {
    /// The `updatedAt` of the sheet the GM read.
    seen: DateTime<Utc>,
}

/// `POST /api/campaigns/{id}/characters/{character}/validate`
///
/// # Errors
///
/// 404 as [`character`]; 409 `CHARACTER_NOT_SUBMITTED`,
/// `CHARACTER_CHANGED`; 400 `INVALID_BODY`.
pub async fn validate_character(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, character)): Path<(String, String)>,
    Body(body): Body<ValidateBody>,
) -> Result<Response, AppError> {
    let id = owned_campaign(&state, &gm, &id).await?;
    review::decide(
        &state.pool,
        id,
        parse_id(&character)?,
        body.seen,
        review::Decision::Validate,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReturnBody {
    seen: DateTime<Utc>,
    /// The word to the player.
    note: String,
}

/// `POST /api/campaigns/{id}/characters/{character}/return`
///
/// # Errors
///
/// As [`validate_character`]; 400 `NOTE_REQUIRED`, `NOTE_TOO_LONG`.
pub async fn return_character(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, character)): Path<(String, String)>,
    Body(body): Body<ReturnBody>,
) -> Result<Response, AppError> {
    let id = owned_campaign(&state, &gm, &id).await?;
    let note = review::clean_note(&body.note)?;
    review::decide(
        &state.pool,
        id,
        parse_id(&character)?,
        body.seen,
        review::Decision::Return(note),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `GET /api/campaigns/{id}/hooks`
///
/// # Errors
///
/// 404 when missing or another GM's; a database error.
pub async fn hooks(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let hooks = review::hooks(&state.pool, row.id).await?;
    Ok(Json(json!({
        "data": { "hooks": hooks, "targets": review::hook_targets(&row.story) }
    }))
    .into_response())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewHook {
    character_id: Uuid,
    title: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    links: Vec<String>,
}

/// `POST /api/campaigns/{id}/hooks`
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, or the character
/// does not sit at its table; 400 `HOOK_TITLE_REQUIRED`,
/// `HOOK_TOO_LONG`, `HOOK_LINK_UNKNOWN`, `INVALID_BODY`.
pub async fn add_hook(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(body): Body<NewHook>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let input = review::HookInput {
        title: body.title,
        body: body.body,
        links: body.links,
    };
    let hook = review::add_hook(&state.pool, &row, body.character_id, input).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": hook }))).into_response())
}

/// `PUT /api/campaigns/{id}/hooks/{hook}`
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, or the hook is not
/// one of its; the 400s of [`add_hook`].
pub async fn edit_hook(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, hook)): Path<(String, String)>,
    Body(body): Body<review::HookInput>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let hook = review::edit_hook(&state.pool, &row, parse_id(&hook)?, body).await?;
    Ok(Json(json!({ "data": hook })).into_response())
}

/// `DELETE /api/campaigns/{id}/hooks/{hook}`
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, or the hook is not
/// one of its.
pub async fn delete_hook(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, hook)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let id = owned_campaign(&state, &gm, &id).await?;
    review::delete_hook(&state.pool, id, parse_id(&hook)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
