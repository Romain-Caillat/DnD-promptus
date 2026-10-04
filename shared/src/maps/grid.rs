//! Grid geometry: cell coordinates, neighbours and distances.
//!
//! Square grids (place and encounter scales) use 8 neighbours. Hex grids
//! (world scale) are stored as rows too, "pointy top, odd rows shifted
//! right" (odd-r offset), and converted to axial coordinates for maths.

use serde::{Deserialize, Serialize};

use super::hex::Hex;

/// One cell of a grid, by column `x` and row `y` (row 0 at the top).
/// Written `[x, y]` in map files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "[i32; 2]", into = "[i32; 2]")]
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

impl Cell {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn offset(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }
}

impl From<[i32; 2]> for Cell {
    fn from([x, y]: [i32; 2]) -> Self {
        Self::new(x, y)
    }
}

impl From<Cell> for [i32; 2] {
    fn from(c: Cell) -> Self {
        [c.x, c.y]
    }
}

impl From<(i32, i32)> for Cell {
    fn from((x, y): (i32, i32)) -> Self {
        Self::new(x, y)
    }
}

/// Shape of the cells, fixed by the map's scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Geometry {
    Square,
    Hex,
}

/// How a diagonal step counts on a square grid. Chosen by the rule
/// system; both give whole numbers of cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Diagonal {
    /// Every diagonal costs one cell (D&D 5e default).
    #[default]
    Chebyshev,
    /// Diagonals alternate 1, 2, 1, 2… (the "5-10-5" variant).
    Alternate,
}

/// The eight compass directions, north being row 0. Used for wind
/// today; vehicle facing and firing arcs will reuse it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
    Nw,
}

impl Direction {
    pub const ALL: [Direction; 8] = [
        Direction::N,
        Direction::Ne,
        Direction::E,
        Direction::Se,
        Direction::S,
        Direction::Sw,
        Direction::W,
        Direction::Nw,
    ];

    /// Unit step on a square grid.
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Direction::N => (0, -1),
            Direction::Ne => (1, -1),
            Direction::E => (1, 0),
            Direction::Se => (1, 1),
            Direction::S => (0, 1),
            Direction::Sw => (-1, 1),
            Direction::W => (-1, 0),
            Direction::Nw => (-1, -1),
        }
    }
}

/// Distance in cells between two cells.
pub fn distance(geometry: Geometry, a: Cell, b: Cell, diagonal: Diagonal) -> u32 {
    match geometry {
        Geometry::Hex => Hex::from_offset(a).distance(Hex::from_offset(b)),
        Geometry::Square => {
            let dx = a.x.abs_diff(b.x);
            let dy = a.y.abs_diff(b.y);
            let diag = dx.min(dy);
            let straight = dx.max(dy) - diag;
            match diagonal {
                Diagonal::Chebyshev => straight + diag,
                Diagonal::Alternate => straight + diag + diag / 2,
            }
        }
    }
}

/// Every neighbour of a cell, bounds not checked.
pub fn neighbours(geometry: Geometry, c: Cell) -> Vec<Cell> {
    match geometry {
        Geometry::Square => Direction::ALL
            .iter()
            .map(|d| {
                let (dx, dy) = d.delta();
                c.offset(dx, dy)
            })
            .collect(),
        Geometry::Hex => Hex::from_offset(c)
            .neighbours()
            .iter()
            .map(|h| h.to_offset())
            .collect(),
    }
}

/// Whether two cells touch.
pub fn adjacent(geometry: Geometry, a: Cell, b: Cell) -> bool {
    a != b && distance(geometry, a, b, Diagonal::Chebyshev) == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_rules_differ_only_on_diagonals() {
        let a = Cell::new(0, 0);
        let b = Cell::new(4, 4);
        assert_eq!(distance(Geometry::Square, a, b, Diagonal::Chebyshev), 4);
        assert_eq!(distance(Geometry::Square, a, b, Diagonal::Alternate), 6);
        // 3 straight + 2 diagonal: alternate adds one for the second diagonal.
        let c = Cell::new(5, 2);
        assert_eq!(distance(Geometry::Square, a, c, Diagonal::Chebyshev), 5);
        assert_eq!(distance(Geometry::Square, a, c, Diagonal::Alternate), 6);
        assert_eq!(distance(Geometry::Square, c, a, Diagonal::Alternate), 6);
    }

    #[test]
    fn square_cells_have_eight_neighbours() {
        let n = neighbours(Geometry::Square, Cell::new(3, 3));
        assert_eq!(n.len(), 8);
        assert!(n.contains(&Cell::new(2, 2)) && n.contains(&Cell::new(4, 4)));
        assert!(!n.contains(&Cell::new(3, 3)));
    }
}
