//! The evening from a player's phone (behind `require_player`; every
//! answer built by `campaigns::projection::evening`):
//! - `GET  /api/play/{campaign}/evening` → the session, the scene, the
//!   music, the journal, my requests and my cards;
//! - `POST /api/play/{campaign}/lobby` → `{ soundOk, remote }`: I am here;
//! - `POST /api/play/{campaign}/requests` → `{ card, text }`: ask the GM;
//! - `POST /api/play/{campaign}/requests/{request}/roll` → roll the check
//!   the GM asked for (the server rolls);
//! - `POST /api/play/{campaign}/requests/{request}/withdraw`;
//! - `POST /api/play/{campaign}/requests/{request}/contest`;
//! - `POST /api/play/{campaign}/feedback` → my three answers about the
//!   last session.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::player::CurrentPlayer;
use crate::campaigns::projection::evening::{EveningInput, project_evening};
use crate::campaigns::projection::project_play;
use crate::campaigns::{self, CampaignRow};
use crate::content;
use crate::error::AppError;
use crate::evening::feedback::{self, Answers};
use crate::evening::knowledge;
use crate::evening::requests::{self, Ask, Follow};
use crate::evening::session::{self, Arrival};
use crate::players::{self, CharacterStatus, Role};
use crate::state::AppState;

async fn campaign_of(state: &AppState, p: &CurrentPlayer) -> Result<CampaignRow, AppError> {
    campaigns::find(&state.pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))
}

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NO_SUCH_REQUEST"))
}

/// The evening as `p` may see it now.
async fn evening_json(state: &AppState, p: &CurrentPlayer) -> Result<serde_json::Value, AppError> {
    let pool = &state.pool;
    let row = campaign_of(state, p).await?;
    let rules = content::rule_system(&row.story.rules);
    let current = session::current(pool, row.id).await?;
    let last = session::last_ended(pool, row.id).await?;
    let seats = players::seats(pool, row.id).await?;
    let lobby = match &current {
        Some(s) => session::attendance(pool, s.id)
            .await?
            .into_iter()
            .filter_map(|a| {
                let nickname = seats.iter().find(|x| x.id == a.player_id)?.nickname.clone();
                Some((a, nickname))
            })
            .collect(),
        None => Vec::new(),
    };
    let journal = knowledge::journal(pool, row.id, true).await?;
    let mine = match &current {
        Some(s) => requests::of_session(pool, s.id)
            .await?
            .into_iter()
            .filter(|r| r.player_id == p.0.id)
            .collect(),
        None => Vec::new(),
    };
    let character = players::character_of(pool, &p.0).await?;
    let play = match (rules, &character) {
        (Some(r), Some(c)) if c.status == CharacterStatus::Validated => {
            project_play(r, &c.sheet, c.play.as_ref())
        }
        _ => None,
    };
    let feedback_answered = match (&last, &current, p.0.role) {
        (Some(s), None, Role::Player) => Some(feedback::answered(pool, s.id, p.0.id).await?),
        _ => None,
    };
    let view = project_evening(&EveningInput {
        campaign: &row.story,
        world: &row.world,
        rules,
        current: current.as_ref(),
        last_ended: last.as_ref(),
        lobby: &lobby,
        journal: &journal,
        requests: &mine,
        play: play.as_ref(),
        feedback_answered,
    });
    serde_json::to_value(view).map_err(|e| AppError::internal("evening view", e))
}

/// `GET /api/play/{campaign}/evening`
///
/// # Errors
///
/// 401 `NOT_JOINED`; a database error.
pub async fn evening(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    let view = evening_json(&state, &p).await?;
    Ok(Json(json!({ "data": view })).into_response())
}

/// `POST /api/play/{campaign}/lobby` — answers the evening.
///
/// # Errors
///
/// 409 `NO_SESSION`; 400 `INVALID_BODY`.
pub async fn arrive(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(arrival): Body<Arrival>,
) -> Result<Response, AppError> {
    session::arrive(&state.pool, &p.0, arrival).await?;
    let view = evening_json(&state, &p).await?;
    Ok(Json(json!({ "data": view })).into_response())
}

/// `POST /api/play/{campaign}/requests` — answers the evening (201).
///
/// # Errors
///
/// The codes of `evening::requests::ask`.
pub async fn ask(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(ask): Body<Ask>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    requests::ask(
        &state.pool,
        &p.0,
        content::rule_system(&row.story.rules),
        &ask,
    )
    .await?;
    let view = evening_json(&state, &p).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": view }))).into_response())
}

/// `POST /api/play/{campaign}/requests/{request}/roll` — answers the
/// evening, the roll inside the request.
///
/// # Errors
///
/// The codes of `evening::requests::roll`.
pub async fn roll(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Path((_, request)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    requests::roll(
        &state.pool,
        &p.0,
        content::rule_system(&row.story.rules),
        parse_id(&request)?,
    )
    .await?;
    let view = evening_json(&state, &p).await?;
    Ok(Json(json!({ "data": view })).into_response())
}

async fn follow(
    state: &AppState,
    p: &CurrentPlayer,
    request: &str,
    f: Follow,
) -> Result<Response, AppError> {
    requests::follow(&state.pool, &p.0, parse_id(request)?, f).await?;
    let view = evening_json(state, p).await?;
    Ok(Json(json!({ "data": view })).into_response())
}

/// `POST /api/play/{campaign}/requests/{request}/withdraw`
///
/// # Errors
///
/// 404 `NO_SUCH_REQUEST`; 409 `ALREADY_DECIDED`.
pub async fn withdraw(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Path((_, request)): Path<(String, String)>,
) -> Result<Response, AppError> {
    follow(&state, &p, &request, Follow::Withdraw).await
}

/// `POST /api/play/{campaign}/requests/{request}/contest`
///
/// # Errors
///
/// 404 `NO_SUCH_REQUEST`; 409 `NOT_CONTESTABLE`.
pub async fn contest(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Path((_, request)): Path<(String, String)>,
) -> Result<Response, AppError> {
    follow(&state, &p, &request, Follow::Contest).await
}

/// `POST /api/play/{campaign}/feedback` — answers the evening.
///
/// # Errors
///
/// 403 `SPECTATOR`; 409 `NO_ENDED_SESSION`; 400 `TEXT_TOO_LONG`,
/// `INVALID_BODY`.
pub async fn answer_feedback(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(answers): Body<Answers>,
) -> Result<Response, AppError> {
    feedback::answer(&state.pool, &p.0, &answers).await?;
    let view = evening_json(&state, &p).await?;
    Ok(Json(json!({ "data": view })).into_response())
}
