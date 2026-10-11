//! Preparing a campaign (`campaign/review-story-graph`), behind
//! `require_gm`:
//! - `POST /api/campaigns/{id}/story/edits` → the GM's edits by id
//!   (`story::edit::Edit`), applied whole; answers the campaign and
//!   what changed. With the GM's token (Claude) they are proposed
//!   instead (202, `prep::claude`), unless Claude has the hand;
//! - `GET  /api/campaigns/{id}/revisions?since=N` → the preparation
//!   journal after revision N (`platform/stream-prep-edits`);
//! - `POST /api/campaigns/{id}/revisions/{revision}/undo` → take one
//!   back, while no later revision touched the same entities;
//! - `GET  /api/campaigns/{id}/readiness` → one gauge per act;
//! - `POST /api/campaigns/{id}/story/validate` → declare it playable;
//! - `GET  /api/campaigns/{id}/workshop` → the co-GM's proposals;
//! - `POST /api/campaigns/{id}/workshop` → ask the co-GM (201);
//! - `POST /api/campaigns/{id}/workshop/{proposal}/accept|reject`;
//! - `GET  /api/campaigns/{id}/proposals` → Claude's batches and the
//!   GM's word on each item (`platform/propose-edits-by-claude`; a token
//!   reads it);
//! - `POST /api/campaigns/{id}/proposals/{proposal}/decide` → keep or
//!   discard items of Claude's batch (the GM's session only);
//! - `PUT  /api/campaigns/{id}/claude` → « Claude a la main », on or off
//!   (the GM's session only);
//! - `GET  /api/campaigns/{id}/generation` → the generation jobs, and
//!   what one more costs (`ai/generate-campaign`);
//! - `POST /api/campaigns/{id}/generation` → start one (202);
//! - `POST /api/campaigns/{id}/generation/{job}/apply` → install its
//!   draft as the campaign's story, to review.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use promptus_shared::story::edit::Edit;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use super::campaigns::detail;
use crate::auth::guard::{Credential, CurrentGm, SessionOnly};
use crate::campaigns::revisions;
use crate::error::AppError;
use crate::prep::claude::{self, Outcome};
use crate::prep::proposals::Focus;
use crate::prep::{self, generation, workshop};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditsBody {
    edits: Vec<Edit>,
    /// Claude's batch: its title, the batch to add the edits to, where
    /// on a map it is about. Ignored from the GM's session.
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    proposal: Option<Uuid>,
    #[serde(default)]
    focus: Option<Focus>,
}

/// What a write proposed by Claude answers (202).
pub(crate) fn proposed<T>(outcome: Outcome<T>, applied: impl FnOnce(T) -> Response) -> Response {
    match outcome {
        Outcome::Applied(v) => applied(v),
        Outcome::Proposed {
            proposal,
            why,
            issues,
        } => (
            StatusCode::ACCEPTED,
            Json(json!({ "data": {
                "status": "proposed",
                "why": why,
                "proposal": proposal,
                "issues": issues,
            } })),
        )
            .into_response(),
    }
}

/// `POST /api/campaigns/{id}/story/edits`
///
/// # Errors
///
/// As `prep::apply`.
pub async fn edits(
    State(state): State<AppState>,
    gm: CurrentGm,
    credential: Credential,
    Path(id): Path<String>,
    Body(input): Body<EditsBody>,
) -> Result<Response, AppError> {
    let id = parse_id(&id)?;
    let answer = |(row, changes)| {
        Json(json!({ "data": {
            "status": "applied",
            "campaign": detail(row),
            "changes": changes,
        } }))
        .into_response()
    };
    if credential == Credential::Token {
        let meta = claude::Meta {
            title: input.title,
            proposal: input.proposal,
            focus: input.focus,
        };
        let outcome = claude::story_edits(&state.pool, &gm, id, &input.edits, &meta).await?;
        return Ok(proposed(outcome, answer));
    }
    let done = prep::apply(&state.pool, &gm, credential.into(), id, &input.edits).await?;
    Ok(answer(done))
}

/// `GET /api/campaigns/{id}/proposals`
///
/// # Errors
///
/// As `prep::claude::list`.
pub async fn claude_proposals(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let list = claude::list(&state.pool, &gm, parse_id(&id)?).await?;
    Ok(Json(json!({ "data": list })).into_response())
}

/// `POST /api/campaigns/{id}/proposals/{proposal}/decide` — the GM's
/// session only.
///
/// # Errors
///
/// 403 `TOKEN_NOT_ALLOWED` for a token; as `prep::claude::decide`.
pub async fn decide_claude(
    State(state): State<AppState>,
    gm: CurrentGm,
    _session: SessionOnly,
    Path((id, proposal)): Path<(String, String)>,
    Body(input): Body<claude::Decide>,
) -> Result<Response, AppError> {
    let proposal =
        Uuid::parse_str(&proposal).map_err(|_| AppError::NotFound("NO_SUCH_PROPOSAL"))?;
    let (row, p) = claude::decide(&state.pool, &gm, parse_id(&id)?, proposal, &input).await?;
    Ok(Json(json!({ "data": { "campaign": detail(row), "proposal": p } })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandBody {
    hand: bool,
}

/// `PUT /api/campaigns/{id}/claude` — the GM's session only: Claude
/// never gives itself the hand.
///
/// # Errors
///
/// 403 `TOKEN_NOT_ALLOWED` for a token; as `prep::claude::set_hand`.
pub async fn claude_hand(
    State(state): State<AppState>,
    gm: CurrentGm,
    _session: SessionOnly,
    Path(id): Path<String>,
    Body(input): Body<HandBody>,
) -> Result<Response, AppError> {
    let row = claude::set_hand(&state.pool, &gm, parse_id(&id)?, input.hand).await?;
    Ok(Json(json!({ "data": detail(row) })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SinceQuery {
    #[serde(default)]
    since: i64,
}

/// `GET /api/campaigns/{id}/revisions?since=N`
///
/// # Errors
///
/// As `campaigns::revisions::since`.
pub async fn revisions(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Query(q): Query<SinceQuery>,
) -> Result<Response, AppError> {
    let since = revisions::since(&state.pool, &gm, parse_id(&id)?, q.since).await?;
    Ok(Json(json!({ "data": since })).into_response())
}

/// `POST /api/campaigns/{id}/revisions/{revision}/undo`
///
/// # Errors
///
/// 404 `NO_SUCH_REVISION` for a revision that is not a number; as
/// `campaigns::revisions::undo`.
pub async fn undo(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, revision)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let revision: i64 = revision
        .parse()
        .map_err(|_| AppError::NotFound("NO_SUCH_REVISION"))?;
    let row = revisions::undo(&state.pool, &gm, parse_id(&id)?, revision).await?;
    Ok(Json(json!({ "data": detail(row) })).into_response())
}

/// `POST /api/campaigns/{id}/story/validate`
///
/// # Errors
///
/// As `prep::validate_campaign`.
pub async fn validate(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = prep::validate_campaign(&state.pool, &gm, parse_id(&id)?).await?;
    Ok(Json(json!({ "data": detail(row) })).into_response())
}

/// `GET /api/campaigns/{id}/readiness`
///
/// # Errors
///
/// As `prep::act_readiness`.
pub async fn readiness(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let acts = prep::act_readiness(&state.pool, &gm, parse_id(&id)?).await?;
    Ok(Json(json!({ "data": acts })).into_response())
}

/// `GET /api/campaigns/{id}/workshop`
///
/// # Errors
///
/// As `workshop::list`.
pub async fn proposals(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let list = workshop::list(&state.pool, &gm, parse_id(&id)?).await?;
    Ok(Json(json!({ "data": list })).into_response())
}

/// `POST /api/campaigns/{id}/workshop`
///
/// # Errors
///
/// As `workshop::ask`.
pub async fn ask(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(input): Body<workshop::Ask>,
) -> Result<Response, AppError> {
    let p = workshop::ask(&state.pool, &state.ai, &gm, parse_id(&id)?, &input).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": p }))).into_response())
}

async fn decide(
    state: &AppState,
    gm: &CurrentGm,
    id: &str,
    proposal: &str,
    accept: bool,
) -> Result<Response, AppError> {
    let proposal = Uuid::parse_str(proposal).map_err(|_| AppError::NotFound("NO_SUCH_PROPOSAL"))?;
    let (row, p) = workshop::decide(&state.pool, gm, parse_id(id)?, proposal, accept).await?;
    Ok(Json(json!({ "data": { "campaign": detail(row), "proposal": p } })).into_response())
}

/// `POST /api/campaigns/{id}/workshop/{proposal}/accept`
///
/// # Errors
///
/// As `workshop::decide`.
pub async fn accept(
    State(state): State<AppState>,
    gm: CurrentGm,
    _session: SessionOnly,
    Path((id, proposal)): Path<(String, String)>,
) -> Result<Response, AppError> {
    decide(&state, &gm, &id, &proposal, true).await
}

/// `POST /api/campaigns/{id}/workshop/{proposal}/reject`
///
/// # Errors
///
/// As `workshop::decide`.
pub async fn reject(
    State(state): State<AppState>,
    gm: CurrentGm,
    _session: SessionOnly,
    Path((id, proposal)): Path<(String, String)>,
) -> Result<Response, AppError> {
    decide(&state, &gm, &id, &proposal, false).await
}

/// `GET /api/campaigns/{id}/generation`
///
/// # Errors
///
/// As `generation::desk`.
pub async fn generations(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let desk = generation::desk(&state.pool, &state.ai, &gm, parse_id(&id)?).await?;
    Ok(Json(json!({ "data": desk })).into_response())
}

/// `POST /api/campaigns/{id}/generation`
///
/// # Errors
///
/// As `generation::start`.
pub async fn generate(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(input): Body<generation::Input>,
) -> Result<Response, AppError> {
    let job = generation::start(&state.pool, &state.ai, &gm, parse_id(&id)?, input).await?;
    Ok((StatusCode::ACCEPTED, Json(json!({ "data": job }))).into_response())
}

/// `POST /api/campaigns/{id}/generation/{job}/apply`
///
/// # Errors
///
/// As `generation::apply`.
pub async fn apply_generation(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, job)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let job = Uuid::parse_str(&job).map_err(|_| AppError::NotFound("NO_SUCH_GENERATION"))?;
    let (row, job) = generation::apply(&state.pool, &gm, parse_id(&id)?, job).await?;
    Ok(Json(json!({ "data": { "campaign": detail(row), "job": job } })).into_response())
}
