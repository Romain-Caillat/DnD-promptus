//! The journey on the world map as a player (or the shared screen)
//! receives it, built here by allow-list (`MEMORY.md` §3,
//! maps/travel-hex-world). The map itself and the party's token reach
//! players through the board (`super::board`): fogged hexes, places on a
//! GM layer and the map's GM notes never do.
//!
//! Reaches players: the day and the portion, the supplies, where the
//! journey goes (unless the GM aims at a place the table does not know),
//! the routes proposed with who voted for each, the route chosen and how
//! far along it the party is, the events the GM kept as the GM worded
//! them, the group check with each roll, and the night's watches — the
//! GM's words to the watcher on that watcher's phone only.
//!
//! Never: the events drawn and not kept, an event's GM notes, the guide's
//! tables, a secret destination's name, the GM's message to another
//! player's watch.

use promptus_shared::maps::{Cell, Map};
use promptus_shared::rules::check::{OutcomeBand, RollBreakdown};
use promptus_shared::travel::Guide;
use serde::Serialize;
use uuid::Uuid;

use crate::travel::{Member, Travel, known_place};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelView {
    pub map_name: String,
    pub day: u32,
    /// The portion about to be played; `None` at night.
    pub portion: Option<String>,
    pub night: bool,
    pub portions: Vec<String>,
    pub supplies: SuppliesView,
    pub journey: Option<JourneyView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuppliesView {
    pub name: String,
    pub value: i32,
    /// Eaten by the whole party at each dawn.
    pub per_day: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JourneyView {
    /// Empty when the place is not known to the table.
    pub destination: String,
    pub destination_at: Option<Cell>,
    pub routes: Vec<RouteView>,
    pub chosen: Option<usize>,
    pub arrived: bool,
    /// Hexes behind the party on the chosen route, and in all.
    pub progress: Option<[usize; 2]>,
    pub events: Vec<KeptView>,
    pub check: Option<CheckView>,
    pub watch: Option<WatchView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteView {
    pub name: String,
    pub description: String,
    pub days: u32,
    pub portions: u32,
    /// The way the GM proposes, shown whole: a road the table is told of.
    pub hexes: Vec<Cell>,
    pub terrains: Vec<(String, u32)>,
    pub voters: Vec<String>,
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeptView {
    pub day: u32,
    pub portion: String,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckView {
    pub label: String,
    pub ability_name: String,
    pub difficulty: i32,
    pub needed: usize,
    pub rolls: Vec<RollView>,
    /// The caller is in it and has not rolled yet.
    pub can_roll: bool,
    pub my_roll: Option<RollBreakdown>,
    pub success: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollView {
    pub name: String,
    pub total: Option<i32>,
    pub success: Option<bool>,
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchView {
    pub slots: Vec<WatchSlotView>,
    pub current: Option<usize>,
    /// The caller keeps the watch being played.
    pub mine: bool,
    /// The GM's words, on the watcher's phone only.
    pub message: Option<String>,
    pub woken: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchSlotView {
    pub name: String,
    pub who: Option<String>,
    pub mine: bool,
}

/// The journey as player `me` (`None`: the shared screen, a spectator)
/// may see it.
#[must_use]
pub fn project_travel(
    t: &Travel,
    map: &Map,
    guide: &Guide,
    members: &[Member],
    me: Option<Uuid>,
) -> TravelView {
    let p = &t.party;
    let name_of = |player: &Uuid| {
        members
            .iter()
            .find(|m| m.player == *player)
            .map(|m| m.name.clone())
    };
    let journey = t.journey.as_ref().map(|j| {
        let known = known_place(map, j.destination.at);
        let routes = j
            .routes
            .iter()
            .enumerate()
            .map(|(i, r)| RouteView {
                name: r.name.clone(),
                description: r.description.clone(),
                days: r.route.days,
                portions: r.route.portions,
                hexes: r.route.hexes.clone(),
                terrains: r.route.terrains.clone(),
                voters: j
                    .votes
                    .iter()
                    .filter(|(_, v)| **v == i)
                    .filter_map(|(who, _)| name_of(who))
                    .collect(),
                mine: me.is_some_and(|m| j.votes.get(&m) == Some(&i)),
            })
            .collect();
        let check = j.check.as_ref().map(|c| {
            let mine = me.and_then(|m| c.rolls.iter().find(|r| r.player == m));
            CheckView {
                label: c.label.clone(),
                ability_name: c.ability_name.clone(),
                difficulty: c.difficulty,
                needed: c.needed,
                rolls: c
                    .rolls
                    .iter()
                    .map(|r| RollView {
                        name: r.name.clone(),
                        total: r.roll.as_ref().map(|b| b.total),
                        success: r
                            .roll
                            .as_ref()
                            .map(|b| b.band.is_some_and(OutcomeBand::is_success)),
                        mine: Some(r.player) == me,
                    })
                    .collect(),
                can_roll: c.success.is_none() && mine.is_some_and(|r| r.roll.is_none()),
                my_roll: mine.and_then(|r| r.roll.clone()),
                success: c.success,
            }
        });
        let watch = j.watch.as_ref().map(|w| {
            let mine = me.is_some_and(|m| {
                w.current
                    .and_then(|i| w.slots.get(i))
                    .is_some_and(|s| s.player == Some(m))
            });
            WatchView {
                slots: w
                    .slots
                    .iter()
                    .map(|s| WatchSlotView {
                        name: s.name.clone(),
                        who: s.who.clone(),
                        mine: me.is_some() && s.player == me,
                    })
                    .collect(),
                current: w.current,
                mine,
                message: (mine && !w.message.is_empty()).then(|| w.message.clone()),
                woken: w.woken,
            }
        });
        JourneyView {
            destination: if known {
                j.destination.name.clone()
            } else {
                String::new()
            },
            destination_at: known.then_some(j.destination.at),
            routes,
            chosen: j.chosen,
            arrived: j.arrived,
            progress: p.way.as_ref().map(|w| [w.step, w.hexes.len()]),
            events: j
                .events
                .iter()
                .map(|e| KeptView {
                    day: e.day,
                    portion: e.portion.clone(),
                    title: e.title.clone(),
                    text: e.text.clone(),
                })
                .collect(),
            check,
            watch,
        }
    });
    TravelView {
        map_name: map.name.clone(),
        day: p.clock.day,
        portion: (!p.clock.night)
            .then(|| guide.portions.get(p.clock.portion as usize).cloned())
            .flatten(),
        night: p.clock.night,
        portions: guide.portions.clone(),
        supplies: SuppliesView {
            name: guide.supplies.name.clone(),
            value: p.supplies,
            per_day: guide.supplies.per_person_per_day
                * u32::try_from(members.len()).unwrap_or(u32::MAX),
        },
        journey,
    }
}
