//! Packs: the pieces a character is assembled from
//! (`content/sprites/<pack>/pack.yaml`).
//!
//! A pack is data. Each piece is a stack of layers; a layer is a small
//! grid of characters placed on the sprite grid at a depth, and the
//! pack's legend turns each character into a colour: a fixed colour of
//! the pack, or a slot that the character's description recolours
//! (`skin`, `hair`, and the piece's own `dye` and `accent`). A piece
//! worn on the body may have one variant per body (`per_body`).
//!
//! Frames are kept per direction so `characters/walk-in-four-directions`
//! can add the others; today only `east` (right profile) is drawn, and
//! `west` is its mirror.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use super::SpriteError;
use super::colour::Rgb;

/// Where a piece goes on a character. The order is the paint order of
/// pieces that share a depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    Body,
    Outfit,
    Armour,
    Hair,
    Beard,
    Headwear,
    Accessory,
    Weapon,
}

impl Slot {
    pub const ALL: [Slot; 8] = [
        Slot::Body,
        Slot::Outfit,
        Slot::Armour,
        Slot::Hair,
        Slot::Beard,
        Slot::Headwear,
        Slot::Accessory,
        Slot::Weapon,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Slot::Body => "body",
            Slot::Outfit => "outfit",
            Slot::Armour => "armour",
            Slot::Hair => "hair",
            Slot::Beard => "beard",
            Slot::Headwear => "headwear",
            Slot::Accessory => "accessory",
            Slot::Weapon => "weapon",
        }
    }
}

/// How far back a layer sits, from the cape behind the body to what the
/// hand holds in front of it. Layers are painted back to front.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Depth {
    Back,
    BackArm,
    Legs,
    Feet,
    Torso,
    Armour,
    Belt,
    /// The face: the outline is never drawn over it.
    Head,
    Face,
    Hair,
    Beard,
    Headwear,
    Weapon,
    FrontArm,
    Hand,
    Front,
}

/// The four directions a character can face. Only `East` has frames in
/// the starter packs; `West` falls back to the mirrored `East`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    East,
    West,
    North,
    South,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::East => "east",
            Direction::West => "west",
            Direction::North => "north",
            Direction::South => "south",
        }
    }
}

/// A colour a pack cell is painted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paint {
    Skin,
    Hair,
    Dye,
    Accent,
    Fixed { colour: Rgb, shaded: bool },
}

/// One painted cell of a layer, in sprite-grid coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub x: u32,
    pub y: u32,
    pub paint: Paint,
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub depth: Depth,
    pub cells: Vec<Cell>,
}

/// One direction of a piece: its layers.
pub type Frame = Vec<Layer>;

#[derive(Debug, Clone, Default)]
pub struct Frames {
    pub east: Option<Frame>,
    pub west: Option<Frame>,
    pub north: Option<Frame>,
    pub south: Option<Frame>,
}

impl Frames {
    pub fn get(&self, direction: Direction) -> Option<&Frame> {
        match direction {
            Direction::East => self.east.as_ref(),
            Direction::West => self.west.as_ref(),
            Direction::North => self.north.as_ref(),
            Direction::South => self.south.as_ref(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Piece {
    pub id: String,
    /// What the creator shows, in French.
    pub name: String,
    pub dye: Option<Rgb>,
    pub accent: Option<Rgb>,
    pub frames: Frames,
    /// Variants for a body other than the default one, by body id.
    pub per_body: BTreeMap<String, Frames>,
    pub uses_dye: bool,
    pub uses_accent: bool,
}

impl Piece {
    /// The frames that fit `body`.
    pub fn frames_for(&self, body: &str) -> &Frames {
        self.per_body.get(body).unwrap_or(&self.frames)
    }
}

/// A named colour of a palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Swatch {
    pub id: String,
    pub name: String,
    pub colour: String,
}

/// The palettes a description picks from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palettes {
    pub skin: Vec<Swatch>,
    pub hair: Vec<Swatch>,
    /// Dye and accent of outfits, headwear, armour, accessories.
    pub cloth: Vec<Swatch>,
}

/// Which palette a colour of a description is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteName {
    Skin,
    Hair,
    Cloth,
}

impl PaletteName {
    pub fn as_str(self) -> &'static str {
        match self {
            PaletteName::Skin => "skin",
            PaletteName::Hair => "hair",
            PaletteName::Cloth => "cloth",
        }
    }
}

/// A pack as the character creator lists it ([`Pack::catalogue`]).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalogue {
    pub id: String,
    pub name: String,
    /// Every slot, in paint order, with its pieces in the pack's order.
    pub slots: BTreeMap<Slot, Vec<CataloguePiece>>,
    pub palettes: Palettes,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CataloguePiece {
    pub id: String,
    /// In French, as the creator shows it.
    pub name: String,
    /// Takes a `dye` from the `cloth` palette.
    pub dyed: bool,
    /// Takes an `accent` from the `cloth` palette.
    pub accented: bool,
}

/// A pack, checked and ready to draw from.
#[derive(Debug, Clone)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub outline: Rgb,
    pub palettes: Palettes,
    pieces: BTreeMap<Slot, Vec<Piece>>,
}

impl Pack {
    /// Read and check a pack file.
    ///
    /// # Errors
    ///
    /// `SpriteError::Pack` when the YAML does not parse, or names a
    /// colour, legend character, palette swatch or body that does not
    /// exist, or draws outside the grid.
    pub fn from_yaml(text: &str) -> Result<Self, SpriteError> {
        let file: PackFile =
            serde_yaml_ng::from_str(text).map_err(|e| SpriteError::Pack(e.to_string()))?;
        file.compile()
    }

    /// The pieces of one slot, in the pack's order.
    pub fn pieces(&self, slot: Slot) -> &[Piece] {
        self.pieces.get(&slot).map_or(&[], Vec::as_slice)
    }

    pub fn piece(&self, slot: Slot, id: &str) -> Option<&Piece> {
        self.pieces(slot).iter().find(|p| p.id == id)
    }

    /// What the character creator offers from this pack: each slot's
    /// pieces, whether they take colours, and the palettes. No pixels:
    /// the creator previews through the server's renderer.
    pub fn catalogue(&self) -> Catalogue {
        Catalogue {
            id: self.id.clone(),
            name: self.name.clone(),
            slots: Slot::ALL
                .into_iter()
                .map(|slot| {
                    let pieces = self
                        .pieces(slot)
                        .iter()
                        .map(|p| CataloguePiece {
                            id: p.id.clone(),
                            name: p.name.clone(),
                            dyed: p.uses_dye,
                            accented: p.uses_accent,
                        })
                        .collect();
                    (slot, pieces)
                })
                .collect(),
            palettes: self.palettes.clone(),
        }
    }

    /// A colour of a description: a swatch id of `palette`, or `#RRGGBB`.
    ///
    /// # Errors
    ///
    /// `SpriteError::UnknownColour` for anything else.
    pub fn colour(&self, palette: PaletteName, value: &str) -> Result<Rgb, SpriteError> {
        if value.starts_with('#') {
            return Rgb::parse(value).ok_or_else(|| SpriteError::UnknownColour {
                palette: palette.as_str(),
                value: value.to_string(),
            });
        }
        let swatches = match palette {
            PaletteName::Skin => &self.palettes.skin,
            PaletteName::Hair => &self.palettes.hair,
            PaletteName::Cloth => &self.palettes.cloth,
        };
        swatches
            .iter()
            .find(|s| s.id == value)
            .and_then(|s| Rgb::parse(&s.colour))
            .ok_or_else(|| SpriteError::UnknownColour {
                palette: palette.as_str(),
                value: value.to_string(),
            })
    }
}

// --- The file as written ---------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackFile {
    format: u32,
    id: String,
    name: String,
    grid: [u32; 2],
    outline: String,
    colours: BTreeMap<String, String>,
    #[serde(default)]
    unshaded: Vec<String>,
    palettes: Palettes,
    legend: BTreeMap<char, String>,
    pieces: PiecesFile,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PiecesFile {
    #[serde(default)]
    body: Vec<PieceFile>,
    #[serde(default)]
    outfit: Vec<PieceFile>,
    #[serde(default)]
    armour: Vec<PieceFile>,
    #[serde(default)]
    hair: Vec<PieceFile>,
    #[serde(default)]
    beard: Vec<PieceFile>,
    #[serde(default)]
    headwear: Vec<PieceFile>,
    #[serde(default)]
    accessory: Vec<PieceFile>,
    #[serde(default)]
    weapon: Vec<PieceFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PieceFile {
    id: String,
    name: String,
    #[serde(default)]
    dye: Option<String>,
    #[serde(default)]
    accent: Option<String>,
    frames: FramesFile,
    #[serde(default)]
    per_body: BTreeMap<String, FramesFile>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FramesFile {
    #[serde(default)]
    east: Option<Vec<LayerFile>>,
    #[serde(default)]
    west: Option<Vec<LayerFile>>,
    #[serde(default)]
    north: Option<Vec<LayerFile>>,
    #[serde(default)]
    south: Option<Vec<LayerFile>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LayerFile {
    depth: Depth,
    #[serde(default)]
    at: [u32; 2],
    rows: Vec<String>,
}

/// What every piece's frames are checked against.
struct Context<'a> {
    width: u32,
    height: u32,
    legend: &'a BTreeMap<char, Paint>,
}

impl PackFile {
    fn compile(self) -> Result<Pack, SpriteError> {
        let fail = |m: String| SpriteError::Pack(format!("pack {}: {m}", self.id));
        if self.format != 1 {
            return Err(fail(format!("unknown format {}", self.format)));
        }
        let [width, height] = self.grid;
        if width == 0 || height == 0 {
            return Err(fail("empty grid".into()));
        }
        let outline = Rgb::parse(&self.outline)
            .ok_or_else(|| fail(format!("outline: not a colour: {}", self.outline)))?;

        let mut colours = BTreeMap::new();
        for (name, hex) in &self.colours {
            let rgb = Rgb::parse(hex)
                .ok_or_else(|| fail(format!("colour {name}: not #RRGGBB: {hex}")))?;
            colours.insert(name.as_str(), rgb);
        }
        for name in &self.unshaded {
            if !colours.contains_key(name.as_str()) {
                return Err(fail(format!("unshaded: unknown colour {name}")));
            }
        }
        for (palette, swatches) in [
            ("skin", &self.palettes.skin),
            ("hair", &self.palettes.hair),
            ("cloth", &self.palettes.cloth),
        ] {
            let mut seen = HashSet::new();
            for s in swatches {
                if Rgb::parse(&s.colour).is_none() {
                    return Err(fail(format!("palette {palette}: {} is not #RRGGBB", s.id)));
                }
                if !seen.insert(s.id.as_str()) {
                    return Err(fail(format!("palette {palette}: {} twice", s.id)));
                }
            }
        }

        let mut legend = BTreeMap::new();
        for (&ch, name) in &self.legend {
            if ch == '.' {
                return Err(fail("legend: '.' is the empty cell".into()));
            }
            let paint = match name.as_str() {
                "skin" => Paint::Skin,
                "hair" => Paint::Hair,
                "dye" => Paint::Dye,
                "accent" => Paint::Accent,
                other => Paint::Fixed {
                    colour: *colours
                        .get(other)
                        .ok_or_else(|| fail(format!("legend {ch}: unknown colour {other}")))?,
                    shaded: !self.unshaded.iter().any(|u| u == other),
                },
            };
            legend.insert(ch, paint);
        }
        let ctx = Context {
            width,
            height,
            legend: &legend,
        };

        let mut pack = Pack {
            id: self.id.clone(),
            name: self.name,
            width,
            height,
            outline,
            palettes: self.palettes,
            pieces: BTreeMap::new(),
        };
        let PiecesFile {
            body,
            outfit,
            armour,
            hair,
            beard,
            headwear,
            accessory,
            weapon,
        } = self.pieces;
        if body.is_empty() {
            return Err(fail("no body piece".into()));
        }
        let bodies: Vec<String> = body.iter().map(|p| p.id.clone()).collect();
        for (slot, files) in [
            (Slot::Body, body),
            (Slot::Outfit, outfit),
            (Slot::Armour, armour),
            (Slot::Hair, hair),
            (Slot::Beard, beard),
            (Slot::Headwear, headwear),
            (Slot::Accessory, accessory),
            (Slot::Weapon, weapon),
        ] {
            let mut pieces = Vec::with_capacity(files.len());
            for file in files {
                let where_ = format!("{} {}", slot.as_str(), file.id);
                if pieces.iter().any(|p: &Piece| p.id == file.id) {
                    return Err(fail(format!("{where_}: defined twice")));
                }
                let piece = file
                    .compile(&ctx, &pack, &bodies)
                    .map_err(|m| fail(format!("{where_}: {m}")))?;
                if slot == Slot::Body && (piece.uses_dye || piece.uses_accent) {
                    return Err(fail(format!("{where_}: a body has no dye")));
                }
                pieces.push(piece);
            }
            pack.pieces.insert(slot, pieces);
        }
        Ok(pack)
    }
}

impl PieceFile {
    fn compile(self, ctx: &Context, pack: &Pack, bodies: &[String]) -> Result<Piece, String> {
        let colour = |v: &Option<String>| {
            v.as_deref()
                .map(|v| {
                    pack.colour(PaletteName::Cloth, v)
                        .map_err(|e| e.to_string())
                })
                .transpose()
        };
        let dye = colour(&self.dye)?;
        let accent = colour(&self.accent)?;
        let frames = self.frames.compile(ctx)?;
        if frames.east.is_none() {
            return Err("no east frame".into());
        }
        let mut per_body = BTreeMap::new();
        for (body, f) in self.per_body {
            if !bodies.contains(&body) {
                return Err(format!("per_body: unknown body {body}"));
            }
            let f = f
                .compile(ctx)
                .map_err(|m| format!("per_body {body}: {m}"))?;
            if f.east.is_none() {
                return Err(format!("per_body {body}: no east frame"));
            }
            per_body.insert(body, f);
        }
        let all = std::iter::once(&frames).chain(per_body.values());
        let uses = |wanted: Paint| {
            all.clone().any(|f| {
                [&f.east, &f.west, &f.north, &f.south]
                    .into_iter()
                    .flatten()
                    .flatten()
                    .any(|l| l.cells.iter().any(|c| c.paint == wanted))
            })
        };
        let (uses_dye, uses_accent) = (uses(Paint::Dye), uses(Paint::Accent));
        // A description may leave the colours out: the piece's own are
        // then used, so a piece that is dyed must have them.
        if uses_dye && dye.is_none() {
            return Err("paints with dye but has no default dye".into());
        }
        if uses_accent && accent.is_none() {
            return Err("paints with accent but has no default accent".into());
        }
        Ok(Piece {
            id: self.id,
            name: self.name,
            dye,
            accent,
            frames,
            per_body,
            uses_dye,
            uses_accent,
        })
    }
}

impl FramesFile {
    fn compile(self, ctx: &Context) -> Result<Frames, String> {
        let one = |layers: Option<Vec<LayerFile>>, dir: &str| {
            layers
                .map(|ls| {
                    ls.into_iter()
                        .map(|l| l.compile(ctx).map_err(|m| format!("{dir}: {m}")))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()
        };
        Ok(Frames {
            east: one(self.east, "east")?,
            west: one(self.west, "west")?,
            north: one(self.north, "north")?,
            south: one(self.south, "south")?,
        })
    }
}

impl LayerFile {
    fn compile(self, ctx: &Context) -> Result<Layer, String> {
        let [x0, y0] = self.at;
        let mut cells = Vec::new();
        for (dy, row) in (0u32..).zip(&self.rows) {
            for (dx, ch) in (0u32..).zip(row.chars()) {
                if ch == '.' {
                    continue;
                }
                let paint = *ctx
                    .legend
                    .get(&ch)
                    .ok_or_else(|| format!("character {ch:?} is not in the legend"))?;
                let (x, y) = (x0 + dx, y0 + dy);
                if x >= ctx.width || y >= ctx.height {
                    return Err(format!("cell ({x}, {y}) is outside the grid"));
                }
                cells.push(Cell { x, y, paint });
            }
        }
        Ok(Layer {
            depth: self.depth,
            cells,
        })
    }
}
