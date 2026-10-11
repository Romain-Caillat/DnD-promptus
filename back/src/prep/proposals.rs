//! One model of batch of proposals (`story_proposals`, migrations 016
//! and 049) for the co-GM's workshop and for Claude
//! (`platform/propose-edits-by-claude`): what was asked or its title,
//! the story edits and map operations proposed, and the GM's word on
//! each item. Nothing in a batch is in the campaign until the GM keeps
//! it; a batch is shown applied « virtually » ([`Virtual`]) on top of
//! the campaign as it is now, never stored that way.
//!
//! Items have keys: `e<i>` for the i-th story edit, `m<i>` for the i-th
//! batch of map operations.

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use promptus_shared::maps::edit::{self as ops, MapOp, MapTouch};
use promptus_shared::maps::{Cell, Map};
use promptus_shared::story::Campaign;
use promptus_shared::story::edit::{self, Change, Edit};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

use crate::campaigns::CampaignRow;
use crate::error::AppError;

/// Who wrote a batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// The co-GM's workshop.
    Copilot,
    /// Claude, with the GM's personal token.
    Token,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// An item still waits for the GM.
    Pending,
    /// Every item is decided, at least one kept.
    Accepted,
    /// Every item is decided, none kept.
    Rejected,
}

impl Status {
    fn parse(s: &str) -> Result<Self, AppError> {
        Ok(match s {
            "pending" => Self::Pending,
            "accepted" => Self::Accepted,
            "rejected" => Self::Rejected,
            other => return Err(AppError::Internal(format!("proposal status {other}"))),
        })
    }
}

/// Where on a map a batch is about: the map, and a cell if Claude named
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Focus {
    pub map: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Cell>,
}

/// A batch of map operations proposed, and the campaign revision it was
/// checked at: kept, it replays under the editor's rules from there
/// (`campaign_maps::prepare`), so it never overwrites what the GM
/// painted since.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapItem {
    pub map: String,
    pub revision: i64,
    pub ops: Vec<MapOp>,
}

/// The GM's word on one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Pending,
    Kept,
    Discarded,
    /// Kept with the rest, but it no longer applied: nothing written.
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub status: ItemStatus,
    /// The GM's word when discarding: Claude reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word: Option<String>,
    /// Why a kept item was refused (`EDIT_UNKNOWN_TARGET`, `MAP_STALE`…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// A batch as stored.
#[derive(Debug, Clone)]
pub(crate) struct Stored {
    pub id: Uuid,
    pub prompt: String,
    pub reply: String,
    pub edits: Vec<Edit>,
    pub dropped: i32,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub author: Origin,
    pub focus: Option<Focus>,
    pub map_ops: Vec<MapItem>,
    pub decisions: BTreeMap<String, Decision>,
    pub feedback: String,
}

pub(crate) type Row = (
    Uuid,
    String,
    String,
    Json<Vec<Edit>>,
    i32,
    String,
    DateTime<Utc>,
    String,
    Option<Json<Focus>>,
    Json<Vec<MapItem>>,
    Json<BTreeMap<String, Decision>>,
    String,
);

pub(crate) const COLUMNS: &str = "id, prompt, reply, edits, dropped, status, created_at, author, \
     focus, map_ops, decisions, feedback";

pub(crate) fn stored(
    (
        id,
        prompt,
        reply,
        Json(edits),
        dropped,
        status,
        created_at,
        author,
        focus,
        Json(map_ops),
        Json(decisions),
        feedback,
    ): Row,
) -> Result<Stored, AppError> {
    Ok(Stored {
        id,
        prompt,
        reply,
        edits,
        dropped,
        status: Status::parse(&status)?,
        created_at,
        author: if author == "token" {
            Origin::Token
        } else {
            Origin::Copilot
        },
        focus: focus.map(|f| f.0),
        map_ops,
        decisions,
        feedback,
    })
}

pub(crate) fn edit_key(i: usize) -> String {
    format!("e{i}")
}

pub(crate) fn map_key(i: usize) -> String {
    format!("m{i}")
}

impl Stored {
    /// Every item's key, story edits first, in order.
    pub fn keys(&self) -> Vec<String> {
        (0..self.edits.len())
            .map(edit_key)
            .chain((0..self.map_ops.len()).map(map_key))
            .collect()
    }

    pub fn status_of(&self, key: &str) -> ItemStatus {
        self.decisions
            .get(key)
            .map_or(ItemStatus::Pending, |d| d.status)
    }

    pub fn pending(&self, key: &str) -> bool {
        self.status_of(key) == ItemStatus::Pending
    }

    /// The batch's status once its decisions are in.
    pub fn settled(&self) -> Status {
        let keys = self.keys();
        if keys.iter().any(|k| self.pending(k)) {
            Status::Pending
        } else if keys.iter().any(|k| self.status_of(k) == ItemStatus::Kept) {
            Status::Accepted
        } else {
            Status::Rejected
        }
    }
}

/// A batch applied on the campaign as it is now, without storing it:
/// each pending item in order on what the earlier ones made, an item
/// that no longer applies skipped.
pub(crate) struct Virtual {
    pub story: Campaign,
    /// What each story edit changes; `None` when decided or stale.
    pub changes: Vec<Option<Change>>,
    /// Each map batch's map after it and what it touched; `None` when
    /// decided or stale.
    pub maps: Vec<Option<(Map, Vec<MapTouch>)>>,
}

impl Virtual {
    /// Every map as the batch leaves it, among `maps`.
    pub fn maps_after(&self, maps: &[Map]) -> Vec<Map> {
        let mut out: Vec<Map> = maps.to_vec();
        for (after, _) in self.maps.iter().flatten() {
            if let Some(m) = out.iter_mut().find(|m| m.id == after.id) {
                *m = after.clone();
            }
        }
        out
    }
}

/// `s` applied virtually on `row`, its map batches on `maps` (the
/// campaign's own maps; a world map is never edited).
pub(crate) fn virtual_of(row: &CampaignRow, s: &Stored, maps: &[Map]) -> Virtual {
    let library = super::edit_library(row);
    let mut story = row.story.clone();
    let changes = s
        .edits
        .iter()
        .enumerate()
        .map(|(i, e)| {
            if !s.pending(&edit_key(i)) {
                return None;
            }
            let (next, mut changes) =
                edit::apply_with(&story, std::slice::from_ref(e), &library).ok()?;
            story = next;
            changes.pop()
        })
        .collect();
    let mut now: HashMap<&str, Map> = HashMap::new();
    let maps = s
        .map_ops
        .iter()
        .enumerate()
        .map(|(i, item)| {
            if !s.pending(&map_key(i)) {
                return None;
            }
            let before = now
                .get(item.map.as_str())
                .cloned()
                .or_else(|| maps.iter().find(|m| m.id == item.map).cloned())?;
            let after = ops::apply(&before, &item.ops).ok()?;
            after.validate().ok()?;
            let touched = ops::diff(&before, &after);
            now.insert(item.map.as_str(), after.clone());
            Some((after, touched))
        })
        .collect();
    Virtual {
        story,
        changes,
        maps,
    }
}

/// A batch as the GM's screens and Claude read it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub id: Uuid,
    pub author: Origin,
    /// What the GM asked the co-GM, or the title Claude gave the batch.
    pub prompt: String,
    pub reply: String,
    pub edits: Vec<Edit>,
    /// What the pending edits change, read against the campaign as it
    /// is now. Empty when they no longer apply (`stale`).
    pub changes: Vec<Change>,
    /// The co-GM's edits no longer apply as a whole; Claude's batch has
    /// a pending item that no longer applies.
    pub stale: bool,
    /// Edits the co-GM wrote that were dropped (invented ids…).
    pub dropped: i32,
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<Focus>,
    /// Every item and the GM's word on it.
    pub items: Vec<ItemView>,
    /// The GM's word on the whole batch.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub feedback: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    pub key: String,
    pub status: ItemStatus,
    /// Pending, but it no longer applies to the campaign as it is now.
    pub stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// A story edit: the edit, and what it changes (pending only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit: Option<Edit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change: Option<Change>,
    /// A batch of map operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<MapItemView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapItemView {
    pub map: String,
    /// The map's name, when it still exists.
    pub name: String,
    pub ops: usize,
    /// The cells and things it touches (pending only).
    pub touches: Vec<MapTouch>,
}

/// `s` as the screens read it, against `row` as it is now and the
/// campaign's `maps`.
pub(crate) fn view(row: &CampaignRow, s: Stored, maps: &[Map]) -> Proposal {
    let pending = s.status == Status::Pending;
    let v = virtual_of(row, &s, maps);
    let mut items: Vec<ItemView> = Vec::new();
    for (i, e) in s.edits.iter().enumerate() {
        let key = edit_key(i);
        let d = s.decisions.get(&key);
        let status = d.map_or(ItemStatus::Pending, |d| d.status);
        let change = v.changes[i].clone();
        items.push(ItemView {
            stale: status == ItemStatus::Pending && change.is_none(),
            word: d.and_then(|d| d.word.clone()),
            code: d.and_then(|d| d.code.clone()),
            edit: Some(e.clone()),
            change,
            map: None,
            status,
            key,
        });
    }
    for (i, m) in s.map_ops.iter().enumerate() {
        let key = map_key(i);
        let d = s.decisions.get(&key);
        let status = d.map_or(ItemStatus::Pending, |d| d.status);
        let after = v.maps[i].as_ref();
        items.push(ItemView {
            stale: status == ItemStatus::Pending && after.is_none(),
            word: d.and_then(|d| d.word.clone()),
            code: d.and_then(|d| d.code.clone()),
            edit: None,
            change: None,
            map: Some(MapItemView {
                map: m.map.clone(),
                name: maps
                    .iter()
                    .find(|x| x.id == m.map)
                    .map_or_else(String::new, |x| x.name.clone()),
                ops: m.ops.len(),
                touches: after.map(|(_, t)| t.clone()).unwrap_or_default(),
            }),
            status,
            key,
        });
    }
    // The co-GM's proposal is taken whole: it is stale as soon as the
    // whole no longer applies, and then shows no change.
    let (changes, stale) = match (pending, s.author) {
        (false, _) => (Vec::new(), false),
        (true, Origin::Copilot) => {
            match edit::apply_with(&row.story, &s.edits, &super::edit_library(row)) {
                Ok((_, changes)) => (changes, false),
                Err(_) => (Vec::new(), true),
            }
        }
        (true, Origin::Token) => (
            v.changes.into_iter().flatten().collect(),
            items.iter().any(|i| i.stale),
        ),
    };
    Proposal {
        id: s.id,
        author: s.author,
        prompt: s.prompt,
        reply: s.reply,
        edits: s.edits,
        changes,
        stale,
        dropped: s.dropped,
        status: s.status,
        focus: s.focus,
        items,
        feedback: s.feedback,
        created_at: s.created_at,
    }
}

/// The campaign's own maps, when `s` carries map batches (a world map
/// is never edited).
///
/// # Errors
///
/// A database error.
pub(crate) async fn maps_for(
    db: impl sqlx::PgExecutor<'_>,
    campaign: Uuid,
    batches: &[&Stored],
) -> Result<Vec<Map>, AppError> {
    if batches.iter().all(|s| s.map_ops.is_empty()) {
        return Ok(Vec::new());
    }
    Ok(crate::campaign_maps::all(db, campaign)
        .await?
        .into_iter()
        .map(|m| m.map)
        .collect())
}
