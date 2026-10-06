//! The shared screen (`session/pair-shared-screen`, `gm/launch-session`).
//!
//! Public — a TV has no account and no seat yet:
//! - `POST /api/tv/pairings` → `{ code, secret }`: the code to show;
//! - `GET /api/tv/pairings/{secret}` → `{ state: "waiting", code }`, or
//!   `{ state: "paired", campaign }` with the seat's cookie, once;
//! - `GET /api/tv/pairings/{secret}/qr.svg` → the QR of the address
//!   where the GM pairs it.
//!
//! Once seated, a screen is a spectator: it calls the player routes and
//! sees what the player projection gives a spectator.
//!
//! GM, behind `require_gm`:
//! - `GET /api/campaigns/{id}/tv` → the screens seated;
//! - `POST /api/campaigns/{id}/tv` → `{ code }`: pair the TV showing it;
//! - `POST /api/campaigns/{id}/tv/window` → `{ secret }`: a window to
//!   share, paired at once;
//! - `DELETE /api/campaigns/{id}/tv/{screen}` → forget a screen;
//! - `POST /api/campaigns/{id}/session/reading` → `{ line }` (or null):
//!   how many lines of « Précédemment… » the table sees.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::auth::player;
use crate::campaigns;
use crate::error::AppError;
use crate::evening::scenes;
use crate::players::screens::{self, PairingState};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

async fn gm_campaign(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<Uuid, AppError> {
    let row = owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)?;
    Ok(row.id)
}

/// `POST /api/tv/pairings`
///
/// # Errors
///
/// 409 `TV_BUSY` when no free code was found; a database error.
pub async fn start(State(state): State<AppState>) -> Result<Response, AppError> {
    let pairing = screens::start(&state.pool).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": pairing }))).into_response())
}

/// `GET /api/tv/pairings/{secret}`
///
/// # Errors
///
/// 404 `TV_PAIRING_UNKNOWN`.
pub async fn poll(
    State(state): State<AppState>,
    Path(secret): Path<String>,
) -> Result<Response, AppError> {
    Ok(match screens::poll(&state.pool, &secret).await? {
        PairingState::Waiting { code } => {
            Json(json!({ "data": { "state": "waiting", "code": code } })).into_response()
        }
        PairingState::Paired { campaign, token } => (
            [(
                header::SET_COOKIE,
                player::set_cookie(campaign, &token, state.auth.secure_cookie),
            )],
            Json(json!({ "data": { "state": "paired", "campaign": campaign } })),
        )
            .into_response(),
    })
}

/// `GET /api/tv/pairings/{secret}/qr.svg`
///
/// # Errors
///
/// 404 `TV_PAIRING_UNKNOWN`.
pub async fn qr(
    State(state): State<AppState>,
    Path(secret): Path<String>,
) -> Result<Response, AppError> {
    let code = screens::code_of(&state.pool, &secret).await?;
    let url = format!("{}/tv/jumeler?code={code}", state.auth.public_origin);
    Ok((
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        screens::qr_svg(&url),
    )
        .into_response())
}

/// `GET /api/campaigns/{id}/tv`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn list(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let campaign = gm_campaign(&state, &gm, &id).await?;
    let list = screens::list(&state.pool, campaign).await?;
    Ok(Json(json!({ "data": list })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairBody {
    code: String,
}

/// `POST /api/campaigns/{id}/tv`
///
/// # Errors
///
/// 404 when missing or another GM's, `TV_CODE_UNKNOWN`.
pub async fn pair(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(body): Body<PairBody>,
) -> Result<Response, AppError> {
    let campaign = gm_campaign(&state, &gm, &id).await?;
    screens::pair(&state.pool, campaign, &body.code).await?;
    let list = screens::list(&state.pool, campaign).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": list }))).into_response())
}

/// `POST /api/campaigns/{id}/tv/window`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn window(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let campaign = gm_campaign(&state, &gm, &id).await?;
    let secret = screens::open_window(&state.pool, campaign).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": { "secret": secret } })),
    )
        .into_response())
}

/// `DELETE /api/campaigns/{id}/tv/{screen}`
///
/// # Errors
///
/// 404 when missing, another GM's, or no screen of the campaign.
pub async fn forget(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, screen)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let campaign = gm_campaign(&state, &gm, &id).await?;
    screens::forget(&state.pool, campaign, parse_id(&screen)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingBody {
    line: Option<u32>,
}

/// `POST /api/campaigns/{id}/session/reading`
///
/// # Errors
///
/// 404; 409 `NO_SESSION`, `NO_PREVIOUSLY`; 400 `INVALID_LINE`.
pub async fn reading(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(body): Body<ReadingBody>,
) -> Result<Response, AppError> {
    let line = scenes::reading(&state.pool, &gm, parse_id(&id)?, body.line).await?;
    Ok(Json(json!({ "data": { "line": line } })).into_response())
}
