//! The GM's rule system editor (`campaign/edit-rule-system`), behind
//! `require_gm`:
//! - `GET    /api/campaigns/{id}/rules` → versions, and the draft with
//!   its report;
//! - `POST   /api/campaigns/{id}/rules/draft` → start a draft (or answer
//!   the one in progress);
//! - `PUT    /api/campaigns/{id}/rules/draft` → `versions::Save`;
//! - `DELETE /api/campaigns/{id}/rules/draft` → drop it;
//! - `POST   /api/campaigns/{id}/rules/draft/lock` → lock it for the
//!   next session;
//! - `GET    /api/campaigns/{id}/rules/compare?from=&to=` → two versions;
//! - `POST   /api/campaigns/{id}/rules/house-rules/formalise` → the co-GM's
//!   formal form of a house rule, checked against the draft
//!   (`engine/formalise-house-rules`), not stored;
//! - `POST   /api/campaigns/{id}/rules/house-rules/try` → the same check
//!   on a form the GM edited.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::CurrentGm;
use crate::error::AppError;
use crate::rules::house::{self, RuleInput};
use crate::rules::versions::{self, Save};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

fn data(value: impl serde::Serialize) -> Response {
    Json(json!({ "data": value })).into_response()
}

/// `GET /api/campaigns/{id}/rules`
///
/// # Errors
///
/// As `versions::get`.
pub async fn editor(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    Ok(data(versions::get(&state.pool, &gm, parse_id(&id)?).await?))
}

/// `POST /api/campaigns/{id}/rules/draft`
///
/// # Errors
///
/// As `versions::start`.
pub async fn start_draft(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    Ok(data(
        versions::start(&state.pool, &gm, parse_id(&id)?).await?,
    ))
}

/// `PUT /api/campaigns/{id}/rules/draft`
///
/// # Errors
///
/// As `versions::save`.
pub async fn save_draft(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(input): Body<Save>,
) -> Result<Response, AppError> {
    Ok(data(
        versions::save(&state.pool, &gm, parse_id(&id)?, &input).await?,
    ))
}

/// `DELETE /api/campaigns/{id}/rules/draft`
///
/// # Errors
///
/// As `versions::discard`.
pub async fn discard_draft(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    Ok(data(
        versions::discard(&state.pool, &gm, parse_id(&id)?).await?,
    ))
}

/// `POST /api/campaigns/{id}/rules/draft/lock`
///
/// # Errors
///
/// As `versions::lock`.
pub async fn lock_draft(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    Ok(data(
        versions::lock(&state.pool, &gm, parse_id(&id)?).await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct Versions {
    from: u32,
    to: u32,
}

/// `GET /api/campaigns/{id}/rules/compare?from=&to=`
///
/// # Errors
///
/// As `versions::compare`.
pub async fn compare(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Query(v): Query<Versions>,
) -> Result<Response, AppError> {
    Ok(data(
        versions::compare(&state.pool, &gm, parse_id(&id)?, v.from, v.to).await?,
    ))
}

/// `POST /api/campaigns/{id}/rules/house-rules/formalise`
///
/// # Errors
///
/// As `house::formalise`.
pub async fn formalise_house_rule(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(input): Body<RuleInput>,
) -> Result<Response, AppError> {
    Ok(data(
        house::formalise(&state.pool, &state.ai, &gm, parse_id(&id)?, &input).await?,
    ))
}

/// `POST /api/campaigns/{id}/rules/house-rules/try`
///
/// # Errors
///
/// As `house::try_rule`.
pub async fn try_house_rule(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(input): Body<RuleInput>,
) -> Result<Response, AppError> {
    Ok(data(
        house::try_rule(&state.pool, &gm, parse_id(&id)?, &input).await?,
    ))
}
