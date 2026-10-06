//! gm/balance-spotlight — six players, and who has not had a moment.
//!
//! For each seated player with a character in play: when they last had a
//! moment in this session (`player_moments`: a request, a roll, a move or
//! an action in a fight, or a moment the GM gave them), their requests
//! waiting for an answer, and the hooks of their backstory not played
//! yet. Past [`ALERT_AFTER_MINUTES`] without a moment during a live
//! session, the GM is warned; the co-GM can then propose an opening for
//! them in the current scene (`copilot`, kind `spotlight`).

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use super::session::{self, Session, Status};
use super::{MomentKind, moment};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;

/// Minutes without a moment before the GM is warned.
pub const ALERT_AFTER_MINUTES: i64 = 20;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookBrief {
    pub id: Uuid,
    pub title: String,
}

/// One player in the spotlight panel.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spot {
    pub player_id: Uuid,
    pub nickname: String,
    pub character_id: Uuid,
    pub character_name: String,
    /// Their latest moment in this session, if any.
    pub last_moment_at: Option<DateTime<Utc>>,
    /// Minutes since their latest moment, or since the session started.
    pub idle_minutes: i64,
    pub pending_requests: i64,
    pub hooks: Vec<HookBrief>,
    /// Idle for [`ALERT_AFTER_MINUTES`] or more during a live session.
    pub alert: bool,
}

/// The spotlight of `session`, as of `now`, longest idle first.
///
/// # Errors
///
/// A database error.
pub async fn spots(
    db: &PgPool,
    campaign: Uuid,
    session: &Session,
    now: DateTime<Utc>,
) -> Result<Vec<Spot>, AppError> {
    type Row = (Uuid, String, Uuid, String, Option<DateTime<Utc>>, i64);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT p.id, p.nickname, c.id, COALESCE(c.sheet->>'name', ''),
                (SELECT MAX(m.at) FROM player_moments m
                  WHERE m.session_id = $2 AND m.player_id = p.id),
                (SELECT COUNT(*) FROM player_requests r
                  WHERE r.session_id = $2 AND r.player_id = p.id AND r.status IN ('pending', 'check'))
         FROM players p JOIN characters c ON c.player_id = p.id
         WHERE p.campaign_id = $1 AND p.role = 'player' AND c.status = 'validated'
         ORDER BY p.created_at",
    )
    .bind(campaign)
    .bind(session.id)
    .fetch_all(db)
    .await?;
    let since = session.started_at.unwrap_or(session.opened_at);
    let mut out = Vec::with_capacity(rows.len());
    for (player_id, nickname, character_id, character_name, last, pending) in rows {
        let idle = (now - last.unwrap_or(since)).num_minutes().max(0);
        out.push(Spot {
            player_id,
            nickname,
            character_id,
            character_name,
            last_moment_at: last,
            idle_minutes: idle,
            pending_requests: pending,
            hooks: unplayed_hooks(db, character_id).await?,
            alert: session.status == Status::Live && idle >= ALERT_AFTER_MINUTES,
        });
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.idle_minutes));
    Ok(out)
}

async fn unplayed_hooks(
    db: impl PgExecutor<'_>,
    character: Uuid,
) -> Result<Vec<HookBrief>, AppError> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, title FROM secret_hooks WHERE character_id = $1 AND played_at IS NULL
         ORDER BY created_at",
    )
    .bind(character)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, title)| HookBrief { id, title })
        .collect())
}

/// The GM gives `player` a moment (they described a scene of their own,
/// an NPC spoke to them).
///
/// # Errors
///
/// As `session::lock_for_gm` (live session); 404 `NO_SUCH_PLAYER`.
pub async fn give(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    player: Uuid,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let (_, live) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    let seated: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM players WHERE id = $1 AND campaign_id = $2")
            .bind(player)
            .bind(campaign)
            .fetch_optional(&mut *tx)
            .await?;
    seated.ok_or(AppError::NotFound("NO_SUCH_PLAYER"))?;
    moment(&mut tx, live.id, player, MomentKind::Spotlight).await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(())
}

/// The GM played hook `hook` (or takes it back to the list).
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, or the hook is not
/// of this campaign.
pub async fn hook_played(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    hook: Uuid,
    played: bool,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let done = sqlx::query(
        "UPDATE secret_hooks SET played_at = CASE WHEN $3 THEN COALESCE(played_at, now()) END
         WHERE id = $1 AND campaign_id = $2",
    )
    .bind(hook)
    .bind(campaign)
    .bind(played)
    .execute(&mut *tx)
    .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound("NO_SUCH_HOOK"));
    }
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(())
}

/// The longest stretch without a moment for each player of `session`,
/// in minutes, from its start to its end (or `now`).
///
/// # Errors
///
/// A database error.
pub async fn longest_idle(
    db: &PgPool,
    session: &Session,
    players: &[Uuid],
    now: DateTime<Utc>,
) -> Result<Vec<(Uuid, i64)>, AppError> {
    let start = session.started_at.unwrap_or(session.opened_at);
    let end = session.ended_at.unwrap_or(now);
    let mut out = Vec::with_capacity(players.len());
    for player in players {
        let moments: Vec<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT at FROM player_moments WHERE session_id = $1 AND player_id = $2 ORDER BY at",
        )
        .bind(session.id)
        .bind(player)
        .fetch_all(db)
        .await?;
        let mut longest = Duration::zero();
        let mut previous = start;
        for at in moments.into_iter().chain(std::iter::once(end)) {
            let at = at.clamp(start, end);
            longest = longest.max(at - previous);
            previous = previous.max(at);
        }
        out.push((*player, longest.num_minutes()));
    }
    Ok(out)
}
