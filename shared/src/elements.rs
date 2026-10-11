//! campaign/import-pixellab-assets — the HD-2D elements of a theme,
//! brought in from PixelLab (`MEMORY.md` §2, « the platform renders in
//! HD-2D »).
//!
//! An element is one image the 3D scene draws for one slot of a
//! tileset, named `<tileset>/<slot>`:
//!
//! - `material/<id>` — a floor material, a **texture** seen from the
//!   front, laid on the ground;
//! - `wall` — the walls' face, a **texture** on the wall volumes;
//! - `prop/<id>/side` and `prop/<id>/top` — the faces of a prop that has
//!   volume (crate, barrel, hatch…), **textures** on a box;
//! - `prop/<id>` — a thin or irregular prop (lantern, rope, sign, mast),
//!   one **flat** image turned to the camera;
//!
//! Building facades will come with the scene that draws them: until then
//! no slot names one, so nothing is generated that the GM cannot review.
//!
//! The shape rule: what has volume is geometry textured with images
//! seen from the front (`view: side`), never isometric; what is thin is
//! a billboard. A character (an NPC's or an adversary's) is not a slot
//! of the theme but of the story: its directions and animations.
//!
//! Colours are brought back onto the theme's palette
//! ([`reduce_to_palette`]) so every element of a world reads as one set.

use std::collections::BTreeSet;
use std::io::Cursor;

use serde::Serialize;

use crate::maps::Map;
use crate::theme::{Shape, Tileset};

/// How the scene draws an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    /// Seen from the front, wrapped on a volume or laid on the ground.
    Texture,
    /// A flat image turned to the camera.
    Flat,
    /// A character: four directions, animations.
    Character,
}

impl Form {
    /// PixelLab generations one element of this form costs, as far as we
    /// know (test of 11 October 2026): one image, or for a character its
    /// four directions then the five animations of `hd2d/show-pixellab-
    /// characters` (rest, walk, attack, hit, fall). An estimate announced
    /// to the GM before a batch, never a bill.
    #[must_use]
    pub fn generations(self) -> u32 {
        match self {
            Self::Texture | Self::Flat => 1,
            Self::Character => 6,
        }
    }
}

/// Whether a prop of `shape` has volume (a box with textured faces) or is
/// thin enough to be one flat image.
#[must_use]
pub fn has_volume(shape: Shape) -> bool {
    match shape {
        Shape::Crate
        | Shape::Crates
        | Shape::Barrel
        | Shape::Hatch
        | Shape::Console
        | Shape::Lockers
        | Shape::Reactor
        | Shape::Car
        | Shape::Barricade
        | Shape::Rock => true,
        Shape::Post
        | Shape::Rail
        | Shape::Mast
        | Shape::Rope
        | Shape::Ladder
        | Shape::Pipes
        | Shape::Beacon
        | Shape::Tree => false,
    }
}

/// One slot of a tileset an element fills.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Slot {
    /// `<tileset>/<slot>`, the element's subject.
    pub subject: String,
    /// In French, for the GM.
    pub name: String,
    pub form: Form,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotError {
    /// Not `<tileset>/<slot>`, or not a slot this module names.
    Malformed,
    UnknownTileset(String),
    UnknownMaterial(String),
    UnknownProp(String),
    /// `prop/<id>/<face>` on a thin prop, or `prop/<id>` on a volume.
    WrongForm(String),
}

impl std::fmt::Display for SlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => write!(
                f,
                "an element is `<tileset>/material/<id>`, `<tileset>/wall`, `<tileset>/prop/<id>[/side|/top]`"
            ),
            Self::UnknownTileset(t) => write!(f, "no tileset `{t}` in the campaign's theme"),
            Self::UnknownMaterial(m) => write!(f, "no material `{m}` in this tileset"),
            Self::UnknownProp(p) => write!(f, "no prop `{p}` in this tileset"),
            Self::WrongForm(why) => write!(f, "{why}"),
        }
    }
}

/// The slot `subject` names among `tilesets`, checked.
///
/// # Errors
///
/// What is wrong with it, in English for the log and the MCP server.
pub fn slot(tilesets: &[Tileset], subject: &str) -> Result<Slot, SlotError> {
    let (tileset, rest) = subject.split_once('/').ok_or(SlotError::Malformed)?;
    let t = tilesets
        .iter()
        .find(|t| t.id == tileset)
        .ok_or_else(|| SlotError::UnknownTileset(tileset.to_string()))?;
    let parts: Vec<&str> = rest.split('/').collect();
    let made = |name: String, form| Slot {
        subject: subject.to_string(),
        name,
        form,
    };
    match parts.as_slice() {
        ["wall"] => Ok(made(format!("Murs · {}", t.name), Form::Texture)),
        ["material", id] => t
            .materials
            .get(*id)
            .map(|m| made(format!("Sol · {}", m.name), Form::Texture))
            .ok_or_else(|| SlotError::UnknownMaterial((*id).to_string())),
        ["prop", id] => {
            let p = t
                .props
                .get(*id)
                .ok_or_else(|| SlotError::UnknownProp((*id).to_string()))?;
            if has_volume(p.shape) {
                return Err(SlotError::WrongForm(format!(
                    "`{id}` has volume: send its faces, `prop/{id}/side` and `prop/{id}/top`"
                )));
            }
            Ok(made(p.name.clone(), Form::Flat))
        }
        ["prop", id, face @ ("side" | "top")] => {
            let p = t
                .props
                .get(*id)
                .ok_or_else(|| SlotError::UnknownProp((*id).to_string()))?;
            if !has_volume(p.shape) {
                return Err(SlotError::WrongForm(format!(
                    "`{id}` is thin: send one flat image, `prop/{id}`"
                )));
            }
            let face = if *face == "side" { "côté" } else { "dessus" };
            Ok(made(format!("{} · {face}", p.name), Form::Texture))
        }
        _ => Err(SlotError::Malformed),
    }
}

/// Every slot `map` needs from its tileset: the floor materials its grid
/// uses, the walls when it has some, and each prop or object kind
/// standing on it (a volume by its two faces). Unknown kinds are left to
/// the map's own checks.
#[must_use]
pub fn needs_for_map(tileset: &Tileset, map: &Map) -> Vec<Slot> {
    let mut subjects = BTreeSet::new();
    let mut walls = false;
    for cell in map.grid.cells() {
        let Some(kind) = map.grid.kind(cell) else {
            continue;
        };
        if kind.void {
            continue;
        }
        if kind.wall {
            walls = true;
        } else if tileset.materials.contains_key(&kind.terrain) {
            subjects.insert(format!("{}/material/{}", tileset.id, kind.terrain));
        }
    }
    if walls {
        subjects.insert(format!("{}/wall", tileset.id));
    }
    let kinds = map
        .props
        .iter()
        .map(|p| &p.kind)
        .chain(map.objects.iter().map(|o| &o.kind));
    for kind in kinds {
        subjects.extend(prop_subjects(tileset, kind));
    }
    let tilesets = std::slice::from_ref(tileset);
    subjects
        .iter()
        .filter_map(|s| slot(tilesets, s).ok())
        .collect()
}

fn prop_subjects(tileset: &Tileset, kind: &str) -> Vec<String> {
    match tileset.props.get(kind) {
        Some(p) if has_volume(p.shape) => vec![
            format!("{}/prop/{kind}/side", tileset.id),
            format!("{}/prop/{kind}/top", tileset.id),
        ],
        Some(_) => vec![format!("{}/prop/{kind}", tileset.id)],
        None => Vec::new(),
    }
}

/// Every slot of `tileset`: its materials, its walls, its props.
#[must_use]
pub fn every_slot(tileset: &Tileset) -> Vec<Slot> {
    let mut subjects = vec![format!("{}/wall", tileset.id)];
    subjects.extend(
        tileset
            .materials
            .keys()
            .map(|m| format!("{}/material/{m}", tileset.id)),
    );
    for kind in tileset.props.keys() {
        subjects.extend(prop_subjects(tileset, kind));
    }
    let tilesets = std::slice::from_ref(tileset);
    subjects
        .iter()
        .filter_map(|s| slot(tilesets, s).ok())
        .collect()
}

/// The tileset's colours, `#rrggbb`, without repeats: the void, the
/// walls, every material's three tones, every prop's two.
#[must_use]
pub fn palette(tileset: &Tileset) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |c: &String| {
        let c = c.to_lowercase();
        if parse_hex(&c).is_some() && !out.contains(&c) {
            out.push(c);
        }
    };
    add(&tileset.void);
    add(&tileset.walls.cap);
    add(&tileset.walls.face);
    add(&tileset.walls.joint);
    for m in tileset.materials.values() {
        add(&m.base);
        add(&m.dark);
        add(&m.light);
    }
    for p in tileset.props.values() {
        add(&p.color);
        add(&p.accent);
    }
    out
}

fn parse_hex(c: &str) -> Option<[u8; 3]> {
    let h = c.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    // Each byte of a 24-bit value.
    #[allow(clippy::cast_possible_truncation)]
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    NotPng,
    TooLarge { width: u32, height: u32 },
    Encode,
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPng => write!(f, "not a readable PNG image"),
            Self::TooLarge { width, height } => write!(
                f,
                "{width}×{height} is too large (at most {MAX_SIDE}×{MAX_SIDE})"
            ),
            Self::Encode => write!(f, "the image could not be written back"),
        }
    }
}

/// The largest side of an element, in pixels: PixelLab draws 16 to 400.
pub const MAX_SIDE: u32 = 1024;

/// An RGBA8 image read from PNG bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Read PNG `bytes` as RGBA8, whatever their colour type.
///
/// # Errors
///
/// `NotPng`, `TooLarge`.
pub fn read_png(bytes: &[u8]) -> Result<Rgba, ImageError> {
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: 64 * 1024 * 1024,
        },
    );
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| ImageError::NotPng)?;
    let (width, height) = {
        let info = reader.info();
        (info.width, info.height)
    };
    if width > MAX_SIDE || height > MAX_SIDE || width == 0 || height == 0 {
        return Err(ImageError::TooLarge { width, height });
    }
    let mut buf = vec![0; reader.output_buffer_size().ok_or(ImageError::NotPng)?];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|_| ImageError::NotPng)?;
    let raw = &buf[..info.buffer_size()];
    let pixels: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => raw.to_vec(),
        png::ColorType::Rgb => raw
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => raw
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => raw.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err(ImageError::NotPng),
    };
    Ok(Rgba {
        width,
        height,
        pixels,
    })
}

/// Write `image` as PNG bytes.
///
/// # Errors
///
/// `Encode`.
pub fn write_png(image: &Rgba) -> Result<Vec<u8>, ImageError> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| ImageError::Encode)?;
        writer
            .write_image_data(&image.pixels)
            .map_err(|_| ImageError::Encode)?;
    }
    Ok(out)
}

/// Bring every visible pixel of PNG `bytes` onto the nearest colour of
/// `palette` (`#rrggbb`); transparent pixels stay, alpha is kept. The
/// distance weighs green most, as the eye does. An empty palette leaves
/// the colours. Returns PNG bytes.
///
/// # Errors
///
/// As [`read_png`] and [`write_png`].
pub fn reduce_to_palette(bytes: &[u8], palette: &[String]) -> Result<Vec<u8>, ImageError> {
    let mut image = read_png(bytes)?;
    let colours: Vec<[u8; 3]> = palette.iter().filter_map(|c| parse_hex(c)).collect();
    if !colours.is_empty() {
        for px in image.pixels.chunks_exact_mut(4) {
            if px[3] == 0 {
                continue;
            }
            let nearest = colours
                .iter()
                .min_by_key(|c| distance([px[0], px[1], px[2]], **c))
                .copied()
                .unwrap_or([px[0], px[1], px[2]]);
            px[..3].copy_from_slice(&nearest);
        }
    }
    write_png(&image)
}

fn distance(a: [u8; 3], b: [u8; 3]) -> u32 {
    let d = |i: usize| u32::from(a[i].abs_diff(b[i]));
    2 * d(0) * d(0) + 4 * d(1) * d(1) + 3 * d(2) * d(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    fn corsaires() -> Theme {
        Theme::from_yaml(include_str!("../../content/themes/corsaires.yaml")).unwrap()
    }

    #[test]
    fn a_slot_is_checked_against_the_theme_and_its_shape() {
        let t = corsaires();
        let ts = &t.tilesets;
        assert_eq!(slot(ts, "port-1718/wall").unwrap().form, Form::Texture);
        let floor = slot(ts, "port-1718/material/pavés").unwrap();
        assert_eq!(
            (floor.form, floor.name.as_str()),
            (Form::Texture, "Sol · Pavés")
        );
        // A crate has volume: its faces, never one flat image.
        assert_eq!(
            slot(ts, "port-1718/prop/caisse/side").unwrap().form,
            Form::Texture
        );
        assert!(matches!(
            slot(ts, "port-1718/prop/caisse"),
            Err(SlotError::WrongForm(_))
        ));
        // A rope is thin: one flat image.
        assert_eq!(
            slot(ts, "port-1718/prop/cordages").unwrap().form,
            Form::Flat
        );
        assert!(matches!(
            slot(ts, "port-1718/prop/cordages/top"),
            Err(SlotError::WrongForm(_))
        ));
        assert_eq!(
            slot(ts, "port-1718/facade/taverne"),
            Err(SlotError::Malformed)
        );
        assert_eq!(
            slot(ts, "ailleurs/wall"),
            Err(SlotError::UnknownTileset("ailleurs".into()))
        );
        assert_eq!(
            slot(ts, "port-1718/material/lave"),
            Err(SlotError::UnknownMaterial("lave".into()))
        );
        assert_eq!(slot(ts, "port-1718/porte"), Err(SlotError::Malformed));
    }

    #[test]
    fn a_map_needs_its_floors_its_walls_and_what_stands_on_it() {
        let t = corsaires();
        let map = Map::from_yaml(include_str!(
            "../../content/maps/corsaires/quai-port-louis.yaml"
        ))
        .unwrap();
        let tileset = t.tileset(&map.theme).unwrap();
        let needs = needs_for_map(tileset, &map);
        let subjects: Vec<&str> = needs.iter().map(|s| s.subject.as_str()).collect();
        assert!(
            subjects.contains(&"port-1718/material/pavés"),
            "{subjects:?}"
        );
        assert!(subjects.iter().all(|s| s.starts_with("port-1718/")));
        // Each prop kind once (a volume by its two faces), never twice.
        let mut sorted = subjects.clone();
        sorted.dedup();
        assert_eq!(sorted, subjects);
        assert!(every_slot(tileset).len() >= needs.len());
    }

    #[test]
    fn colours_come_back_onto_the_palette_and_transparency_stays() {
        let image = Rgba {
            width: 2,
            height: 1,
            pixels: vec![250, 10, 10, 255, 1, 2, 3, 0],
        };
        let bytes = write_png(&image).unwrap();
        let out = reduce_to_palette(&bytes, &["#000000".into(), "#e2564a".into()]).unwrap();
        let back = read_png(&out).unwrap();
        assert_eq!(back.pixels, vec![0xe2, 0x56, 0x4a, 255, 1, 2, 3, 0]);
        assert_eq!(read_png(b"not a png"), Err(ImageError::NotPng));
        let p = palette(corsaires().tileset("port-1718").unwrap());
        assert!(p.contains(&"#5d5850".to_string()));
        assert!(p.iter().all(|c| parse_hex(c).is_some()));
    }
}
