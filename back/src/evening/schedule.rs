//! session/schedule-sessions — the next session's date (migration
//! `021_recaps_and_schedule.sql`).
//!
//! The GM proposes a few start times ([`propose`]); each player says
//! which suit them ([`answer`]); the GM picks one ([`choose`]). When the
//! chosen time comes, minus the lobby's head start, the lobby opens by
//! itself ([`tick`], run by [`spawn`]): the reminder on a phone leads
//! straight into it. Opening the lobby, by hand or on time, spends the
//! plan (`session::open_locked`).

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;
use crate::players::{Player, Role};

/// Most start times one proposal offers.
pub const OPTIONS_MAX: usize = 6;
/// How often the server looks for a lobby to open.
const TICK: Duration = Duration::from_secs(30);

/// The next session as it is being planned. GM-side; players read it
/// through `campaigns::projection::between`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub options: Vec<DateTime<Utc>>,
    pub chosen_at: Option<DateTime<Utc>>,
    pub lobby_minutes: i32,
    /// Who answered, and which options suit them.
    pub answers: Vec<Availability>,
}

impl Plan {
    /// When the lobby opens, once a time is chosen.
    #[must_use]
    pub fn lobby_at(&self) -> Option<DateTime<Utc>> {
        self.chosen_at
            .map(|at| at - chrono::Duration::minutes(i64::from(self.lobby_minutes)))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Availability {
    pub player_id: Uuid,
    pub available: Vec<DateTime<Utc>>,
}

/// The plan of `campaign`, if the GM started one.
///
/// # Errors
///
/// A database error.
pub async fn get(pool: &PgPool, campaign: Uuid) -> Result<Option<Plan>, AppError> {
    type Row = (Vec<DateTime<Utc>>, Option<DateTime<Utc>>, i32);
    let row: Option<Row> = sqlx::query_as(
        "SELECT options, chosen_at, lobby_minutes FROM session_plans WHERE campaign_id = $1",
    )
    .bind(campaign)
    .fetch_optional(pool)
    .await?;
    let Some((options, chosen_at, lobby_minutes)) = row else {
        return Ok(None);
    };
    let answers: Vec<(Uuid, Vec<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT a.player_id, a.available FROM session_availability a
         JOIN players p ON p.id = a.player_id
         WHERE a.campaign_id = $1 ORDER BY p.created_at, p.id",
    )
    .bind(campaign)
    .fetch_all(pool)
    .await?;
    Ok(Some(Plan {
        answers: answers
            .into_iter()
            .map(|(player_id, available)| Availability {
                player_id,
                // Only what is still proposed.
                available: available
                    .into_iter()
                    .filter(|a| options.contains(a))
                    .collect(),
            })
            .collect(),
        options,
        chosen_at,
        lobby_minutes,
    }))
}

/// What the GM proposes.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Proposal {
    pub options: Vec<DateTime<Utc>>,
    /// How long before the start the lobby opens; 15 by default.
    #[serde(default)]
    pub lobby_minutes: Option<i32>,
}

/// Times to the minute, in the future, each once, in order.
fn clean_times(raw: &[DateTime<Utc>], now: DateTime<Utc>) -> Result<Vec<DateTime<Utc>>, AppError> {
    let mut times: Vec<DateTime<Utc>> = raw.iter().map(|t| t.with_second_and_nano_zero()).collect();
    times.sort();
    times.dedup();
    if times.iter().any(|t| *t <= now) {
        return Err(AppError::BadRequest("DATE_IN_PAST"));
    }
    Ok(times)
}

trait ZeroSeconds {
    fn with_second_and_nano_zero(self) -> Self;
}

impl ZeroSeconds for DateTime<Utc> {
    fn with_second_and_nano_zero(self) -> Self {
        use chrono::Timelike;
        self.with_second(0)
            .and_then(|t| t.with_nanosecond(0))
            .unwrap_or(self)
    }
}

async fn touch(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    campaign: Uuid,
) -> Result<(), AppError> {
    super::touch(tx, campaign).await
}

/// The GM proposes start times (one to [`OPTIONS_MAX`]). A chosen time
/// no longer proposed is unchosen.
///
/// # Errors
///
/// 404 when missing or another GM's; 400 `INVALID_OPTIONS`,
/// `DATE_IN_PAST`, `INVALID_LOBBY_MINUTES`.
pub async fn propose(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    proposal: &Proposal,
) -> Result<Plan, AppError> {
    let options = clean_times(&proposal.options, Utc::now())?;
    if options.is_empty() || options.len() > OPTIONS_MAX {
        return Err(AppError::BadRequest("INVALID_OPTIONS"));
    }
    let lobby_minutes = proposal.lobby_minutes.unwrap_or(15);
    if !(0..=180).contains(&lobby_minutes) {
        return Err(AppError::BadRequest("INVALID_LOBBY_MINUTES"));
    }
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    sqlx::query(
        "INSERT INTO session_plans (campaign_id, options, lobby_minutes) VALUES ($1, $2, $3)
         ON CONFLICT (campaign_id) DO UPDATE
           SET options = EXCLUDED.options, lobby_minutes = EXCLUDED.lobby_minutes,
               chosen_at = CASE WHEN session_plans.chosen_at = ANY(EXCLUDED.options)
                                THEN session_plans.chosen_at END",
    )
    .bind(campaign)
    .bind(&options)
    .bind(lobby_minutes)
    .execute(&mut *tx)
    .await?;
    touch(&mut tx, campaign).await?;
    tx.commit().await?;
    get(pool, campaign)
        .await?
        .ok_or_else(|| AppError::Internal("plan vanished".into()))
}

/// The GM fixes the date: one of the proposed times, or `None` to undo.
///
/// # Errors
///
/// 404 when missing or another GM's; 409 `NO_PLAN`; 400
/// `NOT_PROPOSED`, `DATE_IN_PAST`.
pub async fn choose(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    at: Option<DateTime<Utc>>,
) -> Result<Plan, AppError> {
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let options: Option<Vec<DateTime<Utc>>> =
        sqlx::query_scalar("SELECT options FROM session_plans WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_optional(&mut *tx)
            .await?;
    let options = options.ok_or(AppError::Conflict("NO_PLAN"))?;
    let at = match at {
        Some(at) => {
            let at = clean_times(&[at], Utc::now())?[0];
            if !options.contains(&at) {
                return Err(AppError::BadRequest("NOT_PROPOSED"));
            }
            Some(at)
        }
        None => None,
    };
    sqlx::query("UPDATE session_plans SET chosen_at = $2 WHERE campaign_id = $1")
        .bind(campaign)
        .bind(at)
        .execute(&mut *tx)
        .await?;
    touch(&mut tx, campaign).await?;
    tx.commit().await?;
    get(pool, campaign)
        .await?
        .ok_or_else(|| AppError::Internal("plan vanished".into()))
}

/// A player says which proposed times suit them (all at once).
///
/// # Errors
///
/// 403 `SPECTATOR`; 409 `NO_PLAN`; 400 `NOT_PROPOSED`.
pub async fn answer(
    pool: &PgPool,
    player: &Player,
    available: &[DateTime<Utc>],
) -> Result<(), AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let mut tx = pool.begin().await?;
    campaigns::lock(&mut tx, player.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))?;
    let options: Option<Vec<DateTime<Utc>>> =
        sqlx::query_scalar("SELECT options FROM session_plans WHERE campaign_id = $1")
            .bind(player.campaign_id)
            .fetch_optional(&mut *tx)
            .await?;
    let options = options.ok_or(AppError::Conflict("NO_PLAN"))?;
    let mut mine: Vec<DateTime<Utc>> = available
        .iter()
        .map(|t| t.with_second_and_nano_zero())
        .collect();
    mine.sort();
    mine.dedup();
    if mine.iter().any(|t| !options.contains(t)) {
        return Err(AppError::BadRequest("NOT_PROPOSED"));
    }
    sqlx::query(
        "INSERT INTO session_availability (campaign_id, player_id, available) VALUES ($1, $2, $3)
         ON CONFLICT (campaign_id, player_id)
         DO UPDATE SET available = EXCLUDED.available, updated_at = now()",
    )
    .bind(player.campaign_id)
    .bind(player.id)
    .bind(&mine)
    .execute(&mut *tx)
    .await?;
    touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(())
}

/// Opens every lobby whose time has come: a chosen date minus its head
/// start, on a validated campaign with no session open. Returns the
/// campaigns opened.
///
/// # Errors
///
/// A database error.
pub async fn tick(pool: &PgPool, now: DateTime<Utc>) -> Result<Vec<Uuid>, AppError> {
    let due: Vec<Uuid> = sqlx::query_scalar(
        "SELECT campaign_id FROM session_plans
         WHERE chosen_at IS NOT NULL
           AND chosen_at - make_interval(mins => lobby_minutes) <= $1",
    )
    .bind(now)
    .fetch_all(pool)
    .await?;
    let mut opened = Vec::new();
    for campaign in due {
        let mut tx = pool.begin().await?;
        let Some(row) = campaigns::lock(&mut tx, campaign).await? else {
            continue;
        };
        // Re-read under the lock: the GM may have opened it, or moved
        // the date, meanwhile.
        let still_due: Option<bool> = sqlx::query_scalar(
            "SELECT chosen_at - make_interval(mins => lobby_minutes) <= $2
             FROM session_plans WHERE campaign_id = $1 AND chosen_at IS NOT NULL",
        )
        .bind(campaign)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await?;
        if still_due != Some(true)
            || row.validated_at.is_none()
            || super::session::current(&mut *tx, campaign).await?.is_some()
        {
            continue;
        }
        super::session::open_locked(&mut tx, &row).await?;
        tx.commit().await?;
        opened.push(campaign);
    }
    Ok(opened)
}

/// Runs [`tick`] every few seconds for as long as the server lives.
pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        let mut every = tokio::time::interval(TICK);
        loop {
            every.tick().await;
            match tick(&pool, Utc::now()).await {
                Ok(opened) => {
                    for c in opened {
                        tracing::info!(campaign = %c, "planned session: lobby opened");
                    }
                }
                Err(e) => tracing::warn!("planned sessions: {e:?}"),
            }
        }
    });
}

/// The planned session as a calendar event with a reminder an hour
/// before, its link straight to the player's table. `None` until the GM
/// chose a time.
#[must_use]
pub fn ics(plan: &Plan, title: &str, number: i32, url: &str, now: DateTime<Utc>) -> Option<String> {
    let at = plan.chosen_at?;
    let fmt = |t: DateTime<Utc>| t.format("%Y%m%dT%H%M%SZ").to_string();
    // Text values escape `\`, `;`, `,` and newlines (RFC 5545 §3.3.11).
    let text = |s: &str| {
        s.replace('\\', "\\\\")
            .replace(';', "\\;")
            .replace(',', "\\,")
            .replace('\n', "\\n")
    };
    let summary = text(&format!("{title} · session {number}"));
    let lines = [
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".into(),
        "PRODID:-//Promptus//FR".into(),
        "BEGIN:VEVENT".into(),
        format!("UID:{}-{number}@promptus", fmt(at)),
        format!("DTSTAMP:{}", fmt(now)),
        format!("DTSTART:{}", fmt(at)),
        format!("DTEND:{}", fmt(at + chrono::Duration::hours(3))),
        format!("SUMMARY:{summary}"),
        format!("URL:{url}"),
        format!("DESCRIPTION:{}", text(url)),
        "BEGIN:VALARM".into(),
        "ACTION:DISPLAY".into(),
        "TRIGGER:-PT1H".into(),
        format!("DESCRIPTION:{summary}"),
        "END:VALARM".into(),
        "END:VEVENT".into(),
        "END:VCALENDAR".into(),
    ];
    Some(lines.join("\r\n") + "\r\n")
}

/// The number the next session of `campaign` will take.
///
/// # Errors
///
/// A database error.
pub async fn next_number(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<i32, AppError> {
    let n: Option<i32> =
        sqlx::query_scalar("SELECT MAX(number) FROM game_sessions WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_one(db)
            .await?;
    Ok(n.unwrap_or(0) + 1)
}
