//! Between two sessions, from a player's phone (player/play-between-sessions,
//! behind `require_player`; every answer built by `campaigns::projection`):
//! - `GET  /api/play/{campaign}/between` → the last evening and what it
//!   brought me, the level-up moment, the chronicle, the open threads;
//! - `POST /api/play/{campaign}/character/upgrade` → `{ ability }`: spend
//!   one upgrade point, +1 to that score (`players::play::upgrade`);
//!   answers my character as `me` shows it.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use promptus_shared::rules::progression::level_for;
use serde::Deserialize;
use serde_json::json;

use super::body::Body;
use crate::auth::player::CurrentPlayer;
use crate::campaigns::projection::between::{BetweenInput, Mine, project_between};
use crate::campaigns::projection::{project_character, project_play};
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::evening::knowledge;
use crate::evening::session::{self, Status};
use crate::players::{self, CharacterStatus, play};
use crate::state::AppState;

async fn campaign_of(state: &AppState, p: &CurrentPlayer) -> Result<CampaignRow, AppError> {
    campaigns::find(&state.pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))
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
    let rules = row.rules();
    let current = session::current(pool, row.id).await?;
    let mut ended = Vec::new();
    for s in session::all(pool, row.id).await?.into_iter().rev() {
        if s.status == Status::Ended {
            let world = session::world_at_end(pool, s.id).await?;
            ended.push((s, world));
        }
    }
    let journal = knowledge::journal(pool, row.id, true).await?;
    let character = players::character_of(pool, &p.0).await?;
    let in_play = match (rules, &character) {
        (Some(r), Some(c)) if c.status == CharacterStatus::Validated => {
            project_play(r, &c.sheet, c.play.as_ref()).map(|v| (c.id, v))
        }
        _ => None,
    };
    let level = |xp: u32| rules.map_or(1, |r| level_for(r, xp));
    let mut mine = None;
    if let Some((id, view)) = &in_play {
        let started = ended.last().and_then(|(s, _)| Some((s, s.started_at?)));
        let (xp_in_session, xp_since_start, got) = match started {
            Some((s, start)) => (
                play::xp_earned(pool, *id, start, s.ended_at).await?,
                play::xp_earned(pool, *id, start, None).await?,
                knowledge::shared_about_character(pool, s.id, *id).await?,
            ),
            None => (0, 0, Vec::new()),
        };
        mine = Some(Mine {
            play: view,
            xp_in_session,
            xp_since_start,
            level_for: &level,
            got,
        });
    }
    let view = project_between(&BetweenInput {
        campaign: &row.story,
        current: current.as_ref(),
        ended: &ended,
        journal: &journal,
        mine,
    });
    Ok(Json(json!({ "data": view })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpgradeBody {
    pub ability: String,
}

/// `POST /api/play/{campaign}/character/upgrade`
///
/// # Errors
///
/// 401 `NOT_JOINED`; 404 `NO_CHARACTER`; 400 `UNKNOWN_ABILITY`,
/// `INVALID_BODY`; 409 `CHARACTER_NOT_VALIDATED`, `RULES_UNKNOWN`,
/// `SESSION_LIVE`, `NO_UPGRADE_POINT`.
pub async fn upgrade(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(body): Body<UpgradeBody>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let rules = row.rules();
    let Some(system) = rules else {
        players::character_of(&state.pool, &p.0)
            .await?
            .ok_or(AppError::NotFound("NO_CHARACTER"))?;
        return Err(AppError::Conflict("RULES_UNKNOWN"));
    };
    play::upgrade(&state.pool, &p.0, system, &body.ability).await?;
    let character = players::character_of(&state.pool, &p.0)
        .await?
        .ok_or(AppError::NotFound("NO_CHARACTER"))?;
    let view = project_character(rules, &character);
    Ok(Json(json!({ "data": view })).into_response())
}
