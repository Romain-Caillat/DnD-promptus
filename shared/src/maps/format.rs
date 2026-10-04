//! Reading, writing and checking map files (YAML or JSON, same shape).
//!
//! A map is stored as JSON in Postgres and authored as YAML; both go
//! through `Map::validate`, so a map that loads is a map the rules
//! engine can use.

use std::collections::BTreeSet;

use super::grid::{Cell, Geometry};
use super::model::{FORMAT_VERSION, Map, Water};

#[derive(Debug, thiserror::Error)]
pub enum MapError {
    #[error("map YAML: {0}")]
    Yaml(#[from] serde_norway::Error),
    #[error("map JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("map grid: {0}")]
    Grid(#[from] super::model::GridError),
    #[error("invalid map: {}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))]
    Invalid(Vec<Issue>),
}

/// One problem found in a map. Messages are for developers and logs;
/// screens translate the variant.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Issue {
    #[error("format version {0} is not supported (expected {FORMAT_VERSION})")]
    UnsupportedVersion(u32),
    #[error("cell size must be a positive number of metres")]
    BadCellSize,
    #[error("id {0:?} is used twice")]
    DuplicateId(String),
    #[error("layer {0:?} is declared twice")]
    DuplicateLayer(String),
    #[error("{item:?} is on layer {layer:?}, which is not declared")]
    UnknownLayer { item: String, layer: String },
    #[error("{item:?} lies outside the grid at {cell:?}")]
    OutOfBounds { item: String, cell: Cell },
    #[error("door {door:?} at {cell:?} is not on a wall cell")]
    DoorNotOnWall { door: String, cell: Cell },
    #[error("start {start:?} at {cell:?} is not a cell a token can stand on")]
    StartNotWalkable { start: String, cell: Cell },
    #[error("light {0:?}: the dim radius must be at least the bright one")]
    LightRadius(String),
    #[error("exit {0:?} has no cell")]
    EmptyExit(String),
    #[error("prop {0:?} has an empty footprint")]
    EmptyProp(String),
    #[error("doors only exist on square grids ({0:?})")]
    DoorOnHex(String),
}

impl Map {
    pub fn from_yaml(text: &str) -> Result<Map, MapError> {
        let map: Map = serde_norway::from_str(text)?;
        map.validate()?;
        Ok(map)
    }

    pub fn from_json(text: &str) -> Result<Map, MapError> {
        let map: Map = serde_json::from_str(text)?;
        map.validate()?;
        Ok(map)
    }

    pub fn to_yaml(&self) -> Result<String, MapError> {
        Ok(serde_norway::to_string(self)?)
    }

    pub fn to_json(&self) -> Result<String, MapError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Checks everything the types cannot: references, bounds, and that
    /// rules-bearing data is coherent.
    pub fn validate(&self) -> Result<(), MapError> {
        let issues = self.issues();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(MapError::Invalid(issues))
        }
    }

    pub fn issues(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        if self.version != FORMAT_VERSION {
            issues.push(Issue::UnsupportedVersion(self.version));
        }
        if self
            .cell_meters
            .is_some_and(|m| !(m.is_finite() && m > 0.0))
        {
            issues.push(Issue::BadCellSize);
        }

        let mut layers = BTreeSet::new();
        for layer in &self.layers {
            if !layers.insert(layer.id.as_str()) {
                issues.push(Issue::DuplicateLayer(layer.id.clone()));
            }
        }

        let mut ids = BTreeSet::new();
        let mut placed = |id: &str, layer: &str, cells: &[Cell], issues: &mut Vec<Issue>| {
            if !ids.insert(id.to_owned()) {
                issues.push(Issue::DuplicateId(id.to_owned()));
            }
            if !layers.contains(layer) {
                issues.push(Issue::UnknownLayer {
                    item: id.into(),
                    layer: layer.into(),
                });
            }
            if let Some(&cell) = cells.iter().find(|&&c| !self.grid.contains(c)) {
                issues.push(Issue::OutOfBounds {
                    item: id.into(),
                    cell,
                });
            }
        };

        for d in &self.doors {
            placed(&d.id, &d.layer, &[d.at], &mut issues);
            if self.geometry() == Geometry::Hex {
                issues.push(Issue::DoorOnHex(d.id.clone()));
            } else if self.grid.kind(d.at).is_some_and(|k| !k.wall) {
                issues.push(Issue::DoorNotOnWall {
                    door: d.id.clone(),
                    cell: d.at,
                });
            }
        }
        for p in &self.props {
            if p.size.w == 0 || p.size.h == 0 {
                issues.push(Issue::EmptyProp(p.id.clone()));
            }
            let cells: Vec<Cell> = p.cells().collect();
            placed(&p.id, &p.layer, &cells, &mut issues);
        }
        for o in &self.objects {
            placed(&o.id, &o.layer, &[o.at], &mut issues);
        }
        for l in &self.lights {
            placed(&l.id, &l.layer, &[l.at], &mut issues);
            if l.dim < l.bright {
                issues.push(Issue::LightRadius(l.id.clone()));
            }
        }
        for e in &self.exits {
            placed(&e.id, &e.layer, &e.cells, &mut issues);
            if e.cells.is_empty() {
                issues.push(Issue::EmptyExit(e.id.clone()));
            }
        }
        for (i, l) in self.labels.iter().enumerate() {
            // Labels have no id; name them by position for messages.
            let name = format!("label #{i} {:?}", l.text);
            if !layers.contains(l.layer.as_str()) {
                issues.push(Issue::UnknownLayer {
                    item: name.clone(),
                    layer: l.layer.clone(),
                });
            }
            if !self.grid.contains(l.at) {
                issues.push(Issue::OutOfBounds {
                    item: name,
                    cell: l.at,
                });
            }
        }
        for s in &self.starts {
            placed(&s.id, &s.layer, &[s.at], &mut issues);
            if self.grid.contains(s.at) && !self.can_stand(s.at) {
                issues.push(Issue::StartNotWalkable {
                    start: s.id.clone(),
                    cell: s.at,
                });
            }
        }
        issues
    }

    /// Whether a token can stand on a cell, doors and props as they are.
    fn can_stand(&self, c: Cell) -> bool {
        let Some(kind) = self.grid.kind(c) else {
            return false;
        };
        let open_door = self
            .doors
            .iter()
            .any(|d| d.at == c && d.state == super::model::DoorState::Open);
        let walkable = !kind.void && kind.water != Water::Deep && (!kind.wall || open_door);
        let blocked = self
            .props
            .iter()
            .any(|p| p.blocks_movement && p.cells().any(|pc| pc == c));
        walkable && !blocked
    }
}
