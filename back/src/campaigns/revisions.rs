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
//!
//! Map operations (`maps/edit-maps-by-ops`) join the same journal and
//! the same number through [`record_map`], called by
//! `campaign_maps::write`: a line whose `target` is `map:<id>`, whose
//! changes are the cells and things the batch touched
//! (`maps::edit::MapTouch`) and whose inverse is the whole map before.
//! Taking it back puts back only what it touched (`maps::edit::revert`);
//! it is covered by a later line on the same map that touched the same
//! cells or things (`maps::edit::overlaps`).

use chrono::{DateTime, Utc};
use promptus_shared::maps::Map;
use promptus_shared::maps::edit::{self, MapTouch};
use promptus_shared::story::Campaign;
use promptus_shared::story::revision::{self, Op, Restore, Touch};
use serde::Serialize;
use serde_json::Value;
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

/// The target of the story's lines.
pub const STORY: &str = "story";

/// The target of map `id`'s lines.
#[must_use]
pub fn map_target(id: &str) -> String {
    format!("map:{id}")
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
    let inverse = (author != Author::Server).then(|| json(&diff.inverse));
    let line = Line {
        target: STORY,
        changes: json(&diff.touched),
        inverse,
        author,
        undoes,
    };
    append(tx, id, line).await.map(Some)
}

/// Journal a write of map `map` on campaign `id`, locked in `tx`: a
/// batch of operations (`before` and `after`), a new map (`before` is
/// `None`) or a deleted one (`after` is `None`). Only a batch can be
/// taken back: its inverse is the map before. Nothing when nothing
/// moved.
///
/// # Errors
///
/// Fails on a database error.
pub async fn record_map(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    before: Option<&Map>,
    after: Option<&Map>,
    author: Author,
    undoes: Option<i64>,
) -> Result<Option<i64>, AppError> {
    let (touched, inverse) = match (before, after) {
        (Some(b), Some(a)) => (edit::diff(b, a), Some(json(b))),
        (None, Some(a)) => (vec![MapTouch::whole(Op::Add, a)], None),
        (Some(b), None) => (vec![MapTouch::whole(Op::Remove, b)], None),
        (None, None) => (vec![], None),
    };
    let Some(map) = after.or(before) else {
        return Ok(None);
    };
    if touched.is_empty() {
        return Ok(None);
    }
    let target = map_target(&map.id);
    let line = Line {
        target: &target,
        changes: json(&touched),
        inverse: inverse.filter(|_| author != Author::Server),
        author,
        undoes,
    };
    append(tx, id, line).await.map(Some)
}

/// A touch on map `id` as a whole, standing for a line that does not
/// read: it overlaps everything.
fn whole(id: &str) -> MapTouch {
    MapTouch {
        op: Op::Remove,
        kind: edit::MapKind::Map,
        id: id.to_string(),
        name: String::new(),
        cells: vec![],
    }
}

fn json(value: &impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// One line about to be journaled.
struct Line<'a> {
    target: &'a str,
    changes: Value,
    inverse: Option<Value>,
    author: Author,
    undoes: Option<i64>,
}

/// The one place a journal line is written: the next revision under the
/// campaign lock, the line, the GM-only signal.
async fn append(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    line: Line<'_>,
) -> Result<i64, AppError> {
    let rev: i64 = sqlx::query_scalar(
        "UPDATE campaigns SET prep_revision = prep_revision + 1 WHERE id = $1
         RETURNING prep_revision",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO prep_revisions
           (campaign_id, revision, author, target, changes, inverse, undoes)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(rev)
    .bind(line.author.key())
    .bind(line.target)
    .bind(Json(&line.changes))
    .bind(line.inverse.map(Json))
    .bind(line.undoes)
    .execute(&mut **tx)
    .await?;
    let activity = (line.author == Author::Token).then(|| Activity {
        author: line.author.key().to_string(),
        revision: rev,
    });
    live::announce(tx, id, &Topic::Prep, rev, activity).await?;
    Ok(rev)
}

/// The touches of the map lines of `campaign` on map `map` after
/// revision `after`, for a batch sent on that revision to check it does
/// not step on them. A line that does not read counts as the whole map.
///
/// # Errors
///
/// Fails on a database error.
pub async fn map_lines_since(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    map: &str,
    after: i64,
) -> Result<Vec<Vec<MapTouch>>, AppError> {
    let rows: Vec<Json<Value>> = sqlx::query_scalar(
        "SELECT changes FROM prep_revisions
         WHERE campaign_id = $1 AND target = $2 AND revision > $3 ORDER BY revision",
    )
    .bind(campaign)
    .bind(map_target(map))
    .bind(after)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows
        .into_iter()
        .map(|Json(v)| serde_json::from_value(v).unwrap_or_else(|_| vec![whole(map)]))
        .collect())
}

/// One line of the journal, as the GM's screen reads it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub revision: i64,
    /// `gm`, `token`, `copilot` or `server`.
    pub author: String,
    /// `story`, or `map:<id>` for a batch of map operations.
    pub target: String,
    pub changes: Changes,
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

/// What a line touched: story entities, or the cells and things of a
/// map (each carrying the map's name).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Changes {
    Story(Vec<Touch>),
    Map(Vec<MapTouch>),
}

impl Changes {
    fn read(target: &str, value: Value) -> Self {
        if target == STORY {
            Self::Story(serde_json::from_value(value).unwrap_or_default())
        } else {
            Self::Map(serde_json::from_value(value).unwrap_or_default())
        }
    }
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
    Json<Value>,
    bool,
    Option<i64>,
    DateTime<Utc>,
);

/// A line's target, changes and inverse, as an undo reads them.
type UndoRow = (String, Json<Value>, Option<Json<Value>>);

/// Whether a story line touching `changes` is covered by one of `later`.
fn covered(changes: &[Touch], later: &[&[Touch]]) -> bool {
    later
        .iter()
        .any(|l| l.iter().any(|t| changes.iter().any(|c| c.same_entity(t))))
}

/// Whether a map line touching `changes` is covered by one of `later`
/// (lines on the same map): a later batch touched the same cells or
/// things, or the map was deleted.
fn map_covered(changes: &[MapTouch], later: &[Vec<MapTouch>]) -> bool {
    later.iter().any(|l| edit::overlaps(changes, l))
}

/// Whether the line at `i` of `rows` is covered by a later one on the
/// same target.
fn covered_in(rows: &[(String, Changes)], i: usize) -> bool {
    let (target, changes) = &rows[i];
    let later = rows[i + 1..].iter().filter(|(t, _)| t == target);
    match changes {
        Changes::Story(mine) => {
            let later: Vec<&[Touch]> = later
                .filter_map(|(_, c)| match c {
                    Changes::Story(t) => Some(t.as_slice()),
                    Changes::Map(_) => None,
                })
                .collect();
            covered(mine, &later)
        }
        Changes::Map(mine) => {
            let later: Vec<Vec<MapTouch>> = later
                .map(|(_, c)| match c {
                    Changes::Map(t) => t.clone(),
                    Changes::Story(_) => vec![],
                })
                .collect();
            map_covered(mine, &later)
        }
    }
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
    let read: Vec<(String, Changes)> = rows
        .iter_mut()
        .map(|r| (r.2.clone(), Changes::read(&r.2, std::mem::take(&mut r.3.0))))
        .collect();
    let entries = rows
        .iter()
        .zip(&read)
        .enumerate()
        .map(
            |(i, ((revision, author, target, _, has_inverse, undoes, at), (_, changes)))| Entry {
                revision: *revision,
                author: author.clone(),
                target: target.clone(),
                changes: changes.clone(),
                undoes: *undoes,
                undone: undone.contains(revision),
                undoable: *has_inverse && !covered_in(&read, i),
                created_at: *at,
            },
        )
        .collect();
    Ok(Since {
        revision: row.prep_revision,
        entries,
    })
}

/// Take back `revision` of `campaign`: put each entity it touched back
/// as it was — or, for a map batch, the map as it was — as a new
/// revision by the GM.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, `NO_SUCH_REVISION`;
/// 409 `REVISION_NOT_UNDOABLE` (a write the server made),
/// `REVISION_COVERED` (a later revision touched the same entities, or
/// the same map),
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
    let line: Option<UndoRow> = sqlx::query_as(
        "SELECT target, changes, inverse FROM prep_revisions
         WHERE campaign_id = $1 AND revision = $2",
    )
    .bind(campaign)
    .bind(revision)
    .fetch_optional(&mut *tx)
    .await?;
    let (target, changes, inverse) = line.ok_or(AppError::NotFound("NO_SUCH_REVISION"))?;
    let inverse = inverse.ok_or(AppError::Conflict("REVISION_NOT_UNDOABLE"))?;
    let later: Vec<Json<Value>> = sqlx::query_scalar(
        "SELECT changes FROM prep_revisions
         WHERE campaign_id = $1 AND revision > $2 AND target = $3",
    )
    .bind(campaign)
    .bind(revision)
    .bind(&target)
    .fetch_all(&mut *tx)
    .await?;
    if target == STORY {
        let changes: Vec<Touch> = serde_json::from_value(changes.0).unwrap_or_default();
        let later: Vec<Vec<Touch>> = later
            .into_iter()
            .map(|Json(v)| serde_json::from_value(v).unwrap_or_default())
            .collect();
        let later: Vec<&[Touch]> = later.iter().map(Vec::as_slice).collect();
        if covered(&changes, &later) {
            return Err(AppError::Conflict("REVISION_COVERED"));
        }
        let inverse: Vec<Restore> = serde_json::from_value(inverse.0)
            .map_err(|_| AppError::Conflict("REVISION_NOT_REVERTIBLE"))?;
        let story = revision::revert(&row.story, &inverse)
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
    } else {
        let map_id = target.strip_prefix("map:").unwrap_or(&target);
        let changes: Vec<MapTouch> = serde_json::from_value(changes.0)
            .map_err(|_| AppError::Conflict("REVISION_NOT_REVERTIBLE"))?;
        let later: Vec<Vec<MapTouch>> = later
            .into_iter()
            .map(|Json(v)| serde_json::from_value(v).unwrap_or_else(|_| vec![whole(map_id)]))
            .collect();
        if map_covered(&changes, &later) {
            return Err(AppError::Conflict("REVISION_COVERED"));
        }
        let before: Map = serde_json::from_value(inverse.0)
            .map_err(|_| AppError::Conflict("REVISION_NOT_REVERTIBLE"))?;
        crate::campaign_maps::restore(&mut tx, campaign, map_id, &before, &changes, revision)
            .await?;
    }
    let row = find(&mut *tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    tx.commit().await?;
    Ok(row)
}
