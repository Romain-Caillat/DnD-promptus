//! What the grid says about one action: is the target within range, in
//! sight, behind cover, at long range — and which enemies an area
//! catches. Pure geometry over a map and the cells where bodies stand.
//!
//! - Range is counted in cells with the rule system's diagonal rule. An
//!   action reaches `range` cells (1, adjacent, when it does not say);
//!   up to its `long_range` it still goes, with the system's penalty.
//! - Contact needs no line of sight: two touching cells always see each
//!   other, even squeezed diagonally between two walls (V1 rule).
//! - Otherwise the target must be in sight (`maps::line_of_sight`) and
//!   no farther than the map's `sight_limit` (fog, smoke); the cover it
//!   gets there is what the system's `combat.cover` turns into a modifier.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::maps::{Cell, Cover, Diagonal, Map, line_of_sight};
use crate::rules::check::{Modifier, ModifierSource};
use crate::rules::model::{ActionDef, LongRangeRule, RuleSystem};

/// Why the grid refuses an action on a target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "refusal", rename_all = "snake_case")]
pub enum ReachRefusal {
    OutOfRange { distance: u32, range: u32 },
    NoLineOfSight,
}

/// Where a target stands relative to the one acting on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reach {
    pub distance: u32,
    /// Beyond `range`, within `long_range`.
    pub long_range: bool,
    pub cover: Cover,
}

/// Whether `from` sees `to` for a fight: contact always does; farther,
/// the line must be clear and within the map's sight limit. Returns the
/// cover `to` gets.
pub fn sight(
    map: &Map,
    diagonal: Diagonal,
    from: Cell,
    to: Cell,
    bodies: &HashSet<Cell>,
) -> Option<Cover> {
    let d = map.distance(from, to, diagonal);
    if d <= 1 {
        return Some(Cover::None);
    }
    if let Some(limit) = map.ambience.sight_limit
        && map.distance(from, to, Diagonal::Chebyshev) > limit
    {
        return None;
    }
    let s = line_of_sight(map, from, to, bodies);
    s.visible.then_some(s.cover)
}

/// Range and sight of `action` from `from` to `to`.
pub fn reach(
    map: &Map,
    diagonal: Diagonal,
    action: &ActionDef,
    from: Cell,
    to: Cell,
    bodies: &HashSet<Cell>,
) -> Result<Reach, ReachRefusal> {
    let distance = map.distance(from, to, diagonal);
    let normal = action.reach();
    let max = action.long_range.unwrap_or(normal).max(normal);
    if distance > max {
        return Err(ReachRefusal::OutOfRange {
            distance,
            range: max,
        });
    }
    let cover = sight(map, diagonal, from, to, bodies).ok_or(ReachRefusal::NoLineOfSight)?;
    Ok(Reach {
        distance,
        long_range: distance > normal,
        cover,
    })
}

/// What a reach adds to an attack roll, per the system.
pub fn roll_effects(system: &RuleSystem, reach: &Reach) -> (Vec<Modifier>, bool) {
    let mut mods = Vec::new();
    let mut disadvantage = false;
    let cover = match reach.cover {
        Cover::Half => system.combat.cover.half,
        Cover::ThreeQuarters => system.combat.cover.three_quarters,
        Cover::None | Cover::Total => 0,
    };
    if cover != 0 {
        mods.push(Modifier {
            source: ModifierSource::Cover(reach.cover),
            value: cover,
        });
    }
    if reach.long_range {
        match system.combat.long_range {
            LongRangeRule::Disadvantage => disadvantage = true,
            LongRangeRule::Modifier(0) => {}
            LongRangeRule::Modifier(v) => mods.push(Modifier {
                source: ModifierSource::LongRange,
                value: v,
            }),
        }
    }
    (mods, disadvantage)
}

/// Whether `cell` lies on the line from `from` through `aim`: ahead of
/// `from`, its centre within half a cell of the ray.
/// Square grids (encounter maps) only: on a hex map the offset rows
/// skew the test.
pub fn on_line(from: Cell, aim: Cell, cell: Cell) -> bool {
    if cell == from || aim == from {
        return false;
    }
    let (dx, dy) = (i64::from(aim.x - from.x), i64::from(aim.y - from.y));
    let (vx, vy) = (i64::from(cell.x - from.x), i64::from(cell.y - from.y));
    let dot = vx * dx + vy * dy;
    let cross = vx * dy - vy * dx;
    dot > 0 && 4 * cross * cross <= dx * dx + dy * dy
}
