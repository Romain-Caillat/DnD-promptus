//! Where a ship points and what it can shoot at: the bow's facing (four
//! quarters, as the source turns « d'un quart de tour »), the four arcs
//! around a ship, and the distance between two ships.
//!
//! The arcs on a square grid. A target is placed in the ship's own frame
//! — `ahead` cells towards the bow, `right` cells to starboard — and:
//! - **front** when `ahead > 0` and `ahead >= |right|`;
//! - **rear** when `ahead < 0` and `-ahead >= |right|`;
//! - **starboard** (`right > 0`) or **port** (`right < 0`) otherwise.
//!
//! So the two diagonals ahead belong to the front arc and the two behind
//! to the rear: INTERPRETATION — the source draws four arcs around a
//! cross and says nothing of the corners. A broadside (port/starboard)
//! therefore covers strictly less than half the sea on each side, and a
//! bow chaser covers its diagonals.

use serde::{Deserialize, Serialize};

use crate::maps::{Cell, Direction};

/// The bow's direction, north being row 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Facing {
    N,
    E,
    S,
    W,
}

impl Facing {
    pub const ALL: [Facing; 4] = [Facing::N, Facing::E, Facing::S, Facing::W];

    /// Unit step ahead.
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Facing::N => (0, -1),
            Facing::E => (1, 0),
            Facing::S => (0, 1),
            Facing::W => (-1, 0),
        }
    }

    const fn index(self) -> i32 {
        match self {
            Facing::N => 0,
            Facing::E => 1,
            Facing::S => 2,
            Facing::W => 3,
        }
    }

    /// Quarter turns between two facings (0, 1 or 2).
    pub fn quarters_to(self, other: Facing) -> u32 {
        let d = (other.index() - self.index()).rem_euclid(4);
        d.min(4 - d) as u32
    }

    /// How the bow lies to a wind (or pull) blowing *from* `from`: 1 when
    /// it points where the wind goes (even at 45°), -1 into it, 0 across.
    pub fn to_current(self, from: Direction) -> i32 {
        let (fx, fy) = from.delta();
        let (bx, by) = self.delta();
        // The wind goes the other way from where it blows from.
        (-(fx * bx + fy * by)).signum()
    }

    pub const fn opposite(self) -> Facing {
        match self {
            Facing::N => Facing::S,
            Facing::E => Facing::W,
            Facing::S => Facing::N,
            Facing::W => Facing::E,
        }
    }
}

/// The four arcs around a ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arc {
    Front,
    Rear,
    Port,
    Starboard,
}

/// `to` in the frame of a ship at `from` whose bow points to `facing`:
/// (ahead, right).
pub fn relative(from: Cell, facing: Facing, to: Cell) -> (i32, i32) {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    match facing {
        Facing::N => (-dy, dx),
        Facing::E => (dx, dy),
        Facing::S => (dy, -dx),
        Facing::W => (-dx, -dy),
    }
}

/// The arc of a ship at `from`, bow to `facing`, that `to` lies in;
/// `None` for its own cell.
pub fn arc_of(from: Cell, facing: Facing, to: Cell) -> Option<Arc> {
    let (ahead, right) = relative(from, facing, to);
    if ahead == 0 && right == 0 {
        return None;
    }
    Some(if ahead > 0 && ahead >= right.abs() {
        Arc::Front
    } else if ahead < 0 && -ahead >= right.abs() {
        Arc::Rear
    } else if right > 0 {
        Arc::Starboard
    } else {
        Arc::Port
    })
}

/// Distance between two ships in cells: every diagonal counts one, as
/// the source counts « cases entre les deux vaisseaux ».
pub fn distance(a: Cell, b: Cell) -> u32 {
    (a.x - b.x).unsigned_abs().max((a.y - b.y).unsigned_abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arcs_follow_the_bow() {
        let at = Cell::new(5, 5);
        // Bow north: ahead is row 4, starboard is column 6.
        assert_eq!(arc_of(at, Facing::N, Cell::new(5, 2)), Some(Arc::Front));
        assert_eq!(arc_of(at, Facing::N, Cell::new(5, 8)), Some(Arc::Rear));
        assert_eq!(arc_of(at, Facing::N, Cell::new(8, 5)), Some(Arc::Starboard));
        assert_eq!(arc_of(at, Facing::N, Cell::new(2, 5)), Some(Arc::Port));
        // Turned to the east, the same cells change arcs.
        assert_eq!(arc_of(at, Facing::E, Cell::new(8, 5)), Some(Arc::Front));
        assert_eq!(arc_of(at, Facing::E, Cell::new(5, 2)), Some(Arc::Port));
        assert_eq!(arc_of(at, Facing::E, Cell::new(5, 8)), Some(Arc::Starboard));
        assert_eq!(arc_of(at, Facing::W, Cell::new(5, 2)), Some(Arc::Starboard));
        assert_eq!(arc_of(at, Facing::S, Cell::new(2, 5)), Some(Arc::Starboard));
        assert_eq!(arc_of(at, at_facing(), at), None);
    }

    fn at_facing() -> Facing {
        Facing::N
    }

    #[test]
    fn diagonals_belong_to_bow_and_stern() {
        let at = Cell::new(5, 5);
        assert_eq!(arc_of(at, Facing::N, Cell::new(7, 3)), Some(Arc::Front));
        assert_eq!(arc_of(at, Facing::N, Cell::new(3, 3)), Some(Arc::Front));
        assert_eq!(arc_of(at, Facing::N, Cell::new(7, 7)), Some(Arc::Rear));
        assert_eq!(arc_of(at, Facing::N, Cell::new(3, 7)), Some(Arc::Rear));
        // One step off the diagonal is the broadside.
        assert_eq!(arc_of(at, Facing::N, Cell::new(8, 3)), Some(Arc::Starboard));
        assert_eq!(arc_of(at, Facing::N, Cell::new(2, 7)), Some(Arc::Port));
    }

    #[test]
    fn quarter_turns() {
        assert_eq!(Facing::N.quarters_to(Facing::E), 1);
        assert_eq!(Facing::N.quarters_to(Facing::W), 1);
        assert_eq!(Facing::N.quarters_to(Facing::S), 2);
        assert_eq!(Facing::W.quarters_to(Facing::W), 0);
        // A westerly pushes east.
        assert_eq!(Facing::E.to_current(Direction::W), 1);
        assert_eq!(Facing::W.to_current(Direction::W), -1);
        assert_eq!(Facing::N.to_current(Direction::W), 0);
        assert_eq!(Facing::E.to_current(Direction::Nw), 1);
    }

    #[test]
    fn distance_counts_diagonals_as_one() {
        assert_eq!(distance(Cell::new(0, 0), Cell::new(3, 2)), 3);
        assert_eq!(distance(Cell::new(4, 4), Cell::new(4, 4)), 0);
        assert_eq!(distance(Cell::new(1, 9), Cell::new(7, 2)), 7);
    }
}
