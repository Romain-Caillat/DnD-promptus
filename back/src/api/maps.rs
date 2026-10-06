//! The campaign's own maps over HTTP (`campaign_maps`).
//!
//! GM (behind `require_gm`):
//! - `GET    /api/campaigns/{id}/maps` → its maps, and the world's to copy;
//! - `POST   /api/campaigns/{id}/maps` → `NewMap`: blank or copied (201);
//! - `POST   /api/campaigns/{id}/maps/import` → `Import`: a Universal VTT
//!   file or an aligned image (201);
//! - `POST   /api/campaigns/{id}/maps/generate` → `{ node }`: the model's
//!   draft for a scene (201);
//! - `GET    /api/campaigns/{id}/maps/{map}` → one map;
//! - `PUT    /api/campaigns/{id}/maps/{map}` → the whole map, from the
//!   editor (withdraws the validation);
//! - `DELETE /api/campaigns/{id}/maps/{map}` (204);
//! - `POST   /api/campaigns/{id}/maps/{map}/validate` → may be shown;
//! - `GET    /api/campaigns/{id}/maps/{map}/backdrop` → its image.
//!
//! Player (behind `require_player`):
//! - `GET /api/play/{campaign}/board/backdrop` → the image behind the map
//!   shown at the table, if it has one.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use super::media::image_response;
use crate::auth::guard::CurrentGm;
use crate::auth::player::CurrentPlayer;
use crate::campaign_maps::{self, Import, NewMap, generate};
use crate::error::AppError;
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

fn data(status: StatusCode, value: impl serde::Serialize) -> Response {
    (status, Json(json!({ "data": value }))).into_response()
}

/// `GET /api/campaigns/{id}/maps`
///
/// # Errors
///
/// 404.
pub async fn list(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let l = campaign_maps::listing(&state.pool, &gm, parse_id(&id)?).await?;
    Ok(data(StatusCode::OK, l))
}

/// `POST /api/campaigns/{id}/maps`
///
/// # Errors
///
/// As `campaign_maps::create`.
pub async fn create(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(new): Body<NewMap>,
) -> Result<Response, AppError> {
    let m = campaign_maps::create(&state.pool, &gm, parse_id(&id)?, &new).await?;
    Ok(data(StatusCode::CREATED, m))
}

/// `POST /api/campaigns/{id}/maps/import`
///
/// # Errors
///
/// As `campaign_maps::import`.
pub async fn import(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(import): Body<Import>,
) -> Result<Response, AppError> {
    let m = campaign_maps::import(&state.pool, &gm, parse_id(&id)?, &import).await?;
    Ok(data(StatusCode::CREATED, m))
}

/// `POST /api/campaigns/{id}/maps/generate`
///
/// # Errors
///
/// As `campaign_maps::generate::generate`.
pub async fn generate(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(ask): Body<generate::Ask>,
) -> Result<Response, AppError> {
    let g = generate::generate(&state.pool, &state.ai, &gm, parse_id(&id)?, &ask).await?;
    Ok(data(StatusCode::CREATED, g))
}

/// `GET /api/campaigns/{id}/maps/{map}`
///
/// # Errors
///
/// 404 `NO_SUCH_MAP`.
pub async fn get(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, map)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let m = campaign_maps::get(&state.pool, &gm, parse_id(&id)?, &map).await?;
    Ok(data(StatusCode::OK, m))
}

/// `PUT /api/campaigns/{id}/maps/{map}`
///
/// # Errors
///
/// As `campaign_maps::save`.
pub async fn save(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, map)): Path<(String, String)>,
    Body(raw): Body<serde_json::Value>,
) -> Result<Response, AppError> {
    let m = campaign_maps::save(&state.pool, &gm, parse_id(&id)?, &map, raw).await?;
    Ok(data(StatusCode::OK, m))
}

/// `DELETE /api/campaigns/{id}/maps/{map}` (204)
///
/// # Errors
///
/// As `campaign_maps::delete`.
pub async fn delete(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, map)): Path<(String, String)>,
) -> Result<Response, AppError> {
    campaign_maps::delete(&state.pool, &gm, parse_id(&id)?, &map).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `POST /api/campaigns/{id}/maps/{map}/validate`
///
/// # Errors
///
/// 404 `NO_SUCH_MAP`.
pub async fn validate(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, map)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let m = campaign_maps::validate(&state.pool, &gm, parse_id(&id)?, &map).await?;
    Ok(data(StatusCode::OK, m))
}

/// `GET /api/campaigns/{id}/maps/{map}/backdrop`
///
/// # Errors
///
/// 404 `NO_SUCH_MAP` (also for a map without an image).
pub async fn gm_backdrop(
    State(state): State<AppState>,
    gm: CurrentGm,
    headers: HeaderMap,
    Path((id, map)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let campaign = parse_id(&id)?;
    campaign_maps::get(&state.pool, &gm, campaign, &map).await?;
    let (bytes, mime) = campaign_maps::backdrop(&state.pool, campaign, &map).await?;
    Ok(image_response(&headers, bytes, mime))
}

/// `GET /api/play/{campaign}/board/backdrop`
///
/// # Errors
///
/// 404 `NO_SUCH_MAP` when no map with an image is shown.
pub async fn player_backdrop(
    State(state): State<AppState>,
    p: CurrentPlayer,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let campaign = p.0.campaign_id;
    let shown: Option<String> =
        sqlx::query_scalar("SELECT map_id FROM map_states WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_optional(&state.pool)
            .await?;
    let shown = shown.ok_or(AppError::NotFound("NO_SUCH_MAP"))?;
    let (bytes, mime) = campaign_maps::backdrop(&state.pool, campaign, &shown).await?;
    Ok(image_response(&headers, bytes, mime))
}
