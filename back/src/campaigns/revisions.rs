//! The preparation journal of a campaign (`platform/stream-prep-edits`,
//! migration `047_prep_revisions.sql`).
//!
//! Every story write goes through `campaigns::save_story`, which calls
//! [`record`]: the write takes the next revision under the campaign lock
//! and leaves a line — who wrote it, which entities moved, and each of
//! them as it was (`story::revision`). Then:
//! - the GM's screens hear the new number on the GM-only `prep` topic,
//!   with an `activity` when a token wrote (Claude through the MCP
//!   server), and ask [`since`] what changed;
//! - the GM may [`undo`] a revision as long as no later one touched the
//!   same entities. An undo is a revision of its own, by the GM.

use chrono::{DateTime, Utc};
use promptus_shared::story::Campaign;
use promptus_shared::story::revision::{self, Restore, Touch};
use serde::Serialize;
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{CampaignRow, find, lock, write_story};
use crate::auth::guard::{Credential, CurrentGm, owned_by};
use crate::error::AppError;
use crate::live::{self, Activity, Topic};

/// Who wrote a preparation change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Author {
    /// The GM, in Promptus.
    Gm,
    /// A program with the GM's personal token: Claude, through the MCP
    /// server.
    Token,
    /// The co-GM: a workshop proposal or a generation the GM accepted.
    Copilot,
    /// The server on its own: a rule version adopted when a session
    /// opens. Not undone.
    Server,
}

impl Author {
    fn key(self) -> &'static str {
        match self {
            Self::Gm => "gm",
            Self::Token => "token",
            Self::Copilot => "copilot",
            Self::Server => "server",
        }
    }
}

impl From<Credential> for Author {
    fn from(credential: Credential) -> Self {
        match credential {
            Credential::Session => Self::Gm,
            Credential::Token => Self::Token,
        }
    }
}

/// Journal the write of `after` over `before` on campaign `id`, locked
/// in `tx`: the next revision, its line, and the GM-only signal. Nothing
/// when nothing moved.
///
/// # Errors
///
/// Fails on a database error.
pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    before: &Campaign,
    after: &Campaign,
    author: Author,
    undoes: Option<i64>,
) -> Result<Option<i64>, AppError> {
    let diff = revision::diff(before, after);
    if diff.is_empty() {
        return Ok(None);
    }
    let rev: i64 = sqlx::query_scalar(
        "UPDATE campaigns SET prep_revision = prep_revision + 1 WHERE id = $1
         RETURNING prep_revision",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    let inverse = (author != Author::Server).then_some(Json(&diff.inverse));
    sqlx::query(
        "INSERT INTO prep_revisions (campaign_id, revision, author, changes, inverse, undoes)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(rev)
    .bind(author.key())
    .bind(Json(&diff.touched))
    .bind(inverse)
    .bind(undoes)
    .execute(&mut **tx)
    .await?;
    let activity = (author == Author::Token).then(|| Activity {
        author: author.key().to_string(),
        revision: rev,
    });
    live::announce(tx, id, &Topic::Prep, rev, activity).await?;
    Ok(Some(rev))
}

/// One line of the journal, as the GM's screen reads it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub revision: i64,
    /// `gm`, `token`, `copilot` or `server`.
    pub author: String,
    /// `story` (maps join later, `maps/edit-maps-by-ops`).
    pub target: String,
    pub changes: Vec<Touch>,
    /// When it is an undo, the revision it took back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undoes: Option<i64>,
    /// A later revision took it back.
    pub undone: bool,
    /// The GM may take it back now: it has an inverse and no later
    /// revision touched what it touched.
    pub undoable: bool,
    pub created_at: DateTime<Utc>,
}

/// What changed since a revision the screen held.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Since {
    /// The campaign's revision now.
    pub revision: i64,
    /// The lines after the one asked, oldest first, at most [`SINCE_MAX`]
    /// (the newest ones).
    pub entries: Vec<Entry>,
}

/// The most lines one catch-up returns.
pub const SINCE_MAX: i64 = 200;

type EntryRow = (
    i64,
    String,
    String,
    Json<Vec<Touch>>,
    bool,
    Option<i64>,
    DateTime<Utc>,
);

/// A line's changes and inverse, as an undo reads them.
type UndoRow = (Json<Vec<Touch>>, Option<Json<Vec<Restore>>>);

/// Whether a line touching `changes` is covered by one of `later`.
fn covered(changes: &[Touch], later: &[&[Touch]]) -> bool {
    later
        .iter()
        .any(|l| l.iter().any(|t| changes.iter().any(|c| c.same_entity(t))))
}

/// The journal of `campaign` after revision `after`.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; a database error.
pub async fn since(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    after: i64,
) -> Result<Since, AppError> {
    let row = owned_by(find(pool, campaign).await?, gm)?;
    let mut rows: Vec<EntryRow> = sqlx::query_as(
        "SELECT revision, author, target, changes, inverse IS NOT NULL, undoes, created_at
         FROM prep_revisions WHERE campaign_id = $1 AND revision > $2
         ORDER BY revision DESC LIMIT $3",
    )
    .bind(campaign)
    .bind(after)
    .bind(SINCE_MAX)
    .fetch_all(pool)
    .await?;
    rows.reverse();
    let undone: Vec<i64> = rows.iter().filter_map(|r| r.5).collect();
    let entries = rows
        .iter()
        .enumerate()
        .map(
            |(i, (revision, author, target, changes, has_inverse, undoes, at))| {
                let later: Vec<&[Touch]> = rows[i + 1..].iter().map(|r| r.3.0.as_slice()).collect();
                Entry {
                    revision: *revision,
                    author: author.clone(),
                    target: target.clone(),
                    changes: changes.0.clone(),
                    undoes: *undoes,
                    undone: undone.contains(revision),
                    undoable: *has_inverse && !covered(&changes.0, &later),
                    created_at: *at,
                }
            },
        )
        .collect();
    Ok(Since {
        revision: row.prep_revision,
        entries,
    })
}

/// Take back `revision` of `campaign`: put each entity it touched back
/// as it was, as a new revision by the GM.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, `NO_SUCH_REVISION`;
/// 409 `REVISION_NOT_UNDOABLE` (a write the server made),
/// `REVISION_COVERED` (a later revision touched the same entities),
/// `REVISION_NOT_REVERTIBLE` (what it put back no longer fits the
/// campaign); a database error.
pub async fn undo(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    revision: i64,
) -> Result<CampaignRow, AppError> {
    let mut tx = pool.begin().await?;
    let row = owned_by(lock(&mut tx, campaign).await?, gm)?;
    let target: Option<UndoRow> = sqlx::query_as(
        "SELECT changes, inverse FROM prep_revisions WHERE campaign_id = $1 AND revision = $2",
    )
    .bind(campaign)
    .bind(revision)
    .fetch_optional(&mut *tx)
    .await?;
    let (changes, inverse) = target.ok_or(AppError::NotFound("NO_SUCH_REVISION"))?;
    let inverse = inverse.ok_or(AppError::Conflict("REVISION_NOT_UNDOABLE"))?;
    let later: Vec<Json<Vec<Touch>>> = sqlx::query_scalar(
        "SELECT changes FROM prep_revisions WHERE campaign_id = $1 AND revision > $2",
    )
    .bind(campaign)
    .bind(revision)
    .fetch_all(&mut *tx)
    .await?;
    let later: Vec<&[Touch]> = later.iter().map(|l| l.0.as_slice()).collect();
    if covered(&changes.0, &later) {
        return Err(AppError::Conflict("REVISION_COVERED"));
    }
    let story = revision::revert(&row.story, &inverse.0)
        .map_err(|_| AppError::Conflict("REVISION_NOT_REVERTIBLE"))?;
    write_story(
        &mut tx,
        campaign,
        &row.story,
        &story,
        Author::Gm,
        Some(revision),
    )
    .await?;
    let row = find(&mut *tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    tx.commit().await?;
    Ok(row)
}
