//! platform/propose-edits-by-claude — Claude proposes, the GM keeps.
//!
//! A write with the GM's personal token (`Credential::Token`: Claude,
//! through the MCP server) on the story or a map does not land in the
//! campaign: it is checked exactly like the GM's own write, then stored
//! as a batch of proposals (`prep::proposals`, the workshop's model) —
//! one item per story edit, one per batch of map operations — and the
//! GM's screens hear of it on the GM-only `proposals` topic. The atlas
//! draws a batch applied virtually, dotted. The GM, in Promptus only
//! (`SessionOnly`), keeps an item, discards one with a word, or keeps
//! or discards everything left ([`decide`]); keeping replays under the
//! campaign lock on the campaign as it is then, and what no longer
//! applies is refused, item by item. Claude reads the outcome and the
//! GM's words back ([`list`]).
//!
//! « Claude a la main » ([`set_hand`]): the GM may let Claude's writes
//! land directly — each one a line of the preparation journal, undone
//! like any other. Never while an evening is open (the mode cannot be
//! set then, and a write that comes then is proposed), and never on
//! what players already know ([`revealed`]): such a write is proposed.

use promptus_shared::maps::Map;
use promptus_shared::story::edit::{self, Change, Edit, Kind};
use promptus_shared::story::revision;
use promptus_shared::story::{Campaign, Issue, WorldState, validate};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::proposals::{
    self, COLUMNS, Decision, Focus, ItemStatus, MapItem, Proposal, Row, Status, Stored, edit_key,
    map_key, stored, view, virtual_of,
};
use super::{EDITS_MAX, edit_error, edit_library};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaign_maps::{self, Batch, CampaignMap};
use crate::campaigns::revisions::Author;
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::live::{self, Topic};

/// The longest title and word, in characters.
const TITLE_MAX: usize = 200;
const WORD_MAX: usize = 500;

/// What Claude says of its write, besides the write itself: the batch's
/// title, the batch to add it to, where on a map it is about.
#[derive(Debug, Clone, Default)]
pub struct Meta {
    pub title: Option<String>,
    pub proposal: Option<Uuid>,
    pub focus: Option<Focus>,
}

/// Why a write with the token was proposed rather than applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// The GM did not give Claude the hand: the default.
    Proposals,
    /// An evening is open.
    Evening,
    /// It touches what players already know.
    Revealed,
    /// Claude added it to a batch of its own.
    Batch,
}

/// What became of a write with the token.
#[derive(Debug)]
pub enum Outcome<T> {
    Applied(T),
    Proposed {
        proposal: Box<Proposal>,
        why: Why,
        /// The validator's report on the campaign as the batch would
        /// leave it (story edits only).
        issues: Vec<Issue>,
    },
}

/// Whether players already know the entity `kind`/`id` of `story`: a
/// scene the table entered, a clue found, an NPC or adversary named, a
/// faction or a goal known, a place where an entered scene stands — and
/// always the campaign's own fields, the bible (its hook) and the party.
#[must_use]
pub fn revealed(story: &Campaign, world: &WorldState, kind: Kind, id: &str) -> bool {
    let entered = |node: &str| {
        world.node_status.contains_key(node) || world.current_node.as_deref() == Some(node)
    };
    match kind {
        Kind::Node => entered(id),
        Kind::Clue => world.found_clues.contains(id),
        Kind::Npc | Kind::Adversary => world.revealed.contains(id),
        Kind::Faction => world.known_factions.contains(id),
        Kind::Goal => world.goals.contains_key(id),
        // The place of an entered scene, or a place containing it: the
        // pin players see is the first anchored place up the `parent`
        // chain (`story::pins`), named after that place.
        Kind::Location => story
            .nodes
            .iter()
            .filter(|n| entered(&n.id))
            .any(|n| within(story, n.location.as_deref(), id)),
        // Players always read the campaign's title and universe, the
        // bible's hook and the party (`projection::project_for_players`).
        Kind::Campaign | Kind::Bible | Kind::PartyMember => true,
        _ => false,
    }
}

/// Whether place `place` is `id` or lies inside it, following `parent`
/// (a loop in the chain stops the walk).
fn within(story: &Campaign, place: Option<&str>, id: &str) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    let mut at = place;
    while let Some(p) = at {
        if p == id {
            return true;
        }
        if !seen.insert(p) {
            return false;
        }
        at = story.location(p).and_then(|l| l.parent.as_deref());
    }
    false
}

/// Whether `after` touches anything players know of `row`.
fn touches_revealed(row: &CampaignRow, after: &Campaign) -> bool {
    revision::diff(&row.story, after)
        .touched
        .iter()
        .any(|t| revealed(&row.story, &row.world, t.kind, &t.id))
}

/// `None` when Claude's write may land directly on `row` (locked in
/// `tx`); otherwise why it is proposed.
async fn hand(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
    known: bool,
) -> Result<Option<Why>, AppError> {
    if !row.claude_hand {
        return Ok(Some(Why::Proposals));
    }
    if crate::evening::session::current(&mut **tx, row.id)
        .await?
        .is_some()
    {
        return Ok(Some(Why::Evening));
    }
    Ok(known.then_some(Why::Revealed))
}

/// Refuse a whole story replaced with the token (an import over the
/// campaign) unless Claude has the hand and it touches nothing players
/// know: a whole file cannot be proposed item by item.
///
/// # Errors
///
/// 409 `CLAUDE_PROPOSES`; a database error.
pub async fn may_replace(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
    after: &Campaign,
) -> Result<(), AppError> {
    match hand(tx, row, touches_revealed(row, after)).await? {
        None => Ok(()),
        Some(_) => Err(AppError::Conflict("CLAUDE_PROPOSES")),
    }
}

/// Claude's batch `id` of campaign `campaign`, locked in `tx`, still
/// waiting on an item.
async fn locked(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    id: Uuid,
) -> Result<Stored, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM story_proposals
         WHERE id = $1 AND campaign_id = $2 AND author = 'token' FOR UPDATE"
    ))
    .bind(id)
    .bind(campaign)
    .fetch_optional(&mut **tx)
    .await?;
    let s = stored(row.ok_or(AppError::NotFound("NO_SUCH_PROPOSAL"))?)?;
    if s.status != Status::Pending {
        return Err(AppError::Conflict("PROPOSAL_ALREADY_DECIDED"));
    }
    Ok(s)
}

fn title_of(meta: &Meta) -> Result<String, AppError> {
    crate::evening::clean_text(meta.title.as_deref().unwrap_or_default(), TITLE_MAX)
}

/// Store `edits` and `map` as a new batch of Claude's, or add them to
/// `existing`; tell the GM's screens.
async fn store(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    existing: Option<&Stored>,
    meta: &Meta,
    edits: &[Edit],
    map: Option<&MapItem>,
) -> Result<Stored, AppError> {
    let maps: Vec<&MapItem> = map.into_iter().collect();
    let row: Row = match existing {
        Some(s) => {
            sqlx::query_as(&format!(
                "UPDATE story_proposals SET edits = edits || $2, map_ops = map_ops || $3,
                   focus = COALESCE(focus, $4)
                 WHERE id = $1 RETURNING {COLUMNS}"
            ))
            .bind(s.id)
            .bind(Json(edits))
            .bind(Json(&maps))
            .bind(meta.focus.as_ref().map(Json))
            .fetch_one(&mut **tx)
            .await?
        }
        None => {
            sqlx::query_as(&format!(
                "INSERT INTO story_proposals (campaign_id, prompt, edits, map_ops, focus, author)
                 VALUES ($1, $2, $3, $4, $5, 'token') RETURNING {COLUMNS}"
            ))
            .bind(campaign)
            .bind(title_of(meta)?)
            .bind(Json(edits))
            .bind(Json(&maps))
            .bind(meta.focus.as_ref().map(Json))
            .fetch_one(&mut **tx)
            .await?
        }
    };
    live::touch(tx, campaign, &Topic::Proposals).await?;
    stored(row)
}

/// Claude's story edits on `campaign`: checked like the GM's (all or
/// nothing, the same errors), then applied when Claude has the hand,
/// proposed otherwise.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, `NO_SUCH_PROPOSAL`;
/// 400 as `prep::apply`, `TEXT_TOO_LONG` (title); 409
/// `PROPOSAL_ALREADY_DECIDED`; a database error.
pub async fn story_edits(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    edits: &[Edit],
    meta: &Meta,
) -> Result<Outcome<(CampaignRow, Vec<Change>)>, AppError> {
    if edits.len() > EDITS_MAX {
        return Err(AppError::BadRequest("TOO_MANY_EDITS"));
    }
    let (outcome, ()) = story_edits_with(pool, gm, campaign, meta, |_| {
        Ok(Made {
            edits: edits.to_vec(),
            focus: None,
            extra: (),
        })
    })
    .await?;
    Ok(outcome)
}

/// The edits Claude's write makes, computed on the story they apply to,
/// and where on a map they are about.
pub struct Made<T> {
    pub edits: Vec<Edit>,
    pub focus: Option<Focus>,
    pub extra: T,
}

/// [`story_edits`], the edits made by `make` on the story they apply to
/// — the campaign locked, with what the batch named by `meta` already
/// proposes — so that what it reads (an NPC's spots, the cells already
/// taken) cannot change before it lands (`platform/edit-maps-by-claude`).
///
/// # Errors
///
/// As [`story_edits`], and `make`'s.
pub async fn story_edits_with<T>(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    meta: &Meta,
    make: impl FnOnce(&Campaign) -> Result<Made<T>, AppError>,
) -> Result<(Outcome<(CampaignRow, Vec<Change>)>, T), AppError> {
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let library = edit_library(&row);
    let existing = match meta.proposal {
        Some(id) => Some(locked(&mut tx, campaign, id).await?),
        None => None,
    };
    // Checked on what the batch already proposes, so a clue added in it
    // can be placed by the next call.
    let maps =
        proposals::maps_for(&mut *tx, campaign, &existing.iter().collect::<Vec<_>>()).await?;
    let base = existing
        .as_ref()
        .map_or_else(|| row.story.clone(), |s| virtual_of(&row, s, &maps).story);
    let made = make(&base)?;
    if made.edits.len() > EDITS_MAX {
        return Err(AppError::BadRequest("TOO_MANY_EDITS"));
    }
    let edits = made.edits.as_slice();
    let mut focused;
    let meta = if made.focus.is_some() {
        focused = meta.clone();
        focused.focus = made.focus;
        &focused
    } else {
        meta
    };
    let (after, changes) = edit::apply_with(&base, edits, &library).map_err(|e| edit_error(&e))?;
    let why = match &existing {
        Some(_) => Why::Batch,
        None => match hand(&mut tx, &row, touches_revealed(&row, &after)).await? {
            Some(why) => why,
            None => {
                campaigns::save_story(&mut tx, campaign, &row.story, &after, Author::Token).await?;
                let row = campaigns::find(&mut *tx, campaign)
                    .await?
                    .ok_or(AppError::NotFound("NOT_FOUND"))?;
                tx.commit().await?;
                return Ok((Outcome::Applied((row, changes)), made.extra));
            }
        },
    };
    let s = store(&mut tx, campaign, existing.as_ref(), meta, edits, None).await?;
    let all = proposals::maps_for(&mut *tx, campaign, &[&s]).await?;
    tx.commit().await?;
    let issues = validate(&after);
    Ok((
        Outcome::Proposed {
            proposal: Box::new(view(&row, s, &all)),
            why,
            issues,
        },
        made.extra,
    ))
}

/// Whether map `id` is on the table's board now: players have seen it.
async fn shown(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    id: &str,
) -> Result<bool, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM map_states WHERE campaign_id = $1 AND map_id = $2)",
    )
    .bind(campaign)
    .bind(id)
    .fetch_one(&mut **tx)
    .await?)
}

/// Claude's batch of operations on map `id`: checked like the editor's
/// (`campaign_maps::prepare`: the same staleness, the same errors), then
/// applied when Claude has the hand and the map is not on the table,
/// proposed otherwise — focused on the map and the first cell it
/// touches.
///
/// # Errors
///
/// As `campaign_maps::edit`; 404 `NO_SUCH_PROPOSAL`; 400
/// `BATCH_CHANGES_NOTHING`; 409 `PROPOSAL_ALREADY_DECIDED`.
pub async fn map_ops(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: &str,
    batch: &Batch,
) -> Result<Outcome<CampaignMap>, AppError> {
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let existing = match batch.proposal {
        Some(p) => Some(locked(&mut tx, campaign, p).await?),
        None => None,
    };
    let why = match existing {
        Some(_) => Why::Batch,
        None => {
            let known = shown(&mut tx, campaign, id).await?;
            match hand(&mut tx, &row, known).await? {
                Some(why) => why,
                None => {
                    let m =
                        campaign_maps::edit_locked(&mut tx, &row, Author::Token, id, batch).await?;
                    tx.commit().await?;
                    return Ok(Outcome::Applied(m));
                }
            }
        }
    };
    let before = campaign_maps::read(&mut *tx, campaign, id)
        .await?
        .at(row.prep_revision);
    let after = campaign_maps::prepare(&mut tx, &row, &before, batch)
        .await?
        .ok_or(AppError::BadRequest("BATCH_CHANGES_NOTHING"))?;
    let touched = promptus_shared::maps::edit::diff(&before.map, &after);
    let at = touched.iter().find_map(|t| t.cells.first().copied());
    let meta = Meta {
        title: batch.title.clone(),
        proposal: batch.proposal,
        focus: Some(Focus {
            map: id.to_string(),
            at,
        }),
    };
    let item = MapItem {
        map: id.to_string(),
        revision: row.prep_revision,
        ops: batch.ops.clone(),
    };
    let s = store(
        &mut tx,
        campaign,
        existing.as_ref(),
        &meta,
        &[],
        Some(&item),
    )
    .await?;
    let all = proposals::maps_for(&mut *tx, campaign, &[&s]).await?;
    tx.commit().await?;
    Ok(Outcome::Proposed {
        proposal: Box::new(view(&row, s, &all)),
        why,
        issues: Vec::new(),
    })
}

/// Everything left to keep or discard at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum All {
    Keep,
    Discard,
}

/// The GM's word on a batch: items kept, items discarded (with `word`),
/// or everything left.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Decide {
    #[serde(default)]
    pub keep: Vec<String>,
    #[serde(default)]
    pub discard: Vec<String>,
    /// Why the GM discards: Claude reads it.
    #[serde(default)]
    pub word: String,
    #[serde(default)]
    pub all: Option<All>,
}

/// Keep story edits `keep` of `s` on `story`, in order. In `strict`
/// mode an edit that no longer applies refuses the whole decision;
/// otherwise it is refused alone.
fn keep_edits(
    row: &CampaignRow,
    s: &Stored,
    keep: &[String],
    strict: bool,
    decisions: &mut std::collections::BTreeMap<String, Decision>,
) -> Result<Campaign, AppError> {
    let library = edit_library(row);
    let mut story = row.story.clone();
    for (i, e) in s.edits.iter().enumerate() {
        let key = edit_key(i);
        if !keep.contains(&key) {
            continue;
        }
        let decision = match edit::apply_with(&story, std::slice::from_ref(e), &library) {
            Ok((next, _)) => {
                story = next;
                kept(None)
            }
            // It may hang on an earlier item of the batch, not kept yet.
            Err(_) if strict && virtual_of(row, s, &[]).changes[i].is_some() => {
                return Err(AppError::Conflict("PROPOSAL_ITEM_NEEDS_EARLIER"));
            }
            Err(_) if strict => return Err(AppError::Conflict("PROPOSAL_ITEM_STALE")),
            Err(err) => kept(Some(err.code)),
        };
        decisions.insert(key, decision);
    }
    Ok(story)
}

/// Kept — or refused, with why, when it no longer applied.
fn kept(refused: Option<&str>) -> Decision {
    Decision {
        status: if refused.is_some() {
            ItemStatus::Refused
        } else {
            ItemStatus::Kept
        },
        word: None,
        code: refused.map(str::to_string),
    }
}

/// The GM decides on Claude's batch `id`: kept items replay under the
/// campaign lock on the campaign as it is now — story edits as one line
/// of the journal, each map batch as one, all by `copilot` (a proposal
/// the GM accepted, as the workshop's); discarded ones keep the GM's
/// word for Claude. Keeping named items is strict (one that no longer
/// applies refuses the decision, nothing written); keeping everything
/// left refuses what no longer applies one by one and keeps the rest.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, `NO_SUCH_PROPOSAL`,
/// `NO_SUCH_ITEM`; 400 `NOTHING_DECIDED`, `TEXT_TOO_LONG`; 409
/// `PROPOSAL_ALREADY_DECIDED`, `ITEM_ALREADY_DECIDED`,
/// `PROPOSAL_ITEM_STALE`, `PROPOSAL_ITEM_NEEDS_EARLIER` (it hangs on an
/// earlier item of the batch the GM has not kept); a database error.
pub async fn decide(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
    d: &Decide,
) -> Result<(CampaignRow, Proposal), AppError> {
    let word = crate::evening::clean_text(&d.word, WORD_MAX)?;
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let s = locked(&mut tx, campaign, id).await?;
    let keys = s.keys();
    let left: Vec<String> = keys.iter().filter(|k| s.pending(k)).cloned().collect();
    let (keep, discard, strict) = match d.all {
        Some(All::Keep) => (left, vec![], false),
        Some(All::Discard) => (vec![], left, false),
        None => (d.keep.clone(), d.discard.clone(), true),
    };
    if keep.is_empty() && discard.is_empty() {
        return Err(AppError::BadRequest("NOTHING_DECIDED"));
    }
    for k in keep.iter().chain(&discard) {
        if !keys.contains(k) {
            return Err(AppError::NotFound("NO_SUCH_ITEM"));
        }
        if !s.pending(k) || (keep.contains(k) && discard.contains(k)) {
            return Err(AppError::Conflict("ITEM_ALREADY_DECIDED"));
        }
    }
    let mut decisions = s.decisions.clone();
    let story = keep_edits(&row, &s, &keep, strict, &mut decisions)?;
    // Nothing kept in the story moves no topic a player hears.
    if !revision::diff(&row.story, &story).is_empty() {
        campaigns::save_story(&mut tx, campaign, &row.story, &story, Author::Copilot).await?;
    }
    for (i, item) in s.map_ops.iter().enumerate() {
        let key = map_key(i);
        if !keep.contains(&key) {
            continue;
        }
        // Read again: the story kept just above moved the revision.
        let now = campaigns::find(&mut *tx, campaign)
            .await?
            .ok_or(AppError::NotFound("NOT_FOUND"))?;
        let batch = Batch {
            revision: item.revision,
            ops: item.ops.clone(),
            title: None,
            proposal: None,
        };
        let decision =
            match campaign_maps::edit_locked(&mut tx, &now, Author::Copilot, &item.map, &batch)
                .await
            {
                Ok(_) => kept(None),
                Err(AppError::Internal(e)) => return Err(AppError::Internal(e)),
                Err(_) if strict => return Err(AppError::Conflict("PROPOSAL_ITEM_STALE")),
                Err(e) => kept(Some(e.code())),
            };
        decisions.insert(key, decision);
    }
    for k in &discard {
        decisions.insert(
            k.clone(),
            Decision {
                status: ItemStatus::Discarded,
                word: (!word.is_empty()).then(|| word.clone()),
                code: None,
            },
        );
    }
    let settled = Stored { decisions, ..s };
    let status = match settled.settled() {
        Status::Pending => "pending",
        Status::Accepted => "accepted",
        Status::Rejected => "rejected",
    };
    let feedback = if d.all == Some(All::Discard) {
        word
    } else {
        settled.feedback.clone()
    };
    let decided: Row = sqlx::query_as(&format!(
        "UPDATE story_proposals SET decisions = $2, status = $3, feedback = $4,
           decided_at = CASE WHEN $3 = 'pending' THEN NULL ELSE now() END
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(Json(&settled.decisions))
    .bind(status)
    .bind(feedback)
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Proposals).await?;
    let row = campaigns::find(&mut *tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    let s = stored(decided)?;
    let maps = proposals::maps_for(&mut *tx, campaign, &[&s]).await?;
    tx.commit().await?;
    let p = view(&row, s, &maps);
    Ok((row, p))
}

/// Claude's batches on `campaign`, `pending` ones only or the latest 30
/// whatever their state, newest first.
async fn batches(
    db: impl sqlx::PgExecutor<'_>,
    campaign: Uuid,
    pending: bool,
) -> Result<Vec<Stored>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM story_proposals
         WHERE campaign_id = $1 AND author = 'token' AND ($2 = false OR status = 'pending')
         ORDER BY created_at DESC, id LIMIT 30"
    ))
    .bind(campaign)
    .bind(pending)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(stored).collect()
}

/// Claude's latest batches and the GM's word on each item — what
/// Claude reads back through the MCP server.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; a database error.
pub async fn list(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
) -> Result<Vec<Proposal>, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let all = batches(pool, campaign, false).await?;
    let maps = proposals::maps_for(pool, campaign, &all.iter().collect::<Vec<_>>()).await?;
    Ok(all.into_iter().map(|s| view(&row, s, &maps)).collect())
}

/// Claude's batches still waiting on `row`, each with the campaign it
/// would leave: what the atlas draws dotted. `maps` are the campaign's
/// own maps.
///
/// # Errors
///
/// A database error.
pub(crate) async fn pending(
    pool: &PgPool,
    row: &CampaignRow,
    maps: &[Map],
) -> Result<Vec<(Proposal, Campaign, Vec<Map>)>, AppError> {
    let mut all = batches(pool, row.id, true).await?;
    all.reverse();
    Ok(all
        .into_iter()
        .map(|s| {
            let v = virtual_of(row, &s, maps);
            let after = v.maps_after(maps);
            (view(row, s, maps), v.story, after)
        })
        .collect())
}

/// Give Claude the hand on `campaign`, or take it back. Refused while
/// an evening is open; taking it back always works.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 409
/// `EVENING_OPEN`; a database error.
pub async fn set_hand(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    on: bool,
) -> Result<CampaignRow, AppError> {
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    if on
        && crate::evening::session::current(&mut *tx, campaign)
            .await?
            .is_some()
    {
        return Err(AppError::Conflict("EVENING_OPEN"));
    }
    sqlx::query("UPDATE campaigns SET claude_hand = $2 WHERE id = $1")
        .bind(campaign)
        .bind(on)
        .execute(&mut *tx)
        .await?;
    live::touch(&mut tx, campaign, &Topic::Proposals).await?;
    let row = campaigns::find(&mut *tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    tx.commit().await?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use promptus_shared::story::from_yaml;
    use promptus_shared::story::world::NodeStatus;

    const FIXTURE: &str = include_str!("../../../content/fixtures/phare-de-kerbrume.yaml");

    #[test]
    fn what_players_know_is_revealed_and_nothing_else() {
        let story = from_yaml(FIXTURE).unwrap();
        let node = &story.nodes[0];
        let mut world = WorldState::default();
        assert!(!revealed(&story, &world, Kind::Node, &node.id));
        world
            .node_status
            .insert(node.id.clone(), NodeStatus::Visited);
        assert!(revealed(&story, &world, Kind::Node, &node.id));
        if let Some(place) = &node.location {
            assert!(revealed(&story, &world, Kind::Location, place));
        }
        // The place containing it is known too: the pin players see may
        // be that place's (the fixture's tavern stands in the port).
        let place = node.location.as_deref().and_then(|l| story.location(l));
        let parent = place
            .and_then(|l| l.parent.clone())
            .expect("a nested place");
        assert!(revealed(&story, &world, Kind::Location, &parent));
        let clue = &story.clues[0].id;
        assert!(!revealed(&story, &world, Kind::Clue, clue));
        world.found_clues.insert(clue.clone());
        assert!(revealed(&story, &world, Kind::Clue, clue));
        let npc = &story.npcs[0].id;
        assert!(!revealed(&story, &world, Kind::Npc, npc));
        world.revealed.insert(npc.clone());
        assert!(revealed(&story, &world, Kind::Npc, npc));
        assert!(!revealed(&story, &world, Kind::Item, &story.items[0].id));
        // What players always read: the campaign, its hook, the party.
        assert!(revealed(&story, &world, Kind::Bible, "bible"));
        assert!(revealed(&story, &world, Kind::Campaign, "campaign"));
    }
}
