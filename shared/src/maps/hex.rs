//! Hex coordinates for the world scale.
//!
//! Maps store hexes as rows and columns ("pointy top", odd rows shifted
//! half a hex right: odd-r offset). Maths happens in axial coordinates
//! `(q, r)`, where distance and neighbours are simple.

use serde::{Deserialize, Serialize};

use super::grid::Cell;

/// A hex in axial coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Hex {
    pub q: i32,
    pub r: i32,
}

const AXIAL_DIRS: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];

impl Hex {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// From a stored (column, row) cell, odd rows shifted right.
    pub const fn from_offset(c: Cell) -> Self {
        Self::new(c.x - (c.y - (c.y & 1)) / 2, c.y)
    }

    /// Back to a stored (column, row) cell.
    pub const fn to_offset(self) -> Cell {
        Cell::new(self.q + (self.r - (self.r & 1)) / 2, self.r)
    }

    const fn s(self) -> i32 {
        -self.q - self.r
    }

    /// Number of hex steps between two hexes.
    pub fn distance(self, other: Hex) -> u32 {
        let dq = self.q.abs_diff(other.q);
        let dr = self.r.abs_diff(other.r);
        let ds = self.s().abs_diff(other.s());
        dq.max(dr).max(ds)
    }

    /// The six touching hexes.
    pub fn neighbours(self) -> [Hex; 6] {
        AXIAL_DIRS.map(|(dq, dr)| Hex::new(self.q + dq, self.r + dr))
    }

    /// The hexes a straight line from `self` to `other` crosses, both
    /// ends included (cube interpolation, nudged off exact edges).
    pub fn line_to(self, other: Hex) -> Vec<Hex> {
        let n = self.distance(other);
        if n == 0 {
            return vec![self];
        }
        let (aq, ar) = (f64::from(self.q) + 1e-6, f64::from(self.r) + 1e-6);
        let (bq, br) = (f64::from(other.q) + 1e-6, f64::from(other.r) + 1e-6);
        (0..=n)
            .map(|i| {
                let t = f64::from(i) / f64::from(n);
                round_axial(aq + (bq - aq) * t, ar + (br - ar) * t)
            })
            .collect()
    }
}

fn round_axial(q: f64, r: f64) -> Hex {
    let s = -q - r;
    let (mut rq, mut rr, rs) = (q.round(), r.round(), s.round());
    let (dq, dr, ds) = ((rq - q).abs(), (rr - r).abs(), (rs - s).abs());
    if dq > dr && dq > ds {
        rq = -rr - rs;
    } else if dr > ds {
        rr = -rq - rs;
    }
    // Map coordinates are far below i32 limits; the rounding is exact.
    #[allow(clippy::cast_possible_truncation)]
    Hex::new(rq as i32, rr as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn off(x: i32, y: i32) -> Hex {
        Hex::from_offset(Cell::new(x, y))
    }

    #[test]
    fn distance_counts_hex_steps_across_shifted_rows() {
        // Ported from V1 (odd-r offset coordinates).
        assert_eq!(off(0, 0).distance(off(3, 0)), 3);
        assert_eq!(off(0, 0).distance(off(1, 2)), 2);
        assert_eq!(off(2, 4).distance(off(7, 1)), 7);
        assert_eq!(off(7, 1).distance(off(2, 4)), 7);
        // Odd row: (3,3) touches (4,2) but not (2,2).
        assert_eq!(off(3, 3).distance(off(4, 2)), 1);
        assert_eq!(off(3, 3).distance(off(2, 2)), 2);
    }

    #[test]
    fn neighbours_depend_on_row_parity() {
        let even: Vec<Cell> = off(3, 2)
            .neighbours()
            .iter()
            .map(|h| h.to_offset())
            .collect();
        assert_eq!(even.len(), 6);
        assert!(even.contains(&Cell::new(2, 1)));
        assert!(!even.contains(&Cell::new(4, 1)));
        let odd: Vec<Cell> = off(3, 3)
            .neighbours()
            .iter()
            .map(|h| h.to_offset())
            .collect();
        assert!(odd.contains(&Cell::new(4, 2)));
        assert!(!odd.contains(&Cell::new(2, 2)));
        for h in off(3, 3).neighbours() {
            assert_eq!(h.distance(off(3, 3)), 1);
        }
    }

    #[test]
    fn offset_round_trips() {
        for y in -3..6 {
            for x in -3..6 {
                let c = Cell::new(x, y);
                assert_eq!(Hex::from_offset(c).to_offset(), c);
            }
        }
    }

    #[test]
    fn line_steps_one_hex_at_a_time() {
        let line = off(0, 0).line_to(off(5, 3));
        assert_eq!(line.len() as u32, off(0, 0).distance(off(5, 3)) + 1);
        assert_eq!(line.last(), Some(&off(5, 3)));
        for w in line.windows(2) {
            assert_eq!(w[0].distance(w[1]), 1);
        }
    }
}
