//! The maps a campaign owns (migration `019_campaign_maps.sql`), next to
//! the worlds' maps compiled into the server (`content::maps`).
//!
//! - maps/edit-map-gm: the GM creates a map (blank, or a copy of one of
//!   the world's), paints and places in the editor, checked by
//!   `Map::validate`;
//! - maps/edit-maps-by-ops ([`edit`]): the editor — and Claude, through
//!   the GM's personal token — sends batches of operations
//!   (`maps::edit::MapOp`) with the campaign revision it read, never the
//!   whole map. A batch on a stale revision lands when no later batch on
//!   the same map touched the same cells or things, else it is refused
//!   (409 `MAP_STALE`, with the map as it is now) and never overwrites.
//!   Every write of a map's content goes through [`write`], which
//!   journals it in the campaign's one preparation journal
//!   (`campaigns::revisions::record_map`);
//! - maps/import-image-map: a Universal VTT file (`maps::from_uvtt`) or a
//!   plain image aligned on a grid becomes a map whose image is only the
//!   backdrop; the GM traces or touches up its walls in the editor;
//! - maps/generate-map-llm ([`generate`]): the model proposes a map for a
//!   scene.
//!
//! A map reaches the table only once the GM validated it; any change
//! withdraws the validation (the GM has the last word on what is
//! shown, `MEMORY.md` §3).

pub mod generate;

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Utc};
use promptus_shared::maps::edit::{self as ops, MapOp, OpError};
use promptus_shared::maps::{
    Ambience, BASE_LAYER, Backdrop, CellKind, FORMAT_VERSION, Grid, Issue, Layer, Map, MapError,
    Scale, Visibility, Water, from_uvtt,
};
use promptus_shared::story::cast::{self, Cast};
use promptus_shared::story::pins::{Pin, PinScene, is_pinned, pins_on};
use promptus_shared::story::thread::{Thread, path_issues, thread_on};
use promptus_shared::story::{Anchor, Campaign, anchor_issues};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::revisions::{self, Author};
use crate::campaigns::{self, CampaignRow};
use crate::content;
use crate::error::AppError;
use crate::live::{self, Topic};

/// Largest imported image, in bytes.
pub const MAX_BACKDROP_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Editor,
    Generated,
    Imported,
}

impl Source {
    fn as_str(self) -> &'static str {
        match self {
            Self::Editor => "editor",
            Self::Generated => "generated",
            Self::Imported => "imported",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "generated" => Self::Generated,
            "imported" => Self::Imported,
            _ => Self::Editor,
        }
    }
}

/// One map of the campaign, as the GM's screens read it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignMap {
    pub map: Map,
    pub source: Source,
    pub node: Option<String>,
    /// It has an imported image behind its grid.
    pub backdrop: bool,
    pub validated_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    /// The campaign's preparation revision this map was read at: the
    /// revision a batch of operations is sent on.
    pub revision: i64,
}

type Row = (
    Json<Map>,
    String,
    Option<String>,
    bool,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
);

const COLUMNS: &str = "map, source, node, backdrop IS NOT NULL, validated_at, updated_at";

fn campaign_map((Json(map), source, node, backdrop, validated_at, updated_at): Row) -> CampaignMap {
    CampaignMap {
        map,
        source: Source::parse(&source),
        node,
        backdrop,
        validated_at,
        updated_at,
        revision: 0,
    }
}

impl CampaignMap {
    pub(crate) fn at(mut self, revision: i64) -> Self {
        self.revision = revision;
        self
    }
}

/// Every map of `campaign`, newest change first.
///
/// # Errors
///
/// A database error.
pub async fn all(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<CampaignMap>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM campaign_maps WHERE campaign_id = $1 ORDER BY updated_at DESC"
    ))
    .bind(campaign)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(campaign_map).collect())
}

async fn one(db: impl PgExecutor<'_>, campaign: Uuid, id: &str) -> Result<CampaignMap, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM campaign_maps WHERE campaign_id = $1 AND id = $2"
    ))
    .bind(campaign)
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(campaign_map)
        .ok_or(AppError::NotFound("NO_SUCH_MAP"))
}

/// The map `id` the table may be shown: a validated map of the campaign,
/// else one of its world's.
///
/// # Errors
///
/// A database error.
pub async fn playable(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    story: &Campaign,
    id: &str,
) -> Result<Option<Map>, AppError> {
    let own: Option<Json<Map>> = sqlx::query_scalar(
        "SELECT map FROM campaign_maps
         WHERE campaign_id = $1 AND id = $2 AND validated_at IS NOT NULL",
    )
    .bind(campaign)
    .bind(id)
    .fetch_optional(db)
    .await?;
    Ok(own
        .map(|Json(m)| m)
        .or_else(|| content::map(story, id).cloned()))
}

/// The validated maps of `campaign`, for the table's map choice.
///
/// # Errors
///
/// A database error.
pub async fn validated(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<Map>, AppError> {
    let rows: Vec<Json<Map>> = sqlx::query_scalar(
        "SELECT map FROM campaign_maps
         WHERE campaign_id = $1 AND validated_at IS NOT NULL ORDER BY created_at",
    )
    .bind(campaign)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|Json(m)| m).collect())
}

/// The GM's words for what is wrong with a map (French).
fn french(issue: &Issue) -> String {
    match issue {
        Issue::UnsupportedVersion(v) => format!("version de format {v} inconnue"),
        Issue::BadCellSize => "la taille d’une case doit être positive".into(),
        Issue::DuplicateId(id) => format!("l’identifiant « {id} » sert deux fois"),
        Issue::DuplicateLayer(id) => format!("le calque « {id} » est déclaré deux fois"),
        Issue::UnknownLayer { item, layer } => {
            format!("« {item} » est sur le calque « {layer} », qui n’existe pas")
        }
        Issue::OutOfBounds { item, cell } => {
            format!("« {item} » sort de la grille ({}, {})", cell.x, cell.y)
        }
        Issue::DoorNotOnWall { door, cell } => format!(
            "la porte « {door} » ({}, {}) n’est pas sur un mur",
            cell.x, cell.y
        ),
        Issue::StartNotWalkable { start, cell } => format!(
            "le départ « {start} » ({}, {}) n’est pas sur une case praticable",
            cell.x, cell.y
        ),
        Issue::LightRadius(id) => {
            format!(
                "la lumière « {id} » : la pénombre doit porter au moins aussi loin que la lumière vive"
            )
        }
        Issue::EmptyExit(id) => format!("la sortie « {id} » n’a aucune case"),
        Issue::EmptyProp(id) => format!("le décor « {id} » n’a pas de taille"),
        Issue::DoorOnHex(id) => format!("la porte « {id} » est sur une carte en hexagones"),
        Issue::BadBackdrop => "l’alignement de l’image de fond est invalide".into(),
    }
}

/// A map error as the GM reads it.
#[must_use]
pub fn invalid(e: &MapError) -> AppError {
    let detail = match e {
        MapError::Invalid(issues) => issues.iter().map(french).collect::<Vec<_>>().join(" ; "),
        MapError::Uvtt(why) => why.clone(),
        other => other.to_string(),
    };
    AppError::Invalid {
        code: "MAP_INVALID",
        detail,
    }
}

/// The tileset a new map of `story` is drawn with, and its first floor.
fn look(story: &Campaign) -> (String, String) {
    let tileset = content::theme(story).and_then(|t| t.tilesets.first());
    (
        tileset.map_or_else(|| "defaut".to_string(), |t| t.id.clone()),
        tileset
            .and_then(|t| t.materials.keys().next().cloned())
            .unwrap_or_else(|| "sol".to_string()),
    )
}

/// The layers a new map starts with: the map, and the GM's secrets.
fn layers() -> Vec<Layer> {
    vec![
        Layer {
            id: BASE_LAYER.into(),
            name: "Carte".into(),
            visibility: Visibility::All,
        },
        Layer {
            id: "secrets".into(),
            name: "Secrets".into(),
            visibility: Visibility::Gm,
        },
    ]
}

/// A free id for a map named `name` in `campaign`.
async fn fresh_id(
    db: &PgPool,
    campaign: Uuid,
    story: &Campaign,
    name: &str,
) -> Result<String, AppError> {
    let base = promptus_shared::story::v1::slug(name);
    let base = if base.is_empty() {
        "carte".to_string()
    } else {
        base
    };
    let taken: Vec<String> =
        sqlx::query_scalar("SELECT id FROM campaign_maps WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_all(db)
            .await?;
    let used = |id: &str| taken.iter().any(|t| t == id) || content::map(story, id).is_some();
    let mut id = base.clone();
    let mut n = 2;
    while used(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    Ok(id)
}

fn clean_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("NAME_REQUIRED"));
    }
    if name.chars().count() > 80 {
        return Err(AppError::BadRequest("TEXT_TOO_LONG"));
    }
    Ok(name.to_string())
}

/// Store a new map of `campaign`, unvalidated, and journal it.
#[allow(clippy::too_many_arguments)]
async fn insert(
    pool: &PgPool,
    campaign: Uuid,
    map: &Map,
    source: Source,
    node: Option<&str>,
    backdrop: Option<(Vec<u8>, String)>,
    author: Author,
) -> Result<CampaignMap, AppError> {
    map.validate().map_err(|e| invalid(&e))?;
    let (bytes, mime) = backdrop.unzip();
    let mut tx = pool.begin().await?;
    let locked = campaigns::lock(&mut tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    let row: Row = sqlx::query_as(&format!(
        "INSERT INTO campaign_maps (campaign_id, id, map, source, node, backdrop, backdrop_mime)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(&map.id)
    .bind(Json(map))
    .bind(source.as_str())
    .bind(node)
    .bind(bytes)
    .bind(mime)
    .fetch_one(&mut *tx)
    .await?;
    let revision = revisions::record_map(&mut tx, campaign, None, Some(map), author, None)
        .await?
        .unwrap_or(locked.prep_revision);
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(campaign_map(row).at(revision))
}

/// What the GM's maps screen lists.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub maps: Vec<CampaignMap>,
    /// The world's maps, to copy from.
    pub world: Vec<WorldMap>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldMap {
    pub id: String,
    pub name: String,
}

/// # Errors
///
/// 404; a database error.
pub async fn listing(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Listing, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    Ok(Listing {
        maps: all(pool, campaign)
            .await?
            .into_iter()
            .map(|m| m.at(row.prep_revision))
            .collect(),
        world: content::maps(&row.story)
            .map(|m| WorldMap {
                id: m.id.clone(),
                name: m.name.clone(),
            })
            .collect(),
    })
}

/// # Errors
///
/// 404, `NO_SUCH_MAP`.
pub async fn get(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: &str,
) -> Result<CampaignMap, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    Ok(one(pool, campaign, id).await?.at(row.prep_revision))
}

/// The GM's atlas (maps/browse-atlas): every map of the world and of the
/// campaign, whole, nested by `maps::atlas::tree` — roots first, then
/// depth first — with the scenes pinned on each (`story::pins`,
/// campaign/pin-scenes-on-maps). GM only: a player receives the one map
/// on the table, through the board's projection, never this.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasView {
    pub revision: i64,
    pub roots: Vec<String>,
    pub maps: Vec<AtlasMap>,
    /// Every place of the story, for the pin tool.
    pub places: Vec<AtlasPlace>,
    /// Scenes no place anchors on a map (a dream, a memory: allowed).
    pub unplaced: Vec<PinScene>,
    /// Pins and placements standing nowhere: a map the atlas lacks, a
    /// cell off its grid, an NPC, clue or fight inside a wall.
    pub issues: Vec<promptus_shared::story::Issue>,
    /// What the GM can place (campaign/place-cast-and-clues): every NPC,
    /// clue and fight of the story, and the scenes an NPC's spot may be
    /// for.
    pub castable: Castable,
    /// The acts, in order, for the thread's act filter.
    pub acts: Vec<AtlasAct>,
    /// Claude's batches still waiting on the GM
    /// (`platform/propose-edits-by-claude`), oldest first; each map says
    /// what they would add on it (`AtlasMap::proposed`).
    pub claude: Vec<crate::prep::proposals::Proposal>,
    /// « Claude a la main »: Claude's writes land directly.
    pub claude_hand: bool,
}

/// What one of Claude's batches would add on a map, applied virtually:
/// drawn dotted, never stored (platform/propose-edits-by-claude).
#[derive(Debug, Clone, Serialize)]
pub struct Proposed {
    pub proposal: Uuid,
    /// Pins, NPCs, clues and fights, and thread arrows, that are not on
    /// the map as it is.
    pub pins: Vec<Pin>,
    pub cast: Cast,
    pub thread: Vec<promptus_shared::story::thread::Segment>,
    /// The cells its map operations touch.
    pub cells: Vec<promptus_shared::maps::Cell>,
}

impl Proposed {
    fn is_empty(&self) -> bool {
        self.pins.is_empty()
            && self.cast.npcs.is_empty()
            && self.cast.clues.is_empty()
            && self.cast.fights.is_empty()
            && self.thread.is_empty()
            && self.cells.is_empty()
    }
}

/// An act, as the thread's filter names it.
#[derive(Debug, Clone, Serialize)]
pub struct AtlasAct {
    pub id: String,
    pub title: String,
}

/// The story's NPCs, clues and fights, and where each is placed.
#[derive(Debug, Clone, Serialize)]
pub struct Castable {
    pub npcs: Vec<CastNpc>,
    pub clues: Vec<CastClue>,
    pub fights: Vec<CastFight>,
    /// Every scene, in campaign order.
    pub scenes: Vec<PinScene>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CastNpc {
    pub id: String,
    pub name: String,
    pub spots: Vec<promptus_shared::story::Spot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CastClue {
    pub id: String,
    pub text: String,
    pub scene: String,
    pub anchor: Option<Anchor>,
}

/// A scene's fight (ship battles open from their own panel, not here).
#[derive(Debug, Clone, Serialize)]
pub struct CastFight {
    pub scene: String,
    pub title: String,
    pub number: usize,
    pub map: Option<String>,
    pub anchor: Option<Anchor>,
}

/// A place of the story and where it is pinned.
#[derive(Debug, Clone, Serialize)]
pub struct AtlasPlace {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub anchor: Option<Anchor>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasMap {
    pub map: Map,
    pub origin: AtlasOrigin,
    /// The map and exit it hangs on, if any.
    pub parent: Option<AtlasAnchor>,
    pub children: Vec<String>,
    /// Exits to maps that are not children (same scale, or back up).
    pub links: Vec<AtlasLink>,
    pub backdrop: bool,
    pub validated: bool,
    /// The scenes standing on this map, by place.
    pub pins: Vec<Pin>,
    /// The NPCs, clues and fights placed on it (campaign/place-cast-and-
    /// clues), each fight with how many opponents its battle plan places.
    pub cast: Cast,
    /// The scenes' exits drawn between its pins (campaign/draw-story-
    /// thread), and the scenes standing on it.
    pub thread: Thread,
    /// What Claude's waiting batches would add on it.
    pub proposed: Vec<Proposed>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AtlasOrigin {
    World,
    Campaign,
}

#[derive(Debug, Clone, Serialize)]
pub struct AtlasAnchor {
    pub map: String,
    pub exit: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AtlasLink {
    pub to: String,
    pub exit: String,
}

/// # Errors
///
/// 404; a database error.
pub async fn atlas(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<AtlasView, AppError> {
    use promptus_shared::maps::atlas::{self as tree, Origin, Sheet};
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let own = all(pool, campaign).await?;
    let sheets: Vec<Sheet<'_>> = content::maps(&row.story)
        .map(|m| Sheet {
            map: m,
            origin: Origin::World,
        })
        .chain(own.iter().map(|m| Sheet {
            map: &m.map,
            origin: Origin::Campaign,
        }))
        .collect();
    let built = tree::tree(&sheets);
    let maps = built
        .nodes
        .into_iter()
        .filter_map(|n| {
            let mine = own.iter().find(|m| m.map.id == n.id);
            let map = match (n.origin, mine) {
                (Origin::Campaign, Some(m)) => m.map.clone(),
                _ => content::map(&row.story, &n.id)?.clone(),
            };
            let campaign = n.origin == Origin::Campaign;
            Some(AtlasMap {
                origin: if campaign {
                    AtlasOrigin::Campaign
                } else {
                    AtlasOrigin::World
                },
                parent: n.parent.map(|p| AtlasAnchor {
                    map: p.map,
                    exit: p.exit,
                }),
                children: n.children,
                links: n
                    .links
                    .into_iter()
                    .map(|l| AtlasLink {
                        to: l.to,
                        exit: l.exit,
                    })
                    .collect(),
                backdrop: campaign && mine.is_some_and(|m| m.backdrop),
                // A world map is always playable (`playable`).
                validated: !campaign || mine.is_some_and(|m| m.validated_at.is_some()),
                pins: pins_on(&row.story, &n.id),
                thread: thread_on(&row.story, &n.id),
                cast: Cast::default(),
                proposed: Vec::new(),
                map,
            })
        })
        .collect::<Vec<_>>();
    let shown: Vec<Map> = maps.iter().map(|m| m.map.clone()).collect();
    let story = &row.story;
    let mut maps = maps;
    for m in &mut maps {
        m.cast = cast::on_map(story, &m.map.id, &shown);
    }
    let own_maps: Vec<Map> = own.iter().map(|m| m.map.clone()).collect();
    let pending = crate::prep::claude::pending(pool, &row, &own_maps).await?;
    for (p, after, after_maps) in &pending {
        let after_shown: Vec<Map> = shown
            .iter()
            .map(|m| {
                after_maps
                    .iter()
                    .find(|x| x.id == m.id)
                    .unwrap_or(m)
                    .clone()
            })
            .collect();
        for m in &mut maps {
            let proposed = proposed_on(m, p, after, &after_shown);
            if !proposed.is_empty() {
                m.proposed.push(proposed);
            }
        }
    }
    Ok(AtlasView {
        claude: pending.into_iter().map(|(p, _, _)| p).collect(),
        claude_hand: row.claude_hand,
        revision: row.prep_revision,
        roots: built.roots,
        places: story
            .locations
            .iter()
            .map(|l| AtlasPlace {
                id: l.id.clone(),
                name: l.name.clone(),
                parent: l.parent.clone(),
                anchor: l.anchor.clone(),
            })
            .collect(),
        unplaced: story
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| !is_pinned(story, n))
            .map(|(i, n)| PinScene::of(i, n))
            .collect(),
        issues: anchor_issues(story, &shown)
            .into_iter()
            .chain(path_issues(story, &shown))
            .collect(),
        acts: story
            .acts
            .iter()
            .map(|a| AtlasAct {
                id: a.id.clone(),
                title: a.title.clone(),
            })
            .collect(),
        castable: Castable {
            npcs: story
                .npcs
                .iter()
                .map(|n| CastNpc {
                    id: n.id.clone(),
                    name: n.name.clone(),
                    spots: n.spots.clone(),
                })
                .collect(),
            clues: story
                .clues
                .iter()
                .map(|c| CastClue {
                    id: c.id.clone(),
                    text: c.text.clone(),
                    scene: c.node.clone(),
                    anchor: c.anchor.clone(),
                })
                .collect(),
            fights: story
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(i, n)| {
                    let e = n.encounter.as_ref().filter(|e| e.vehicles.is_none())?;
                    Some(CastFight {
                        scene: n.id.clone(),
                        title: n.title.clone(),
                        number: i + 1,
                        map: n.map.clone(),
                        anchor: e.anchor.clone(),
                    })
                })
                .collect(),
            scenes: story
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| PinScene::of(i, n))
                .collect(),
        },
        maps,
    })
}

/// What batch `p` adds on map `m`: the marks of the campaign it would
/// leave (`after`, its maps `after_maps`) that `m` does not hold yet.
fn proposed_on(
    m: &AtlasMap,
    p: &crate::prep::proposals::Proposal,
    after: &Campaign,
    after_maps: &[Map],
) -> Proposed {
    let id = &m.map.id;
    let cast = cast::on_map(after, id, after_maps);
    Proposed {
        proposal: p.id,
        pins: pins_on(after, id)
            .into_iter()
            .filter(|x| !m.pins.contains(x))
            .collect(),
        cast: Cast {
            npcs: cast
                .npcs
                .into_iter()
                .filter(|x| !m.cast.npcs.contains(x))
                .collect(),
            clues: cast
                .clues
                .into_iter()
                .filter(|x| !m.cast.clues.contains(x))
                .collect(),
            fights: cast
                .fights
                .into_iter()
                .filter(|x| !m.cast.fights.contains(x))
                .collect(),
        },
        thread: thread_on(after, id)
            .segments
            .into_iter()
            .filter(|x| !m.thread.segments.contains(x))
            .collect(),
        cells: p
            .items
            .iter()
            .filter_map(|i| i.map.as_ref())
            .filter(|i| &i.map == id)
            .flat_map(|i| i.touches.iter().flat_map(|t| t.cells.iter().copied()))
            .collect(),
    }
}

/// A blank map, or a copy of one of the world's.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewMap {
    pub name: String,
    #[serde(default)]
    pub copy: Option<String>,
    #[serde(default = "default_side")]
    pub width: u32,
    #[serde(default = "default_side")]
    pub height: u32,
    #[serde(default)]
    pub node: Option<String>,
}

const fn default_side() -> u32 {
    16
}

/// # Errors
///
/// 404; 400 `NAME_REQUIRED`, `TEXT_TOO_LONG`, `UNKNOWN_MAP`, `BAD_SIZE`,
/// `UNKNOWN_NODE`.
pub async fn create(
    pool: &PgPool,
    gm: &CurrentGm,
    author: Author,
    campaign: Uuid,
    new: &NewMap,
) -> Result<CampaignMap, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let name = clean_name(&new.name)?;
    check_node(&row, new.node.as_deref())?;
    let id = fresh_id(pool, campaign, &row.story, &name).await?;
    let map = match &new.copy {
        Some(copy) => {
            let mut m = content::map(&row.story, copy)
                .ok_or(AppError::BadRequest("UNKNOWN_MAP"))?
                .clone();
            m.id = id;
            m.name = name;
            m
        }
        None => blank(&row.story, id, name, new.width, new.height)?,
    };
    insert(
        pool,
        campaign,
        &map,
        Source::Editor,
        new.node.as_deref(),
        None,
        author,
    )
    .await
}

fn check_node(row: &CampaignRow, node: Option<&str>) -> Result<(), AppError> {
    match node {
        Some(n) if row.story.node(n).is_none() => Err(AppError::BadRequest("UNKNOWN_NODE")),
        _ => Ok(()),
    }
}

/// A floor of `width` × `height` cells inside a wall.
fn blank(
    story: &Campaign,
    id: String,
    name: String,
    width: u32,
    height: u32,
) -> Result<Map, AppError> {
    if !(3..=64).contains(&width) || !(3..=64).contains(&height) {
        return Err(AppError::BadRequest("BAD_SIZE"));
    }
    let (theme, floor) = look(story);
    let ground = CellKind {
        terrain: floor,
        wall: false,
        water: Water::None,
        difficult: false,
        void: false,
        elevation: 0,
    };
    let legend = BTreeMap::from([
        ('.', ground.clone()),
        (
            '#',
            CellKind {
                wall: true,
                ..ground
            },
        ),
    ]);
    let rows: Vec<String> = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| {
                    if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                        '#'
                    } else {
                        '.'
                    }
                })
                .collect()
        })
        .collect();
    Ok(Map {
        version: FORMAT_VERSION,
        id,
        name,
        scale: Scale::Encounter,
        cell_meters: None,
        theme,
        ambience: Ambience::default(),
        backdrop: None,
        gm_notes: None,
        layers: layers(),
        grid: Grid::new(&legend, &rows).map_err(|e| invalid(&e.into()))?,
        doors: vec![],
        props: vec![],
        objects: vec![],
        lights: vec![],
        exits: vec![],
        labels: vec![],
        starts: vec![],
    })
}

/// A batch of operations on one map, and the campaign revision the
/// sender read the map at.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Batch {
    pub revision: i64,
    pub ops: Vec<MapOp>,
    /// Claude's batch (`platform/propose-edits-by-claude`): its title,
    /// the batch of proposals to add it to, where it is about. Ignored
    /// from the GM's session.
    #[serde(default, skip_serializing)]
    pub title: Option<String>,
    #[serde(default, skip_serializing)]
    pub proposal: Option<Uuid>,
}

/// The most operations one batch carries: one gesture of the editor is
/// one batch.
pub const MAX_OPS: usize = 200;

/// The GM's words for an operation that does not apply.
fn refused(e: &OpError) -> AppError {
    match e {
        OpError::NameRequired => AppError::BadRequest("NAME_REQUIRED"),
        OpError::TextTooLong => AppError::BadRequest("TEXT_TOO_LONG"),
        OpError::OffGrid(c) => AppError::Invalid {
            code: "MAP_OP_INVALID",
            detail: format!("la case ({}, {}) sort de la grille", c.x, c.y),
        },
        OpError::NoSuchThing(id) => AppError::Invalid {
            code: "MAP_OP_INVALID",
            detail: format!("rien sur la carte ne s’appelle « {id} »"),
        },
        OpError::LegendFull => AppError::Invalid {
            code: "MAP_OP_INVALID",
            detail: "la carte a trop de sortes de cases".into(),
        },
    }
}

/// Apply `batch` to map `id`, under the campaign lock, all or nothing.
/// On the revision the map was read at, the batch applies as it is; on
/// a stale one it applies on the map as it is now as long as no later
/// batch on this map touched the same cells or things (`ops::overlaps`)
/// and the result holds; otherwise it is refused with the map as it is
/// now, for the sender to reload and replay. The validation is withdrawn.
///
/// # Errors
///
/// 404 `NO_SUCH_MAP`; 400 `BAD_BATCH` (empty or too long),
/// `MAP_OP_INVALID`, `MAP_INVALID`, `NAME_REQUIRED`, `TEXT_TOO_LONG`;
/// 409 `MAP_STALE` with `fresh`, the map as it is now.
pub async fn edit(
    pool: &PgPool,
    gm: &CurrentGm,
    author: Author,
    campaign: Uuid,
    id: &str,
    batch: &Batch,
) -> Result<CampaignMap, AppError> {
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let edited = edit_locked(&mut tx, &row, author, id, batch).await?;
    tx.commit().await?;
    Ok(edited)
}

/// [`edit`] inside `tx`, the campaign `row` locked: what Claude's batch
/// kept by the GM replays (`prep::claude`) under the same rules.
///
/// # Errors
///
/// As [`edit`].
pub async fn edit_locked(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    row: &CampaignRow,
    author: Author,
    id: &str,
    batch: &Batch,
) -> Result<CampaignMap, AppError> {
    let before = one(&mut **tx, row.id, id).await?.at(row.prep_revision);
    let Some(after) = prepare(tx, row, &before, batch).await? else {
        return Ok(before);
    };
    write(tx, row.id, &before.map, &after, author, None).await
}

/// Check `batch` on map `before` (read in `tx`, the campaign `row`
/// locked) as [`edit`] does, and the map it makes; `None` when it
/// changes nothing. Nothing is written: Claude's batches are checked
/// this way before they are proposed.
///
/// # Errors
///
/// As [`edit`].
pub async fn prepare(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    row: &CampaignRow,
    before: &CampaignMap,
    batch: &Batch,
) -> Result<Option<Map>, AppError> {
    if batch.ops.is_empty() || batch.ops.len() > MAX_OPS {
        return Err(AppError::BadRequest("BAD_BATCH"));
    }
    let id = &before.map.id;
    let stale = || AppError::Stale {
        code: "MAP_STALE",
        fresh: serde_json::to_value(before).unwrap_or_default(),
    };
    if batch.revision > row.prep_revision {
        return Err(stale());
    }
    let later = if batch.revision < row.prep_revision {
        revisions::map_lines_since(tx, row.id, id, batch.revision).await?
    } else {
        vec![]
    };
    let after = match ops::apply(&before.map, &batch.ops) {
        Ok(after) => after,
        Err(_) if !later.is_empty() => return Err(stale()),
        Err(e) => return Err(refused(&e)),
    };
    let touched = ops::diff(&before.map, &after);
    if later.iter().any(|l| ops::overlaps(&touched, l)) {
        return Err(stale());
    }
    if let Err(e) = after.validate() {
        return Err(if later.is_empty() {
            invalid(&e)
        } else {
            stale()
        });
    }
    Ok((!touched.is_empty()).then_some(after))
}

/// A map of `campaign` as stored, read in `tx` or on the pool.
///
/// # Errors
///
/// 404 `NO_SUCH_MAP`; a database error.
pub async fn read(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    id: &str,
) -> Result<CampaignMap, AppError> {
    one(db, campaign, id).await
}

/// The one write of a map's content, in `tx` with the campaign locked:
/// store `after` over `before`, withdraw the validation, journal the
/// change and tell the GM's screens.
///
/// # Errors
///
/// 404 `NO_SUCH_MAP`; a database error.
pub async fn write(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    campaign: Uuid,
    before: &Map,
    after: &Map,
    author: Author,
    undoes: Option<i64>,
) -> Result<CampaignMap, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "UPDATE campaign_maps SET map = $3, validated_at = NULL
         WHERE campaign_id = $1 AND id = $2 RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(&before.id)
    .bind(Json(after))
    .fetch_optional(&mut **tx)
    .await?;
    let row = row.ok_or(AppError::NotFound("NO_SUCH_MAP"))?;
    let revision =
        revisions::record_map(tx, campaign, Some(before), Some(after), author, undoes).await?;
    live::touch(tx, campaign, &Topic::Desk).await?;
    let revision = match revision {
        Some(r) => r,
        None => {
            sqlx::query_scalar("SELECT prep_revision FROM campaigns WHERE id = $1")
                .bind(campaign)
                .fetch_one(&mut **tx)
                .await?
        }
    };
    Ok(campaign_map(row).at(revision))
}

/// Take back a batch on map `id`: put back what it `touched` as it was
/// in `before` (the map before the batch), keeping every later change
/// made elsewhere on the map (`maps::edit::revert`), as a new revision
/// by the GM that `undoes` the batch's.
///
/// # Errors
///
/// 409 `REVISION_NOT_REVERTIBLE` when the map is gone or what it puts
/// back no longer holds; a database error.
pub async fn restore(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    campaign: Uuid,
    id: &str,
    before: &Map,
    touched: &[ops::MapTouch],
    undoes: i64,
) -> Result<(), AppError> {
    let not_revertible = AppError::Conflict("REVISION_NOT_REVERTIBLE");
    let now = match one(&mut **tx, campaign, id).await {
        Ok(m) => m.map,
        Err(AppError::NotFound(_)) => return Err(not_revertible),
        Err(e) => return Err(e),
    };
    let Some(back) = ops::revert(&now, before, touched) else {
        return Err(not_revertible);
    };
    if back.id != now.id || back.validate().is_err() {
        return Err(not_revertible);
    }
    write(tx, campaign, &now, &back, Author::Gm, Some(undoes)).await?;
    Ok(())
}

/// The GM validates map `id`: from now on it may be shown at the table.
///
/// # Errors
///
/// 404 `NO_SUCH_MAP`.
pub async fn validate(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: &str,
) -> Result<CampaignMap, AppError> {
    let mut tx = pool.begin().await?;
    let locked = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let row: Option<Row> = sqlx::query_as(&format!(
        "UPDATE campaign_maps SET validated_at = now()
         WHERE campaign_id = $1 AND id = $2 RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let row = row.ok_or(AppError::NotFound("NO_SUCH_MAP"))?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(campaign_map(row).at(locked.prep_revision))
}

/// # Errors
///
/// 404 `NO_SUCH_MAP`; 409 `MAP_ON_THE_TABLE` while it is shown.
pub async fn delete(
    pool: &PgPool,
    gm: &CurrentGm,
    author: Author,
    campaign: Uuid,
    id: &str,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let shown: Option<String> =
        sqlx::query_scalar("SELECT map_id FROM map_states WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_optional(&mut *tx)
            .await?;
    if shown.as_deref() == Some(id) {
        return Err(AppError::Conflict("MAP_ON_THE_TABLE"));
    }
    let gone: Option<Json<Map>> = sqlx::query_scalar(
        "DELETE FROM campaign_maps WHERE campaign_id = $1 AND id = $2 RETURNING map",
    )
    .bind(campaign)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let Json(gone) = gone.ok_or(AppError::NotFound("NO_SUCH_MAP"))?;
    revisions::record_map(&mut tx, campaign, Some(&gone), None, author, None).await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(())
}

/// The type of an image, from its first bytes; `None` for anything else.
fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// Base64, bare or as a `data:` URL.
fn decode_image(raw: &str) -> Result<(Vec<u8>, String), AppError> {
    let data = raw.split_once(";base64,").map_or(raw, |(_, d)| d);
    let bytes = STANDARD
        .decode(data.trim())
        .map_err(|_| AppError::BadRequest("BAD_IMAGE"))?;
    if bytes.len() > MAX_BACKDROP_BYTES {
        return Err(AppError::BadRequest("IMAGE_TOO_LARGE"));
    }
    let mime = image_mime(&bytes).ok_or(AppError::BadRequest("BAD_IMAGE"))?;
    Ok((bytes, mime.to_string()))
}

/// What the GM imports: a Universal VTT file, or an image and the grid
/// they aligned on it.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Import {
    #[serde(rename_all = "camelCase")]
    Uvtt { name: String, file: String },
    #[serde(rename_all = "camelCase")]
    Image {
        name: String,
        /// Base64 or a `data:` URL.
        image: String,
        /// Pixels of the image per cell.
        cell_px: f64,
        /// Pixels left and above the first whole cell.
        offset_x: f64,
        offset_y: f64,
        columns: u32,
        rows: u32,
    },
}

/// # Errors
///
/// 404; 400 `NAME_REQUIRED`, `TEXT_TOO_LONG`, `BAD_IMAGE`,
/// `IMAGE_TOO_LARGE`, `BAD_SIZE`, `MAP_INVALID`.
pub async fn import(
    pool: &PgPool,
    gm: &CurrentGm,
    author: Author,
    campaign: Uuid,
    import: &Import,
) -> Result<CampaignMap, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let (theme, floor) = look(&row.story);
    let (map, image) = match import {
        Import::Uvtt { name, file } => {
            let name = clean_name(name)?;
            let id = fresh_id(pool, campaign, &row.story, &name).await?;
            let read = from_uvtt(file, &id, &name, &theme, &floor).map_err(|e| invalid(&e))?;
            let image = if read.image_base64.is_empty() {
                None
            } else {
                Some(decode_image(&read.image_base64)?)
            };
            let mut map = read.map;
            if image.is_none() {
                map.backdrop = None;
            }
            (map, image)
        }
        Import::Image {
            name,
            image,
            cell_px,
            offset_x,
            offset_y,
            columns,
            rows,
        } => {
            let name = clean_name(name)?;
            let image = decode_image(image)?;
            if !(cell_px.is_finite() && *cell_px >= 8.0) {
                return Err(AppError::BadRequest("BAD_SIZE"));
            }
            let id = fresh_id(pool, campaign, &row.story, &name).await?;
            let mut map = blank(&row.story, id, name, *columns, *rows)?;
            // No wall yet: the GM traces them on the image.
            map.grid = open_floor(&map, *columns, *rows)?;
            map.theme = theme;
            map.backdrop = Some(Backdrop {
                prompt: None,
                image: Some("import".into()),
                cell_px: Some(*cell_px),
                offset: Some([*offset_x, *offset_y]),
            });
            (map, Some(image))
        }
    };
    insert(pool, campaign, &map, Source::Imported, None, image, author).await
}

/// `map`'s floor without its surrounding wall.
fn open_floor(map: &Map, width: u32, height: u32) -> Result<Grid, AppError> {
    let ground = map
        .grid
        .legend()
        .find(|(g, _)| *g == '.')
        .map(|(_, k)| k.clone())
        .ok_or(AppError::Internal("blank map has no floor".into()))?;
    let legend = BTreeMap::from([('.', ground)]);
    let rows: Vec<String> = (0..height).map(|_| ".".repeat(width as usize)).collect();
    Grid::new(&legend, &rows).map_err(|e| invalid(&e.into()))
}

/// The imported image behind map `id` of `campaign`.
///
/// # Errors
///
/// 404 `NO_SUCH_MAP`.
pub async fn backdrop(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    id: &str,
) -> Result<(Vec<u8>, String), AppError> {
    let row: Option<(Option<Vec<u8>>, Option<String>)> = sqlx::query_as(
        "SELECT backdrop, backdrop_mime FROM campaign_maps WHERE campaign_id = $1 AND id = $2",
    )
    .bind(campaign)
    .bind(id)
    .fetch_optional(db)
    .await?;
    match row {
        Some((Some(bytes), mime)) => Ok((bytes, mime.unwrap_or_else(|| "image/png".into()))),
        _ => Err(AppError::NotFound("NO_SUCH_MAP")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_images_pass_as_a_backdrop() {
        let png = STANDARD.encode(b"\x89PNG\r\n\x1a\nrest");
        assert_eq!(decode_image(&png).unwrap().1, "image/png");
        let url = format!("data:image/png;base64,{png}");
        assert_eq!(decode_image(&url).unwrap().1, "image/png");
        let text = STANDARD.encode(b"<svg onload=alert(1)>");
        assert!(matches!(
            decode_image(&text),
            Err(AppError::BadRequest("BAD_IMAGE"))
        ));
    }
}
