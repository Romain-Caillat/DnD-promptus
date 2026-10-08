//! session/schedule-sessions over HTTP.
//!
//! The GM (behind `require_gm`; another GM's campaign answers 404):
//! - `GET    /api/campaigns/{id}/schedule` → the dates, every player's
//!   answers, the reminders sent, whether a Discord channel is set (its
//!   end only);
//! - `POST   /api/campaigns/{id}/schedule/dates` → `{ startsAt, minutes }`
//!   (201);
//! - `POST   /api/campaigns/{id}/schedule/dates/{date}/choose` → the date
//!   is fixed, the table told;
//! - `DELETE /api/campaigns/{id}/schedule/dates/{date}`;
//! - `PUT    /api/campaigns/{id}/schedule/discord` → `{ webhook }` (or
//!   null to stop).
//!
//! A player (behind `require_player`, answers built by
//! `campaigns::projection::schedule`):
//! - `GET /api/play/{campaign}/schedule` → the next date, the proposals;
//! - `PUT /api/play/{campaign}/schedule/{date}` → `{ available }`;
//! - `GET /api/play/{campaign}/schedule.ics` → the fixed date for the
//!   phone's calendar, with its two alarms.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::auth::player::CurrentPlayer;
use crate::campaigns::projection::schedule::project_schedule;
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::players::{self, Role};
use crate::schedule::{self, DateStatus, LogLine, Proposal, SessionDate, notify};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

async fn owned_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GmAnswer {
    player_id: Uuid,
    nickname: String,
    available: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GmDate {
    #[serde(flatten)]
    date: SessionDate,
    lobby_opens_at: chrono::DateTime<Utc>,
    answers: Vec<GmAnswer>,
    log: Vec<LogLine>,
}

/// `GET /api/campaigns/{id}/schedule`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn gm_schedule(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let pool = &state.pool;
    let seats = players::seats(pool, row.id).await?;
    let answers = schedule::answers(pool, row.id).await?;
    let mut dates = Vec::new();
    for d in schedule::open_dates(pool, row.id).await? {
        let log = if d.status == DateStatus::Chosen {
            schedule::log_of(pool, d.id).await?
        } else {
            Vec::new()
        };
        dates.push(GmDate {
            lobby_opens_at: d.lobby_opens_at(),
            answers: answers
                .iter()
                .filter(|a| a.date_id == d.id)
                .filter_map(|a| {
                    let seat = seats.iter().find(|s| s.id == a.player_id)?;
                    Some(GmAnswer {
                        player_id: a.player_id,
                        nickname: seat.nickname.clone(),
                        available: a.available,
                    })
                })
                .collect(),
            log,
            date: d,
        });
    }
    let webhook = schedule::webhook(pool, row.id).await?;
    let data = json!({
        "number": schedule::next_number(pool, row.id).await?,
        "dates": dates,
        "players": seats.iter().filter(|s| s.role == Role::Player)
            .map(|s| json!({ "id": s.id, "nickname": s.nickname })).collect::<Vec<_>>(),
        "discord": { "configured": webhook.is_some(), "hint": webhook.as_deref().map(notify::masked) },
        "lobbyBeforeMinutes": schedule::LOBBY_BEFORE_MINUTES,
    });
    Ok(Json(json!({ "data": data })).into_response())
}

/// `POST /api/campaigns/{id}/schedule/dates` (201)
///
/// # Errors
///
/// The codes of `schedule::propose`; 400 `INVALID_BODY`.
pub async fn propose(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(p): Body<Proposal>,
) -> Result<Response, AppError> {
    let d = schedule::propose(&state.pool, &gm, parse_id(&id)?, &p).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": d }))).into_response())
}

/// `POST /api/campaigns/{id}/schedule/dates/{date}/choose`
///
/// # Errors
///
/// The codes of `schedule::choose`.
pub async fn choose(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, date)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let d = schedule::choose(
        &state.pool,
        &state.notifier,
        &gm,
        parse_id(&id)?,
        parse_id(&date)?,
    )
    .await?;
    Ok(Json(json!({ "data": d })).into_response())
}

/// `DELETE /api/campaigns/{id}/schedule/dates/{date}`
///
/// # Errors
///
/// The codes of `schedule::drop_date`.
pub async fn drop_date(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, date)): Path<(String, String)>,
) -> Result<Response, AppError> {
    schedule::drop_date(&state.pool, &gm, parse_id(&id)?, parse_id(&date)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookBody {
    webhook: Option<String>,
}

/// `PUT /api/campaigns/{id}/schedule/discord`
///
/// # Errors
///
/// 404; 400 `INVALID_WEBHOOK`.
pub async fn set_discord(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(body): Body<WebhookBody>,
) -> Result<Response, AppError> {
    schedule::set_webhook(&state.pool, &gm, parse_id(&id)?, body.webhook.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn player_view(state: &AppState, p: &CurrentPlayer) -> Result<serde_json::Value, AppError> {
    let pool = &state.pool;
    let campaign = p.0.campaign_id;
    let seats = players::seats(pool, campaign).await?;
    let names: Vec<(Uuid, String)> = seats.iter().map(|s| (s.id, s.nickname.clone())).collect();
    let view = project_schedule(
        schedule::next_number(pool, campaign).await?,
        &schedule::open_dates(pool, campaign).await?,
        &schedule::answers(pool, campaign).await?,
        &names,
        p.0.id,
        p.0.role == Role::Player,
    );
    serde_json::to_value(view).map_err(|e| AppError::internal("schedule view", e))
}

/// `GET /api/play/{campaign}/schedule`
///
/// # Errors
///
/// 401 `NOT_JOINED`; a database error.
pub async fn play_schedule(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    Ok(Json(json!({ "data": player_view(&state, &p).await? })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerBody {
    available: bool,
}

/// `PUT /api/play/{campaign}/schedule/{date}` — answers the schedule.
///
/// # Errors
///
/// The codes of `schedule::answer`.
pub async fn answer(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Path((_, date)): Path<(String, String)>,
    Body(body): Body<AnswerBody>,
) -> Result<Response, AppError> {
    let date = Uuid::parse_str(&date).map_err(|_| AppError::NotFound("NO_SUCH_DATE"))?;
    schedule::answer(&state.pool, &p.0, date, body.available).await?;
    Ok(Json(json!({ "data": player_view(&state, &p).await? })).into_response())
}

/// `GET /api/play/{campaign}/schedule.ics`
///
/// # Errors
///
/// 404 `NO_DATE` while no date is fixed.
pub async fn calendar(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    let pool = &state.pool;
    let campaign = p.0.campaign_id;
    let row = campaigns::find(pool, campaign)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))?;
    let date = schedule::open_dates(pool, campaign)
        .await?
        .into_iter()
        .find(|d| d.status == DateStatus::Chosen)
        .ok_or(AppError::NotFound("NO_DATE"))?;
    let number = schedule::next_number(pool, campaign).await?;
    let ics = schedule::ics::calendar(
        &row.story.title,
        number,
        &date,
        &state.notifier.link(campaign),
        Utc::now(),
    );
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "text/calendar; charset=utf-8".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"seance-{number}.ics\""),
            ),
        ],
        ics,
    )
        .into_response())
}
