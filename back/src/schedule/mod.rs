//! session/schedule-sessions — the date of the next session (migration
//! `029_schedule.sql`).
//!
//! The GM proposes a few dates; each player says which suit them (a
//! spectator does not answer); the GM picks one, and the table is told.
//! Then the server keeps time on its own ([`tick`]): the day before and
//! an hour before, a reminder; a quarter of an hour before, the lobby
//! opens — the same opening as the GM's (`evening::session`), under the
//! campaign lock, on a validated campaign only.
//!
//! Reminders reach the phones through the table's Discord channel
//! ([`notify`]), each with the link that opens the lobby in one touch,
//! and each player's own calendar ([`ics`], with the same two alarms).
//! The app shows the date and the lobby too, for whoever opens it.
//!
//! Every write takes the campaign lock and touches the `session` topic:
//! the GM's table and the phones refetch.

pub mod ics;
pub mod notify;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;
use crate::evening;
use crate::players::{Player, Role};
pub use notify::Notifier;

/// The lobby opens this long before the session.
pub const LOBBY_BEFORE_MINUTES: i64 = 15;
/// The first reminder, the day before.
const EVE_BEFORE_HOURS: i64 = 24;
/// The second, an hour before.
const HOUR_BEFORE_MINUTES: i64 = 60;
/// Dates open for answers at once.
const MAX_PROPOSED: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateStatus {
    Proposed,
    Chosen,
    Dropped,
    Done,
}

impl DateStatus {
    fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "proposed" => Ok(Self::Proposed),
            "chosen" => Ok(Self::Chosen),
            "dropped" => Ok(Self::Dropped),
            "done" => Ok(Self::Done),
            other => Err(AppError::Internal(format!("unknown date status {other}"))),
        }
    }
}

/// A date for the next session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDate {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub minutes: i32,
    pub status: DateStatus,
    pub chosen_at: Option<DateTime<Utc>>,
    pub eve_reminded_at: Option<DateTime<Utc>>,
    pub hour_reminded_at: Option<DateTime<Utc>>,
    pub lobby_opened_at: Option<DateTime<Utc>>,
}

impl SessionDate {
    #[must_use]
    pub fn lobby_opens_at(&self) -> DateTime<Utc> {
        self.starts_at - Duration::minutes(LOBBY_BEFORE_MINUTES)
    }

    #[must_use]
    pub fn ends_at(&self) -> DateTime<Utc> {
        self.starts_at + Duration::minutes(i64::from(self.minutes))
    }
}

type Row = (
    Uuid,
    Uuid,
    DateTime<Utc>,
    i32,
    String,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

const COLUMNS: &str = "id, campaign_id, starts_at, minutes, status, chosen_at, \
                       eve_reminded_at, hour_reminded_at, lobby_opened_at";

fn from_row(r: Row) -> Result<SessionDate, AppError> {
    Ok(SessionDate {
        id: r.0,
        campaign_id: r.1,
        starts_at: r.2,
        minutes: r.3,
        status: DateStatus::parse(&r.4)?,
        chosen_at: r.5,
        eve_reminded_at: r.6,
        hour_reminded_at: r.7,
        lobby_opened_at: r.8,
    })
}

/// The dates of `campaign` still in play — proposed or chosen — soonest
/// first.
///
/// # Errors
///
/// A database error.
pub async fn open_dates(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
) -> Result<Vec<SessionDate>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM session_dates
         WHERE campaign_id = $1 AND status IN ('proposed', 'chosen') ORDER BY starts_at"
    ))
    .bind(campaign)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(from_row).collect()
}

async fn date_of(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    id: Uuid,
) -> Result<SessionDate, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM session_dates WHERE campaign_id = $1 AND id = $2"
    ))
    .bind(campaign)
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(from_row)
        .transpose()?
        .ok_or(AppError::NotFound("NO_SUCH_DATE"))
}

/// One player's answer for one date.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DateAnswer {
    pub date_id: Uuid,
    pub player_id: Uuid,
    pub available: bool,
}

/// Every answer given for the open dates of `campaign`.
///
/// # Errors
///
/// A database error.
pub async fn answers(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<DateAnswer>, AppError> {
    let rows: Vec<(Uuid, Uuid, bool)> = sqlx::query_as(
        "SELECT a.date_id, a.player_id, a.available FROM date_answers a
         JOIN session_dates d ON d.id = a.date_id
         WHERE d.campaign_id = $1 AND d.status IN ('proposed', 'chosen')",
    )
    .bind(campaign)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(date_id, player_id, available)| DateAnswer {
            date_id,
            player_id,
            available,
        })
        .collect())
}

/// The number of the session the fixed date is for: the one open, or
/// the one after the last.
///
/// # Errors
///
/// A database error.
pub async fn next_number(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<i32, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(
           (SELECT number FROM game_sessions WHERE campaign_id = $1 AND status <> 'ended'),
           (SELECT COALESCE(MAX(number), 0) + 1 FROM game_sessions WHERE campaign_id = $1))",
    )
    .bind(campaign)
    .fetch_one(db)
    .await?)
}

/// A date the GM proposes.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Proposal {
    pub starts_at: DateTime<Utc>,
    #[serde(default = "default_minutes")]
    pub minutes: i32,
}

fn default_minutes() -> i32 {
    150
}

/// The GM proposes a date to the table.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 400 `DATE_IN_PAST`,
/// `INVALID_DURATION`; 409 `TOO_MANY_DATES`.
pub async fn propose(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    p: &Proposal,
) -> Result<SessionDate, AppError> {
    if p.starts_at <= Utc::now() {
        return Err(AppError::BadRequest("DATE_IN_PAST"));
    }
    if !(30..=720).contains(&p.minutes) {
        return Err(AppError::BadRequest("INVALID_DURATION"));
    }
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let open: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM session_dates WHERE campaign_id = $1 AND status = 'proposed'",
    )
    .bind(campaign)
    .fetch_one(&mut *tx)
    .await?;
    if open >= MAX_PROPOSED {
        return Err(AppError::Conflict("TOO_MANY_DATES"));
    }
    let row: Row = sqlx::query_as(&format!(
        "INSERT INTO session_dates (campaign_id, starts_at, minutes) VALUES ($1, $2, $3)
         RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(p.starts_at)
    .bind(p.minutes)
    .fetch_one(&mut *tx)
    .await?;
    evening::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    from_row(row)
}

/// The GM withdraws a date, proposed or chosen.
///
/// # Errors
///
/// 404 `NO_SUCH_DATE`, or the campaign missing or another GM's; 409
/// `DATE_CLOSED` (already dropped or past).
pub async fn drop_date(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    date: Uuid,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let d = date_of(&mut *tx, campaign, date).await?;
    if !matches!(d.status, DateStatus::Proposed | DateStatus::Chosen) {
        return Err(AppError::Conflict("DATE_CLOSED"));
    }
    sqlx::query("UPDATE session_dates SET status = 'dropped' WHERE id = $1")
        .bind(date)
        .execute(&mut *tx)
        .await?;
    evening::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(())
}

/// The GM fixes the date: the other proposals close, the table is told.
/// A reminder already due by then is not sent again: this message is it.
///
/// # Errors
///
/// 404 `NO_SUCH_DATE`, or the campaign missing or another GM's; 409
/// `DATE_CLOSED` (not a proposal any more); 400 `DATE_IN_PAST`.
pub async fn choose(
    pool: &PgPool,
    notifier: &Notifier,
    gm: &CurrentGm,
    campaign: Uuid,
    date: Uuid,
) -> Result<SessionDate, AppError> {
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let d = date_of(&mut *tx, campaign, date).await?;
    if d.status != DateStatus::Proposed {
        return Err(AppError::Conflict("DATE_CLOSED"));
    }
    let now = Utc::now();
    if d.starts_at <= now {
        return Err(AppError::BadRequest("DATE_IN_PAST"));
    }
    sqlx::query(
        "UPDATE session_dates SET status = 'dropped'
         WHERE campaign_id = $1 AND status IN ('proposed', 'chosen') AND id <> $2",
    )
    .bind(campaign)
    .bind(date)
    .execute(&mut *tx)
    .await?;
    let chosen: Row = sqlx::query_as(&format!(
        "UPDATE session_dates
         SET status = 'chosen', chosen_at = $2,
             eve_reminded_at = CASE WHEN starts_at - make_interval(hours => $3) <= $2
                                    THEN $2 END,
             hour_reminded_at = CASE WHEN starts_at - make_interval(mins => $4) <= $2
                                     THEN $2 END
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(date)
    .bind(now)
    .bind(i32::try_from(EVE_BEFORE_HOURS).unwrap_or(24))
    .bind(i32::try_from(HOUR_BEFORE_MINUTES).unwrap_or(60))
    .fetch_one(&mut *tx)
    .await?;
    evening::touch(&mut tx, campaign).await?;
    let number = next_number(&mut *tx, campaign).await?;
    tx.commit().await?;
    let chosen = from_row(chosen)?;
    let text = message(
        Kind::Chosen,
        &row.story.title,
        number,
        &chosen,
        &notifier.link(campaign),
    );
    send(pool, notifier, &chosen, Kind::Chosen, &text).await?;
    Ok(chosen)
}

/// Set or clear the table's Discord webhook.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 400
/// `INVALID_WEBHOOK`.
pub async fn set_webhook(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    webhook: Option<&str>,
) -> Result<(), AppError> {
    let checked = match webhook.map(str::trim).filter(|w| !w.is_empty()) {
        Some(w) => Some(notify::check_webhook(w)?),
        None => None,
    };
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    match checked {
        Some(w) => {
            sqlx::query(
                "INSERT INTO campaign_reminders (campaign_id, discord_webhook) VALUES ($1, $2)
                 ON CONFLICT (campaign_id) DO UPDATE
                 SET discord_webhook = EXCLUDED.discord_webhook, updated_at = now()",
            )
            .bind(campaign)
            .bind(w)
            .execute(&mut *tx)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM campaign_reminders WHERE campaign_id = $1")
                .bind(campaign)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

/// The table's Discord webhook, if the GM gave one. GM-only.
///
/// # Errors
///
/// A database error.
pub async fn webhook(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Option<String>, AppError> {
    Ok(
        sqlx::query_scalar("SELECT discord_webhook FROM campaign_reminders WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_optional(db)
            .await?,
    )
}

/// A player says whether a proposed date suits them.
///
/// # Errors
///
/// 403 `SPECTATOR`; 404 `NO_SUCH_DATE`; 409 `DATE_CLOSED`.
pub async fn answer(
    pool: &PgPool,
    player: &Player,
    date: Uuid,
    available: bool,
) -> Result<(), AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let mut tx = pool.begin().await?;
    campaigns::lock(&mut tx, player.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))?;
    let d = date_of(&mut *tx, player.campaign_id, date).await?;
    if d.status != DateStatus::Proposed {
        return Err(AppError::Conflict("DATE_CLOSED"));
    }
    sqlx::query(
        "INSERT INTO date_answers (date_id, player_id, available) VALUES ($1, $2, $3)
         ON CONFLICT (date_id, player_id)
         DO UPDATE SET available = EXCLUDED.available, answered_at = now()",
    )
    .bind(date)
    .bind(player.id)
    .bind(available)
    .execute(&mut *tx)
    .await?;
    evening::touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(())
}

/// What a message to the table is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Chosen,
    Eve,
    Hour,
    Lobby,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Chosen => "chosen",
            Self::Eve => "eve",
            Self::Hour => "hour",
            Self::Lobby => "lobby",
        }
    }
}

/// The message posted to the table's channel. Discord shows `<t:…>` in
/// each reader's own time zone.
#[must_use]
pub fn message(kind: Kind, title: &str, number: i32, d: &SessionDate, link: &str) -> String {
    let at = d.starts_at.timestamp();
    let lobby = d.lobby_opens_at().timestamp();
    match kind {
        Kind::Chosen => format!(
            "« {title} » : la séance {number} aura lieu <t:{at}:F>. Le salon ouvrira à <t:{lobby}:t>. \
             Rappels la veille et une heure avant. {link}"
        ),
        Kind::Eve => format!(
            "Rappel : séance {number} de « {title} » demain, <t:{at}:F>. Le salon ouvrira à <t:{lobby}:t>. {link}"
        ),
        Kind::Hour => format!(
            "Dans une heure : séance {number} de « {title} », à <t:{at}:t>. Le salon ouvre à <t:{lobby}:t>, \
             prépare tes écouteurs. {link}"
        ),
        Kind::Lobby => format!(
            "Le salon de « {title} » est ouvert : séance {number}, on commence <t:{at}:R>. Touche le lien pour entrer : {link}"
        ),
    }
}

/// Post `text` to the table's channel and log what happened. No webhook
/// is not an error: the app and the calendar still carry the date.
async fn send(
    pool: &PgPool,
    notifier: &Notifier,
    d: &SessionDate,
    kind: Kind,
    text: &str,
) -> Result<(), AppError> {
    let (ok, detail) = match webhook(pool, d.campaign_id).await? {
        Some(w) => match notifier.post(&w, text).await {
            Ok(()) => (true, String::new()),
            Err(e) => (false, e),
        },
        None => (false, "NO_WEBHOOK".to_string()),
    };
    if !ok {
        tracing::warn!(date = %d.id, kind = kind.as_str(), "reminder not delivered: {detail}");
    }
    log(pool, d.id, kind, ok, &detail).await
}

async fn log(
    db: impl PgExecutor<'_>,
    date: Uuid,
    kind: Kind,
    ok: bool,
    detail: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO reminder_log (date_id, kind, ok, detail) VALUES ($1, $2, $3, $4)")
        .bind(date)
        .bind(kind.as_str())
        .bind(ok)
        .bind(detail)
        .execute(db)
        .await?;
    Ok(())
}

/// One line of the reminder log.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub kind: String,
    pub ok: bool,
    pub detail: String,
    pub at: DateTime<Utc>,
}

/// What was sent about `date`, oldest first.
///
/// # Errors
///
/// A database error.
pub async fn log_of(db: impl PgExecutor<'_>, date: Uuid) -> Result<Vec<LogLine>, AppError> {
    let rows: Vec<(String, bool, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT kind, ok, detail, at FROM reminder_log WHERE date_id = $1 ORDER BY at, id",
    )
    .bind(date)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(kind, ok, detail, at)| LogLine {
            kind,
            ok,
            detail,
            at,
        })
        .collect())
}

/// What one pass of the clock did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Tick {
    /// Reminders posted (or attempted), by date.
    pub reminded: Vec<(Uuid, Kind)>,
    /// Lobbies opened, by campaign.
    pub opened: Vec<Uuid>,
}

/// Claim, at `now`, the `step` of every chosen date where `due` holds;
/// each date is claimed once, ever.
async fn claim(
    pool: &PgPool,
    now: DateTime<Utc>,
    only: Option<Uuid>,
    set: &str,
    due: &str,
) -> Result<Vec<SessionDate>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "UPDATE session_dates SET {set}
         WHERE status = 'chosen' AND ($2::uuid IS NULL OR campaign_id = $2) AND {due}
         RETURNING {COLUMNS}"
    ))
    .bind(now)
    .bind(only)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(from_row).collect()
}

/// One pass of the clock at `now`, over every campaign or only `only`
/// (tests): past dates close; due lobbies open and say so; due reminders
/// go out — the latest one only when the server was down through
/// several.
///
/// # Errors
///
/// A database error.
pub async fn tick(
    pool: &PgPool,
    notifier: &Notifier,
    now: DateTime<Utc>,
    only: Option<Uuid>,
) -> Result<Tick, AppError> {
    let mut out = Tick::default();
    sqlx::query(
        "UPDATE session_dates SET status = 'done'
         WHERE status = 'chosen' AND ($2::uuid IS NULL OR campaign_id = $2)
           AND starts_at + make_interval(mins => minutes) < $1",
    )
    .bind(now)
    .bind(only)
    .execute(pool)
    .await?;

    let lobbies = claim(
        pool,
        now,
        only,
        "lobby_opened_at = $1, hour_reminded_at = COALESCE(hour_reminded_at, $1),
         eve_reminded_at = COALESCE(eve_reminded_at, $1)",
        &format!("lobby_opened_at IS NULL AND starts_at - interval '{LOBBY_BEFORE_MINUTES} minutes' <= $1"),
    )
    .await?;
    for d in lobbies {
        match evening::session::open_scheduled(pool, d.campaign_id).await {
            Ok(session) => {
                out.opened.push(d.campaign_id);
                remind(pool, notifier, &d, Kind::Lobby, Some(session.number)).await?;
                out.reminded.push((d.id, Kind::Lobby));
            }
            Err(e) => {
                let code = match e {
                    AppError::Conflict(c) | AppError::NotFound(c) => c.to_string(),
                    other => format!("{other:?}"),
                };
                log(pool, d.id, Kind::Lobby, false, &code).await?;
            }
        }
    }

    let hours = claim(
        pool,
        now,
        only,
        "hour_reminded_at = $1, eve_reminded_at = COALESCE(eve_reminded_at, $1)",
        &format!("hour_reminded_at IS NULL AND starts_at - interval '{HOUR_BEFORE_MINUTES} minutes' <= $1"),
    )
    .await?;
    for d in hours {
        remind(pool, notifier, &d, Kind::Hour, None).await?;
        out.reminded.push((d.id, Kind::Hour));
    }

    let eves = claim(
        pool,
        now,
        only,
        "eve_reminded_at = $1",
        &format!(
            "eve_reminded_at IS NULL AND starts_at - interval '{EVE_BEFORE_HOURS} hours' <= $1"
        ),
    )
    .await?;
    for d in eves {
        remind(pool, notifier, &d, Kind::Eve, None).await?;
        out.reminded.push((d.id, Kind::Eve));
    }
    Ok(out)
}

async fn remind(
    pool: &PgPool,
    notifier: &Notifier,
    d: &SessionDate,
    kind: Kind,
    number: Option<i32>,
) -> Result<(), AppError> {
    let Some(row) = campaigns::find(pool, d.campaign_id).await? else {
        return Ok(());
    };
    let number = match number {
        Some(n) => n,
        None => next_number(pool, d.campaign_id).await?,
    };
    let text = message(
        kind,
        &row.story.title,
        number,
        d,
        &notifier.link(d.campaign_id),
    );
    send(pool, notifier, d, kind, &text).await
}

/// Run the clock every thirty seconds, for good. Only the server binary
/// starts it: tests call [`tick`] at the time they choose.
pub fn spawn(pool: PgPool, notifier: Notifier) {
    tokio::spawn(async move {
        let mut every = tokio::time::interval(std::time::Duration::from_secs(30));
        every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            every.tick().await;
            if let Err(e) = tick(&pool, &notifier, Utc::now(), None).await {
                tracing::warn!("schedule tick failed: {e:?}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date() -> SessionDate {
        SessionDate {
            id: Uuid::nil(),
            campaign_id: Uuid::nil(),
            starts_at: DateTime::parse_from_rfc3339("2026-10-10T18:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            minutes: 150,
            status: DateStatus::Chosen,
            chosen_at: None,
            eve_reminded_at: None,
            hour_reminded_at: None,
            lobby_opened_at: None,
        }
    }

    #[test]
    fn each_message_carries_the_time_in_the_reader_zone_and_the_way_in() {
        let d = date();
        let link = "https://promptus.example/partie/c1";
        for kind in [Kind::Chosen, Kind::Eve, Kind::Hour, Kind::Lobby] {
            let m = message(kind, "Les Corsaires", 4, &d, link);
            assert!(m.contains("Les Corsaires") && m.contains('4'), "{m}");
            assert!(m.ends_with(link), "{m}");
        }
        let hour = message(Kind::Hour, "C", 4, &d, link);
        // 18:30 UTC, the lobby a quarter of an hour before.
        assert!(hour.contains("<t:1791657000:t>"), "{hour}");
        assert!(hour.contains("<t:1791656100:t>"), "{hour}");
    }
}
