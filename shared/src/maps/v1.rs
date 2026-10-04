//! Loader for V1 maps (the `maps` array of a V1 campaign story, JSON).
//!
//! V1 stored sparse cells (`terrain`, `blocked`, `label`, `sceneId`,
//! `childMapId`) over a default terrain, on three levels. Conversion:
//! - `campaign` → world scale; `region` (500 m hexes) → world scale with
//!   `cell_meters: 500`, since it is hex-gridded; `local` → encounter.
//! - A blocked cell becomes deep water when its terrain is water
//!   (rivière, eau, lac, mer); on a square map, a blocked cell with a
//!   label (a throne, a campfire) becomes a blocking prop on plain
//!   floor; anything else blocked becomes a wall of that terrain.
//! - A cell with `childMapId` becomes an exit; other labels stay labels.
//! - Starting tokens become starts with no side (V1 had none).

use std::collections::BTreeMap;

use serde::Deserialize;

use super::format::MapError;
use super::grid::Cell;
use super::model::{
    Ambience, BASE_LAYER, Backdrop, CellKind, Exit, FORMAT_VERSION, Grid, Label, Map, Prop, Scale,
    Start, Water, default_layers,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V1Map {
    pub id: String,
    pub name: String,
    pub level: V1Level,
    pub grid: V1Grid,
    #[serde(default)]
    pub background_prompt: Option<String>,
    #[serde(default)]
    pub background_url: Option<String>,
    #[serde(default)]
    pub cells: Vec<V1Cell>,
    #[serde(default)]
    pub tokens: Vec<V1Token>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum V1Level {
    Campaign,
    Region,
    Local,
}

#[derive(Debug, Deserialize)]
pub struct V1Grid {
    #[serde(rename = "type")]
    pub kind: V1GridType,
    pub cols: u32,
    pub rows: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum V1GridType {
    Hex,
    Square,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V1Cell {
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub terrain: Option<String>,
    #[serde(default)]
    pub blocked: bool,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub child_map_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V1Token {
    pub entity_id: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Deserialize)]
struct V1Story {
    maps: Vec<V1Map>,
}

/// Loads every map of a V1 story JSON (`{"maps": [...]}`).
pub fn load_v1_story_maps(json: &str) -> Result<Vec<Map>, MapError> {
    let story: V1Story = serde_json::from_str(json)?;
    story.maps.into_iter().map(V1Map::into_map).collect()
}

fn is_water(terrain: &str) -> bool {
    ["rivière", "riviere", "eau", "lac", "mer"]
        .iter()
        .any(|w| terrain.contains(w))
}

const GLYPHS: &str = "#~abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

impl V1Map {
    pub fn into_map(self) -> Result<Map, MapError> {
        let square = self.grid.kind == V1GridType::Square;
        let (scale, cell_meters) = match self.level {
            V1Level::Campaign => (Scale::World, None),
            V1Level::Region => (Scale::World, Some(500.0)),
            V1Level::Local => (Scale::Encounter, None),
        };
        let floor = CellKind {
            terrain: if square { "sol" } else { "plaine" }.into(),
            ..CellKind::default()
        };

        let mut kinds: Vec<CellKind> = vec![floor.clone()];
        let mut at: BTreeMap<Cell, usize> = BTreeMap::new();
        let mut props = Vec::new();
        let mut exits = Vec::new();
        let mut labels = Vec::new();
        for c in &self.cells {
            let cell = Cell::new(c.x, c.y);
            let as_prop = c.blocked
                && square
                && c.label.is_some()
                && c.terrain.as_deref().is_some_and(|t| !is_water(t));
            let kind = match (c.blocked, c.terrain.clone()) {
                (true, Some(t)) if is_water(&t) => Some(CellKind {
                    terrain: t,
                    water: Water::Deep,
                    ..CellKind::default()
                }),
                (true, Some(t)) if as_prop => {
                    props.push(Prop {
                        id: format!("decor-{}-{}", c.x, c.y),
                        kind: t,
                        label: c.label.clone(),
                        at: cell,
                        size: Default::default(),
                        cover: Default::default(),
                        blocks_movement: true,
                        difficult: false,
                        climbable: false,
                        overhead: false,
                        layer: BASE_LAYER.into(),
                    });
                    None
                }
                (true, t) => Some(CellKind {
                    terrain: t.unwrap_or_else(|| "mur".into()),
                    wall: true,
                    ..CellKind::default()
                }),
                (false, Some(t)) => Some(CellKind {
                    terrain: t,
                    ..CellKind::default()
                }),
                (false, None) => None,
            };
            if let Some(kind) = kind {
                let idx = kinds.iter().position(|k| *k == kind).unwrap_or_else(|| {
                    kinds.push(kind);
                    kinds.len() - 1
                });
                at.insert(cell, idx);
            }
            if let Some(child) = &c.child_map_id {
                exits.push(Exit {
                    id: format!("sortie-{}-{}", c.x, c.y),
                    cells: vec![cell],
                    to: child.clone(),
                    arrive: None,
                    label: c.label.clone(),
                    layer: BASE_LAYER.into(),
                });
            } else if let Some(text) = c.label.clone().filter(|_| !as_prop) {
                labels.push(Label {
                    text,
                    at: cell,
                    scene: c.scene_id.clone(),
                    layer: BASE_LAYER.into(),
                });
            }
        }

        let glyphs: Vec<char> = std::iter::once('.').chain(GLYPHS.chars()).collect();
        let legend: BTreeMap<char, CellKind> = kinds
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, k)| (glyphs[i], k))
            .collect();
        let rows: Vec<String> = (0..self.grid.rows as i32)
            .map(|y| {
                (0..self.grid.cols as i32)
                    .map(|x| glyphs[at.get(&Cell::new(x, y)).copied().unwrap_or(0)])
                    .collect()
            })
            .collect();
        let grid = Grid::new(&legend, &rows)?;

        let starts = self
            .tokens
            .into_iter()
            .enumerate()
            .map(|(i, t)| Start {
                id: format!("depart-{}", i + 1),
                side: None,
                at: Cell::new(t.x, t.y),
                entity: Some(t.entity_id),
                layer: BASE_LAYER.into(),
            })
            .collect();
        let backdrop = (self.background_prompt.is_some() || self.background_url.is_some())
            .then_some(Backdrop {
                prompt: self.background_prompt,
                image: self.background_url,
            });

        let map = Map {
            version: FORMAT_VERSION,
            id: self.id,
            name: self.name,
            scale,
            cell_meters,
            theme: "v1-demo".into(),
            ambience: Ambience::default(),
            backdrop,
            gm_notes: None,
            layers: default_layers(),
            grid,
            doors: vec![],
            props,
            objects: vec![],
            lights: vec![],
            exits,
            labels,
            starts,
        };
        map.validate()?;
        Ok(map)
    }
}
