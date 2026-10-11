//! The campaign's own maps over HTTP (`campaign_maps`).
//!
//! GM (behind `require_gm`; a GM's personal token reaches the reads,
//! the atlas, the sketch, the questions, the placements and the
//! operations, `auth::api_tokens::TOKEN_ROUTES`):
//! - `GET    /api/campaigns/{id}/maps` → its maps, and the world's to copy;
//! - `POST   /api/campaigns/{id}/maps` → `NewMap`: blank or copied (201);
//! - `POST   /api/campaigns/{id}/maps/import` → `Import`: a Universal VTT
//!   file or an aligned image (201);
//! - `POST   /api/campaigns/{id}/maps/generate` → `{ node }`: the model's
//!   draft for a scene (201);
//! - `GET    /api/campaigns/{id}/maps/{map}` → one map;
//! - `GET    /api/campaigns/{id}/atlas` → every map of the world and the
//!   campaign, nested by their exits (maps/browse-atlas);
//! - `GET    /api/campaigns/{id}/maps/{map}/sketch` → the map as text;
//! - `POST   /api/campaigns/{id}/maps/{map}/ask` → the engine's answer
//!   (path, sight, what players see);
//! - `POST   /api/campaigns/{id}/maps/{map}/place` → pins, NPCs, clues
//!   and fights placed by name (platform/edit-maps-by-claude: with the
//!   token, proposed);
//! - `POST   /api/campaigns/{id}/maps/{map}/ops` → `Batch`: operations
//!   on the revision read (withdraws the validation; 409 `MAP_STALE`
//!   with the fresh map when it steps on a later batch);
//! - `DELETE /api/campaigns/{id}/maps/{map}` (204);
//! - `POST   /api/campaigns/{id}/maps/{map}/validate` → may be shown;
//! - `GET    /api/campaigns/{id}/maps/{map}/backdrop` → its image.
//!
//! Player (behind `require_player`):
//! - `GET /api/play/{campaign}/board/backdrop` → the image behind the map
//!   shown at the table, if it has one.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use super::media::image_response;
use crate::auth::guard::{Credential, CurrentGm};
use crate::auth::player::CurrentPlayer;
use crate::campaign_maps::{self, Batch, Import, NewMap, generate, survey};
use crate::campaigns::revisions::Author;
use crate::error::AppError;
use crate::prep::claude::Outcome;
use crate::state::AppState;
use promptus_shared::story::placing::Question;

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
    credential: Credential,
    Path(id): Path<String>,
    Body(new): Body<NewMap>,
) -> Result<Response, AppError> {
    let m =
        campaign_maps::create(&state.pool, &gm, credential.into(), parse_id(&id)?, &new).await?;
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
    credential: Credential,
    Path(id): Path<String>,
    Body(import): Body<Import>,
) -> Result<Response, AppError> {
    let author = Author::from(credential);
    let m = campaign_maps::import(&state.pool, &gm, author, parse_id(&id)?, &import).await?;
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

/// `GET /api/campaigns/{id}/atlas`
///
/// # Errors
///
/// 404.
pub async fn atlas(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let a = campaign_maps::atlas(&state.pool, &gm, parse_id(&id)?).await?;
    Ok(data(StatusCode::OK, a))
}

/// `POST /api/campaigns/{id}/maps/{map}/ops`
///
/// # Errors
///
/// As `campaign_maps::edit`.
pub async fn edit(
    State(state): State<AppState>,
    gm: CurrentGm,
    credential: Credential,
    Path((id, map)): Path<(String, String)>,
    Body(batch): Body<Batch>,
) -> Result<Response, AppError> {
    let campaign = parse_id(&id)?;
    if credential == Credential::Token {
        let outcome =
            crate::prep::claude::map_ops(&state.pool, &gm, campaign, &map, &batch).await?;
        return Ok(super::prep::proposed(outcome, |m| data(StatusCode::OK, m)));
    }
    let author = Author::from(credential);
    let m = campaign_maps::edit(&state.pool, &gm, author, campaign, &map, &batch).await?;
    Ok(data(StatusCode::OK, m))
}

/// `GET /api/campaigns/{id}/maps/{map}/sketch?x=&y=&w=&h=` — the map
/// as text, with the preparation's marks (platform/edit-maps-by-claude).
///
/// # Errors
///
/// As `campaign_maps::survey::sketch`.
pub async fn sketch(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, map)): Path<(String, String)>,
    Query(zone): Query<survey::ZoneQuery>,
) -> Result<Response, AppError> {
    let s = survey::sketch(&state.pool, &gm, parse_id(&id)?, &map, &zone).await?;
    Ok(data(StatusCode::OK, s))
}

/// `POST /api/campaigns/{id}/maps/{map}/ask` — a question to the engine.
///
/// # Errors
///
/// As `campaign_maps::survey::ask`.
pub async fn ask(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, map)): Path<(String, String)>,
    Body(question): Body<Question>,
) -> Result<Response, AppError> {
    let a = survey::ask(&state.pool, &gm, parse_id(&id)?, &map, &question).await?;
    Ok(data(StatusCode::OK, a))
}

/// `POST /api/campaigns/{id}/maps/{map}/place` — pins, NPCs, clues and
/// fights placed by name; proposed with the GM's token (202).
///
/// # Errors
///
/// As `campaign_maps::survey::place`.
pub async fn place(
    State(state): State<AppState>,
    gm: CurrentGm,
    credential: Credential,
    Path((id, map)): Path<(String, String)>,
    Body(ask): Body<survey::Place>,
) -> Result<Response, AppError> {
    let survey::Placing { outcome, placed } = survey::place(
        &state.pool,
        &gm,
        credential.into(),
        parse_id(&id)?,
        &map,
        &ask,
    )
    .await?;
    Ok(match outcome {
        Outcome::Applied((row, changes)) => data(
            StatusCode::OK,
            json!({
                "status": "applied",
                "campaign": super::campaigns::detail(row),
                "changes": changes,
                "placed": placed,
            }),
        ),
        Outcome::Proposed {
            proposal,
            why,
            issues,
        } => data(
            StatusCode::ACCEPTED,
            json!({
                "status": "proposed",
                "why": why,
                "proposal": proposal,
                "issues": issues,
                "placed": placed,
            }),
        ),
    })
}

/// `DELETE /api/campaigns/{id}/maps/{map}` (204)
///
/// # Errors
///
/// As `campaign_maps::delete`.
pub async fn delete(
    State(state): State<AppState>,
    gm: CurrentGm,
    credential: Credential,
    Path((id, map)): Path<(String, String)>,
) -> Result<Response, AppError> {
    campaign_maps::delete(&state.pool, &gm, credential.into(), parse_id(&id)?, &map).await?;
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
