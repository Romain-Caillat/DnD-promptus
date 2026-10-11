//! The campaign's maps in words (`platform/edit-maps-by-claude`).
//!
//! Claude reads a map as text and works on it by name, never by pixels:
//! - [`sketch`]: a map — the campaign's own or its world's — as rows of
//!   characters with numbered axes, the grid's glyphs and a symbol over
//!   every thing placed, with the preparation's pins, NPCs, clues and
//!   fights (`story::sketch`);
//! - [`ask`]: the engine's answers — the cheapest walk and its cost, a
//!   line of sight and its cover, what players see from a cell — on
//!   references by name (`story::placing::ask`);
//! - [`place`]: pins, NPCs, clues and fights placed « near » something or
//!   « along » a path, the server choosing a free, dry, reachable cell
//!   (`story::placing::place`); the edits it makes are the GM's own
//!   story edits, so with the GM's token they go through Claude's batch
//!   of proposals (`prep::claude::story_edits_with`), resolved under the
//!   campaign lock on the story the batch would leave.
//!
//! GM routes; a GM's personal token reaches all three
//! (`auth::api_tokens::TOKEN_ROUTES`). Nothing here reaches players.

use promptus_shared::maps::{Cell, Map};
use promptus_shared::story::cast;
use promptus_shared::story::edit::Change;
use promptus_shared::story::pins::pins_on;
use promptus_shared::story::placing::{
    self, Answer, AskError, PlaceError, Placed, Placement, Question,
};
use promptus_shared::story::sketch::{self as text, Sketch, Zone};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::one;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::revisions::Author;
use crate::campaigns::{self, CampaignRow};
use crate::content;
use crate::error::AppError;
use crate::prep::claude::{self, Made, Meta, Outcome};
use crate::prep::proposals::Focus;

/// Most placements in one request.
pub const PLACEMENTS_MAX: usize = 50;

/// Map `id` of `row`: its own, else its world's.
async fn map_of(pool: &PgPool, row: &CampaignRow, id: &str) -> Result<Map, AppError> {
    match one(pool, row.id, id).await {
        Ok(m) => Ok(m.map),
        Err(AppError::NotFound(_)) => content::map(&row.story, id)
            .cloned()
            .ok_or(AppError::NotFound("NO_SUCH_MAP")),
        Err(e) => Err(e),
    }
}

/// The part of a map asked for (`?x=&y=&w=&h=`), all optional.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneQuery {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub w: Option<u32>,
    pub h: Option<u32>,
}

impl ZoneQuery {
    fn zone(&self) -> Option<Zone> {
        if self.x.is_none() && self.y.is_none() && self.w.is_none() && self.h.is_none() {
            return None;
        }
        Some(Zone {
            x: self.x.unwrap_or(0),
            y: self.y.unwrap_or(0),
            w: self.w.unwrap_or(text::MAX_W),
            h: self.h.unwrap_or(text::MAX_H),
        })
    }
}

/// Map `id` of `campaign` as text, with the preparation's marks on it.
///
/// # Errors
///
/// 404, `NO_SUCH_MAP`; a database error.
pub async fn sketch(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: &str,
    zone: &ZoneQuery,
) -> Result<Sketch, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let map = map_of(pool, &row, id).await?;
    let pins = pins_on(&row.story, id);
    let cast = cast::on_map(&row.story, id, &[]);
    Ok(text::sketch(&map, &pins, &cast, zone.zone()))
}

fn ask_error(e: &AskError) -> AppError {
    AppError::Invalid {
        code: e.code(),
        detail: e.detail(),
    }
}

/// The engine's answer to `question` on map `id` of `campaign`, under
/// the campaign's movement rules.
///
/// # Errors
///
/// 404, `NO_SUCH_MAP`; 400 `MAP_REF_UNKNOWN`, `MAP_REF_AMBIGUOUS`,
/// `MAP_REF_OFF_GRID`, `PLACE_NO_ROOM`; a database error.
pub async fn ask(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: &str,
    question: &Question,
) -> Result<Answer, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let map = map_of(pool, &row, id).await?;
    let rules = crate::board::movement(row.rules(), &map);
    placing::ask(&row.story, &map, rules, question).map_err(|e| ask_error(&e))
}

/// Placements asked on one map, and Claude's batch they join.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub placements: Vec<Placement>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub proposal: Option<Uuid>,
}

/// What a placement request did: the edits applied or proposed, and
/// where each placement landed.
pub struct Placing {
    pub outcome: Outcome<(CampaignRow, Vec<Change>)>,
    pub placed: Vec<Placed>,
}

fn place_error(e: &PlaceError) -> AppError {
    AppError::Invalid {
        code: e.code(),
        detail: e.detail(),
    }
}

/// The cells Claude's other waiting batches would take on `map`: a new
/// placement does not land on a mark still proposed.
async fn proposed_cells(
    pool: &PgPool,
    row: &CampaignRow,
    map: &Map,
    but: Option<Uuid>,
) -> Result<Vec<Cell>, AppError> {
    let own: Vec<Map> = super::all(pool, row.id)
        .await?
        .into_iter()
        .map(|m| m.map)
        .collect();
    Ok(claude::pending(pool, row, &own)
        .await?
        .into_iter()
        .filter(|(p, _, _)| Some(p.id) != but)
        .flat_map(|(_, after, _)| placing::marks(&after, map))
        .map(|(_, at)| at)
        .collect())
}

/// `ask`'s placements on map `id` of `campaign`, resolved by the server
/// and made as story edits: the GM's session applies them, the GM's
/// token proposes them (or applies them when Claude has the hand), in
/// the batch `ask.proposal` or a new one titled `ask.title`.
///
/// # Errors
///
/// 404, `NO_SUCH_MAP`, `NO_SUCH_PROPOSAL`; 400 `PLACE_MALFORMED`,
/// `TOO_MANY_PLACEMENTS`, `PLACE_UNKNOWN_ENTITY`, `PLACE_NO_FIGHT`,
/// `PLACE_ANCHORED_ELSEWHERE`, `PLACE_NO_ROOM`, `PLACE_NO_PATH`, the
/// reference codes, and those of a story edit; 409
/// `PROPOSAL_ALREADY_DECIDED`; a database error.
pub async fn place(
    pool: &PgPool,
    gm: &CurrentGm,
    author: Author,
    campaign: Uuid,
    id: &str,
    ask: &Place,
) -> Result<Placing, AppError> {
    if ask.placements.is_empty() {
        return Err(AppError::BadRequest("PLACE_MALFORMED"));
    }
    if ask.placements.len() > PLACEMENTS_MAX {
        return Err(AppError::BadRequest("TOO_MANY_PLACEMENTS"));
    }
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let map = map_of(pool, &row, id).await?;
    let rules = crate::board::movement(row.rules(), &map);
    if author != Author::Token {
        let resolved = placing::place(&row.story, &map, rules, &ask.placements, &[])
            .map_err(|e| place_error(&e))?;
        let done = crate::prep::apply(pool, gm, author, campaign, &resolved.edits).await?;
        return Ok(Placing {
            outcome: Outcome::Applied(done),
            placed: resolved.placed,
        });
    }
    let more = proposed_cells(pool, &row, &map, ask.proposal).await?;
    let meta = Meta {
        title: ask.title.clone(),
        proposal: ask.proposal,
        focus: None,
    };
    let (outcome, placed) = claude::story_edits_with(pool, gm, campaign, &meta, |story| {
        let resolved = placing::place(story, &map, rules, &ask.placements, &more)
            .map_err(|e| place_error(&e))?;
        Ok(Made {
            edits: resolved.edits,
            focus: Some(Focus {
                map: map.id.clone(),
                at: resolved.placed.first().map(|p| p.at),
            }),
            extra: resolved.placed,
        })
    })
    .await?;
    Ok(Placing { outcome, placed })
}
