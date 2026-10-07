//! Between two sessions (`session/write-recaps`,
//! `session/schedule-sessions`, `player/play-between-sessions`).
//!
//! Player, behind `require_player` (answers built by
//! `campaigns::projection::between`):
//! - `GET /api/play/{campaign}/between` → what I gained last session,
//!   the published « Précédemment… », the chronicle, the next date;
//! - `PUT /api/play/{campaign}/availability` → `{ available: [time] }`:
//!   which proposed times suit me;
//! - `GET /api/play/{campaign}/next-session.ics` → the chosen date as a
//!   calendar event, reminding an hour before, its link to my table.
//!
//! GM, behind `require_gm`:
//! - `GET /api/campaigns/{id}/plan` → the plan and every answer;
//! - `PUT /api/campaigns/{id}/plan` → `{ options, lobbyMinutes? }`;
//! - `PUT /api/campaigns/{id}/plan/choice` → `{ at }` (or null).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::auth::player::CurrentPlayer;
use crate::campaigns::projection::between::{BetweenInput, Change, project_between};
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::evening::schedule::{self, Proposal};
use crate::evening::session;
use crate::state::AppState;

async fn campaign_of(state: &AppState, p: &CurrentPlayer) -> Result<CampaignRow, AppError> {
    campaigns::find(&state.pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))
}

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

/// `GET /api/play/{campaign}/between`
///
/// # Errors
///
/// 401 `NOT_JOINED`; a database error.
pub async fn between(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    let pool = &state.pool;
    let row = campaign_of(&state, &p).await?;
    let sessions = session::all(pool, row.id).await?;
    let last = sessions
        .iter()
        .find(|s| s.status == crate::evening::Status::Ended);
    // My character's changes while that session ran: XP, items, purse.
    let changes: Vec<Change> = match last {
        Some(s) => {
            let rows: Vec<(String, Option<String>, i32, i32)> = sqlx::query_as(
                "SELECT a.kind, a.label, a.before_value, a.after_value
                 FROM play_adjustments a JOIN characters c ON c.id = a.character_id
                 WHERE c.player_id = $1 AND a.created_at >= $2 AND a.created_at <= $3
                 ORDER BY a.created_at",
            )
            .bind(p.0.id)
            .bind(s.started_at.unwrap_or(s.opened_at))
            .bind(s.ended_at.unwrap_or_else(Utc::now))
            .fetch_all(pool)
            .await?;
            rows.into_iter()
                .map(|(kind, label, before, after)| Change {
                    kind,
                    label,
                    before,
                    after,
                })
                .collect()
        }
        None => Vec::new(),
    };
    let plan = schedule::get(pool, row.id).await?;
    let view = project_between(&BetweenInput {
        sessions: &sessions,
        changes: &changes,
        plan: plan.as_ref(),
        next_number: schedule::next_number(pool, row.id).await?,
        caller: p.0.id,
    });
    Ok(Json(json!({ "data": view })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AvailabilityBody {
    available: Vec<DateTime<Utc>>,
}

/// `PUT /api/play/{campaign}/availability` — answers the between view.
///
/// # Errors
///
/// 401 `NOT_JOINED`; 403 `SPECTATOR`; 409 `NO_PLAN`; 400 `NOT_PROPOSED`,
/// `INVALID_BODY`.
pub async fn availability(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(body): Body<AvailabilityBody>,
) -> Result<Response, AppError> {
    schedule::answer(&state.pool, &p.0, &body.available).await?;
    between(State(state), p).await
}

/// `GET /api/play/{campaign}/next-session.ics`
///
/// # Errors
///
/// 401 `NOT_JOINED`; 404 `NO_NEXT_SESSION` until the GM chose a date.
pub async fn calendar(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let plan = schedule::get(&state.pool, row.id)
        .await?
        .ok_or(AppError::NotFound("NO_NEXT_SESSION"))?;
    let number = schedule::next_number(&state.pool, row.id).await?;
    let url = format!("{}/partie/{}", state.auth.public_origin, row.id);
    let ics = schedule::ics(&plan, &row.story.title, number, &url, Utc::now())
        .ok_or(AppError::NotFound("NO_NEXT_SESSION"))?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"prochaine-session.ics\"",
            ),
        ],
        ics,
    )
        .into_response())
}

async fn gm_campaign(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<Uuid, AppError> {
    let row = owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)?;
    Ok(row.id)
}

/// `GET /api/campaigns/{id}/plan` — `null` when nothing is planned.
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn plan(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let campaign = gm_campaign(&state, &gm, &id).await?;
    let plan = schedule::get(&state.pool, campaign).await?;
    Ok(Json(json!({ "data": plan })).into_response())
}

/// `PUT /api/campaigns/{id}/plan`
///
/// # Errors
///
/// 404; 400 `INVALID_OPTIONS`, `DATE_IN_PAST`, `INVALID_LOBBY_MINUTES`.
pub async fn propose(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(proposal): Body<Proposal>,
) -> Result<Response, AppError> {
    let plan = schedule::propose(&state.pool, &gm, parse_id(&id)?, &proposal).await?;
    Ok(Json(json!({ "data": plan })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceBody {
    at: Option<DateTime<Utc>>,
}

/// `PUT /api/campaigns/{id}/plan/choice`
///
/// # Errors
///
/// 404; 409 `NO_PLAN`; 400 `NOT_PROPOSED`, `DATE_IN_PAST`.
pub async fn choose(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(body): Body<ChoiceBody>,
) -> Result<Response, AppError> {
    let plan = schedule::choose(&state.pool, &gm, parse_id(&id)?, body.at).await?;
    Ok(Json(json!({ "data": plan })).into_response())
}
