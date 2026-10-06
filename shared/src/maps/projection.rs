//! The one place a map is cut down for a viewer (`MEMORY.md` §3).
//!
//! Players never receive a GM layer, nor anything placed on one (hidden
//! objects, secret doors, ambushers' starts), nor GM notes, the checks
//! that find objects, or the backdrop's generation prompt. A secret
//! door's cell is a wall in the grid, so removing the door leaves no gap.
//! Fog of war (unrevealed cells) is a session state, cut on top of this
//! by [`Map::fogged`] (`maps/reveal-fog-and-hidden`).

use std::collections::{BTreeMap, BTreeSet};

use super::grid::Cell;
use super::model::{CellKind, Grid, Map, Viewer};

/// The material of a cell under the fog: drawn as the fog pattern.
pub const FOG_TERRAIN: &str = "brouillard";

/// Glyphs tried for the fog, in order: the first one the legend does not
/// use.
const FOG_GLYPHS: [char; 6] = ['?', '░', '▒', '¤', '§', '¶'];

impl Map {
    /// The map as `viewer` may receive it.
    pub fn project(&self, viewer: Viewer) -> Map {
        if viewer == Viewer::Gm {
            return self.clone();
        }
        let shown = |layer: &str| self.visibility(layer).shown_to(viewer);
        let mut map = self.clone();
        map.gm_notes = None;
        map.layers.retain(|l| l.visibility.shown_to(viewer));
        map.doors.retain(|d| shown(&d.layer));
        map.props.retain(|p| shown(&p.layer));
        map.objects.retain(|o| shown(&o.layer));
        for o in &mut map.objects {
            o.check = None;
            o.notes = None;
        }
        map.lights.retain(|l| shown(&l.layer));
        map.exits.retain(|e| shown(&e.layer));
        map.labels.retain(|l| shown(&l.layer));
        map.starts.retain(|s| shown(&s.layer));
        if let Some(backdrop) = &mut map.backdrop {
            backdrop.prompt = None;
        }
        map
    }
}

impl Map {
    /// The map with every cell not in `revealed` under the fog: its glyph
    /// becomes the fog (void, `FOG_TERRAIN`), and nothing placed there —
    /// door, prop, object, light, exit, label, start — is kept. Legend
    /// entries no revealed cell uses are dropped too, so the legend
    /// itself names nothing unseen. Cut after [`Map::project`].
    #[must_use]
    pub fn fogged(&self, revealed: &BTreeSet<Cell>) -> Map {
        let seen = |c: Cell| revealed.contains(&c);
        let used: BTreeSet<char> = self.grid.legend().map(|(g, _)| g).collect();
        let fog = FOG_GLYPHS
            .iter()
            .copied()
            .find(|g| !used.contains(g))
            .unwrap_or('?');
        let mut legend: BTreeMap<char, CellKind> = BTreeMap::new();
        let rows: Vec<String> = self
            .grid
            .rows()
            .iter()
            .enumerate()
            .map(|(y, row)| {
                row.chars()
                    .enumerate()
                    .map(|(x, g)| {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                        let c = Cell::new(x as i32, y as i32);
                        if seen(c) {
                            if let Some(kind) = self.grid.kind(c) {
                                legend.entry(g).or_insert_with(|| kind.clone());
                            }
                            g
                        } else {
                            legend.entry(fog).or_insert_with(|| CellKind {
                                terrain: FOG_TERRAIN.to_string(),
                                void: true,
                                ..CellKind::default()
                            });
                            fog
                        }
                    })
                    .collect()
            })
            .collect();
        let mut map = self.clone();
        // Same size, a subset of the legend plus one glyph: always valid.
        if let Ok(grid) = Grid::new(&legend, &rows) {
            map.grid = grid;
        }
        map.doors.retain(|d| seen(d.at));
        map.props.retain(|p| p.cells().any(seen));
        map.objects.retain(|o| seen(o.at));
        map.lights.retain(|l| seen(l.at));
        map.exits.retain(|e| e.cells.iter().any(|c| seen(*c)));
        for e in &mut map.exits {
            e.cells.retain(|c| seen(*c));
        }
        map.labels.retain(|l| seen(l.at));
        map.starts.retain(|s| seen(s.at));
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUAI: &str = include_str!("../../../content/maps/corsaires/quai-port-louis.yaml");

    #[test]
    fn nothing_of_a_fogged_cell_remains() {
        let map = Map::from_yaml(QUAI).unwrap().project(Viewer::Player);
        // The west end of the quay only.
        let revealed: BTreeSet<Cell> = map
            .grid
            .cells()
            .filter(|c| c.x < 4 && (4..=9).contains(&c.y))
            .collect();
        let fogged = map.fogged(&revealed);
        assert_eq!(fogged.grid.width(), map.grid.width());
        for c in fogged.grid.cells() {
            let kind = fogged.grid.kind(c).unwrap();
            if revealed.contains(&c) {
                assert_eq!(kind, map.grid.kind(c).unwrap());
            } else {
                assert_eq!(kind.terrain, FOG_TERRAIN, "{c:?}");
            }
        }
        // The deck, the water and the planks are under the fog: their
        // materials are gone from the legend.
        let terrains: Vec<&str> = fogged
            .grid
            .legend()
            .map(|(_, k)| k.terrain.as_str())
            .collect();
        assert!(!terrains.contains(&"pont"), "{terrains:?}");
        assert!(!terrains.contains(&"eau"), "{terrains:?}");
        assert!(
            fogged
                .props
                .iter()
                .all(|p| p.cells().any(|c| revealed.contains(&c)))
        );
        assert!(fogged.labels.iter().all(|l| revealed.contains(&l.at)));
        assert!(fogged.lights.iter().all(|l| revealed.contains(&l.at)));
        assert!(!fogged.doors.iter().any(|d| d.id == "porte-chantier"));
        let json = serde_json::to_string(&fogged).unwrap();
        assert!(!json.contains("Mâchoire"), "{json}");
        assert!(!json.contains("Tonneaux"), "{json}");
        // Still a valid map.
        let back: Map = serde_json::from_str(&json).unwrap();
        assert_eq!(back, fogged);
    }
}
