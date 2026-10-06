//! Pixel-art images and the world's theme over HTTP.
//!
//! GM (behind `require_gm`):
//! - `GET  /api/campaigns/{id}/media` → every image (pending, approved,
//!   rejected) and the world's theme;
//! - `POST /api/campaigns/{id}/media` → `media::Ask`: draw one (201);
//! - `GET  /api/campaigns/{id}/media/{asset}/image` → its bytes;
//! - `POST /api/campaigns/{id}/media/{asset}/decision` → `{ approve }`.
//!
//! Player (behind `require_player`):
//! - `GET /api/play/{campaign}/media` → the approved images whose subject
//!   the table may see, and the world's theme;
//! - `GET /api/play/{campaign}/media/{asset}/image` → the bytes of one of
//!   those (404 otherwise: the GM's other images never leave).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::auth::player::CurrentPlayer;
use crate::campaigns::{self, CampaignRow};
use crate::content;
use crate::error::AppError;
use crate::media::{self, Ask};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

async fn owned_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)
}

fn image_response(bytes: Vec<u8>, mime: String) -> Response {
    (
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "private, max-age=86400".to_string()),
        ],
        bytes,
    )
        .into_response()
}

/// `GET /api/campaigns/{id}/media`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn gm_list(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let assets = media::all(&state.pool, row.id).await?;
    Ok(Json(json!({ "data": {
        "assets": assets,
        "theme": content::theme(&row.story),
    } }))
    .into_response())
}

/// `POST /api/campaigns/{id}/media` — the new image, pending or (when the
/// model failed) rejected with the reason (201).
///
/// # Errors
///
/// The codes of `media::ask`.
pub async fn ask(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(ask): Body<Ask>,
) -> Result<Response, AppError> {
    let a = media::ask(&state.pool, &state.ai, &gm, parse_id(&id)?, &ask).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": a }))).into_response())
}

/// `GET /api/campaigns/{id}/media/{asset}/image`
///
/// # Errors
///
/// 404 `NO_SUCH_ASSET`.
pub async fn gm_image(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, asset)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let (_, bytes, mime) = media::image(&state.pool, row.id, parse_id(&asset)?).await?;
    Ok(image_response(bytes, mime))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionBody {
    approve: bool,
}

/// `POST /api/campaigns/{id}/media/{asset}/decision` (204).
///
/// # Errors
///
/// The codes of `media::decide`.
pub async fn decide(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, asset)): Path<(String, String)>,
    Body(body): Body<DecisionBody>,
) -> Result<Response, AppError> {
    media::decide(
        &state.pool,
        &gm,
        parse_id(&id)?,
        parse_id(&asset)?,
        body.approve,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// The images player `p` may see.
async fn visible(
    state: &AppState,
    p: &CurrentPlayer,
) -> Result<(CampaignRow, Vec<media::Asset>), AppError> {
    let row = campaigns::find(&state.pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))?;
    let given = media::given_items(&state.pool, row.id).await?;
    let assets = media::all(&state.pool, row.id)
        .await?
        .into_iter()
        .filter(|a| media::shown_to_players(&row.story, &row.world, &given, a))
        .collect();
    Ok((row, assets))
}

/// `GET /api/play/{campaign}/media`
///
/// # Errors
///
/// 401 `NOT_JOINED`.
pub async fn player_list(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    let (row, assets) = visible(&state, &p).await?;
    let assets: Vec<_> = assets
        .iter()
        .map(|a| json!({ "id": a.id, "kind": a.kind, "subject": a.subject }))
        .collect();
    Ok(Json(json!({ "data": {
        "assets": assets,
        "theme": content::theme(&row.story),
    } }))
    .into_response())
}

/// `GET /api/play/{campaign}/media/{asset}/image`
///
/// # Errors
///
/// 401 `NOT_JOINED`; 404 `NO_SUCH_ASSET` (also for an image the table
/// may not see).
pub async fn player_image(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Path((_, asset)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let id = parse_id(&asset)?;
    let (row, assets) = visible(&state, &p).await?;
    if !assets.iter().any(|a| a.id == id) {
        return Err(AppError::NotFound("NO_SUCH_ASSET"));
    }
    let (_, bytes, mime) = media::image(&state.pool, row.id, id).await?;
    Ok(image_response(bytes, mime))
}
