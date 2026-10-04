//! Line of sight, cover, what a viewpoint sees, and light.
//!
//! Method: a straight segment from the centre of the viewer's cell to
//! the centre of the target's cell, walked cell by cell (supercover, in
//! exact integer arithmetic). Only the cells strictly between the two
//! ends matter.
//! - Sight is blocked by an opaque cell on the way: a wall, a closed or
//!   locked door, a prop with total cover, or a cell higher than both
//!   ends (a ridge, a cliff top, a raised deck).
//! - When the segment passes exactly through a grid corner, it squeezes
//!   between two cells: blocked if both are opaque, half cover if one is
//!   (peeking round a wall's corner).
//! - Cover is the best of: props on the way (half or three-quarters),
//!   creatures on the way (half), grazed corners (half). Obstacles on a
//!   cell touching the viewer are ignored: one leans over one's own
//!   cover, so a crate shields whoever hides behind it, not whoever
//!   shoots from behind it.
//!
//! Hex maps use the same rules on a cube-interpolated line, without
//! corner grazing.

use std::collections::{BTreeSet, HashSet};

use super::board::{Board, Info};
use super::grid::{Cell, Diagonal, Geometry, distance};
use super::hex::Hex;
use super::model::{Cover, LightLevel, Map};

/// Result of a line of sight check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sight {
    pub visible: bool,
    /// `Total` exactly when not visible.
    pub cover: Cover,
}

impl Sight {
    const BLOCKED: Sight = Sight {
        visible: false,
        cover: Cover::Total,
    };
}

/// One cell the segment crosses, or a corner it squeezes through.
enum Crossing {
    Cell(Cell),
    Corner(Cell, Cell),
}

/// Cells and corners crossed between `a` and `b`, ends excluded.
fn crossings(geometry: Geometry, a: Cell, b: Cell) -> Vec<Crossing> {
    match geometry {
        Geometry::Hex => {
            let line = Hex::from_offset(a).line_to(Hex::from_offset(b));
            let n = line.len();
            line.into_iter()
                .skip(1)
                .take(n.saturating_sub(2))
                .map(|h| Crossing::Cell(h.to_offset()))
                .collect()
        }
        Geometry::Square => {
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let (nx, ny) = (i64::from(dx.unsigned_abs()), i64::from(dy.unsigned_abs()));
            let (sx, sy) = (dx.signum(), dy.signum());
            let (mut ix, mut iy) = (0i64, 0i64);
            let mut p = a;
            let mut out = Vec::new();
            while ix < nx || iy < ny {
                // Compare when the segment next crosses a vertical and a
                // horizontal grid line: equal means through a corner.
                let decision = (1 + 2 * ix) * ny - (1 + 2 * iy) * nx;
                if decision == 0 {
                    out.push(Crossing::Corner(p.offset(sx, 0), p.offset(0, sy)));
                    p = p.offset(sx, sy);
                    ix += 1;
                    iy += 1;
                } else if decision < 0 {
                    p = p.offset(sx, 0);
                    ix += 1;
                } else {
                    p = p.offset(0, sy);
                    iy += 1;
                }
                if p != b {
                    out.push(Crossing::Cell(p));
                }
            }
            out
        }
    }
}

struct Viewpoint<'a> {
    board: Board<'a>,
}

impl Viewpoint<'_> {
    fn sight(&self, from: Cell, to: Cell, bodies: &HashSet<Cell>) -> Sight {
        let map = self.board.map;
        if !map.grid.contains(from) || !map.grid.contains(to) {
            return Sight::BLOCKED;
        }
        let elevation = |c: Cell| self.board.get(c).map_or(0, |i| i.elevation);
        let ridge = elevation(from).max(elevation(to));
        let opaque = |i: &Info| i.opaque || i.elevation > ridge;
        let near_viewer = |c: Cell| distance(map.geometry(), from, c, Diagonal::Chebyshev) <= 1;
        let mut cover = Cover::None;
        for crossing in crossings(map.geometry(), from, to) {
            match crossing {
                Crossing::Cell(c) => {
                    let Some(info) = self.board.get(c) else {
                        return Sight::BLOCKED;
                    };
                    if opaque(info) {
                        return Sight::BLOCKED;
                    }
                    if !near_viewer(c) {
                        cover = cover.max(info.cover);
                        if bodies.contains(&c) {
                            cover = cover.max(Cover::Half);
                        }
                    }
                }
                Crossing::Corner(a, b) => {
                    let blocks = |c: Cell| self.board.get(c).is_none_or(opaque);
                    match (blocks(a), blocks(b)) {
                        (true, true) => return Sight::BLOCKED,
                        (true, false) if !near_viewer(a) => cover = cover.max(Cover::Half),
                        (false, true) if !near_viewer(b) => cover = cover.max(Cover::Half),
                        _ => {}
                    }
                }
            }
        }
        Sight {
            visible: true,
            cover,
        }
    }
}

/// Whether `from` sees `to`, and the cover `to` gets. `bodies` are the
/// cells where creatures stand (they give half cover); the two ends are
/// ignored if listed.
pub fn line_of_sight(map: &Map, from: Cell, to: Cell, bodies: &HashSet<Cell>) -> Sight {
    Viewpoint {
        board: Board::new(map),
    }
    .sight(from, to, bodies)
}

/// Every cell `from` sees, itself included, up to `limit` cells away
/// (pass `map.ambience.sight_limit` or a darkvision range). Opaque cells
/// that bound the view (wall faces) are included. Fog of war reveals
/// from this.
pub fn visible_cells(map: &Map, from: Cell, limit: Option<u32>) -> BTreeSet<Cell> {
    let view = Viewpoint {
        board: Board::new(map),
    };
    let none = HashSet::new();
    map.grid
        .cells()
        .filter(|&c| {
            limit.is_none_or(|l| distance(map.geometry(), from, c, Diagonal::Chebyshev) <= l)
                && view.sight(from, c, &none).visible
        })
        .collect()
}

/// Light on a cell: the ambience's base light, raised by every light
/// source (of a real layer) that reaches the cell in its radius with a
/// clear line.
pub fn illumination(map: &Map, cell: Cell) -> LightLevel {
    let view = Viewpoint {
        board: Board::new(map),
    };
    let none = HashSet::new();
    let mut level = map.ambience.base_light();
    for light in &map.lights {
        if !map.visibility(&light.layer).is_real() {
            continue;
        }
        let d = distance(map.geometry(), light.at, cell, Diagonal::Chebyshev);
        if d > light.dim || (d > 0 && !view.sight(light.at, cell, &none).visible) {
            continue;
        }
        let lit = if d <= light.bright {
            LightLevel::Bright
        } else {
            LightLevel::Dim
        };
        level = level.max(lit);
    }
    level
}
