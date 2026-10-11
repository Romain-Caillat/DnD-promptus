//! Targeted operations on a map, and what a batch of them touched
//! (`maps/edit-maps-by-ops`).
//!
//! The editor — and Claude, through a GM's personal token — never sends
//! a whole map: it sends a batch of [`MapOp`]s (paint an area, place,
//! move, remove, erase a cell, toggle a door, set the map's own fields).
//! The server applies the batch with [`apply`] on the map as it is now,
//! under the campaign lock, then checks the result with `Map::validate`:
//! all of it lands, or none.
//!
//! [`diff`] says which cells and which things a batch touched
//! ([`MapTouch`]); the server journals it, and refuses a batch sent on a
//! stale revision only when a later batch on the same map [`overlaps`]
//! it — two writers on different cells both keep their change.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::grid::Cell;
use super::model::{
    Ambience, BASE_LAYER, CellKind, Cover, Door, DoorState, Footprint, Grid, Light, Map, MapObject,
    Prop, Side, Start,
};
use crate::story::revision::Op;

/// Glyphs a new kind of cell may take, in order.
const GLYPHS: &str =
    ".#,~_=:;+*%&@!?abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// Longest map name and GM notes, in characters.
pub const MAX_NAME_CHARS: usize = 80;
pub const MAX_NOTES_CHARS: usize = 4000;

fn base_layer() -> String {
    BASE_LAYER.into()
}

/// One operation of a batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum MapOp {
    /// `cells` take `kind`. Cells off the grid are ignored (a brush
    /// stroke that ran past the edge); a door whose wall is painted over
    /// leaves with it.
    Paint { cells: Vec<Cell>, kind: CellKind },
    /// A new thing on `at`, with an id of its own.
    Place {
        at: Cell,
        thing: Thing,
        #[serde(default = "base_layer")]
        layer: String,
    },
    /// The thing `id` (door, prop, object, light, start) moves to `to`.
    Move { id: String, to: Cell },
    /// The thing `id` (door, prop, object, light, exit, start) leaves.
    Remove { id: String },
    /// Everything placed on `at` leaves; the ground stays.
    Erase { at: Cell },
    /// A door on `at`: a new one is closed (its cell becomes a wall of
    /// the same ground); one already there goes closed → open → locked →
    /// gone.
    Door {
        at: Cell,
        #[serde(default = "base_layer")]
        layer: String,
    },
    /// The map's own fields. `notes` empty clears them.
    Set {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ambience: Option<Ambience>,
    },
}

/// What a [`MapOp::Place`] puts down.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Thing {
    /// Decor from the tileset: half cover, blocks movement.
    Prop {
        prop: String,
        label: String,
    },
    /// Something hidden to find.
    Object {
        #[serde(default)]
        label: String,
    },
    Light,
    /// Where a token starts; `entity` is who stands there (an NPC, an
    /// adversary).
    Start {
        side: Side,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        entity: Option<String>,
    },
}

/// Why a batch does not apply.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpError {
    #[error("cell {0:?} is off the grid")]
    OffGrid(Cell),
    #[error("nothing on the map has the id {0:?}")]
    NoSuchThing(String),
    #[error("the legend has no glyph left for a new kind of cell")]
    LegendFull,
    #[error("the map needs a name")]
    NameRequired,
    #[error("text too long")]
    TextTooLong,
}

fn kinds(map: &Map) -> BTreeMap<char, CellKind> {
    map.grid.legend().map(|(g, k)| (g, k.clone())).collect()
}

fn rows(map: &Map) -> Vec<Vec<char>> {
    map.grid
        .rows()
        .iter()
        .map(|r| r.chars().collect())
        .collect()
}

fn in_grid(map: &Map, c: Cell) -> Result<(), OpError> {
    if map.grid.contains(c) {
        Ok(())
    } else {
        Err(OpError::OffGrid(c))
    }
}

/// `cells` painted with `kind`: the legend keeps one glyph per kind of
/// cell, a new kind takes the first free glyph, and a glyph no cell uses
/// any more leaves the legend.
fn paint(map: &mut Map, cells: &[Cell], kind: &CellKind) -> Result<(), OpError> {
    let mut legend = kinds(map);
    let glyph = match legend.iter().find(|(_, k)| *k == kind) {
        Some((g, _)) => *g,
        None => {
            let g = GLYPHS
                .chars()
                .find(|g| !legend.contains_key(g))
                .ok_or(OpError::LegendFull)?;
            legend.insert(g, kind.clone());
            g
        }
    };
    let mut grid = rows(map);
    for &c in cells {
        if map.grid.contains(c) {
            grid[c.y as usize][c.x as usize] = glyph;
        }
    }
    let used: BTreeSet<char> = grid.iter().flatten().copied().collect();
    legend.retain(|g, _| used.contains(g));
    let lines: Vec<String> = grid.iter().map(|r| r.iter().collect()).collect();
    // Same size, every glyph in the legend: a grid that was valid stays so.
    map.grid = Grid::new(&legend, &lines).map_err(|_| OpError::LegendFull)?;
    // A door needs its wall: one painted over leaves with it.
    let grid = &map.grid;
    map.doors
        .retain(|d| grid.kind(d.at).is_some_and(|k| k.wall));
    Ok(())
}

/// An id no door, prop, object, light, exit or start of `map` has yet.
fn fresh_id(map: &Map, prefix: &str) -> String {
    let taken: BTreeSet<&str> = ids(map).collect();
    let mut n = 1;
    loop {
        let id = format!("{prefix}-{n}");
        if !taken.contains(id.as_str()) {
            return id;
        }
        n += 1;
    }
}

fn ids(map: &Map) -> impl Iterator<Item = &str> {
    map.doors
        .iter()
        .map(|d| d.id.as_str())
        .chain(map.props.iter().map(|p| p.id.as_str()))
        .chain(map.objects.iter().map(|o| o.id.as_str()))
        .chain(map.lights.iter().map(|l| l.id.as_str()))
        .chain(map.exits.iter().map(|e| e.id.as_str()))
        .chain(map.starts.iter().map(|s| s.id.as_str()))
}

fn toggle_door(map: &mut Map, at: Cell, layer: &str) -> Result<(), OpError> {
    in_grid(map, at)?;
    if let Some(i) = map.doors.iter().position(|d| d.at == at) {
        match map.doors[i].state {
            DoorState::Closed => map.doors[i].state = DoorState::Open,
            DoorState::Open => map.doors[i].state = DoorState::Locked,
            DoorState::Locked => {
                map.doors.remove(i);
            }
        }
        return Ok(());
    }
    if let Some(ground) = map.grid.kind(at).filter(|k| !k.wall).cloned() {
        let wall = CellKind {
            wall: true,
            void: false,
            ..ground
        };
        paint(map, &[at], &wall)?;
    }
    let id = fresh_id(map, "porte");
    map.doors.push(Door {
        id,
        at,
        state: DoorState::Closed,
        label: None,
        layer: layer.into(),
    });
    Ok(())
}

fn place(map: &mut Map, at: Cell, thing: &Thing, layer: &str) -> Result<(), OpError> {
    in_grid(map, at)?;
    let layer = layer.to_string();
    match thing {
        Thing::Prop { prop, label } => {
            let id = fresh_id(map, "decor");
            map.props.push(Prop {
                id,
                kind: prop.clone(),
                label: Some(label.clone()),
                at,
                size: Footprint::default(),
                cover: Cover::Half,
                blocks_movement: true,
                difficult: false,
                climbable: false,
                overhead: false,
                layer,
            });
        }
        Thing::Object { label } => {
            let id = fresh_id(map, "objet");
            let label = label.trim();
            map.objects.push(MapObject {
                id,
                kind: "cache".into(),
                label: (!label.is_empty()).then(|| label.to_string()),
                at,
                check: None,
                notes: None,
                layer,
            });
        }
        Thing::Light => {
            let id = fresh_id(map, "lumiere");
            map.lights.push(Light {
                id,
                at,
                bright: 1,
                dim: 4,
                color: Some("#ffb35c".into()),
                flicker: false,
                layer,
            });
        }
        Thing::Start { side, entity } => {
            let id = fresh_id(map, "depart");
            map.starts.push(Start {
                id,
                side: Some(*side),
                at,
                entity: entity.clone(),
                layer,
            });
        }
    }
    Ok(())
}

fn move_thing(map: &mut Map, id: &str, to: Cell) -> Result<(), OpError> {
    in_grid(map, to)?;
    let at = map
        .doors
        .iter_mut()
        .find(|d| d.id == id)
        .map(|d| &mut d.at)
        .or_else(|| map.props.iter_mut().find(|p| p.id == id).map(|p| &mut p.at))
        .or_else(|| {
            map.objects
                .iter_mut()
                .find(|o| o.id == id)
                .map(|o| &mut o.at)
        })
        .or_else(|| {
            map.lights
                .iter_mut()
                .find(|l| l.id == id)
                .map(|l| &mut l.at)
        })
        .or_else(|| {
            map.starts
                .iter_mut()
                .find(|s| s.id == id)
                .map(|s| &mut s.at)
        })
        .ok_or_else(|| OpError::NoSuchThing(id.into()))?;
    *at = to;
    Ok(())
}

fn remove(map: &mut Map, id: &str) -> Result<(), OpError> {
    let before = ids(map).count();
    map.doors.retain(|d| d.id != id);
    map.props.retain(|p| p.id != id);
    map.objects.retain(|o| o.id != id);
    map.lights.retain(|l| l.id != id);
    map.exits.retain(|e| e.id != id);
    map.starts.retain(|s| s.id != id);
    if ids(map).count() == before {
        return Err(OpError::NoSuchThing(id.into()));
    }
    Ok(())
}

fn erase(map: &mut Map, at: Cell) {
    map.doors.retain(|d| d.at != at);
    map.props.retain(|p| !p.cells().any(|c| c == at));
    map.objects.retain(|o| o.at != at);
    map.lights.retain(|l| l.at != at);
    map.starts.retain(|s| s.at != at);
}

fn set(
    map: &mut Map,
    name: Option<&str>,
    notes: Option<&str>,
    ambience: Option<&Ambience>,
) -> Result<(), OpError> {
    if let Some(name) = name {
        let name = name.trim();
        if name.is_empty() {
            return Err(OpError::NameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(OpError::TextTooLong);
        }
        map.name = name.into();
    }
    if let Some(notes) = notes {
        if notes.chars().count() > MAX_NOTES_CHARS {
            return Err(OpError::TextTooLong);
        }
        map.gm_notes = (!notes.trim().is_empty()).then(|| notes.to_string());
    }
    if let Some(ambience) = ambience {
        map.ambience = ambience.clone();
    }
    Ok(())
}

/// `ops` applied in order on a copy of `map`. The caller checks the
/// result with `Map::validate` before keeping it.
///
/// # Errors
///
/// The first operation that does not apply: a cell off the grid, an
/// unknown id, a full legend, a missing or too long name.
pub fn apply(map: &Map, ops: &[MapOp]) -> Result<Map, OpError> {
    let mut map = map.clone();
    for op in ops {
        match op {
            MapOp::Paint { cells, kind } => paint(&mut map, cells, kind)?,
            MapOp::Place { at, thing, layer } => place(&mut map, *at, thing, layer)?,
            MapOp::Move { id, to } => move_thing(&mut map, id, *to)?,
            MapOp::Remove { id } => remove(&mut map, id)?,
            MapOp::Erase { at } => erase(&mut map, *at),
            MapOp::Door { at, layer } => toggle_door(&mut map, *at, layer)?,
            MapOp::Set {
                name,
                notes,
                ambience,
            } => set(
                &mut map,
                name.as_deref(),
                notes.as_deref(),
                ambience.as_ref(),
            )?,
        }
    }
    Ok(map)
}

/// What part of a map a touch is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapKind {
    /// The map itself: created, deleted, or its own fields (name, notes,
    /// ambience, layers, labels…).
    Map,
    /// Ground painted.
    Cells,
    Door,
    Prop,
    Object,
    Light,
    Exit,
    Start,
}

/// One part of a map a batch touched. Shaped like a story `Touch`, so
/// the journal reads the same for both: `id` is the thing's (the map's
/// for [`MapKind::Map`] and [`MapKind::Cells`]), `name` the map's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapTouch {
    pub op: Op,
    pub kind: MapKind,
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// The cells it covers, before and after.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<Cell>,
}

impl MapTouch {
    /// The map `map` as a whole, created (`Add`) or deleted (`Remove`).
    #[must_use]
    pub fn whole(op: Op, map: &Map) -> Self {
        Self {
            op,
            kind: MapKind::Map,
            id: map.id.clone(),
            name: map.name.clone(),
            cells: vec![],
        }
    }
}

/// Each id'd thing of `map` of one kind, as JSON with the cells it covers.
fn things(map: &Map, kind: MapKind) -> Vec<(String, Value, Vec<Cell>)> {
    fn each<T: Serialize>(
        items: &[T],
        id: impl Fn(&T) -> &str,
        cells: impl Fn(&T) -> Vec<Cell>,
    ) -> Vec<(String, Value, Vec<Cell>)> {
        items
            .iter()
            .map(|t| {
                (
                    id(t).to_string(),
                    serde_json::to_value(t).unwrap_or(Value::Null),
                    cells(t),
                )
            })
            .collect()
    }
    match kind {
        MapKind::Door => each(&map.doors, |d| &d.id, |d| vec![d.at]),
        MapKind::Prop => each(&map.props, |p| &p.id, |p| p.cells().collect()),
        MapKind::Object => each(&map.objects, |o| &o.id, |o| vec![o.at]),
        MapKind::Light => each(&map.lights, |l| &l.id, |l| vec![l.at]),
        MapKind::Exit => each(&map.exits, |e| &e.id, |e| e.cells.clone()),
        MapKind::Start => each(&map.starts, |s| &s.id, |s| vec![s.at]),
        MapKind::Map | MapKind::Cells => vec![],
    }
}

/// The map's own fields: everything that is neither the grid nor an
/// id'd thing.
fn own_fields(map: &Map) -> Value {
    let mut v = serde_json::to_value(map).unwrap_or(Value::Null);
    if let Value::Object(o) = &mut v {
        for k in [
            "grid", "doors", "props", "objects", "lights", "exits", "starts",
        ] {
            o.remove(k);
        }
    }
    v
}

/// The list of `map` a kind of thing lives in.
const fn list(kind: MapKind) -> Option<&'static str> {
    match kind {
        MapKind::Door => Some("doors"),
        MapKind::Prop => Some("props"),
        MapKind::Object => Some("objects"),
        MapKind::Light => Some("lights"),
        MapKind::Exit => Some("exits"),
        MapKind::Start => Some("starts"),
        MapKind::Map | MapKind::Cells => None,
    }
}

const THINGS: [MapKind; 6] = [
    MapKind::Door,
    MapKind::Prop,
    MapKind::Object,
    MapKind::Light,
    MapKind::Exit,
    MapKind::Start,
];

/// What changed from `before` to `after` (the same map): the cells
/// painted, each thing added, set or removed, and the map's own fields.
#[must_use]
pub fn diff(before: &Map, after: &Map) -> Vec<MapTouch> {
    let name = after.name.clone();
    let touch = |op, kind, id: &str, cells: Vec<Cell>| MapTouch {
        op,
        kind,
        id: id.to_string(),
        name: name.clone(),
        cells,
    };
    let mut out = Vec::new();
    if own_fields(before) != own_fields(after) {
        out.push(touch(Op::Set, MapKind::Map, &after.id, vec![]));
    }
    let painted: Vec<Cell> = if before.grid.width() == after.grid.width()
        && before.grid.height() == after.grid.height()
    {
        before
            .grid
            .cells()
            .filter(|&c| before.grid.kind(c) != after.grid.kind(c))
            .collect()
    } else {
        after.grid.cells().collect()
    };
    if !painted.is_empty() {
        out.push(touch(Op::Set, MapKind::Cells, &after.id, painted));
    }
    for kind in THINGS {
        let was = things(before, kind);
        let now = things(after, kind);
        for (id, value, cells) in &now {
            match was.iter().find(|(i, _, _)| i == id) {
                None => out.push(touch(Op::Add, kind, id, cells.clone())),
                Some((_, old, old_cells)) if old != value => {
                    let mut all = old_cells.clone();
                    all.extend(cells.iter().filter(|c| !old_cells.contains(c)));
                    out.push(touch(Op::Set, kind, id, all));
                }
                Some(_) => {}
            }
        }
        for (id, _, cells) in &was {
            if !now.iter().any(|(i, _, _)| i == id) {
                out.push(touch(Op::Remove, kind, id, cells.clone()));
            }
        }
    }
    out
}

/// `now` with only what `touched` names put back as it was in `before`
/// (the map before the batch that touched it): its cells repainted,
/// each thing removed, restored or put back in place, the map's own
/// fields. Everything else stays as it is now, so taking a batch back
/// keeps later batches made elsewhere on the map. `None` when what it
/// puts back no longer reads as a map; the caller validates the result.
#[must_use]
pub fn revert(now: &Map, before: &Map, touched: &[MapTouch]) -> Option<Map> {
    let mut map = now.clone();
    // Cells first: a door comes back only once its wall is back.
    let mut by_kind: Vec<(CellKind, Vec<Cell>)> = Vec::new();
    for c in touched
        .iter()
        .filter(|t| t.kind == MapKind::Cells)
        .flat_map(|t| t.cells.iter().copied())
    {
        let Some(kind) = before.grid.kind(c) else {
            continue;
        };
        match by_kind.iter_mut().find(|(k, _)| k == kind) {
            Some((_, cells)) => cells.push(c),
            None => by_kind.push((kind.clone(), vec![c])),
        }
    }
    for (kind, cells) in &by_kind {
        paint(&mut map, cells, kind).ok()?;
    }
    let mut doc = serde_json::to_value(&map).ok()?;
    let was = serde_json::to_value(before).ok()?;
    let items = |v: &Value, l: &str| v.get(l).and_then(Value::as_array).cloned();
    for t in touched {
        let Some(l) = list(t.kind) else { continue };
        let mut now_items = items(&doc, l).unwrap_or_default();
        now_items.retain(|i| i.get("id").and_then(Value::as_str) != Some(t.id.as_str()));
        let old = items(&was, l).unwrap_or_default();
        if let Some(index) = old
            .iter()
            .position(|i| i.get("id").and_then(Value::as_str) == Some(t.id.as_str()))
        {
            now_items.insert(index.min(now_items.len()), old[index].clone());
        }
        doc.as_object_mut()?
            .insert(l.to_string(), Value::Array(now_items));
    }
    if touched
        .iter()
        .any(|t| t.kind == MapKind::Map && t.op == Op::Set)
    {
        let (Value::Object(own_now), Value::Object(own_was)) =
            (own_fields(&map), own_fields(before))
        else {
            return None;
        };
        let obj = doc.as_object_mut()?;
        for k in own_now.keys() {
            obj.remove(k);
        }
        obj.extend(own_was);
    }
    serde_json::from_value(doc).ok()
}

/// Whether two batches on the same map step on each other: the map
/// created or deleted, the same thing or the map's own fields touched by
/// both, or a cell either covers.
#[must_use]
pub fn overlaps(a: &[MapTouch], b: &[MapTouch]) -> bool {
    let whole = |t: &MapTouch| t.kind == MapKind::Map && t.op != Op::Set;
    if a.iter().chain(b).any(whole) {
        return true;
    }
    let cells: BTreeSet<Cell> = a.iter().flat_map(|t| t.cells.iter().copied()).collect();
    b.iter().any(|t| {
        t.cells.iter().any(|c| cells.contains(c))
            || a.iter()
                .any(|u| u.kind == t.kind && u.id == t.id && t.kind != MapKind::Cells)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{Scale, Visibility, Water};

    fn room() -> Map {
        Map::from_yaml(
            r######"
version: 1
id: salle
name: Salle
scale: encounter
theme: port-1718
layers:
  - { id: base, name: Carte, visibility: all }
  - { id: secrets, name: Secrets, visibility: gm }
grid:
  legend:
    "#": { terrain: pierre, wall: true }
    ".": { terrain: pavés }
  rows:
    - "#####"
    - "#...#"
    - "#####"
"######,
        )
        .unwrap()
    }

    fn floor(terrain: &str) -> CellKind {
        CellKind {
            terrain: terrain.into(),
            ..CellKind::default()
        }
    }

    fn c(x: i32, y: i32) -> Cell {
        Cell::new(x, y)
    }

    #[test]
    fn paints_with_one_glyph_per_kind_and_forgets_the_unused_ones() {
        let water = CellKind {
            water: Water::Deep,
            ..floor("eau")
        };
        let wet = apply(
            &room(),
            &[MapOp::Paint {
                cells: vec![c(1, 1), c(2, 1), c(9, 9)],
                kind: water.clone(),
            }],
        )
        .unwrap();
        assert_eq!(wet.grid.rows(), ["#####", "#,,.#", "#####"]);
        assert_eq!(wet.grid.kind(c(1, 1)), Some(&water));
        let walled = apply(
            &wet,
            &[MapOp::Paint {
                cells: vec![c(1, 1), c(2, 1), c(3, 1)],
                kind: CellKind {
                    wall: true,
                    ..floor("pierre")
                },
            }],
        )
        .unwrap();
        assert_eq!(walled.grid.rows(), ["#####"; 3]);
        assert_eq!(walled.grid.legend().count(), 1);
    }

    #[test]
    fn a_door_walls_its_cell_cycles_and_leaves_with_its_wall() {
        let door = |m: &Map| {
            apply(
                m,
                &[MapOp::Door {
                    at: c(2, 1),
                    layer: BASE_LAYER.into(),
                }],
            )
            .unwrap()
        };
        let mut map = door(&room());
        assert_eq!(map.grid.rows()[1], "#.,.#");
        assert!(map.grid.kind(c(2, 1)).unwrap().wall);
        assert_eq!(map.doors[0].id, "porte-1");
        assert_eq!(map.doors[0].state, DoorState::Closed);
        map = door(&map);
        assert_eq!(map.doors[0].state, DoorState::Open);
        map = door(&door(&map));
        assert!(map.doors.is_empty());
        map = door(&map);
        let painted = apply(
            &map,
            &[MapOp::Paint {
                cells: vec![c(2, 1)],
                kind: floor("pavés"),
            }],
        )
        .unwrap();
        assert!(painted.doors.is_empty());
        assert!(painted.validate().is_ok());
    }

    #[test]
    fn places_moves_removes_and_erases_things_with_ids_of_their_own() {
        let place = |at, thing, layer: &str| MapOp::Place {
            at,
            thing,
            layer: layer.into(),
        };
        let map = apply(
            &room(),
            &[
                place(
                    c(1, 1),
                    Thing::Object {
                        label: "Trappe".into(),
                    },
                    "secrets",
                ),
                place(c(1, 1), Thing::Light, BASE_LAYER),
                place(
                    c(3, 1),
                    Thing::Start {
                        side: Side::Foes,
                        entity: Some("garde".into()),
                    },
                    BASE_LAYER,
                ),
                place(
                    c(2, 1),
                    Thing::Prop {
                        prop: "caisses".into(),
                        label: "Caisses".into(),
                    },
                    BASE_LAYER,
                ),
                MapOp::Move {
                    id: "depart-1".into(),
                    to: c(1, 1),
                },
            ],
        )
        .unwrap();
        assert!(map.validate().is_ok());
        assert_eq!(map.objects[0].id, "objet-1");
        assert_eq!(map.visibility(&map.objects[0].layer), Visibility::Gm);
        assert_eq!(map.starts[0].at, c(1, 1));
        assert_eq!(map.starts[0].entity.as_deref(), Some("garde"));
        let erased = apply(&map, &[MapOp::Erase { at: c(1, 1) }]).unwrap();
        assert!(erased.objects.is_empty() && erased.lights.is_empty() && erased.starts.is_empty());
        assert_eq!(erased.props.len(), 1);
        let removed = apply(
            &map,
            &[MapOp::Remove {
                id: "decor-1".into(),
            }],
        )
        .unwrap();
        assert!(removed.props.is_empty());
        assert_eq!(
            apply(&room(), &[place(c(9, 9), Thing::Light, BASE_LAYER)]),
            Err(OpError::OffGrid(c(9, 9)))
        );
        assert_eq!(
            apply(&room(), &[MapOp::Remove { id: "nope".into() }]),
            Err(OpError::NoSuchThing("nope".into()))
        );
    }

    #[test]
    fn sets_the_maps_own_fields() {
        let map = apply(
            &room(),
            &[MapOp::Set {
                name: Some(" Crypte ".into()),
                notes: Some("Le prêtre ment.".into()),
                ambience: None,
            }],
        )
        .unwrap();
        assert_eq!(map.name, "Crypte");
        assert_eq!(map.gm_notes.as_deref(), Some("Le prêtre ment."));
        let cleared = apply(
            &map,
            &[MapOp::Set {
                name: None,
                notes: Some(String::new()),
                ambience: None,
            }],
        )
        .unwrap();
        assert_eq!(cleared.gm_notes, None);
        assert_eq!(
            apply(
                &room(),
                &[MapOp::Set {
                    name: Some("  ".into()),
                    notes: None,
                    ambience: None
                }]
            ),
            Err(OpError::NameRequired)
        );
        assert_eq!(map.scale, Scale::Encounter);
    }

    #[test]
    fn reverting_a_batch_keeps_what_others_changed_elsewhere() {
        let base = apply(
            &room(),
            &[MapOp::Door {
                at: c(2, 1),
                layer: BASE_LAYER.into(),
            }],
        )
        .unwrap();
        // The batch to take back: the door's wall painted over (the door
        // leaves with it), a light, a new name.
        let batch = apply(
            &base,
            &[
                MapOp::Paint {
                    cells: vec![c(2, 1)],
                    kind: floor("pavés"),
                },
                MapOp::Place {
                    at: c(1, 1),
                    thing: Thing::Light,
                    layer: BASE_LAYER.into(),
                },
                MapOp::Set {
                    name: Some("Crypte".into()),
                    notes: None,
                    ambience: None,
                },
            ],
        )
        .unwrap();
        let touched = diff(&base, &batch);
        // Someone else, later, elsewhere.
        let later = apply(
            &batch,
            &[MapOp::Place {
                at: c(3, 1),
                thing: Thing::Object {
                    label: "Clé".into(),
                },
                layer: BASE_LAYER.into(),
            }],
        )
        .unwrap();
        assert!(!overlaps(&touched, &diff(&batch, &later)));
        let back = revert(&later, &base, &touched).unwrap();
        assert!(back.validate().is_ok());
        assert_eq!(back.grid, base.grid);
        assert_eq!(back.doors, base.doors);
        assert!(back.lights.is_empty());
        assert_eq!(back.name, "Salle");
        assert_eq!(back.objects.len(), 1, "the later object stays");
    }

    #[test]
    fn batches_on_different_cells_do_not_overlap_and_the_same_cell_does() {
        let base = room();
        let wall = apply(
            &base,
            &[MapOp::Paint {
                cells: vec![c(1, 1)],
                kind: CellKind {
                    wall: true,
                    ..floor("pierre")
                },
            }],
        )
        .unwrap();
        let npc = apply(
            &base,
            &[MapOp::Place {
                at: c(3, 1),
                thing: Thing::Start {
                    side: Side::Neutral,
                    entity: Some("aubergiste".into()),
                },
                layer: BASE_LAYER.into(),
            }],
        )
        .unwrap();
        let a = diff(&base, &wall);
        let b = diff(&base, &npc);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].kind, MapKind::Cells);
        assert_eq!(a[0].cells, [c(1, 1)]);
        assert_eq!((b[0].op, b[0].kind), (Op::Add, MapKind::Start));
        assert!(!overlaps(&a, &b));
        let there = apply(
            &base,
            &[MapOp::Paint {
                cells: vec![c(3, 1)],
                kind: CellKind {
                    wall: true,
                    ..floor("pierre")
                },
            }],
        )
        .unwrap();
        assert!(overlaps(&diff(&base, &there), &b));
        assert!(overlaps(&[MapTouch::whole(Op::Remove, &base)], &b));
        assert!(diff(&base, &base).is_empty());
    }
}
