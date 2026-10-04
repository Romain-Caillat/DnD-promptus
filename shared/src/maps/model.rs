//! The map model: what a map *is*, independent of how it is drawn.
//!
//! The grid carries the rules. A generated or imported image is only a
//! backdrop (`Backdrop`); walls, doors, props, hidden objects and lights
//! are data on the grid, each on a layer whose visibility decides who
//! receives it. The serialised form of these types is the map file
//! format, documented in `docs/map-format.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::grid::{Cell, Diagonal, Direction, Geometry, distance};

/// Version of the map file format this code reads and writes.
pub const FORMAT_VERSION: u32 = 1;

/// The layer everything sits on unless it says otherwise.
pub const BASE_LAYER: &str = "base";

/// Largest grid side accepted, in cells.
pub const MAX_SIDE: u32 = 512;

/// A map at one of the three scales.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Map {
    /// Format version, `FORMAT_VERSION`.
    pub version: u32,
    pub id: String,
    /// Shown to people, in French.
    pub name: String,
    pub scale: Scale,
    /// Overrides the scale's default cell size, in metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_meters: Option<f64>,
    /// Tileset / theme pack id. Rendering picks the tiles from it.
    pub theme: String,
    #[serde(default)]
    pub ambience: Ambience,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<Backdrop>,
    /// Never sent to players.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gm_notes: Option<String>,
    #[serde(default = "default_layers")]
    pub layers: Vec<Layer>,
    pub grid: Grid,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub doors: Vec<Door>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Prop>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<MapObject>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lights: Vec<Light>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exits: Vec<Exit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<Label>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub starts: Vec<Start>,
}

/// The three map scales (`MEMORY.md` §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    /// Hexes of about 10 km, travel by portions of a day.
    World,
    /// Squares of about 5 m: a village, a station, a ship.
    Place,
    /// Squares of 1.5 m, 6-second turns.
    Encounter,
}

impl Scale {
    pub const fn geometry(self) -> Geometry {
        match self {
            Scale::World => Geometry::Hex,
            Scale::Place | Scale::Encounter => Geometry::Square,
        }
    }

    pub const fn default_cell_meters(self) -> f64 {
        match self {
            Scale::World => 10_000.0,
            Scale::Place => 5.0,
            Scale::Encounter => 1.5,
        }
    }
}

/// What one glyph of the grid stands for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellKind {
    /// Material id (`pavés`, `planches`, `métal`…), drawn by the tileset.
    pub terrain: String,
    /// Blocks movement and sight. Doors sit on wall cells.
    #[serde(default, skip_serializing_if = "is_false")]
    pub wall: bool,
    #[serde(default, skip_serializing_if = "Water::is_none")]
    pub water: Water,
    #[serde(default, skip_serializing_if = "is_false")]
    pub difficult: bool,
    /// Nothing there: a hole, open space. Never walkable, does not block sight.
    #[serde(default, skip_serializing_if = "is_false")]
    pub void: bool,
    /// Height in levels; one level is drawn one tile up.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub elevation: i8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Water {
    #[default]
    None,
    /// Wading: difficult terrain.
    Shallow,
    /// Swimming: impassable unless the rule system allows swimming.
    Deep,
}

impl Water {
    fn is_none(&self) -> bool {
        *self == Water::None
    }
}

/// The cells of a map, stored as a legend of glyphs and rows of glyphs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "GridRepr", into = "GridRepr")]
pub struct Grid {
    width: u32,
    height: u32,
    glyphs: Vec<char>,
    kinds: Vec<CellKind>,
    cells: Vec<u16>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GridRepr {
    legend: BTreeMap<String, CellKind>,
    rows: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GridError {
    #[error("the grid has no rows")]
    Empty,
    #[error("legend key {0:?} must be one character, not a space")]
    BadLegendKey(String),
    #[error("row {row} has {found} cells, expected {expected}")]
    RaggedRow {
        row: usize,
        expected: usize,
        found: usize,
    },
    #[error("glyph {glyph:?} at row {row}, column {col} is not in the legend")]
    UnknownGlyph { glyph: char, row: usize, col: usize },
    #[error("the grid is larger than {MAX_SIDE} cells on a side")]
    TooLarge,
}

impl Grid {
    /// Builds a grid from a legend and rows of glyphs.
    pub fn new<S: AsRef<str>>(
        legend: &BTreeMap<char, CellKind>,
        rows: &[S],
    ) -> Result<Self, GridError> {
        let first = rows.first().ok_or(GridError::Empty)?;
        let width = first.as_ref().chars().count();
        if width == 0 {
            return Err(GridError::Empty);
        }
        if width > MAX_SIDE as usize || rows.len() > MAX_SIDE as usize {
            return Err(GridError::TooLarge);
        }
        let glyphs: Vec<char> = legend.keys().copied().collect();
        if let Some(g) = glyphs.iter().find(|g| g.is_whitespace()) {
            return Err(GridError::BadLegendKey(g.to_string()));
        }
        let kinds: Vec<CellKind> = legend.values().cloned().collect();
        let mut cells = Vec::with_capacity(width * rows.len());
        for (row, line) in rows.iter().enumerate() {
            let line = line.as_ref();
            let found = line.chars().count();
            if found != width {
                return Err(GridError::RaggedRow {
                    row,
                    expected: width,
                    found,
                });
            }
            for (col, glyph) in line.chars().enumerate() {
                let idx = glyphs
                    .binary_search(&glyph)
                    .map_err(|_| GridError::UnknownGlyph { glyph, row, col })?;
                // At most MAX_SIDE² glyphs are distinct in practice; the
                // legend is a BTreeMap of chars and far below u16::MAX.
                cells.push(u16::try_from(idx).map_err(|_| GridError::TooLarge)?);
            }
        }
        Ok(Self {
            width: u32::try_from(width).map_err(|_| GridError::TooLarge)?,
            height: u32::try_from(rows.len()).map_err(|_| GridError::TooLarge)?,
            glyphs,
            kinds,
            cells,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn contains(&self, c: Cell) -> bool {
        c.x >= 0 && c.y >= 0 && (c.x as u32) < self.width && (c.y as u32) < self.height
    }

    /// Row-major index of a cell inside the grid.
    pub fn index(&self, c: Cell) -> Option<usize> {
        self.contains(c)
            .then(|| c.y as usize * self.width as usize + c.x as usize)
    }

    pub fn cell_at(&self, index: usize) -> Cell {
        let w = self.width as usize;
        // Indexes come from `index`, so both fit in i32.
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        Cell::new((index % w) as i32, (index / w) as i32)
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn kind(&self, c: Cell) -> Option<&CellKind> {
        self.index(c).map(|i| &self.kinds[self.cells[i] as usize])
    }

    pub fn glyph(&self, c: Cell) -> Option<char> {
        self.index(c).map(|i| self.glyphs[self.cells[i] as usize])
    }

    /// Every cell of the grid, row by row.
    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        (0..self.cells.len()).map(|i| self.cell_at(i))
    }

    pub fn legend(&self) -> impl Iterator<Item = (char, &CellKind)> {
        self.glyphs.iter().copied().zip(self.kinds.iter())
    }

    pub fn rows(&self) -> Vec<String> {
        self.cells
            .chunks(self.width as usize)
            .map(|row| row.iter().map(|&k| self.glyphs[k as usize]).collect())
            .collect()
    }
}

impl TryFrom<GridRepr> for Grid {
    type Error = GridError;

    fn try_from(repr: GridRepr) -> Result<Self, GridError> {
        let mut legend = BTreeMap::new();
        for (key, kind) in repr.legend {
            let mut chars = key.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => legend.insert(c, kind),
                _ => return Err(GridError::BadLegendKey(key)),
            };
        }
        Grid::new(&legend, &repr.rows)
    }
}

impl From<Grid> for GridRepr {
    fn from(grid: Grid) -> Self {
        let rows = grid.rows();
        let legend = grid
            .glyphs
            .into_iter()
            .map(|g| g.to_string())
            .zip(grid.kinds)
            .collect();
        GridRepr { legend, rows }
    }
}

/// Light, weather and mood of a map.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ambience {
    #[serde(default)]
    pub time: TimeOfDay,
    #[serde(default)]
    pub weather: Weather,
    /// Base light, overriding the time of day (interiors, caves, space).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light: Option<LightLevel>,
    /// How far anyone sees, in cells (thick fog, smoke, sandstorm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sight_limit: Option<u32>,
    /// Mood id: music cue and colour grade.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind: Option<Wind>,
}

impl Ambience {
    /// Light everywhere before light sources are added.
    pub fn base_light(&self) -> LightLevel {
        self.light.unwrap_or(match self.time {
            TimeOfDay::Day => LightLevel::Bright,
            TimeOfDay::Dawn | TimeOfDay::Dusk => LightLevel::Dim,
            TimeOfDay::Night => LightLevel::Dark,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDay {
    Dawn,
    #[default]
    Day,
    Dusk,
    Night,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weather {
    #[default]
    Clear,
    Cloudy,
    Rain,
    Storm,
    Fog,
    Snow,
    Sandstorm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightLevel {
    Dark,
    Dim,
    Bright,
}

/// Where the wind blows from. Sailing and vehicle rules will read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wind {
    pub from: Direction,
    /// 0 (calm) to 5 (storm).
    pub strength: u8,
}

/// An image or prompt behind the grid. Decor only, never rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backdrop {
    /// Generation prompt; may describe secrets, so never sent to players.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// A named group of placed things and who receives it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visibility: Visibility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Everyone: GM, players, shared screen. Part of the rules.
    All,
    /// The GM only, until revealed. Part of the rules (a hidden trap,
    /// a secret door still exist).
    Gm,
    /// What players see in place of the truth (an illusory wall, a
    /// decoy). Shown to the GM as an overlay, never used by the rules.
    Players,
}

/// Who a map is projected for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Viewer {
    Gm,
    Player,
}

impl Visibility {
    pub fn shown_to(self, viewer: Viewer) -> bool {
        match (self, viewer) {
            (Visibility::Gm, Viewer::Player) => false,
            (Visibility::All | Visibility::Players, Viewer::Player) => true,
            (_, Viewer::Gm) => true,
        }
    }

    /// Whether the rules engine takes this layer into account.
    pub fn is_real(self) -> bool {
        self != Visibility::Players
    }
}

pub(crate) fn default_layers() -> Vec<Layer> {
    vec![Layer {
        id: BASE_LAYER.into(),
        name: "Carte".into(),
        visibility: Visibility::All,
    }]
}

fn base_layer() -> String {
    BASE_LAYER.into()
}

fn is_base_layer(l: &str) -> bool {
    l == BASE_LAYER
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(n: &i8) -> bool {
    *n == 0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorState {
    Open,
    Closed,
    /// Closed, and opening needs a key or a check.
    Locked,
}

/// A door on a wall cell. Open, the cell is passable and transparent;
/// closed or locked, it blocks both. A secret door is a door on a GM
/// layer: removed from the players' projection, its cell stays a wall.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Door {
    pub id: String,
    pub at: Cell,
    pub state: DoorState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default = "base_layer", skip_serializing_if = "is_base_layer")]
    pub layer: String,
}

/// Cover a target gets, from none to total (out of sight).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Cover {
    #[default]
    None,
    Half,
    ThreeQuarters,
    /// Blocks line of sight.
    Total,
}

impl Cover {
    fn is_none(&self) -> bool {
        *self == Cover::None
    }
}

/// Width and height of a prop, in cells. Written `[w, h]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "[u32; 2]", into = "[u32; 2]")]
pub struct Footprint {
    pub w: u32,
    pub h: u32,
}

impl Default for Footprint {
    fn default() -> Self {
        Self { w: 1, h: 1 }
    }
}

impl From<[u32; 2]> for Footprint {
    fn from([w, h]: [u32; 2]) -> Self {
        Self { w, h }
    }
}

impl From<Footprint> for [u32; 2] {
    fn from(f: Footprint) -> Self {
        [f.w, f.h]
    }
}

fn is_unit(f: &Footprint) -> bool {
    *f == Footprint::default()
}

/// A piece of decor placed on the grid that carries rules: crates,
/// barrels, a mast, a console, a tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prop {
    pub id: String,
    /// Sprite id in the theme pack (`caisses`, `tonneaux`, `console`…).
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Top-left cell of the footprint.
    pub at: Cell,
    #[serde(default, skip_serializing_if = "is_unit")]
    pub size: Footprint,
    /// Cover given to a target behind it; `total` also blocks sight.
    #[serde(default, skip_serializing_if = "Cover::is_none")]
    pub cover: Cover,
    #[serde(default, skip_serializing_if = "is_false")]
    pub blocks_movement: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub difficult: bool,
    /// A ladder, rigging: lets a creature change height beyond a step.
    #[serde(default, skip_serializing_if = "is_false")]
    pub climbable: bool,
    /// Drawn above the tokens (tree canopy, awning). No rule effect.
    #[serde(default, skip_serializing_if = "is_false")]
    pub overhead: bool,
    #[serde(default = "base_layer", skip_serializing_if = "is_base_layer")]
    pub layer: String,
}

impl Prop {
    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        let (w, h) = (self.size.w as i32, self.size.h as i32);
        (0..h).flat_map(move |dy| (0..w).map(move |dx| self.at.offset(dx, dy)))
    }
}

/// Something to find or use: a chest, a trap, a clue, a lever.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapObject {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub at: Cell,
    /// The check that finds it; stat ids come from the rule system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<Check>,
    /// GM notes; stripped for players.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default = "base_layer", skip_serializing_if = "is_base_layer")]
    pub layer: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub stat: String,
    pub dc: u32,
}

/// A light source. Radii in cells.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Light {
    pub id: String,
    pub at: Cell,
    pub bright: u32,
    pub dim: u32,
    /// `#rrggbb`, for rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flicker: bool,
    #[serde(default = "base_layer", skip_serializing_if = "is_base_layer")]
    pub layer: String,
}

/// Cells that lead to another map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exit {
    pub id: String,
    pub cells: Vec<Cell>,
    /// Target map id.
    pub to: String,
    /// Start or exit id on the target map where the party arrives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrive: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default = "base_layer", skip_serializing_if = "is_base_layer")]
    pub layer: String,
}

/// A name written on the map: a place, a zone, a building.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Label {
    pub text: String,
    pub at: Cell,
    /// Story-graph node this place opens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(default = "base_layer", skip_serializing_if = "is_base_layer")]
    pub layer: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Party,
    Foes,
    Neutral,
}

/// A starting position for a token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Start {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
    pub at: Cell,
    /// Entity (character, NPC, adversary) placed there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    #[serde(default = "base_layer", skip_serializing_if = "is_base_layer")]
    pub layer: String,
}

impl Map {
    pub fn geometry(&self) -> Geometry {
        self.scale.geometry()
    }

    pub fn cell_meters(&self) -> f64 {
        self.cell_meters
            .unwrap_or_else(|| self.scale.default_cell_meters())
    }

    /// Distance in cells, by the scale's geometry and the rule system's
    /// diagonal rule.
    pub fn distance(&self, a: Cell, b: Cell, diagonal: Diagonal) -> u32 {
        distance(self.geometry(), a, b, diagonal)
    }

    /// Distance in metres.
    pub fn range_meters(&self, a: Cell, b: Cell, diagonal: Diagonal) -> f64 {
        f64::from(self.distance(a, b, diagonal)) * self.cell_meters()
    }

    /// A length in metres as whole cells (a speed, a weapon range),
    /// rounded to the nearest cell, at least one.
    pub fn meters_to_cells(&self, meters: f64) -> u32 {
        let cells = (meters / self.cell_meters()).round();
        // Lengths in a rule system are small and positive.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let cells = cells.max(1.0) as u32;
        cells
    }

    pub fn layer(&self, id: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// Visibility of a layer; an unknown layer counts as GM-only, so a
    /// typo never leaks.
    pub fn visibility(&self, layer: &str) -> Visibility {
        self.layer(layer).map_or(Visibility::Gm, |l| l.visibility)
    }

    pub fn door(&self, id: &str) -> Option<&Door> {
        self.doors.iter().find(|d| d.id == id)
    }

    /// Opens, closes or locks a door. Returns false if no door has that id.
    pub fn set_door_state(&mut self, id: &str, state: DoorState) -> bool {
        match self.doors.iter_mut().find(|d| d.id == id) {
            Some(door) => {
                door.state = state;
                true
            }
            None => false,
        }
    }

    pub fn object(&self, id: &str) -> Option<&MapObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn prop(&self, id: &str) -> Option<&Prop> {
        self.props.iter().find(|p| p.id == id)
    }
}
