//! Drawing a look: layers composited in depth order, palette slots
//! recoloured, three-tone shading, a 1-px outline around the silhouette
//! and a drop shadow, into an RGBA buffer.
//!
//! A look moves through four frames ([`Frame`], board « Sprite »: `repos`
//! and `marche`), made from the same layers by moving cells by depth: the
//! breath lowers everything above the legs by one pixel, a step spreads
//! the legs (profile) or lifts one foot (front and back). [`render_sheet`]
//! lays the four side by side, so a screen animates a character from one
//! image per direction and never draws a pixel of it itself.
//!
//! Every step is integer or bit-exact float work with a fixed order, so
//! the server, the Tauri shell and the tests produce the same pixels.

use super::colour::{DARK, LIGHT, Rgb};
use super::look::{CharacterLook, Worn};
use super::pack::{Depth, Direction, Pack, Paint, PaletteName, Piece, Slot};
use super::{Packs, SpriteError};

/// Empty cells around the grid, so the outline of a piece that touches
/// the edge of the grid is not cut off.
pub const MARGIN: u32 = 1;
/// Opacity of the drop shadow under the feet.
const SHADOW_ALPHA: u8 = 0x60;

/// An RGBA image, row by row, 4 bytes per pixel, unpremultiplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// Encode as PNG. Deterministic: same pixels, same bytes.
    ///
    /// # Panics
    ///
    /// Never for an image built by `render` (the buffer always matches
    /// its size).
    pub fn to_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .expect("writing to a Vec cannot fail");
            writer
                .write_image_data(&self.rgba)
                .expect("the buffer matches the declared size");
        }
        out
    }
}

/// What each cell of the composited grid holds, before shading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Painted {
    colour: Rgb,
    shaded: bool,
}

/// The composited sprite, before it becomes pixels. Kept separate so
/// tests can check where the outline went without guessing from colours
/// (the eye and the outline are the same ink).
#[derive(Debug, Clone)]
pub struct Composite {
    /// Grid size plus the margin on each side.
    pub width: u32,
    pub height: u32,
    cells: Vec<Option<Painted>>,
    /// Cells of the body's head layer: the face.
    pub face: Vec<bool>,
    /// Cells painted with the outline ink.
    pub outline: Vec<bool>,
    outline_colour: Rgb,
    mirrored: bool,
}

/// One frame of a character's motion (characters/walk-in-four-directions).
/// At rest a character alternates `Rest` and `Breath`; walking, `StepA`
/// and `StepB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    Rest,
    /// Everything above the legs one pixel lower.
    Breath,
    /// Profile: legs spread in a stride. Front and back: the left foot up.
    StepA,
    /// Profile: legs together, the body low (the breath). Front and back:
    /// the right foot up.
    StepB,
}

impl Frame {
    /// The frames of a sheet, left to right.
    pub const SHEET: [Frame; 4] = [Frame::Rest, Frame::Breath, Frame::StepA, Frame::StepB];
}

/// Draw a look facing `direction`, at rest.
///
/// # Errors
///
/// `SpriteError::UnknownPack`, `UnknownPiece` or `UnknownColour` when
/// the description names something its pack does not have, and
/// `MissingFrame` when a piece has no frame for that direction.
pub fn render(
    packs: &Packs,
    look: &CharacterLook,
    direction: Direction,
) -> Result<Image, SpriteError> {
    Ok(compose(packs, look, direction)?.to_image())
}

/// The four frames of `Frame::SHEET` side by side, each as wide as one
/// `render`: what the map and the creator animate.
///
/// # Errors
///
/// As `render`.
pub fn render_sheet(
    packs: &Packs,
    look: &CharacterLook,
    direction: Direction,
) -> Result<Image, SpriteError> {
    let frames = Frame::SHEET
        .iter()
        .map(|&f| Ok(compose_frame(packs, look, direction, f)?.to_image()))
        .collect::<Result<Vec<_>, SpriteError>>()?;
    let (w, h) = (frames[0].width, frames[0].height);
    let count = u32::try_from(frames.len()).unwrap_or(1);
    let mut rgba = Vec::with_capacity((w * h * count * 4) as usize);
    for y in 0..h {
        for f in &frames {
            let row = (y * w * 4) as usize;
            rgba.extend_from_slice(&f.rgba[row..row + (w * 4) as usize]);
        }
    }
    Ok(Image {
        width: w * count,
        height: h,
        rgba,
    })
}

/// The composited grid at rest, outline included (see `Composite`).
///
/// # Errors
///
/// As `render`.
pub fn compose(
    packs: &Packs,
    look: &CharacterLook,
    direction: Direction,
) -> Result<Composite, SpriteError> {
    compose_frame(packs, look, direction, Frame::Rest)
}

/// Where the legs are, read from the body: what a step moves.
struct Legs {
    /// Twice the column between the two legs (it may fall between two).
    split2: i64,
    /// The first row of the legs.
    hip: i64,
}

impl Legs {
    fn of(body: &[super::pack::Layer]) -> Self {
        let cells = body
            .iter()
            .filter(|l| l.depth == Depth::Legs)
            .flat_map(|l| &l.cells);
        let (mut lo, mut hi, mut top) = (i64::MAX, i64::MIN, i64::MAX);
        for c in cells {
            lo = lo.min(i64::from(c.x));
            hi = hi.max(i64::from(c.x));
            top = top.min(i64::from(c.y));
        }
        if top == i64::MAX {
            return Self {
                split2: 0,
                hip: i64::MAX,
            };
        }
        Self {
            split2: lo + hi,
            hip: top,
        }
    }

    /// Where a cell of `depth` goes in `frame`, in the drawn direction
    /// (before the west mirror).
    fn move_cell(&self, frame: Frame, profile: bool, depth: Depth, x: i64, y: i64) -> (i64, i64) {
        let leg = matches!(depth, Depth::Legs | Depth::Feet);
        let side = (2 * x - self.split2).signum();
        match (frame, profile) {
            (Frame::Rest, _) => (x, y),
            (Frame::Breath, _) | (Frame::StepB, true) => (x, if leg { y } else { y + 1 }),
            (Frame::StepA, true) if leg && y >= self.hip => (x + side * ((y - self.hip) / 2), y),
            (Frame::StepA, false) if leg && side < 0 => (x, y - 1),
            (Frame::StepB, false) if leg && side > 0 => (x, y - 1),
            _ => (x, y),
        }
    }
}

/// The composited grid of one frame (see `Composite`).
///
/// # Errors
///
/// As `render`.
pub fn compose_frame(
    packs: &Packs,
    look: &CharacterLook,
    direction: Direction,
    frame: Frame,
) -> Result<Composite, SpriteError> {
    let pack = packs
        .get(&look.pack)
        .ok_or_else(|| SpriteError::UnknownPack(look.pack.clone()))?;
    let worn = resolve(pack, look)?;

    // West is drawn as east, then mirrored, unless every piece draws it.
    let (drawn, mirrored) = match direction {
        Direction::West
            if worn
                .iter()
                .any(|w| w.piece.frames_for(&look.body).west.is_none()) =>
        {
            (Direction::East, true)
        }
        d => (d, false),
    };

    // Every layer of every piece, painted back to front: by depth, then
    // by slot, then in the piece's own order.
    let mut layers = Vec::new();
    for (order, w) in worn.iter().enumerate() {
        let frame =
            w.piece
                .frames_for(&look.body)
                .get(drawn)
                .ok_or_else(|| SpriteError::MissingFrame {
                    slot: w.slot.as_str(),
                    piece: w.piece.id.clone(),
                    direction: direction.as_str(),
                })?;
        for (i, layer) in frame.iter().enumerate() {
            layers.push((layer.depth, order, i, w, layer));
        }
    }
    layers.sort_by_key(|&(depth, order, i, _, _)| (depth, order, i));
    // The body is the first piece worn: its legs say what a step moves.
    let legs = worn
        .first()
        .and_then(|b| b.piece.frames_for(&look.body).get(drawn))
        .map_or(
            Legs {
                split2: 0,
                hip: i64::MAX,
            },
            |f| Legs::of(f),
        );
    let profile = matches!(drawn, Direction::East | Direction::West);

    let (gw, gh) = (pack.width, pack.height);
    let (width, height) = (gw + 2 * MARGIN, gh + 2 * MARGIN);
    let mut cells: Vec<Option<Painted>> = vec![None; (width * height) as usize];
    let mut face = vec![false; cells.len()];
    for (depth, _, _, w, layer) in layers {
        for c in &layer.cells {
            let (x, y) = legs.move_cell(frame, profile, depth, i64::from(c.x), i64::from(c.y));
            // A step never leaves the grid on the starter packs; a cell
            // that would is dropped, not wrapped.
            let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
                continue;
            };
            if x >= gw || y >= gh {
                continue;
            }
            let i = ((y + MARGIN) * width + x + MARGIN) as usize;
            cells[i] = Some(w.paint(c.paint));
            if w.slot == Slot::Body && depth == Depth::Head {
                face[i] = true;
            }
        }
    }

    // The outline only ever lands on empty cells next to the silhouette,
    // so it can never cover the face, whatever a beard or a helmet does.
    let filled = |x: i64, y: i64| {
        x >= 0
            && y >= 0
            && x < i64::from(width)
            && y < i64::from(height)
            && cells[(y * i64::from(width) + x) as usize].is_some()
    };
    let mut outline = vec![false; cells.len()];
    for y in 0..i64::from(height) {
        for x in 0..i64::from(width) {
            if !filled(x, y)
                && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|(dx, dy)| filled(x + dx, y + dy))
            {
                outline[(y * i64::from(width) + x) as usize] = true;
            }
        }
    }

    Ok(Composite {
        width,
        height,
        cells,
        face,
        outline,
        outline_colour: pack.outline,
        mirrored,
    })
}

impl Composite {
    fn at(&self, x: i64, y: i64) -> Option<Painted> {
        if x < 0 || y < 0 || x >= i64::from(self.width) || y >= i64::from(self.height) {
            return None;
        }
        self.cells[(y * i64::from(self.width) + x) as usize]
    }

    /// Shade, outline, mirror and cast the shadow.
    pub fn to_image(&self) -> Image {
        let (w, h) = (self.width, self.height);
        let mut px = vec![[0u8; 4]; (w * h) as usize];
        for y in 0..i64::from(h) {
            for x in 0..i64::from(w) {
                let i = (y * i64::from(w) + x) as usize;
                if self.outline[i] {
                    px[i] = opaque(self.outline_colour);
                    continue;
                }
                let Some(c) = self.at(x, y) else { continue };
                px[i] = opaque(if c.shaded {
                    self.shade(x, y, c)
                } else {
                    c.colour
                });
            }
        }
        if self.mirrored {
            for row in px.chunks_mut(w as usize) {
                row.reverse();
            }
        }
        self.cast_shadow(&mut px);
        Image {
            width: w,
            height: h,
            rgba: px.into_iter().flatten().collect(),
        }
    }

    /// The prototype's three tones: lit on a top edge, dark on the back
    /// (left, before mirroring) and bottom edges, base colour inside.
    fn shade(&self, x: i64, y: i64, c: Painted) -> Rgb {
        let above = self.at(x, y - 1);
        let top = above.is_none_or(|a| a.colour != c.colour);
        let back = self.at(x - 1, y).is_none();
        let bottom = self.at(x, y + 1).is_none();
        if top && !bottom {
            c.colour.adjust(LIGHT)
        } else if back || bottom {
            c.colour.adjust(DARK)
        } else {
            c.colour
        }
    }

    /// A flat ellipse under the feet, on transparent pixels only.
    fn cast_shadow(&self, px: &mut [[u8; 4]]) {
        let w = self.width as usize;
        // The lowest painted row is the soles.
        let Some(sole) = (0..self.height as usize)
            .rev()
            .find(|&y| px[y * w..(y + 1) * w].iter().any(|p| p[3] != 0))
        else {
            return;
        };
        let row = &px[sole * w..(sole + 1) * w];
        let first = row.iter().position(|p| p[3] != 0).unwrap_or(0);
        let last = row.iter().rposition(|p| p[3] != 0).unwrap_or(w - 1);
        #[allow(clippy::cast_precision_loss)]
        let (cx, rx) = (
            (first + last + 1) as f64 / 2.0,
            (last - first + 1) as f64 / 2.0 + 1.5,
        );
        #[allow(clippy::cast_precision_loss)]
        let cy = sole as f64 + 0.5;
        let ry = 1.6;
        for y in sole.saturating_sub(1)..self.height as usize {
            for x in 0..w {
                #[allow(clippy::cast_precision_loss)]
                let (dx, dy) = ((x as f64 + 0.5 - cx) / rx, (y as f64 + 0.5 - cy) / ry);
                let p = &mut px[y * w + x];
                if p[3] == 0 && dx * dx + dy * dy <= 1.0 {
                    *p = [0, 0, 0, SHADOW_ALPHA];
                }
            }
        }
    }
}

fn opaque(c: Rgb) -> [u8; 4] {
    [c.0, c.1, c.2, 0xFF]
}

/// A piece of the look, found in its pack, with its colours resolved.
struct Resolved<'a> {
    slot: Slot,
    piece: &'a Piece,
    skin: Rgb,
    hair: Rgb,
    dye: Option<Rgb>,
    accent: Option<Rgb>,
}

impl Resolved<'_> {
    fn paint(&self, paint: Paint) -> Painted {
        let shaded = |colour| Painted {
            colour,
            shaded: true,
        };
        match paint {
            Paint::Skin => shaded(self.skin),
            Paint::Hair => shaded(self.hair),
            // The pack refuses a dyed piece without a default colour.
            Paint::Dye => shaded(self.dye.or(self.piece.dye).unwrap_or(self.skin)),
            Paint::Accent => shaded(self.accent.or(self.piece.accent).unwrap_or(self.skin)),
            Paint::Fixed { colour, shaded } => Painted { colour, shaded },
        }
    }
}

fn resolve<'a>(pack: &'a Pack, look: &CharacterLook) -> Result<Vec<Resolved<'a>>, SpriteError> {
    let skin = pack.colour(PaletteName::Skin, &look.skin)?;
    let hair = pack.colour(PaletteName::Hair, &look.hair.colour)?;
    let find = |slot: Slot, id: &str| {
        pack.piece(slot, id)
            .ok_or_else(|| SpriteError::UnknownPiece {
                pack: pack.id.clone(),
                slot: slot.as_str(),
                piece: id.to_string(),
            })
    };
    let cloth = |v: &Option<String>| {
        v.as_deref()
            .map(|v| pack.colour(PaletteName::Cloth, v))
            .transpose()
    };
    let worn = |slot: Slot, w: &Worn| -> Result<Resolved<'a>, SpriteError> {
        Ok(Resolved {
            slot,
            piece: find(slot, &w.piece)?,
            skin,
            hair,
            dye: cloth(&w.dye)?,
            accent: cloth(&w.accent)?,
        })
    };
    let bare = |slot: Slot, id: &str| -> Result<Resolved<'a>, SpriteError> {
        worn(slot, &Worn::plain(id))
    };

    let mut out = vec![bare(Slot::Body, &look.body)?];
    if let Some(o) = &look.outfit {
        out.push(worn(Slot::Outfit, o)?);
    }
    if let Some(a) = &look.armour {
        out.push(worn(Slot::Armour, a)?);
    }
    if let Some(style) = &look.hair.style {
        out.push(bare(Slot::Hair, style)?);
    }
    if let Some(b) = &look.beard {
        out.push(bare(Slot::Beard, b)?);
    }
    if let Some(h) = &look.headwear {
        out.push(worn(Slot::Headwear, h)?);
    }
    for a in &look.accessories {
        out.push(worn(Slot::Accessory, a)?);
    }
    if let Some(w) = &look.weapon {
        out.push(worn(Slot::Weapon, w)?);
    }
    Ok(out)
}
