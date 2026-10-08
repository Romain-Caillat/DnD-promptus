//! session/open-lobby, session/end-session, session/write-recaps,
//! gm/launch-session — a session's life.
//!
//! `lobby` → `live` → `ended`. A campaign has at most one session not
//! ended (a partial unique index). The world is the campaign's own
//! document and is never reset: the next session opens exactly where
//! the last one stopped — same scene, same clues, same clocks, same
//! fight if one was left running — and `world_at_end` keeps what it was
//! at the end, for the chronicle; `world_at_start` what it was when the
//! session went live, so the recap knows what changed.
//!
//! The recaps: ending keeps the GM's texts as a draft, and fills what
//! the GM left empty with a factual draft from the session's facts
//! (`promptus_shared::story::recap`). Nothing reaches the players until
//! the GM publishes (`published_at`): « Précédemment… » and the
//! chronicle entry are player text from then on.
//!
//! The launch: when the next session goes live, the last published
//! « Précédemment… » is read to the table sentence by sentence
//! (`previously_shown`), at the GM's pace, until the GM shows a scene.

use chrono::{DateTime, Utc};
use promptus_shared::story::{MusicMood, WorldState};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use promptus_shared::story::recap::{self, FactsRecap};

use super::knowledge::{self, Gap, JournalKind};
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
    pub chronicle_title: String,
    pub chronicle: String,
    /// When the GM published « Précédemment… » and the chronicle entry;
    /// `None` while they are drafts.
    pub published_at: Option<DateTime<Utc>>,
    pub gm_changes: String,
    pub music: Option<Music>,
    pub gaps_at_end: Vec<Gap>,
    /// Sentences of the last « Précédemment… » shown to the table while
    /// launching; `None` outside the reading.
    pub previously_shown: Option<i32>,
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
    String,
    String,
    Option<DateTime<Utc>>,
    Option<i32>,
);

const COLUMNS: &str = "id, campaign_id, number, status, opened_at, started_at, ended_at, \
                       recap, previously, gm_changes, music, gaps_at_end, \
                       chronicle_title, chronicle, published_at, previously_shown";

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
        chronicle_title: r.12,
        chronicle: r.13,
        published_at: r.14,
        previously_shown: r.15,
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
/// The campaign moves to its newest locked rule version first.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 409
/// `CAMPAIGN_NOT_VALIDATED`; a database error.
pub async fn open(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Session, AppError> {
    let mut tx = pool.begin().await?;
    let campaign_row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    open_locked(tx, &campaign_row).await
}

/// session/schedule-sessions: the lobby opens on its own at the time the
/// GM fixed — the same opening as the GM's, without a GM at the keyboard.
///
/// # Errors
///
/// 404 `NO_SUCH_CAMPAIGN`; 409 `CAMPAIGN_NOT_VALIDATED`; a database
/// error.
pub async fn open_scheduled(pool: &PgPool, campaign: Uuid) -> Result<Session, AppError> {
    let mut tx = pool.begin().await?;
    let campaign_row = campaigns::lock(&mut tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NO_SUCH_CAMPAIGN"))?;
    open_locked(tx, &campaign_row).await
}

/// Open the lobby of `campaign_row`, locked in `tx` — or answer the
/// session already open.
async fn open_locked(
    mut tx: Transaction<'_, Postgres>,
    campaign_row: &CampaignRow,
) -> Result<Session, AppError> {
    let campaign = campaign_row.id;
    if let Some(open) = current(&mut *tx, campaign).await? {
        return Ok(open);
    }
    // Only a campaign the GM declared playable is played
    // (`campaign/review-story-graph`).
    if campaign_row.validated_at.is_none() {
        return Err(AppError::Conflict("CAMPAIGN_NOT_VALIDATED"));
    }
    // A rule change ships between sessions: the newest locked version
    // applies from this one (`campaign/edit-rule-system`).
    crate::rules::versions::adopt_newest(&mut tx, campaign_row).await?;
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

/// The table is here: the session goes live (gm/launch-session). The
/// world is photographed for the recap, and when the last session's
/// « Précédemment… » is published its reading begins: the first
/// sentence reaches the table, the GM shows the next ones.
///
/// # Errors
///
/// As [`lock_for_gm`]; 409 `SESSION_NOT_IN_LOBBY`.
pub async fn start(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Session, AppError> {
    let mut tx = pool.begin().await?;
    let (campaign_row, session) = lock_for_gm(&mut tx, gm, campaign, &[Status::Lobby]).await?;
    let reading = published_previously(&mut *tx, campaign)
        .await?
        .is_some()
        .then_some(1);
    let row: Row = sqlx::query_as(&format!(
        "UPDATE game_sessions
         SET status = 'live', started_at = now(), world_at_start = $2, previously_shown = $3
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(session.id)
    .bind(Json(&campaign_row.world))
    .bind(reading)
    .fetch_one(&mut *tx)
    .await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    from_row(row)
}

/// The recaps of a session, as the GM writes them — at the end, or
/// after, rereading the draft.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ending {
    /// The GM's recap (GM-only).
    #[serde(default)]
    pub recap: String,
    /// « Précédemment… », for the players once published.
    #[serde(default)]
    pub previously: String,
    /// The chronicle entry, for the players once published.
    #[serde(default)]
    pub chronicle_title: String,
    #[serde(default)]
    pub chronicle: String,
    /// After the session only: publish « Précédemment… » and the
    /// chronicle entry. A published recap stays published; the GM's
    /// later edits reach the table as they are.
    #[serde(default)]
    pub publish: bool,
}

/// Longest recap, « Précédemment… » or chronicle entry.
const RECAP_MAX: usize = 8_000;
/// Longest chronicle title.
const TITLE_MAX: usize = 120;

/// The texts of `ending`, trimmed and bounded: recap, « Précédemment… »,
/// chronicle title, chronicle entry.
fn clean_ending(ending: &Ending) -> Result<[String; 4], AppError> {
    Ok([
        super::clean_text(&ending.recap, RECAP_MAX)?,
        super::clean_text(&ending.previously, RECAP_MAX)?,
        super::clean_text(&ending.chronicle_title, TITLE_MAX)?,
        super::clean_text(&ending.chronicle, RECAP_MAX)?,
    ])
}

/// « Terminer la session »: the world as it stands is recorded, the music
/// stops, the recaps are kept as drafts — what the GM left empty, the
/// server drafts from the session's facts — and the chronicle grows
/// by this session once the GM publishes. Nothing typed here reaches the
/// players before that. The world itself stays as it is: the next
/// session resumes from it.
///
/// # Errors
///
/// As [`lock_for_gm`]; 400 `TEXT_TOO_LONG`, `PUBLISH_AFTER_THE_END`
/// (asked to publish here: publishing comes after rereading).
pub async fn end(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    ending: &Ending,
) -> Result<Session, AppError> {
    if ending.publish {
        return Err(AppError::BadRequest("PUBLISH_AFTER_THE_END"));
    }
    let [mut recap, mut previously, mut title, mut chronicle] = clean_ending(ending)?;
    let mut tx = pool.begin().await?;
    let (row, session) = lock_for_gm(&mut tx, gm, campaign, &[Status::Live, Status::Lobby]).await?;
    // Whatever the GM left empty, the facts draft.
    if [&recap, &previously, &title, &chronicle]
        .iter()
        .any(|t| t.is_empty())
    {
        let ended = Session {
            ended_at: Some(Utc::now()),
            ..session.clone()
        };
        let draft = facts_draft(&mut tx, &row, &ended, &row.world).await?;
        for (mine, drafted) in [
            (&mut recap, draft.gm),
            (&mut previously, draft.players),
            (&mut title, draft.chronicle_title),
            (&mut chronicle, draft.chronicle),
        ] {
            if mine.is_empty() {
                *mine = drafted;
            }
        }
    }
    let gaps = knowledge::gaps(&row.story, &row.world);
    let saved: Row = sqlx::query_as(&format!(
        "UPDATE game_sessions
         SET status = 'ended', ended_at = now(), started_at = COALESCE(started_at, now()),
             recap = $2, previously = $3, world_at_end = $4, gaps_at_end = $5, music = NULL,
             chronicle_title = $6, chronicle = $7, previously_shown = NULL
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(session.id)
    .bind(&recap)
    .bind(&previously)
    .bind(Json(&row.world))
    .bind(Json(&gaps))
    .bind(&title)
    .bind(&chronicle)
    .fetch_one(&mut *tx)
    .await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    from_row(saved)
}

/// Edit the recaps of an ended session (the GM rereads them after the
/// evening), and with `publish` give « Précédemment… » and the chronicle
/// entry to the players.
///
/// # Errors
///
/// 404 when missing or another GM's; 409 `SESSION_NOT_ENDED`; 400
/// `TEXT_TOO_LONG`, `NOTHING_TO_PUBLISH` (an empty « Précédemment… »).
pub async fn edit_recap(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
    ending: &Ending,
) -> Result<Session, AppError> {
    let [recap, previously, title, chronicle] = clean_ending(ending)?;
    if ending.publish && previously.is_empty() {
        return Err(AppError::BadRequest("NOTHING_TO_PUBLISH"));
    }
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let session = by_id(&mut *tx, campaign, id).await?;
    if session.status != Status::Ended {
        return Err(AppError::Conflict("SESSION_NOT_ENDED"));
    }
    let saved: Row = sqlx::query_as(&format!(
        "UPDATE game_sessions
         SET recap = $2, previously = $3, chronicle_title = $4, chronicle = $5,
             published_at = CASE WHEN $6 THEN COALESCE(published_at, now()) ELSE published_at END
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(&recap)
    .bind(&previously)
    .bind(&title)
    .bind(&chronicle)
    .bind(ending.publish)
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

/// The world when `session` began: its own photograph, or the world the
/// previous session left, or the campaign's untouched world.
///
/// # Errors
///
/// A database error.
pub async fn world_at_start(
    db: &mut sqlx::PgConnection,
    session: &Session,
) -> Result<WorldState, AppError> {
    let own: Option<Option<Json<WorldState>>> =
        sqlx::query_scalar("SELECT world_at_start FROM game_sessions WHERE id = $1")
            .bind(session.id)
            .fetch_optional(&mut *db)
            .await?;
    if let Some(Some(w)) = own {
        return Ok(w.0);
    }
    let previous: Option<Option<Json<WorldState>>> = sqlx::query_scalar(
        "SELECT world_at_end FROM game_sessions
         WHERE campaign_id = $1 AND number < $2 AND status = 'ended'
         ORDER BY number DESC LIMIT 1",
    )
    .bind(session.campaign_id)
    .bind(session.number)
    .fetch_optional(&mut *db)
    .await?;
    Ok(previous.flatten().map(|w| w.0).unwrap_or_default())
}

/// The session's shared journal lines worth retelling: a fight, loot, a
/// promise, a debt, a key item. Scenes, clues and names come from the
/// facts; rolls are too many to retell.
///
/// # Errors
///
/// A database error.
pub async fn table_lines(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    session: Uuid,
) -> Result<Vec<String>, AppError> {
    Ok(knowledge::journal(db, campaign, true)
        .await?
        .into_iter()
        .filter(|l| l.session_id == Some(session))
        .filter(|l| {
            matches!(
                l.kind,
                JournalKind::Fight
                    | JournalKind::Loot
                    | JournalKind::Promise
                    | JournalKind::Debt
                    | JournalKind::Item
            )
        })
        .map(|l| l.text)
        .collect())
}

/// The recaps of `session` drafted from its facts — from its start to
/// `world_now` — without any model. The GM's recap also lists what the
/// next scenes need that the table does not know.
///
/// # Errors
///
/// A database error.
pub async fn facts_draft(
    db: &mut sqlx::PgConnection,
    row: &CampaignRow,
    session: &Session,
    world_now: &WorldState,
) -> Result<FactsRecap, AppError> {
    let before = world_at_start(&mut *db, session).await?;
    let facts = recap::session_facts(&row.story, &before, world_now);
    let lines = table_lines(&mut *db, row.id, session.id).await?;
    let minutes = match (session.started_at, session.ended_at) {
        (Some(a), Some(b)) => Some((b - a).num_minutes().max(0)),
        _ => None,
    };
    let mut draft = recap::facts_recap(&facts, &lines, minutes);
    let gaps = knowledge::gaps(&row.story, world_now);
    if !gaps.is_empty() {
        let missing: Vec<String> = gaps
            .iter()
            .map(|g| format!("- « {} » demande : {}", g.node_title, g.statement))
            .collect();
        draft.gm = format!(
            "{}\n\nCe que la suite demande et que la table ignore :\n{}",
            draft.gm,
            missing.join("\n")
        )
        .trim_start()
        .to_string();
    }
    if draft.chronicle_title.is_empty() {
        draft.chronicle_title = format!("Session {}", session.number);
    }
    Ok(draft)
}

/// « Précédemment… » of the last ended session of `campaign`, with that
/// session's number — only once the GM published it.
///
/// # Errors
///
/// A database error.
pub async fn published_previously(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
) -> Result<Option<(i32, String)>, AppError> {
    let row: Option<(i32, String, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT number, previously, published_at FROM game_sessions
         WHERE campaign_id = $1 AND status = 'ended' ORDER BY number DESC LIMIT 1",
    )
    .bind(campaign)
    .fetch_optional(db)
    .await?;
    Ok(
        row.and_then(|(n, text, at)| {
            (at.is_some() && !text.trim().is_empty()).then_some((n, text))
        }),
    )
}

/// gm/launch-session: the next sentence of « Précédemment… » reaches the
/// table. Past the last one, nothing moves.
///
/// # Errors
///
/// As [`lock_for_gm`] (the session must be live); 409 `NOT_READING`
/// when no reading is under way.
pub async fn read_next(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Session, AppError> {
    let mut tx = pool.begin().await?;
    let (_, session) = lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    let shown = session
        .previously_shown
        .ok_or(AppError::Conflict("NOT_READING"))?;
    let total = published_previously(&mut *tx, campaign)
        .await?
        .map_or(0, |(_, text)| recap::lines(&text).len());
    let next = (shown + 1)
        .min(i32::try_from(total).unwrap_or(i32::MAX))
        .max(shown);
    let row: Row = sqlx::query_as(&format!(
        "UPDATE game_sessions SET previously_shown = $2 WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(session.id)
    .bind(next)
    .fetch_one(&mut *tx)
    .await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    from_row(row)
}

/// The reading of « Précédemment… » is over: a scene is shown.
pub(super) async fn end_reading(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
) -> Result<(), AppError> {
    sqlx::query("UPDATE game_sessions SET previously_shown = NULL WHERE id = $1")
        .bind(session)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
