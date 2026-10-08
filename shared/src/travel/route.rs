//! Routes across a world map: the cheapest way from the party's hex to a
//! place, and a second way that does not follow it — the two the table
//! votes between.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use serde::{Deserialize, Serialize};

use super::guide::Guide;
use crate::maps::{Cell, Geometry, Map, neighbours};

/// One way to a place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    /// The hexes entered, in order; the last one is the destination.
    pub hexes: Vec<Cell>,
    /// Steps it costs in all.
    pub steps: u32,
    /// Portions of a day it takes at the guide's pace.
    pub portions: u32,
    /// Days it takes, the last one started counting whole.
    pub days: u32,
    /// The terrains crossed, most hexes first: `(name, hexes)`.
    pub terrains: Vec<(String, u32)>,
}

/// Interior hexes of the first route weigh this many times more when
/// the second is looked for, so it goes another way when one exists.
const DETOUR_WEIGHT: u32 = 6;

/// A second route sharing more than this share of its interior with the
/// first is the same road: it is not proposed.
const SAME_ROAD: f64 = 0.5;

/// The cheapest path, by entering costs, `None` when `to` is out of
/// reach. `extra` adds a weight to some hexes.
fn cheapest(
    map: &Map,
    guide: &Guide,
    from: Cell,
    to: Cell,
    extra: &BTreeSet<Cell>,
) -> Option<(Vec<Cell>, u32)> {
    guide.cost(map, to)?;
    let mut best: BTreeMap<Cell, u32> = BTreeMap::new();
    let mut back: BTreeMap<Cell, Cell> = BTreeMap::new();
    let mut heap = BinaryHeap::new();
    best.insert(from, 0);
    heap.push(Reverse((0u32, from)));
    while let Some(Reverse((d, at))) = heap.pop() {
        if at == to {
            break;
        }
        if best.get(&at).is_some_and(|&b| d > b) {
            continue;
        }
        for n in neighbours(Geometry::Hex, at) {
            let Some(cost) = guide.cost(map, n) else {
                continue;
            };
            let weight = if extra.contains(&n) {
                cost * DETOUR_WEIGHT
            } else {
                cost
            };
            let nd = d + weight;
            if best.get(&n).is_none_or(|&b| nd < b) {
                best.insert(n, nd);
                back.insert(n, at);
                heap.push(Reverse((nd, n)));
            }
        }
    }
    best.get(&to)?;
    let mut hexes = vec![to];
    let mut at = to;
    while let Some(&prev) = back.get(&at) {
        if prev == from {
            break;
        }
        hexes.push(prev);
        at = prev;
    }
    hexes.reverse();
    let steps = hexes.iter().filter_map(|c| guide.cost(map, *c)).sum();
    Some((hexes, steps))
}

/// Portions it takes to enter `costs` in order at `per_portion` steps
/// each, the leftover carried over.
#[must_use]
pub fn portions_for(costs: &[u32], per_portion: u32) -> u32 {
    let per_portion = per_portion.max(1);
    let mut bank = 0;
    let mut portions = 0;
    for &c in costs {
        while bank < c {
            bank += per_portion;
            portions += 1;
        }
        bank -= c;
    }
    portions
}

fn route(map: &Map, guide: &Guide, hexes: Vec<Cell>, steps: u32) -> Route {
    let costs: Vec<u32> = hexes.iter().filter_map(|c| guide.cost(map, *c)).collect();
    let portions = portions_for(&costs, guide.steps_per_portion);
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for c in &hexes {
        *counts.entry(guide.terrain_name(map, *c)).or_default() += 1;
    }
    let mut terrains: Vec<(String, u32)> = counts.into_iter().collect();
    terrains.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Route {
        hexes,
        steps,
        portions,
        days: portions.div_ceil(guide.portions_per_day()),
        terrains,
    }
}

/// Up to two routes from `from` to `to`: the cheapest, then the best one
/// that leaves it. Empty when `to` cannot be reached or is `from`.
#[must_use]
pub fn routes(map: &Map, guide: &Guide, from: Cell, to: Cell) -> Vec<Route> {
    if from == to || !map.grid.contains(to) {
        return Vec::new();
    }
    let Some((first, steps)) = cheapest(map, guide, from, to, &BTreeSet::new()) else {
        return Vec::new();
    };
    let interior: BTreeSet<Cell> = first[..first.len() - 1].iter().copied().collect();
    let mut out = vec![route(map, guide, first, steps)];
    if let Some((second, _)) = cheapest(map, guide, from, to, &interior) {
        let own: Vec<&Cell> = second[..second.len() - 1].iter().collect();
        let shared = own.iter().filter(|c| interior.contains(c)).count();
        #[allow(clippy::cast_precision_loss)]
        let share = if own.is_empty() {
            1.0
        } else {
            shared as f64 / own.len() as f64
        };
        if share <= SAME_ROAD && second != out[0].hexes {
            let steps = second.iter().filter_map(|c| guide.cost(map, *c)).sum();
            out.push(route(map, guide, second, steps));
        }
    }
    out
}
