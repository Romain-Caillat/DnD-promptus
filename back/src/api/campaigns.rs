//! A GM's campaigns (all behind `require_gm`; another GM's campaign
//! answers 404 like a missing one):
//! - `GET  /api/campaigns` → the GM's campaigns, newest change first;
//! - `POST /api/campaigns` → a new empty campaign (201);
//! - `POST /api/campaigns/import` → a new campaign from YAML (201);
//! - `GET  /api/campaigns/{id}` → story, world and validation report;
//! - `GET  /api/campaigns/{id}/export` → the story as YAML;
//! - `PUT  /api/campaigns/{id}/import` → replace the story from YAML,
//!   keeping the world, under the campaign lock;
//! - `GET  /api/campaigns/{id}/player-view` → what players see now (the
//!   GM's preview of the projection).
//!
//! The validation report never blocks a write: the GM decides.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use promptus_shared::story::{
    Campaign, Issue, RuleSystemRef, WorldState, from_yaml, to_yaml, validate,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::{self, CampaignRow, projection};
use crate::error::AppError;
use crate::state::AppState;

/// A campaign as the GM's prep screens read it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CampaignDetail {
    id: Uuid,
    story: Campaign,
    world: WorldState,
    issues: Vec<Issue>,
    updated_at: DateTime<Utc>,
}

fn detail(row: CampaignRow) -> CampaignDetail {
    CampaignDetail {
        id: row.id,
        issues: validate(&row.story),
        story: row.story,
        world: row.world,
        updated_at: row.updated_at,
    }
}

fn campaign_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

fn parse(yaml: &str) -> Result<Campaign, AppError> {
    from_yaml(yaml).map_err(|e| AppError::Invalid {
        code: "INVALID_CAMPAIGN_YAML",
        detail: e.to_string(),
    })
}

/// A stable id from a title: lowercase ASCII letters and digits joined
/// by `-` (`Le Phare de Kerbrume` → `le-phare-de-kerbrume`).
fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_end_matches('-');
    if trimmed.is_empty() {
        "campagne".to_string()
    } else {
        trimmed.to_string()
    }
}

/// `GET /api/campaigns`
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(State(state): State<AppState>, gm: CurrentGm) -> Result<Response, AppError> {
    let list = campaigns::list(&state.pool, &gm).await?;
    Ok(Json(json!({ "data": list })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewCampaign {
    title: String,
    #[serde(default)]
    world: String,
    rules: RuleSystemRef,
}

/// `POST /api/campaigns`
///
/// # Errors
///
/// 400 `TITLE_REQUIRED` for a blank title; a database error.
pub async fn create(
    State(state): State<AppState>,
    gm: CurrentGm,
    Body(input): Body<NewCampaign>,
) -> Result<Response, AppError> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::BadRequest("TITLE_REQUIRED"));
    }
    let story = Campaign::empty(&slug(title), title, input.world.trim(), input.rules);
    let row = campaigns::create(&state.pool, &gm, &story).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": detail(row) }))).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YamlBody {
    yaml: String,
}

/// `POST /api/campaigns/import`
///
/// # Errors
///
/// 400 `INVALID_CAMPAIGN_YAML` (with the parser's line and column) when
/// the text is not a campaign; a database error.
pub async fn import_new(
    State(state): State<AppState>,
    gm: CurrentGm,
    Body(input): Body<YamlBody>,
) -> Result<Response, AppError> {
    let story = parse(&input.yaml)?;
    let row = campaigns::create(&state.pool, &gm, &story).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": detail(row) }))).into_response())
}

/// `GET /api/campaigns/{id}`
///
/// # Errors
///
/// 404 when missing or another GM's; a database error.
pub async fn get(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_by(campaigns::find(&state.pool, campaign_id(&id)?).await?, &gm)?;
    Ok(Json(json!({ "data": detail(row) })).into_response())
}

/// `GET /api/campaigns/{id}/export`
///
/// # Errors
///
/// 404 when missing or another GM's; a database error.
pub async fn export(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_by(campaigns::find(&state.pool, campaign_id(&id)?).await?, &gm)?;
    let yaml = to_yaml(&row.story).map_err(|e| AppError::internal("export campaign", e))?;
    Ok(Json(json!({ "data": { "yaml": yaml } })).into_response())
}

/// `PUT /api/campaigns/{id}/import`
///
/// # Errors
///
/// 404 when missing or another GM's (checked first, so a stranger
/// learns nothing from the body); 400 `INVALID_CAMPAIGN_YAML`; a
/// database error.
pub async fn import_replace(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(input): Body<YamlBody>,
) -> Result<Response, AppError> {
    let id = campaign_id(&id)?;
    let mut tx = state.pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, id).await?, &gm)?;
    let story = parse(&input.yaml)?;
    campaigns::save_story(&mut tx, id, &story).await?;
    let row = campaigns::find(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    tx.commit().await?;
    Ok(Json(json!({ "data": detail(row) })).into_response())
}

/// `GET /api/campaigns/{id}/player-view`
///
/// # Errors
///
/// 404 when missing or another GM's; a database error.
pub async fn player_view(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_by(campaigns::find(&state.pool, campaign_id(&id)?).await?, &gm)?;
    let view = projection::project_for_players(&row.story, &row.world);
    Ok(Json(json!({ "data": view })).into_response())
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn slugs_are_valid_ids() {
        assert_eq!(slug("Le Phare de Kerbrume"), "le-phare-de-kerbrume");
        assert_eq!(slug("  L'Été — 1718 !"), "l-t-1718");
        assert_eq!(slug("Éé"), "campagne");
    }
}
