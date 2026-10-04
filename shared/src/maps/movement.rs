//! Movement: what a step costs, which cells a creature can reach, and
//! whether a path sent by a client is legal. The server calls these;
//! the client only highlights what they would accept.
//!
//! Rules, in order, for one step from `a` to `b`:
//! - `b` is on the map and touches `a` (8 neighbours on squares, 6 on hexes);
//! - `b` holds no wall, void, closed or locked door, or blocking prop;
//! - `b` holds no enemy (allies can be crossed but not stopped on);
//! - deep water needs the rule system's swimming factor;
//! - height changes of more than `max_step` levels need something
//!   climbable (ladder, rigging) on `a` or `b`;
//! - a diagonal step may not cut a corner: both orthogonal cells it
//!   squeezes between must be free of walls, closed doors, void and
//!   blocking props (creatures and water do not count);
//! - cost = base × terrain factor + climb, where base is 1, or 2 for
//!   every second diagonal under the alternate rule; the factor is
//!   `difficult_factor` on difficult terrain (shallow water included),
//!   `swim_factor` in deep water; climbing up costs `climb_cost` per level.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashSet};

use serde::{Deserialize, Serialize};

use super::board::{Board, Obstacle};
use super::grid::{Cell, Diagonal, Geometry, adjacent};
use super::model::Map;

/// Movement parameters. They belong to the campaign's rule system,
/// which fills them; the defaults are the D&D 5e ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MovementRules {
    pub diagonal: Diagonal,
    /// Cost multiplier on difficult terrain.
    pub difficult_factor: u32,
    /// Extra cost per level climbed.
    pub climb_cost: u32,
    /// Largest height change walked without climbing gear.
    pub max_step: u8,
    /// Cost multiplier in deep water; `None` makes it impassable.
    pub swim_factor: Option<u32>,
}

impl Default for MovementRules {
    fn default() -> Self {
        Self {
            diagonal: Diagonal::Chebyshev,
            difficult_factor: 2,
            climb_cost: 1,
            max_step: 1,
            swim_factor: None,
        }
    }
}

/// Creatures on the map, from the mover's point of view.
#[derive(Clone, Debug, Default)]
pub struct Occupancy {
    /// Cells the mover cannot enter (enemies).
    pub enemies: HashSet<Cell>,
    /// Cells the mover can cross but not stop on (allies).
    pub allies: HashSet<Cell>,
}

/// Why a step or a path is refused. Screens translate the variant.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error("step {step}: {cell:?} does not touch the previous cell")]
    NotAdjacent { step: usize, cell: Cell },
    #[error("step {step}: {cell:?} is blocked by {by:?}")]
    Blocked {
        step: usize,
        cell: Cell,
        by: Obstacle,
    },
    #[error("step {step}: {cell:?} is occupied by an enemy")]
    Occupied { step: usize, cell: Cell },
    #[error("step {step}: the diagonal to {cell:?} cuts a corner")]
    CornerCut { step: usize, cell: Cell },
    #[error("step {step}: {cell:?} is too high or too low to step to")]
    TooSteep { step: usize, cell: Cell },
    #[error("the path ends on {0:?}, where someone already stands")]
    EndsOnOccupied(Cell),
    #[error("the path costs {cost}, more than the {budget} left")]
    OverBudget { cost: u32, budget: u32 },
}

/// Why one step is refused, before it is tied to a step number.
enum Refusal {
    NotAdjacent,
    Blocked(Obstacle),
    Occupied,
    CornerCut,
    TooSteep,
}

impl Refusal {
    fn at(self, step: usize, cell: Cell) -> PathError {
        match self {
            Refusal::NotAdjacent => PathError::NotAdjacent { step, cell },
            Refusal::Blocked(by) => PathError::Blocked { step, cell, by },
            Refusal::Occupied => PathError::Occupied { step, cell },
            Refusal::CornerCut => PathError::CornerCut { step, cell },
            Refusal::TooSteep => PathError::TooSteep { step, cell },
        }
    }
}

struct Mover<'a> {
    board: Board<'a>,
    rules: MovementRules,
    occupancy: &'a Occupancy,
}

impl Mover<'_> {
    fn geometry(&self) -> Geometry {
        self.board.map.geometry()
    }

    /// Cost of one step and the diagonal parity after it.
    fn step(&self, from: Cell, to: Cell, parity: bool) -> Result<(u32, bool), Refusal> {
        if !adjacent(self.geometry(), from, to) {
            return Err(Refusal::NotAdjacent);
        }
        let Some(dest) = self.board.get(to) else {
            return Err(Refusal::Blocked(Obstacle::OutOfMap));
        };
        if let Some(hard) = dest.hard {
            return Err(Refusal::Blocked(self.board.obstacle(hard)));
        }
        if self.occupancy.enemies.contains(&to) {
            return Err(Refusal::Occupied);
        }
        if dest.deep && self.rules.swim_factor.is_none() {
            return Err(Refusal::Blocked(Obstacle::DeepWater));
        }
        let src = self.board.get(from).copied().unwrap_or_default();
        let rise = i32::from(dest.elevation) - i32::from(src.elevation);
        if rise.unsigned_abs() > u32::from(self.rules.max_step)
            && !(src.climbable || dest.climbable)
        {
            return Err(Refusal::TooSteep);
        }
        let diagonal = self.geometry() == Geometry::Square && from.x != to.x && from.y != to.y;
        if diagonal {
            let squeezed = [Cell::new(to.x, from.y), Cell::new(from.x, to.y)];
            if squeezed
                .iter()
                .any(|&c| self.board.get(c).is_none_or(|i| i.hard.is_some()))
            {
                return Err(Refusal::CornerCut);
            }
        }
        let alternate = diagonal && self.rules.diagonal == Diagonal::Alternate;
        let base = if alternate && parity { 2 } else { 1 };
        let next_parity = if alternate { !parity } else { parity };
        let factor = if dest.deep {
            self.rules.swim_factor.unwrap_or(1)
        } else if dest.difficult {
            self.rules.difficult_factor
        } else {
            1
        };
        let climb = rise.max(0).unsigned_abs() * self.rules.climb_cost;
        Ok((base * factor + climb, next_parity))
    }
}

/// Every cell a creature at `start` can end its move on with `budget`
/// cells of movement, with the cheapest cost to get there (`start`
/// itself at 0).
pub fn reachable(
    map: &Map,
    rules: &MovementRules,
    occupancy: &Occupancy,
    start: Cell,
    budget: u32,
) -> BTreeMap<Cell, u32> {
    let mover = Mover {
        board: Board::new(map),
        rules: *rules,
        occupancy,
    };
    let grid = &map.grid;
    let Some(start_idx) = grid.index(start) else {
        return BTreeMap::new();
    };
    // State = cell × diagonal parity (only the alternate rule uses it).
    let state = |idx: usize, parity: bool| idx * 2 + usize::from(parity);
    let mut best = vec![u32::MAX; grid.len() * 2];
    let mut heap = BinaryHeap::new();
    best[state(start_idx, false)] = 0;
    heap.push(Reverse((0u32, start_idx, false)));
    while let Some(Reverse((cost, idx, parity))) = heap.pop() {
        if cost > best[state(idx, parity)] {
            continue;
        }
        let here = grid.cell_at(idx);
        for next in super::grid::neighbours(map.geometry(), here) {
            let Some(next_idx) = grid.index(next) else {
                continue;
            };
            let Ok((step, next_parity)) = mover.step(here, next, parity) else {
                continue;
            };
            let total = cost + step;
            if total <= budget && total < best[state(next_idx, next_parity)] {
                best[state(next_idx, next_parity)] = total;
                heap.push(Reverse((total, next_idx, next_parity)));
            }
        }
    }
    let mut out = BTreeMap::new();
    for (s, &cost) in best.iter().enumerate() {
        if cost == u32::MAX {
            continue;
        }
        let cell = grid.cell_at(s / 2);
        if cell != start && occupancy.allies.contains(&cell) {
            continue;
        }
        out.entry(cell)
            .and_modify(|c: &mut u32| *c = (*c).min(cost))
            .or_insert(cost);
    }
    out
}

/// Checks a path sent by a client: `path` lists the cells entered after
/// `start`, in order. Returns its cost, or the first rule it breaks.
pub fn check_path(
    map: &Map,
    rules: &MovementRules,
    occupancy: &Occupancy,
    start: Cell,
    path: &[Cell],
    budget: u32,
) -> Result<u32, PathError> {
    let mover = Mover {
        board: Board::new(map),
        rules: *rules,
        occupancy,
    };
    let mut cost = 0;
    let mut parity = false;
    let mut here = start;
    for (step, &next) in path.iter().enumerate() {
        let (c, p) = mover
            .step(here, next, parity)
            .map_err(|r| r.at(step, next))?;
        cost += c;
        parity = p;
        here = next;
    }
    if here != start && occupancy.allies.contains(&here) {
        return Err(PathError::EndsOnOccupied(here));
    }
    if cost > budget {
        return Err(PathError::OverBudget { cost, budget });
    }
    Ok(cost)
}
