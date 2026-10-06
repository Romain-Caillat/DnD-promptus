//! session/open-lobby, session/end-session — a session's life.
//!
//! `lobby` → `live` → `ended`. A campaign has at most one session not
//! ended (a partial unique index). The world is the campaign's own
//! document and is never reset: the next session opens exactly where
//! the last one stopped — same scene, same clues, same clocks, same
//! fight if one was left running — and `world_at_end` keeps what it was
//! at the end, for the chronicle.

use chrono::{DateTime, Utc};
use promptus_shared::story::{MusicMood, WorldState};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::knowledge::{self, Gap};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::players::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Lobby,
    Live,
    Ended,
}

impl Status {
    fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "lobby" => Ok(Self::Lobby),
            "live" => Ok(Self::Live),
            "ended" => Ok(Self::Ended),
            other => Err(AppError::Internal(format!(
                "unknown session status {other}"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Lobby => "lobby",
            Self::Live => "live",
            Self::Ended => "ended",
        }
    }
}

/// The track everyone hears, and when it started: a phone that opens
/// later seeks to `now - started_at`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Music {
    pub url: String,
    pub title: String,
    pub mood: MusicMood,
    pub started_at: DateTime<Utc>,
}

/// A session as the server holds it. GM-side: players get
/// `projection::evening`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub number: i32,
    pub status: Status,
    pub opened_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub recap: String,
    pub previously: String,
    pub gm_changes: String,
    pub music: Option<Music>,
    pub gaps_at_end: Vec<Gap>,
}

type Row = (
    Uuid,
    Uuid,
    i32,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    String,
    String,
    String,
    Option<Json<Music>>,
    Option<Json<Vec<Gap>>>,
);

const COLUMNS: &str = "id, campaign_id, number, status, opened_at, started_at, ended_at, \
                       recap, previously, gm_changes, music, gaps_at_end";

fn from_row(r: Row) -> Result<Session, AppError> {
    Ok(Session {
        id: r.0,
        campaign_id: r.1,
        number: r.2,
        status: Status::parse(&r.3)?,
        opened_at: r.4,
        started_at: r.5,
        ended_at: r.6,
        recap: r.7,
        previously: r.8,
        gm_changes: r.9,
        music: r.10.map(|m| m.0),
        gaps_at_end: r.11.map(|g| g.0).unwrap_or_default(),
    })
}

/// The session of `campaign` not yet ended, if any.
///
/// # Errors
///
/// A database error.
pub async fn current(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Option<Session>, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM game_sessions WHERE campaign_id = $1 AND status <> 'ended'"
    ))
    .bind(campaign)
    .fetch_optional(db)
    .await?;
    row.map(from_row).transpose()
}

/// Session `id` of `campaign`.
///
/// # Errors
///
/// 404 `NO_SUCH_SESSION`; a database error.
pub async fn by_id(db: impl PgExecutor<'_>, campaign: Uuid, id: Uuid) -> Result<Session, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM game_sessions WHERE campaign_id = $1 AND id = $2"
    ))
    .bind(campaign)
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(from_row)
        .transpose()?
        .ok_or(AppError::NotFound("NO_SUCH_SESSION"))
}

/// Every session of `campaign`, the latest first: the chronicle.
///
/// # Errors
///
/// A database error.
pub async fn all(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<Session>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM game_sessions WHERE campaign_id = $1 ORDER BY number DESC"
    ))
    .bind(campaign)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(from_row).collect()
}

/// The last ended session of `campaign`, if any.
///
/// # Errors
///
/// A database error.
pub async fn last_ended(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
) -> Result<Option<Session>, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM game_sessions WHERE campaign_id = $1 AND status = 'ended'
         ORDER BY number DESC LIMIT 1"
    ))
    .bind(campaign)
    .fetch_optional(db)
    .await?;
    row.map(from_row).transpose()
}

/// Under the campaign lock taken by `gm`: the campaign and its current
/// session, which must be in one of `allowed`.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 409 `NO_SESSION`
/// when none is open, `SESSION_NOT_LIVE` / `SESSION_NOT_IN_LOBBY` when it
/// is not in an allowed state.
pub async fn lock_for_gm(
    tx: &mut Transaction<'_, Postgres>,
    gm: &CurrentGm,
    campaign: Uuid,
    allowed: &[Status],
) -> Result<(CampaignRow, Session), AppError> {
    let row = owned_by(campaigns::lock(tx, campaign).await?, gm)?;
    let session = current(&mut **tx, campaign)
        .await?
        .ok_or(AppError::Conflict("NO_SESSION"))?;
    check_status(&session, allowed)?;
    Ok((row, session))
}

/// Under the campaign lock taken for `player`: the campaign and its
/// current session, which must be in one of `allowed`.
///
/// # Errors
///
/// 409 `NO_SESSION`, `SESSION_NOT_LIVE`, `SESSION_NOT_IN_LOBBY`.
pub async fn lock_for_player(
    tx: &mut Transaction<'_, Postgres>,
    player: &Player,
    allowed: &[Status],
) -> Result<(CampaignRow, Session), AppError> {
    let row = campaigns::lock(tx, player.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))?;
    let session = current(&mut **tx, player.campaign_id)
        .await?
        .ok_or(AppError::Conflict("NO_SESSION"))?;
    check_status(&session, allowed)?;
    Ok((row, session))
}

fn check_status(session: &Session, allowed: &[Status]) -> Result<(), AppError> {
    if allowed.contains(&session.status) {
        return Ok(());
    }
    Err(AppError::Conflict(match allowed.first() {
        Some(Status::Lobby) => "SESSION_NOT_IN_LOBBY",
        _ => "SESSION_NOT_LIVE",
    }))
}

/// Open the lobby of the next session — or answer the one already open.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; a database error.
pub async fn open(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Session, AppError> {
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    if let Some(open) = current(&mut *tx, campaign).await? {
        return Ok(open);
    }
    let row: Row = sqlx::query_as(&format!(
        "INSERT INTO game_sessions (campaign_id, number)
         VALUES ($1, COALESCE((SELECT MAX(number) FROM game_sessions WHERE campaign_id = $1), 0) + 1)
         RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .fetch_one(&mut *tx)
    .await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    from_row(row)
}

/// The table is here: the session goes live.
///
/// # Errors
///
/// As [`lock_for_gm`]; 409 `SESSION_NOT_IN_LOBBY`.
pub async fn start(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Session, AppError> {
    let mut tx = pool.begin().await?;
    let (_, session) = lock_for_gm(&mut tx, gm, campaign, &[Status::Lobby]).await?;
    let row: Row = sqlx::query_as(&format!(
        "UPDATE game_sessions SET status = 'live', started_at = now()
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(session.id)
    .fetch_one(&mut *tx)
    .await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    from_row(row)
}

/// What the GM writes when ending a session.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ending {
    /// The GM's recap (GM-only).
    #[serde(default)]
    pub recap: String,
    /// « Précédemment… », published to the players.
    #[serde(default)]
    pub previously: String,
}

/// Longest recap or « Précédemment… ».
const RECAP_MAX: usize = 8_000;

/// « Terminer la session »: the world as it stands is recorded, the music
/// stops, the recap and « Précédemment… » are kept; the chronicle grows
/// by this session. The world itself stays as it is: the next session
/// resumes from it.
///
/// # Errors
///
/// As [`lock_for_gm`]; 400 `TEXT_TOO_LONG`.
pub async fn end(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    ending: &Ending,
) -> Result<Session, AppError> {
    let recap = super::clean_text(&ending.recap, RECAP_MAX)?;
    let previously = super::clean_text(&ending.previously, RECAP_MAX)?;
    let mut tx = pool.begin().await?;
    let (row, session) = lock_for_gm(&mut tx, gm, campaign, &[Status::Live, Status::Lobby]).await?;
    let gaps = knowledge::gaps(&row.story, &row.world);
    let saved: Row = sqlx::query_as(&format!(
        "UPDATE game_sessions
         SET status = 'ended', ended_at = now(), started_at = COALESCE(started_at, now()),
             recap = $2, previously = $3, world_at_end = $4, gaps_at_end = $5, music = NULL
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(session.id)
    .bind(&recap)
    .bind(&previously)
    .bind(Json(&row.world))
    .bind(Json(&gaps))
    .fetch_one(&mut *tx)
    .await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    from_row(saved)
}

/// Edit the recap or « Précédemment… » of an ended session (the GM
/// rereads them after the evening).
///
/// # Errors
///
/// 404 when missing or another GM's; 409 `SESSION_NOT_ENDED`; 400
/// `TEXT_TOO_LONG`.
pub async fn edit_recap(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
    ending: &Ending,
) -> Result<Session, AppError> {
    let recap = super::clean_text(&ending.recap, RECAP_MAX)?;
    let previously = super::clean_text(&ending.previously, RECAP_MAX)?;
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let session = by_id(&mut *tx, campaign, id).await?;
    if session.status != Status::Ended {
        return Err(AppError::Conflict("SESSION_NOT_ENDED"));
    }
    let saved: Row = sqlx::query_as(&format!(
        "UPDATE game_sessions SET recap = $2, previously = $3 WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(&recap)
    .bind(&previously)
    .fetch_one(&mut *tx)
    .await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    from_row(saved)
}

/// Write `status` of `session` — for the music and the scene helpers.
pub(super) async fn set_music(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    music: Option<&Music>,
) -> Result<(), AppError> {
    sqlx::query("UPDATE game_sessions SET music = $2 WHERE id = $1")
        .bind(session)
        .bind(music.map(Json))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// One seat in the lobby.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attendance {
    pub player_id: Uuid,
    pub sound_ok: bool,
    pub remote: bool,
    pub joined_at: DateTime<Utc>,
}

/// Who said they are here in `session`.
///
/// # Errors
///
/// A database error.
pub async fn attendance(
    db: impl PgExecutor<'_>,
    session: Uuid,
) -> Result<Vec<Attendance>, AppError> {
    let rows: Vec<(Uuid, bool, bool, DateTime<Utc>)> = sqlx::query_as(
        "SELECT player_id, sound_ok, remote, joined_at FROM session_attendance
         WHERE session_id = $1 ORDER BY joined_at",
    )
    .bind(session)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(player_id, sound_ok, remote, joined_at)| Attendance {
            player_id,
            sound_ok,
            remote,
            joined_at,
        })
        .collect())
}

/// What a player says in the lobby.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Arrival {
    #[serde(default)]
    pub sound_ok: bool,
    #[serde(default = "yes")]
    pub remote: bool,
}

fn yes() -> bool {
    true
}

/// A player arrives in the lobby (or the session already live), and says
/// whether their sound works and where they play from.
///
/// # Errors
///
/// 409 `NO_SESSION`; a database error.
pub async fn arrive(pool: &PgPool, player: &Player, arrival: Arrival) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let (_, session) = lock_for_player(&mut tx, player, &[Status::Lobby, Status::Live]).await?;
    sqlx::query(
        "INSERT INTO session_attendance (session_id, player_id, sound_ok, remote)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (session_id, player_id)
         DO UPDATE SET sound_ok = EXCLUDED.sound_ok, remote = EXCLUDED.remote",
    )
    .bind(session.id)
    .bind(player.id)
    .bind(arrival.sound_ok)
    .bind(arrival.remote)
    .execute(&mut *tx)
    .await?;
    super::touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(())
}

/// The world a session left behind, for the chronicle.
///
/// # Errors
///
/// A database error.
pub async fn world_at_end(
    db: impl PgExecutor<'_>,
    session: Uuid,
) -> Result<Option<WorldState>, AppError> {
    let world: Option<Option<Json<WorldState>>> =
        sqlx::query_scalar("SELECT world_at_end FROM game_sessions WHERE id = $1")
            .bind(session)
            .fetch_optional(db)
            .await?;
    Ok(world.flatten().map(|w| w.0))
}

impl Status {
    /// The status as stored.
    #[must_use]
    pub fn key(self) -> &'static str {
        self.as_str()
    }
}
