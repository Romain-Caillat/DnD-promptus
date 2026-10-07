//! maps/travel-hex-world — travelling on a world map in hexes.
//!
//! The party is one token. The GM picks a place; [`routes`] finds the
//! cheapest way there and a second one that leaves it; the table votes
//! and the GM chooses. Then time passes by portions of a day
//! ([`Party::advance`]): the party moves along its way at the pace of
//! each terrain ([`Guide`]), sees the hexes it crosses, camps at night
//! and eats its supplies at dawn. Events come from the guide's tables,
//! drawn for the GM, who keeps one or none ([`draw_events`]).
//!
//! The same mechanics serve a sea (the Corsaires' coast) and a star
//! system (the Brasier): only the map and its guide change.

mod guide;
mod party;
mod route;

pub use guide::{DEFAULT_STEPS, Guide, GuideIssue, Supplies, Terrain, TravelEvent};
pub use party::{Advanced, Clock, Party, Way};
pub use route::{Route, portions_for, routes};

use crate::maps::{Cell, Map};
use crate::rules::dice::DiceSource;

/// How many events the co-GM puts before the GM for one portion.
pub const EVENTS_PROPOSED: usize = 3;

/// Up to [`EVENTS_PROPOSED`] events for a portion ending on `at`, drawn
/// without repeat from its tables, leaving out those in `spent` (already
/// kept on this journey) when others remain.
#[must_use]
pub fn draw_events<'a>(
    guide: &'a Guide,
    map: &Map,
    at: Cell,
    spent: &[String],
    dice: &mut dyn DiceSource,
) -> Vec<&'a TravelEvent> {
    let all = guide.events_at(map, at);
    let fresh: Vec<&TravelEvent> = all
        .iter()
        .copied()
        .filter(|e| !spent.contains(&e.id))
        .collect();
    let mut pool = if fresh.is_empty() { all } else { fresh };
    let mut out = Vec::new();
    while out.len() < EVENTS_PROPOSED && !pool.is_empty() {
        let n = u32::try_from(pool.len()).unwrap_or(u32::MAX);
        let i = (dice.roll(n) - 1) as usize;
        out.push(pool.remove(i.min(pool.len() - 1)));
    }
    out
}
