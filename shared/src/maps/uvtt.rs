//! maps/import-image-map — the Universal VTT format (`.dd2vtt` from
//! Dungeondraft, `.uvtt`, `.df2vtt`): an image, its grid, its walls as
//! lines, its doors and its lights, read into a map whose image is only
//! the backdrop.
//!
//! The format draws walls on the edges between cells; a Promptus wall is
//! a cell. A wall line is sampled every tenth of a cell and each sample
//! marks the cell it falls in; a sample exactly on an edge marks the cell
//! left of a vertical edge and above a horizontal one (the map's border
//! stays inside the grid). A thin wall between two rooms thus takes one
//! row of cells from one of them: the GM touches it up in the editor.
//! Doors land on the cell of their centre, which becomes a wall; lights
//! on the cell of their centre, their range as the dim radius.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use super::format::MapError;
use super::grid::Cell;
use super::model::{
    Ambience, BASE_LAYER, Backdrop, CellKind, Door, DoorState, FORMAT_VERSION, Grid, Layer, Light,
    MAX_SIDE, Map, Scale, Visibility,
};

#[derive(Debug, Deserialize)]
struct Uvtt {
    resolution: Resolution,
    #[serde(default)]
    line_of_sight: Vec<Vec<Point>>,
    #[serde(default)]
    objects_line_of_sight: Vec<Vec<Point>>,
    #[serde(default)]
    portals: Vec<Portal>,
    #[serde(default)]
    lights: Vec<UvttLight>,
    #[serde(default)]
    image: String,
}

#[derive(Debug, Deserialize)]
struct Resolution {
    map_origin: Point,
    map_size: Point,
    pixels_per_grid: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Deserialize)]
struct Portal {
    position: Point,
    #[serde(default)]
    closed: bool,
}

#[derive(Debug, Deserialize)]
struct UvttLight {
    position: Point,
    range: f64,
    #[serde(default)]
    color: String,
}

/// A map read from a Universal VTT file, and its image (base64, as the
/// file holds it; empty when the file has none).
#[derive(Debug)]
pub struct UvttImport {
    pub map: Map,
    pub image_base64: String,
}

/// The cell a coordinate along one axis falls in, an edge counting for
/// the cell before it; kept within `0..size`.
fn axis_cell(v: f64, size: u32) -> i32 {
    let rounded = v.round();
    let cell = if (v - rounded).abs() < 1e-6 {
        rounded - 1.0
    } else {
        v.floor()
    };
    #[expect(clippy::cast_possible_truncation, reason = "clamped to the grid")]
    let cell = cell.clamp(0.0, f64::from(size) - 1.0) as i32;
    cell
}

fn cell_of(p: Point, w: u32, h: u32) -> Cell {
    Cell::new(axis_cell(p.x, w), axis_cell(p.y, h))
}

/// `ffrrggbb` (alpha first) or `rrggbb` → `#rrggbb`.
fn color(raw: &str) -> Option<String> {
    let hex = raw.trim().trim_start_matches('#');
    let rgb = match hex.len() {
        8 => &hex[2..],
        6 => hex,
        _ => return None,
    };
    rgb.chars()
        .all(|c| c.is_ascii_hexdigit())
        .then(|| format!("#{}", rgb.to_ascii_lowercase()))
}

/// Read a Universal VTT file into map `id`, named `name`, drawn with
/// `theme`, its floor of material `floor`.
///
/// # Errors
///
/// [`MapError::Json`] when the file is not one; [`MapError::Uvtt`] when
/// its grid is empty or too large; [`MapError::Invalid`] if the result
/// does not validate.
pub fn from_uvtt(
    text: &str,
    id: &str,
    name: &str,
    theme: &str,
    floor: &str,
) -> Result<UvttImport, MapError> {
    let file: Uvtt = serde_json::from_str(text)?;
    let r = &file.resolution;
    let size = |v: f64| -> Result<u32, MapError> {
        if !(v.is_finite() && v >= 1.0 && v <= f64::from(MAX_SIDE)) {
            return Err(MapError::Uvtt(format!(
                "la carte doit faire de 1 à {MAX_SIDE} cases de côté"
            )));
        }
        #[expect(clippy::cast_possible_truncation, reason = "checked above")]
        #[expect(clippy::cast_sign_loss, reason = "checked above")]
        Ok(v.round() as u32)
    };
    let (w, h) = (size(r.map_size.x)?, size(r.map_size.y)?);
    if !(r.pixels_per_grid.is_finite() && r.pixels_per_grid > 0.0) {
        return Err(MapError::Uvtt("taille de case en pixels invalide".into()));
    }
    let local = |p: Point| Point {
        x: p.x - r.map_origin.x,
        y: p.y - r.map_origin.y,
    };

    let mut walls = BTreeSet::new();
    for line in file.line_of_sight.iter().chain(&file.objects_line_of_sight) {
        for pair in line.windows(2) {
            let (a, b) = (local(pair[0]), local(pair[1]));
            let length = (b.x - a.x).hypot(b.y - a.y);
            #[expect(clippy::cast_possible_truncation, reason = "a wall is short")]
            #[expect(clippy::cast_sign_loss, reason = "a length is positive")]
            let samples = ((length * 10.0).ceil() as u32).max(1);
            for i in 0..samples {
                let t = (f64::from(i) + 0.5) / f64::from(samples);
                let p = Point {
                    x: a.x + (b.x - a.x) * t,
                    y: a.y + (b.y - a.y) * t,
                };
                walls.insert(cell_of(p, w, h));
            }
        }
    }

    let mut doors = Vec::new();
    let mut door_cells = BTreeSet::new();
    for p in &file.portals {
        let at = cell_of(local(p.position), w, h);
        if !door_cells.insert(at) {
            continue;
        }
        walls.insert(at);
        doors.push(Door {
            id: format!("porte-{}", doors.len() + 1),
            at,
            state: if p.closed {
                DoorState::Closed
            } else {
                DoorState::Open
            },
            label: None,
            layer: BASE_LAYER.into(),
        });
    }

    let lights = file
        .lights
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let p = local(l.position);
            #[expect(clippy::cast_possible_truncation, reason = "clamped to the grid")]
            let at = Cell::new(
                p.x.floor().clamp(0.0, f64::from(w - 1)) as i32,
                p.y.floor().clamp(0.0, f64::from(h - 1)) as i32,
            );
            #[expect(clippy::cast_possible_truncation, reason = "a light's range is small")]
            #[expect(clippy::cast_sign_loss, reason = "clamped to be positive")]
            let dim = l.range.ceil().clamp(1.0, f64::from(MAX_SIDE)) as u32;
            Light {
                id: format!("lumiere-{}", i + 1),
                at,
                bright: dim.div_ceil(2),
                dim,
                color: color(&l.color),
                flicker: false,
                layer: BASE_LAYER.into(),
            }
        })
        .collect();

    let ground = CellKind {
        terrain: floor.to_string(),
        wall: false,
        water: super::model::Water::None,
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
    #[expect(clippy::cast_possible_wrap, reason = "at most MAX_SIDE")]
    let rows: Vec<String> = (0..h as i32)
        .map(|y| {
            (0..w as i32)
                .map(|x| {
                    if walls.contains(&Cell::new(x, y)) {
                        '#'
                    } else {
                        '.'
                    }
                })
                .collect()
        })
        .collect();

    let map = Map {
        version: FORMAT_VERSION,
        id: id.to_string(),
        name: name.to_string(),
        scale: Scale::Encounter,
        cell_meters: None,
        theme: theme.to_string(),
        ambience: Ambience::default(),
        backdrop: Some(Backdrop {
            prompt: None,
            image: (!file.image.is_empty()).then(|| "import".to_string()),
            cell_px: Some(r.pixels_per_grid),
            offset: Some([0.0, 0.0]),
        }),
        gm_notes: None,
        layers: vec![
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
        ],
        grid: Grid::new(&legend, &rows)?,
        doors,
        props: vec![],
        objects: vec![],
        lights,
        exits: vec![],
        labels: vec![],
        starts: vec![],
    };
    map.validate()?;
    Ok(UvttImport {
        map,
        image_base64: file.image,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{line_of_sight, visible_cells};

    /// A 5 × 3 room split by a wall at x = 2 with a door in it, drawn
    /// with an origin that is not zero, as Dungeondraft writes it.
    const ROOMS: &str = r#"{
        "format": 0.3,
        "resolution": {
            "map_origin": { "x": 10, "y": 20 },
            "map_size": { "x": 5, "y": 3 },
            "pixels_per_grid": 256
        },
        "line_of_sight": [
            [{ "x": 12, "y": 20 }, { "x": 12, "y": 21 }],
            [{ "x": 12, "y": 22 }, { "x": 12, "y": 23 }]
        ],
        "objects_line_of_sight": [],
        "portals": [{
            "position": { "x": 12, "y": 21.5 },
            "bounds": [{ "x": 12, "y": 21 }, { "x": 12, "y": 22 }],
            "rotation": 1.57, "closed": true, "freestanding": false
        }],
        "lights": [{
            "position": { "x": 13.5, "y": 21.5 }, "range": 3.2,
            "intensity": 1, "color": "ffeccd8b", "shadows": true
        }],
        "environment": { "baked_lighting": true, "ambient_light": "ffffffff" },
        "image": "iVBORw0KGgo="
    }"#;

    #[test]
    fn walls_doors_and_lights_land_on_cells_and_the_image_is_only_a_backdrop() {
        let import = from_uvtt(ROOMS, "caves", "Les caves", "port-1718", "pavés").unwrap();
        let map = import.map;
        assert_eq!(map.grid.rows(), vec![".#...", ".#...", ".#..."]);
        assert_eq!(map.doors.len(), 1);
        assert_eq!(map.doors[0].at, Cell::new(1, 1));
        assert_eq!(map.doors[0].state, DoorState::Closed);
        let light = &map.lights[0];
        assert_eq!((light.at, light.bright, light.dim), (Cell::new(3, 1), 2, 4));
        assert_eq!(light.color.as_deref(), Some("#eccd8b"));
        let backdrop = map.backdrop.as_ref().unwrap();
        assert_eq!(backdrop.cell_px, Some(256.0));
        assert_eq!(import.image_base64, "iVBORw0KGgo=");

        // The closed door keeps the rooms apart; the rules read the grid.
        let bodies = std::collections::HashSet::new();
        assert!(!line_of_sight(&map, Cell::new(0, 1), Cell::new(3, 1), &bodies).visible);
        assert!(line_of_sight(&map, Cell::new(2, 0), Cell::new(4, 2), &bodies).visible);
        assert!(!visible_cells(&map, Cell::new(0, 1), None).contains(&Cell::new(3, 1)));
    }

    #[test]
    fn a_file_without_a_usable_grid_is_refused() {
        let empty = ROOMS.replace(
            r#""map_size": { "x": 5, "y": 3 }"#,
            r#""map_size": { "x": 0, "y": 3 }"#,
        );
        assert!(matches!(
            from_uvtt(&empty, "a", "A", "t", "f"),
            Err(MapError::Uvtt(_))
        ));
        assert!(matches!(
            from_uvtt("pas du json", "a", "A", "t", "f"),
            Err(MapError::Json(_))
        ));
        assert_eq!(color("ff00aa11").as_deref(), Some("#00aa11"));
        assert_eq!(color("zz"), None);
    }
}
