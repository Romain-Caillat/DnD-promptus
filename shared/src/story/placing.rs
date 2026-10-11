//! Placing by name, and asking the engine (`platform/edit-maps-by-claude`).
//!
//! Claude works on a map in words: « Gwen près du comptoir », « deux
//! indices sur le chemin de la colline ». This module turns such a
//! request into the story edits the GM would make by hand — a place's
//! `anchor`, an NPC's `spots`, a clue's or a fight's `anchor` — choosing
//! the cell itself, so that Claude never sends raw coordinates it cannot
//! see:
//!
//! - a **reference** names something on the map ([`lookup`]): a cell
//!   (`"12,4"`), or by id or by name a prop, object, door, exit, label or
//!   start of the map, or a pin, place or NPC of the preparation — exact
//!   id first, then the name, then a part of it, accents and articles
//!   aside; two different things at the same level are ambiguous;
//! - the cell is the **nearest one a creature can stand on, walking out
//!   from the reference** ([`Ground::spot`]): never through a wall, never
//!   on a wall, void, blocking prop or water (shallow or deep), never
//!   where something already stands (a mark of the preparation, a start,
//!   an object, a prop, an exit), and reachable from where the table
//!   comes in (the party's starts and the exits, when the map has some);
//! - **along** a path, several placements are spread evenly on the
//!   shortest walk between two references, each then on its nearest free
//!   cell.
//!
//! The engine's questions ([`ask`]) read the same references: the path
//! and its cost, a line of sight and its cover, and what the players see
//! from a cell (the players' projection under the fog).
//!
//! Pure: the server feeds it the stored story and map, then sends the
//! edits through Claude's batch of proposals like any other.

use std::collections::{BTreeSet, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::cast::{self, Cast};
use super::edit::{Edit, Kind};
use super::model::{Anchor, Campaign, Id};
use super::pins::{Pin, pins_on};
use super::sketch::{Sketch, Zone, sketch};
use crate::maps::{
    Cell, Cover, Diagonal, Geometry, LightLevel, Map, MovementRules, Occupancy, Side, Viewer,
    Water, distance, illumination, line_of_sight, neighbours, shortest_path, standable,
    visible_cells,
};

/// What a reference names, on one map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Target {
    pub name: String,
    pub cells: Vec<Cell>,
}

/// Why a reference names nothing usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    /// Nothing on the map answers to it; `known` lists what does.
    Unknown {
        reference: String,
        known: Vec<String>,
    },
    /// Several different things answer to it.
    Ambiguous {
        reference: String,
        candidates: Vec<String>,
    },
    /// A cell outside the grid.
    OffGrid { reference: String },
}

impl LookupError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unknown { .. } => "MAP_REF_UNKNOWN",
            Self::Ambiguous { .. } => "MAP_REF_AMBIGUOUS",
            Self::OffGrid { .. } => "MAP_REF_OFF_GRID",
        }
    }

    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::Unknown { reference, known } => {
                format!(
                    "nothing named {reference:?} on this map; known: {}",
                    known.join(" · ")
                )
            }
            Self::Ambiguous {
                reference,
                candidates,
            } => format!("{reference:?} could be: {}", candidates.join(" · ")),
            Self::OffGrid { reference } => format!("{reference:?} is outside the grid"),
        }
    }
}

/// Something a reference may name.
struct Candidate {
    /// Exact ids it answers to.
    ids: Vec<String>,
    /// Names it answers to, normalised.
    names: Vec<String>,
    /// How it is listed back: `id (name)`.
    shown: String,
    cells: Vec<Cell>,
}

/// Lowercase, accents and punctuation aside, without a leading article.
fn norm(s: &str) -> String {
    let plain: String = s
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect();
    let words: Vec<&str> = plain.split_whitespace().collect();
    let skip = usize::from(
        matches!(
            words.first(),
            Some(&("le" | "la" | "les" | "l" | "un" | "une" | "du" | "des" | "de" | "d" | "the"))
        ) && words.len() > 1,
    );
    words[skip..].join(" ")
}

fn shown(id: &str, name: &str) -> String {
    match (id.is_empty(), name.is_empty()) {
        (false, false) => format!("{id} ({name})"),
        (true, _) => name.to_string(),
        (false, true) => id.to_string(),
    }
}

fn candidates(c: &Campaign, map: &Map) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut add = |ids: Vec<&str>, names: Vec<&str>, cells: Vec<Cell>| {
        let id = ids.first().copied().unwrap_or_default();
        let name = names
            .iter()
            .find(|n| !n.is_empty())
            .copied()
            .unwrap_or_default();
        out.push(Candidate {
            shown: shown(id, name),
            ids: ids
                .into_iter()
                .filter(|i| !i.is_empty())
                .map(String::from)
                .collect(),
            names: names
                .into_iter()
                .map(norm)
                .filter(|n| !n.is_empty())
                .collect(),
            cells,
        });
    };
    for p in pins_on(c, &map.id) {
        let mut ids = vec![p.location.as_str()];
        let mut names = vec![p.name.as_str()];
        for s in &p.scenes {
            ids.push(&s.id);
            names.push(&s.title);
        }
        add(ids, names, vec![p.at]);
    }
    for n in &c.npcs {
        let cells: Vec<Cell> = n
            .spots
            .iter()
            .filter(|s| s.map == map.id)
            .map(|s| s.at)
            .collect();
        if !cells.is_empty() {
            add(vec![&n.id], vec![&n.name, &n.title], cells);
        }
    }
    for cl in &c.clues {
        if let Some(a) = cl.anchor.as_ref().filter(|a| a.map == map.id) {
            add(vec![&cl.id], vec![], vec![a.at]);
        }
    }
    for p in &map.props {
        add(
            vec![&p.id],
            vec![p.label.as_deref().unwrap_or_default(), &p.kind],
            p.cells().collect(),
        );
    }
    for o in &map.objects {
        add(
            vec![&o.id],
            vec![o.label.as_deref().unwrap_or_default(), &o.kind],
            vec![o.at],
        );
    }
    for d in &map.doors {
        add(
            vec![&d.id],
            vec![d.label.as_deref().unwrap_or_default()],
            vec![d.at],
        );
    }
    for e in &map.exits {
        add(
            vec![&e.id],
            vec![e.label.as_deref().unwrap_or_default(), &e.to],
            e.cells.clone(),
        );
    }
    for l in &map.labels {
        add(vec![], vec![&l.text], vec![l.at]);
    }
    for s in &map.starts {
        let entity = s.entity.as_deref().unwrap_or_default();
        add(vec![&s.id, entity], vec![], vec![s.at]);
    }
    out
}

fn parse_cell(reference: &str) -> Option<Cell> {
    let inner = reference
        .trim()
        .trim_start_matches(['[', '('])
        .trim_end_matches([']', ')']);
    let mut parts = inner
        .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
        .filter(|p| !p.is_empty());
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some(Cell::new(x, y))
}

/// The thing `reference` names on `map`, `c` holding the preparation's
/// marks.
///
/// # Errors
///
/// [`LookupError`]: nothing, several things, a cell off the grid.
pub fn lookup(c: &Campaign, map: &Map, reference: &str) -> Result<Target, LookupError> {
    if let Some(cell) = parse_cell(reference) {
        if !map.grid.contains(cell) {
            return Err(LookupError::OffGrid {
                reference: reference.to_string(),
            });
        }
        return Ok(Target {
            name: format!("[{}, {}]", cell.x, cell.y),
            cells: vec![cell],
        });
    }
    let all = candidates(c, map);
    let wanted = norm(reference);
    let words: Vec<&str> = wanted.split_whitespace().collect();
    let tiers: [&dyn Fn(&Candidate) -> bool; 3] = [
        &|k| k.ids.iter().any(|i| i == reference.trim()),
        &|k| !wanted.is_empty() && k.names.contains(&wanted),
        &|k| {
            !words.is_empty()
                && k.names.iter().any(|n| {
                    let theirs: Vec<&str> = n.split_whitespace().collect();
                    words.iter().all(|w| theirs.contains(w)) || n.contains(&wanted)
                })
        },
    ];
    for matches in tiers {
        let mut found: Vec<&Candidate> = Vec::new();
        for k in all.iter().filter(|k| matches(k)) {
            // The same cells under two names (a place and its label) are
            // one thing.
            if !found.iter().any(|f| f.cells == k.cells) {
                found.push(k);
            }
        }
        match found.as_slice() {
            [] => {}
            [one] => {
                return Ok(Target {
                    name: one.shown.clone(),
                    cells: one.cells.clone(),
                });
            }
            many => {
                return Err(LookupError::Ambiguous {
                    reference: reference.to_string(),
                    candidates: many.iter().map(|k| k.shown.clone()).collect(),
                });
            }
        }
    }
    Err(LookupError::Unknown {
        reference: reference.to_string(),
        known: all.iter().map(|k| k.shown.clone()).take(40).collect(),
    })
}

/// A map as a creature walks it, with what already stands there.
pub struct Ground<'a> {
    map: &'a Map,
    rules: MovementRules,
    taken: HashSet<Cell>,
    /// Where the table can walk from its way in; `None` when the map
    /// has no party start nor exit.
    region: Option<HashSet<Cell>>,
}

impl<'a> Ground<'a> {
    /// `map` under `rules`, its things and `c`'s marks on it taken, and
    /// `more` (marks still proposed).
    #[must_use]
    pub fn new(
        map: &'a Map,
        rules: MovementRules,
        c: &Campaign,
        more: impl IntoIterator<Item = Cell>,
    ) -> Self {
        let mut taken: HashSet<Cell> = more.into_iter().collect();
        taken.extend(marks(c, map).into_iter().map(|(_, at)| at));
        taken.extend(map.starts.iter().map(|s| s.at));
        taken.extend(map.objects.iter().map(|o| o.at));
        taken.extend(map.props.iter().flat_map(crate::maps::Prop::cells));
        taken.extend(map.exits.iter().flat_map(|e| e.cells.iter().copied()));
        taken.extend(map.doors.iter().map(|d| d.at));
        let mut ground = Self {
            map,
            rules,
            taken,
            region: None,
        };
        let entries: Vec<Cell> = map
            .starts
            .iter()
            .filter(|s| s.side == Some(Side::Party))
            .map(|s| s.at)
            .chain(map.exits.iter().flat_map(|e| e.cells.iter().copied()))
            .collect();
        if !entries.is_empty() {
            ground.region = Some(ground.flood(&entries));
        }
        ground
    }

    fn passable(&self, c: Cell) -> bool {
        standable(self.map, &self.rules, c).is_ok()
    }

    /// Whether a creature may be put on `c`: standable and out of water.
    fn dry(&self, c: Cell) -> bool {
        self.passable(c)
            && self
                .map
                .grid
                .kind(c)
                .is_some_and(|k| k.water == Water::None)
    }

    fn reached(&self, c: Cell) -> bool {
        self.region.as_ref().is_none_or(|r| r.contains(&c))
    }

    /// The steps out of `from`: neighbours a creature can walk to,
    /// without cutting a wall's corner.
    fn steps(&self, from: Cell) -> impl Iterator<Item = Cell> + '_ {
        let square = self.map.geometry() == Geometry::Square;
        neighbours(self.map.geometry(), from)
            .into_iter()
            .filter(move |&to| {
                self.passable(to)
                    && (!square
                        || from.x == to.x
                        || from.y == to.y
                        || (self.passable(Cell::new(to.x, from.y))
                            && self.passable(Cell::new(from.x, to.y))))
            })
    }

    fn flood(&self, from: &[Cell]) -> HashSet<Cell> {
        let mut seen: HashSet<Cell> = from.iter().copied().collect();
        let mut queue: VecDeque<Cell> = from.iter().copied().collect();
        while let Some(c) = queue.pop_front() {
            for n in self.steps(c) {
                if seen.insert(n) {
                    queue.push_back(n);
                }
            }
        }
        seen
    }

    /// The cell nearest `target` a creature may be put on, walking out
    /// from it, and how many steps away: dry, reachable, and free unless
    /// `anywhere`. Ties go to the cell closest as the crow flies, then
    /// the topmost, then the leftmost.
    #[must_use]
    pub fn spot(&self, target: &[Cell], anywhere: bool) -> Option<(Cell, u32)> {
        let geometry = self.map.geometry();
        let crow = |c: Cell| {
            target
                .iter()
                .map(|t| distance(geometry, *t, c, Diagonal::Chebyshev))
                .min()
                .unwrap_or(0)
        };
        let mut seen: HashSet<Cell> = target.iter().copied().collect();
        let mut layer: Vec<Cell> = target
            .iter()
            .copied()
            .filter(|c| self.map.grid.contains(*c))
            .collect();
        let mut steps = 0;
        while !layer.is_empty() {
            let best = layer
                .iter()
                .copied()
                .filter(|&c| {
                    self.dry(c) && self.reached(c) && (anywhere || !self.taken.contains(&c))
                })
                .min_by_key(|&c| (crow(c), c.y, c.x));
            if let Some(c) = best {
                return Some((c, steps));
            }
            let mut next = Vec::new();
            for c in layer {
                for n in self.steps(c) {
                    if seen.insert(n) {
                        next.push(n);
                    }
                }
            }
            layer = next;
            steps += 1;
        }
        None
    }

    /// The cheapest walk from the cell nearest `from` to the cell nearest
    /// `to` (both ends included), and its cost.
    #[must_use]
    pub fn walk(&self, from: &[Cell], to: &[Cell]) -> Option<(Vec<Cell>, u32)> {
        let (start, _) = self.spot(from, true)?;
        let (end, _) = self.spot(to, true)?;
        let (path, cost) = shortest_path(
            self.map,
            &self.rules,
            &Occupancy::default(),
            start,
            end,
            u32::MAX / 2,
        )?;
        Some((std::iter::once(start).chain(path).collect(), cost))
    }

    fn take(&mut self, c: Cell) {
        self.taken.insert(c);
    }

    fn free(&mut self, c: Cell) {
        self.taken.remove(&c);
    }
}

/// Every mark of the preparation on `map`, by key (`pin:<place>`,
/// `npc:<id>`, `clue:<id>`, `fight:<scene>`).
#[must_use]
pub fn marks(c: &Campaign, map: &Map) -> Vec<(String, Cell)> {
    let pins: Vec<Pin> = pins_on(c, &map.id);
    let cast: Cast = cast::on_map(c, &map.id, &[]);
    pins.into_iter()
        .map(|p| (format!("pin:{}", p.location), p.at))
        .chain(
            cast.npcs
                .into_iter()
                .map(|n| (format!("npc:{}", n.npc), n.at)),
        )
        .chain(
            cast.clues
                .into_iter()
                .map(|n| (format!("clue:{}", n.clue), n.at)),
        )
        .chain(
            cast.fights
                .into_iter()
                .map(|n| (format!("fight:{}", n.scene), n.at)),
        )
        .collect()
}

/// What a placement puts on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum What {
    /// A scene's pin: its place's anchor.
    Scene,
    /// A place's pin.
    Place,
    /// One more spot of an NPC.
    Npc,
    Clue,
    /// Where a scene's fight breaks out.
    Fight,
}

/// Two references a path runs between.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Along {
    pub from: String,
    pub to: String,
}

/// One placement asked: what, and near what or along which path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub what: What,
    /// The scene, place, NPC or clue.
    pub id: Id,
    /// For an NPC: the scene the spot is for (none: every scene).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub near: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub along: Option<Along>,
}

/// Where one placement landed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Placed {
    pub what: What,
    pub id: Id,
    pub name: String,
    pub at: Cell,
    /// What it was placed near (a path's point names the path's ends).
    pub near: String,
    /// Steps walked from it.
    pub steps: u32,
    /// The ground there.
    pub terrain: String,
    /// The place a scene's pin needed, created for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<Id>,
    /// Other scenes sharing the place, whose pin moves with it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub with: Vec<Id>,
}

/// The edits that place everything asked, and where each landed.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub edits: Vec<Edit>,
    pub placed: Vec<Placed>,
}

/// Why a placement cannot be made; `index` is its place in the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceError {
    /// Neither `near` nor `along`, or both; or `scene` on anything but an NPC.
    Malformed {
        index: usize,
    },
    /// The scene, place, NPC or clue does not exist.
    NoSuch {
        index: usize,
        id: Id,
    },
    /// The scene has no fight, or a ship battle.
    NoFight {
        index: usize,
    },
    /// The place is pinned on another map: moving it would take its pin
    /// off there.
    AnchoredElsewhere {
        index: usize,
        map: String,
    },
    Lookup {
        index: usize,
        error: LookupError,
    },
    /// No free, dry, reachable cell near it.
    NoRoom {
        index: usize,
    },
    /// No walk between the path's two ends.
    NoPath {
        index: usize,
    },
}

impl PlaceError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => "PLACE_MALFORMED",
            Self::NoSuch { .. } => "PLACE_UNKNOWN_ENTITY",
            Self::NoFight { .. } => "PLACE_NO_FIGHT",
            Self::AnchoredElsewhere { .. } => "PLACE_ANCHORED_ELSEWHERE",
            Self::Lookup { error, .. } => error.code(),
            Self::NoRoom { .. } => "PLACE_NO_ROOM",
            Self::NoPath { .. } => "PLACE_NO_PATH",
        }
    }

    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::Malformed { index } => {
                format!(
                    "placement {index}: give `near` or `along` (not both); `scene` is for an npc"
                )
            }
            Self::NoSuch { index, id } => format!("placement {index}: no {id:?} in the campaign"),
            Self::NoFight { index } => {
                format!("placement {index}: the scene has no fight to place (or a ship battle)")
            }
            Self::AnchoredElsewhere { index, map } => format!(
                "placement {index}: the place is pinned on map {map:?}; pin one of its scenes instead (a place inside it is created)"
            ),
            Self::Lookup { index, error } => format!("placement {index}: {}", error.detail()),
            Self::NoRoom { index } => {
                format!("placement {index}: no free, dry, reachable cell near it")
            }
            Self::NoPath { index } => format!("placement {index}: no walk between the two ends"),
        }
    }
}

/// An id not used by any place, scene, NPC or clue of `c`, nor in `more`.
fn fresh_id(c: &Campaign, base: &str, more: &[String]) -> String {
    let used = |id: &str| {
        c.location(id).is_some()
            || c.node(id).is_some()
            || c.npc(id).is_some()
            || c.clue(id).is_some()
            || more.iter().any(|m| m == id)
    };
    if !used(base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|id| !used(id))
        .unwrap_or_default()
}

fn anchor(map: &Map, at: Cell) -> Value {
    json!({ "map": map.id, "at": [at.x, at.y] })
}

/// Everything in `asks` placed on `map` of campaign `c`, walking under
/// `rules`; `more` are cells still taken by proposed marks.
///
/// # Errors
///
/// The first [`PlaceError`], nothing placed.
#[allow(clippy::too_many_lines)]
pub fn place(
    c: &Campaign,
    map: &Map,
    rules: MovementRules,
    asks: &[Placement],
    more: &[Cell],
) -> Result<Resolved, PlaceError> {
    let mut ground = Ground::new(map, rules, c, more.iter().copied());
    let mut story = c.clone();
    let mut edits: Vec<Edit> = Vec::new();
    let mut placed = Vec::new();
    let mut created: Vec<String> = Vec::new();
    // Each path cut into as many even points as placements along it.
    let mut paths: Vec<(&Along, Vec<Cell>, usize, usize)> = Vec::new();
    for (index, ask) in asks.iter().enumerate() {
        let well_formed = ask.near.is_some() != ask.along.is_some()
            && (ask.scene.is_none() || ask.what == What::Npc);
        if !well_formed {
            return Err(PlaceError::Malformed { index });
        }
        if let Some(along) = &ask.along {
            if let Some(p) = paths.iter_mut().find(|p| p.0 == along) {
                p.2 += 1;
            } else {
                let look = |r: &str| {
                    lookup(c, map, r).map_err(|error| PlaceError::Lookup { index, error })
                };
                let from = look(&along.from)?;
                let to = look(&along.to)?;
                let (cells, _) = ground
                    .walk(&from.cells, &to.cells)
                    .ok_or(PlaceError::NoPath { index })?;
                paths.push((along, cells, 1, 0));
            }
        }
    }
    for (index, ask) in asks.iter().enumerate() {
        let no_such = || PlaceError::NoSuch {
            index,
            id: ask.id.clone(),
        };
        // The entity's own mark on this map, freed if it moves.
        let own: Option<Cell> = match ask.what {
            What::Scene => {
                let node = story.node(&ask.id).ok_or_else(no_such)?;
                node.location
                    .as_deref()
                    .and_then(|l| story.location(l))
                    .and_then(|l| l.anchor.as_ref())
                    .filter(|a| a.map == map.id)
                    .map(|a| a.at)
            }
            What::Place => {
                let l = story.location(&ask.id).ok_or_else(no_such)?;
                if let Some(a) = l.anchor.as_ref().filter(|a| a.map != map.id) {
                    return Err(PlaceError::AnchoredElsewhere {
                        index,
                        map: a.map.clone(),
                    });
                }
                l.anchor.as_ref().map(|a| a.at)
            }
            What::Npc => {
                story.npc(&ask.id).ok_or_else(no_such)?;
                if let Some(scene) = &ask.scene {
                    story.node(scene).ok_or_else(|| PlaceError::NoSuch {
                        index,
                        id: scene.clone(),
                    })?;
                }
                None
            }
            What::Clue => story
                .clue(&ask.id)
                .ok_or_else(no_such)?
                .anchor
                .as_ref()
                .filter(|a| a.map == map.id)
                .map(|a| a.at),
            What::Fight => {
                let node = story.node(&ask.id).ok_or_else(no_such)?;
                let e = node
                    .encounter
                    .as_ref()
                    .filter(|e| e.vehicles.is_none())
                    .ok_or(PlaceError::NoFight { index })?;
                e.anchor.as_ref().filter(|a| a.map == map.id).map(|a| a.at)
            }
        };
        let (target, near) = if let Some(reference) = &ask.near {
            let t = lookup(&story, map, reference)
                .map_err(|error| PlaceError::Lookup { index, error })?;
            (t.cells, t.name)
        } else {
            let along = ask.along.as_ref().ok_or(PlaceError::Malformed { index })?;
            let p = paths
                .iter_mut()
                .find(|p| p.0 == along)
                .ok_or(PlaceError::NoPath { index })?;
            p.3 += 1;
            let last = p.1.len().saturating_sub(1);
            let at = p.1[(p.3 * last) / (p.2 + 1)];
            (vec![at], format!("{} → {}", along.from, along.to))
        };
        if let Some(cell) = own {
            ground.free(cell);
        }
        let (at, steps) = ground
            .spot(&target, false)
            .ok_or(PlaceError::NoRoom { index })?;
        ground.take(at);
        let terrain = map
            .grid
            .kind(at)
            .map(|k| k.terrain.clone())
            .unwrap_or_default();
        let mut done = Placed {
            what: ask.what,
            id: ask.id.clone(),
            name: String::new(),
            at,
            near,
            steps,
            terrain,
            created: None,
            with: Vec::new(),
        };
        let pin = Anchor {
            map: map.id.clone(),
            at,
        };
        match ask.what {
            What::Scene | What::Place => {
                let (place, scene) = if ask.what == What::Place {
                    (Some(ask.id.clone()), None)
                } else {
                    let node = story.node(&ask.id).ok_or_else(no_such)?;
                    (
                        node.location.clone(),
                        Some((node.id.clone(), node.title.clone())),
                    )
                };
                let elsewhere = place
                    .as_deref()
                    .and_then(|l| story.location(l))
                    .and_then(|l| l.anchor.as_ref())
                    .is_some_and(|a| a.map != map.id);
                match (place, scene) {
                    (Some(place), scene) if !elsewhere => {
                        let l = story
                            .locations
                            .iter_mut()
                            .find(|l| l.id == place)
                            .ok_or_else(no_such)?;
                        l.anchor = Some(pin);
                        done.name.clone_from(&l.name);
                        done.with = story
                            .nodes
                            .iter()
                            .filter(|n| n.location.as_deref() == Some(place.as_str()))
                            .filter(|n| scene.as_ref().is_none_or(|(s, _)| *s != n.id))
                            .map(|n| n.id.clone())
                            .collect();
                        edits.push(Edit::Set {
                            target: place,
                            field: "anchor".into(),
                            value: anchor(map, at),
                        });
                    }
                    (parent, Some((scene, title))) => {
                        // No place, or one pinned on another map: a place
                        // of its own, inside the old one, keeps that pin.
                        let base = format!("lieu_{}", scene.strip_prefix("sc_").unwrap_or(&scene));
                        let id = fresh_id(&story, &base, &created);
                        created.push(id.clone());
                        let mut value =
                            json!({ "id": id, "name": title, "anchor": anchor(map, at) });
                        if let Some(parent) = &parent {
                            value["parent"] = json!(parent);
                        }
                        let location = serde_json::from_value(value.clone())
                            .map_err(|_| PlaceError::Malformed { index })?;
                        story.locations.push(location);
                        if let Some(n) = story.nodes.iter_mut().find(|n| n.id == scene) {
                            n.location = Some(id.clone());
                        }
                        done.name = title;
                        done.created = Some(id.clone());
                        edits.push(Edit::Add {
                            kind: Kind::Location,
                            value,
                        });
                        edits.push(Edit::Set {
                            target: scene,
                            field: "location".into(),
                            value: json!(id),
                        });
                    }
                    (_, None) => return Err(no_such()),
                }
            }
            What::Npc => {
                let n = story
                    .npcs
                    .iter_mut()
                    .find(|n| n.id == ask.id)
                    .ok_or_else(no_such)?;
                n.spots.push(super::model::Spot {
                    map: map.id.clone(),
                    at,
                    scene: ask.scene.clone(),
                });
                done.name.clone_from(&n.name);
                let spots = serde_json::to_value(&n.spots).unwrap_or(Value::Null);
                // One edit per NPC: the whole list, as the batch leaves it.
                edits.retain(|e| !matches!(e, Edit::Set { target, field, .. } if *target == ask.id && field == "spots"));
                edits.push(Edit::Set {
                    target: ask.id.clone(),
                    field: "spots".into(),
                    value: spots,
                });
            }
            What::Clue => {
                let cl = story
                    .clues
                    .iter_mut()
                    .find(|cl| cl.id == ask.id)
                    .ok_or_else(no_such)?;
                cl.anchor = Some(pin);
                done.name.clone_from(&cl.text);
                edits.push(Edit::Set {
                    target: ask.id.clone(),
                    field: "anchor".into(),
                    value: anchor(map, at),
                });
            }
            What::Fight => {
                let n = story
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == ask.id)
                    .ok_or_else(no_such)?;
                if let Some(e) = n.encounter.as_mut() {
                    e.anchor = Some(pin);
                }
                done.name.clone_from(&n.title);
                edits.push(Edit::Set {
                    target: ask.id.clone(),
                    field: "encounter.anchor".into(),
                    value: anchor(map, at),
                });
            }
        }
        placed.push(done);
    }
    Ok(Resolved { edits, placed })
}

/// A question to the engine about one map.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "question", rename_all = "snake_case", deny_unknown_fields)]
pub enum Question {
    /// The cheapest walk between two references, and its cost.
    Path { from: String, to: String },
    /// Whether one sees the other, and the cover in between.
    Sight { from: String, to: String },
    /// What the players see standing there.
    View { from: String },
}

/// The engine's answer.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "question", rename_all = "snake_case")]
pub enum Answer {
    Path {
        from: Target,
        to: Target,
        /// The walk, both ends included; empty when there is none.
        path: Vec<Cell>,
        /// In cells of movement (difficult ground, climbing included).
        #[serde(skip_serializing_if = "Option::is_none")]
        cost: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        meters: Option<f64>,
    },
    Sight {
        from: Target,
        to: Target,
        /// The cell the viewer stands on, and the target's cell looked at.
        viewer: Cell,
        target: Cell,
        visible: bool,
        cover: Cover,
        cells: u32,
        meters: f64,
    },
    View {
        from: Target,
        viewer: Cell,
        /// How many cells they see.
        seen: usize,
        light: LightLevel,
        /// The players' map, fogged beyond what they see, around it.
        sketch: Box<Sketch>,
    },
}

/// `question` answered on `map` of campaign `c`, walking under `rules`.
///
/// # Errors
///
/// A reference that names nothing usable; `None` inside when no cell
/// around a reference can be stood on.
pub fn ask(
    c: &Campaign,
    map: &Map,
    rules: MovementRules,
    question: &Question,
) -> Result<Answer, AskError> {
    let ground = Ground::new(map, rules, c, []);
    let look = |r: &str| lookup(c, map, r).map_err(AskError::Lookup);
    let stand = |t: &Target| {
        ground
            .spot(&t.cells, true)
            .map(|(c, _)| c)
            .ok_or(AskError::NoStanding)
    };
    match question {
        Question::Path { from, to } => {
            let (from, to) = (look(from)?, look(to)?);
            let walk = ground.walk(&from.cells, &to.cells);
            Ok(Answer::Path {
                path: walk.as_ref().map(|(p, _)| p.clone()).unwrap_or_default(),
                cost: walk.as_ref().map(|(_, c)| *c),
                meters: walk
                    .as_ref()
                    .map(|(_, c)| f64::from(*c) * map.cell_meters()),
                from,
                to,
            })
        }
        Question::Sight { from, to } => {
            let (from, to) = (look(from)?, look(to)?);
            let viewer = stand(&from)?;
            let geometry = map.geometry();
            let target = to
                .cells
                .iter()
                .copied()
                .min_by_key(|t| {
                    (
                        distance(geometry, viewer, *t, Diagonal::Chebyshev),
                        t.y,
                        t.x,
                    )
                })
                .ok_or(AskError::NoStanding)?;
            let sight = line_of_sight(map, viewer, target, &HashSet::new());
            Ok(Answer::Sight {
                from,
                to,
                viewer,
                target,
                visible: sight.visible,
                cover: sight.cover,
                cells: map.distance(viewer, target, Diagonal::Chebyshev),
                meters: map.range_meters(viewer, target, Diagonal::Chebyshev),
            })
        }
        Question::View { from } => {
            let from = look(from)?;
            let viewer = stand(&from)?;
            let players = map.project(Viewer::Player);
            let seen: BTreeSet<Cell> = visible_cells(&players, viewer, map.ambience.sight_limit);
            let fogged = players.fogged(&seen);
            let zone: Option<Zone> = Zone::around(map, &seen, 1);
            Ok(Answer::View {
                from,
                viewer,
                seen: seen.len(),
                light: illumination(map, viewer),
                sketch: Box::new(sketch(&fogged, &[], &Cast::default(), zone)),
            })
        }
    }
}

/// Why a question has no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskError {
    Lookup(LookupError),
    /// No cell around the reference can be stood on.
    NoStanding,
}

impl AskError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Lookup(e) => e.code(),
            Self::NoStanding => "PLACE_NO_ROOM",
        }
    }

    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::Lookup(e) => e.detail(),
            Self::NoStanding => "no cell to stand on near it".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::from_yaml;

    const MAP: &str = r##"
version: 1
id: auberge
name: L'auberge
scale: place
theme: port-1718
grid:
  legend:
    "X": { terrain: pierre, wall: true }
    ",": { terrain: plancher }
    ".": { terrain: pavés }
    "~": { terrain: gué, water: shallow }
    "=": { terrain: eau, water: deep }
  rows:
    - "XXXXXXXXXX"
    - "X,,,,X,,,X"
    - "X,,,,X,,,X"
    - "X,,,,XXXXX"
    - "X..~~~~..X"
    - "X...====.X"
    - "XXXXXXXXXX"
props:
  - { id: comptoir, kind: comptoir, label: Le comptoir, at: [2, 2], size: [2, 1], blocks_movement: true }
exits:
  - { id: porte, cells: [[8, 5]], to: rue, label: La rue }
labels:
  - { text: Le gué, at: [4, 4] }
  - { text: La réserve, at: [7, 2] }
  - { text: Le mur, at: [5, 2] }
  - { text: Le ponton, at: [5, 5] }
starts:
  - { id: pj-1, side: party, at: [1, 4] }
"##;

    const STORY: &str = r"
id: t
title: T
rules: { id: r, version: 1 }
bible: {}
acts: [{ id: a1, title: Un }]
nodes:
  - { id: sc_salle, act: a1, title: La salle, location: lieu_salle }
  - { id: sc_veillee, act: a1, title: La veillée, location: lieu_salle }
  - id: sc_cave
    act: a1
    title: La cave
    encounter:
      opponents: [{ who: adv_rat, count: 2 }]
  - { id: sc_quai, act: a1, title: Le quai, location: lieu_port }
revelations: [{ id: rev_a, statement: A, importance: optional }]
clues:
  - { id: cl_a, revelation: rev_a, node: sc_salle, text: Un verre brisé }
  - { id: cl_b, revelation: rev_a, node: sc_salle, text: Une pièce }
npcs:
  - id: pnj_gwen
    name: Gwen
    disposition: neutral
    spots: [{ map: region, at: [0, 0] }]
  - { id: pnj_loic, name: Loïc, disposition: neutral }
adversaries:
  - { id: adv_rat, name: Rat, stats: { hit_points: 2 } }
locations:
  - { id: lieu_salle, name: La salle commune }
  - { id: lieu_port, name: Le port, anchor: { map: region, at: [1, 1] } }
";

    fn setup() -> (Campaign, Map) {
        (from_yaml(STORY).unwrap(), Map::from_yaml(MAP).unwrap())
    }

    fn near(what: What, id: &str, reference: &str) -> Placement {
        Placement {
            what,
            id: id.into(),
            scene: None,
            near: Some(reference.into()),
            along: None,
        }
    }

    fn one(c: &Campaign, map: &Map, ask: Placement) -> Result<Placed, PlaceError> {
        place(c, map, MovementRules::default(), &[ask], &[]).map(|r| r.placed[0].clone())
    }

    /// A cell a creature stands on: no wall, no blocking prop, no water.
    fn dry_and_open(map: &Map, at: Cell) {
        let kind = map.grid.kind(at).unwrap();
        assert!(!kind.wall, "{at:?} is a wall");
        assert_eq!(kind.water, Water::None, "{at:?} is in the water");
        assert!(
            standable(map, &MovementRules::default(), at).is_ok(),
            "{at:?}"
        );
    }

    #[test]
    fn a_placement_near_water_lands_on_dry_ground_never_in_the_water() {
        let (c, map) = setup();
        for reference in ["le gué", "Le ponton"] {
            let p = one(&c, &map, near(What::Npc, "pnj_loic", reference)).unwrap();
            dry_and_open(&map, p.at);
            assert!(p.steps >= 1, "{reference}: {p:?}");
        }
    }

    #[test]
    fn near_a_blocking_prop_is_beside_it_not_on_it() {
        let (c, map) = setup();
        let p = one(&c, &map, near(What::Clue, "cl_a", "comptoir")).unwrap();
        dry_and_open(&map, p.at);
        assert_eq!(p.steps, 1);
        assert_eq!(p.near, "comptoir (Le comptoir)");
    }

    #[test]
    fn near_a_wall_lands_on_the_side_the_table_can_reach() {
        let (c, map) = setup();
        // The wall at x 5 parts the hall from a walled-in store room.
        let p = one(&c, &map, near(What::Npc, "pnj_loic", "le mur")).unwrap();
        assert!(p.at.x < 5, "{p:?} is in the walled-in room");
        dry_and_open(&map, p.at);
    }

    #[test]
    fn a_walled_in_pocket_is_never_chosen() {
        let (c, map) = setup();
        assert_eq!(
            one(&c, &map, near(What::Npc, "pnj_loic", "réserve")),
            Err(PlaceError::NoRoom { index: 0 })
        );
    }

    #[test]
    fn two_placements_near_the_same_thing_take_two_cells_and_one_npc_edit() {
        let (c, map) = setup();
        let r = place(
            &c,
            &map,
            MovementRules::default(),
            &[
                near(What::Npc, "pnj_gwen", "comptoir"),
                near(What::Npc, "pnj_gwen", "comptoir"),
                near(What::Clue, "cl_a", "comptoir"),
            ],
            &[],
        )
        .unwrap();
        let cells: BTreeSet<Cell> = r.placed.iter().map(|p| p.at).collect();
        assert_eq!(cells.len(), 3);
        // Gwen's whole list once: the region spot kept, two added.
        let spots: Vec<&Edit> = r
            .edits
            .iter()
            .filter(|e| matches!(e, Edit::Set { field, .. } if field == "spots"))
            .collect();
        assert_eq!(spots.len(), 1);
        let Edit::Set { value, .. } = spots[0] else {
            unreachable!()
        };
        assert_eq!(value.as_array().unwrap().len(), 3);
        assert_eq!(value[0]["map"], "region");
        // A cell still proposed elsewhere is not taken again.
        let first = r.placed[0].at;
        let again = place(
            &c,
            &map,
            MovementRules::default(),
            &[near(What::Npc, "pnj_loic", "comptoir")],
            &[first],
        )
        .unwrap();
        assert_ne!(again.placed[0].at, first);
    }

    #[test]
    fn placements_along_a_path_are_spread_on_it() {
        let (c, map) = setup();
        let along = Some(Along {
            from: "pj-1".into(),
            to: "la rue".into(),
        });
        let ask = |id: &str| Placement {
            what: What::Clue,
            id: id.into(),
            scene: None,
            near: None,
            along: along.clone(),
        };
        let r = place(
            &c,
            &map,
            MovementRules::default(),
            &[ask("cl_a"), ask("cl_b")],
            &[],
        )
        .unwrap();
        let (a, b) = (r.placed[0].at, r.placed[1].at);
        assert_ne!(a, b);
        dry_and_open(&map, a);
        dry_and_open(&map, b);
        // In order along the way out: the first nearer the start.
        assert!(a.x < b.x, "{a:?} {b:?}");
    }

    #[test]
    fn pinning_a_scene_anchors_its_place_or_gives_it_one() {
        let (c, map) = setup();
        let r = place(
            &c,
            &map,
            MovementRules::default(),
            &[near(What::Scene, "sc_salle", "comptoir")],
            &[],
        )
        .unwrap();
        assert_eq!(r.placed[0].with, vec!["sc_veillee".to_string()]);
        assert!(
            matches!(&r.edits[0], Edit::Set { target, field, .. } if target == "lieu_salle" && field == "anchor")
        );
        // The quay's place is pinned on the region: it keeps that pin,
        // and the scene gets a place of its own inside it.
        let r = place(
            &c,
            &map,
            MovementRules::default(),
            &[near(What::Scene, "sc_quai", "la rue")],
            &[],
        )
        .unwrap();
        assert_eq!(r.placed[0].created.as_deref(), Some("lieu_quai"));
        let Edit::Add { value, .. } = &r.edits[0] else {
            panic!("{:?}", r.edits)
        };
        assert_eq!(value["parent"], "lieu_port");
        let after = crate::story::edit::apply(&c, &r.edits).unwrap().0;
        assert_eq!(pins_on(&after, "region")[0].scenes.len(), 1);
        assert_eq!(pins_on(&after, "auberge")[0].location, "lieu_quai");
    }

    #[test]
    fn a_reference_names_one_thing_or_says_what_it_could_be() {
        let (c, map) = setup();
        assert!(matches!(
            lookup(&c, &map, "le"),
            Err(LookupError::Ambiguous { .. }) | Err(LookupError::Unknown { .. })
        ));
        let Err(LookupError::Unknown { known, .. }) = lookup(&c, &map, "cimetière") else {
            panic!()
        };
        assert!(known.iter().any(|k| k.contains("comptoir")));
        assert_eq!(
            lookup(&c, &map, "3, 4").unwrap().cells,
            vec![Cell::new(3, 4)]
        );
        assert!(matches!(
            lookup(&c, &map, "40,4"),
            Err(LookupError::OffGrid { .. })
        ));
        assert!(matches!(
            one(&c, &map, near(What::Fight, "sc_salle", "comptoir")),
            Err(PlaceError::NoFight { index: 0 })
        ));
    }

    #[test]
    fn the_engine_answers_path_sight_and_what_players_see() {
        let (c, map) = setup();
        let rules = MovementRules::default();
        let Answer::Path { path, cost, .. } = ask(
            &c,
            &map,
            rules,
            &Question::Path {
                from: "pj-1".into(),
                to: "la rue".into(),
            },
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!(path.first(), Some(&Cell::new(1, 4)));
        assert!(cost.unwrap() >= 7);
        let Answer::Sight { visible, .. } = ask(
            &c,
            &map,
            rules,
            &Question::Sight {
                from: "pj-1".into(),
                to: "6,1".into(),
            },
        )
        .unwrap() else {
            panic!()
        };
        assert!(!visible, "the store room is walled in");
        let Answer::View { sketch, .. } = ask(
            &c,
            &map,
            rules,
            &Question::View {
                from: "pj-1".into(),
            },
        )
        .unwrap() else {
            panic!()
        };
        // The walled-in store room stays under the fog.
        let z = sketch.zone;
        let glyph = |x: i32, y: i32| {
            sketch.rows[usize::try_from(2 + y - z.y).unwrap()]
                .chars()
                .nth(usize::try_from(4 + x - z.x).unwrap())
        };
        assert_eq!(glyph(6, 1), Some('?'));
        assert_eq!(glyph(2, 1), Some(','));
    }
}
