//! The party on the world map: one token, a calendar cut in portions of
//! a day, its supplies, the hexes it has seen, and the way it follows.
//!
//! Time moves by [`Party::advance`], one portion at a time: the party
//! spends the portion's steps along its way (or stays put), sees the
//! hexes it enters and their neighbours, and after the last portion the
//! night falls. Leaving the night starts a new day and eats the
//! supplies. The server applies it; nothing here is drawn.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::guide::Guide;
use crate::maps::{Cell, Geometry, Map, Side, neighbours};

/// Where the party stands in time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clock {
    /// From 1.
    pub day: u32,
    /// The portion about to be played, an index in the guide's portions.
    pub portion: u32,
    /// After the last portion, until the camp is lifted.
    pub night: bool,
}

/// The road the party is on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Way {
    /// The hexes to enter, in order (`Route::hexes`).
    pub hexes: Vec<Cell>,
    /// How many are behind the party.
    pub step: usize,
    /// Steps spent towards the next hex.
    pub bank: u32,
}

impl Way {
    #[must_use]
    pub fn arrived(&self) -> bool {
        self.step >= self.hexes.len()
    }
}

/// The party on one world map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Party {
    pub at: Cell,
    pub revealed: BTreeSet<Cell>,
    pub clock: Clock,
    /// In the guide's unit; below zero the party goes hungry.
    pub supplies: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub way: Option<Way>,
}

/// What one gesture of time did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Advanced {
    /// A portion was played.
    #[serde(rename_all = "camelCase")]
    Portion {
        day: u32,
        /// The portion's name.
        name: String,
        /// The hexes entered, in order.
        entered: Vec<Cell>,
        /// The way's last hex was reached.
        arrived: bool,
        /// The night fell after it.
        night: bool,
    },
    /// The camp was lifted: a new day, supplies eaten.
    #[serde(rename_all = "camelCase")]
    Dawn { day: u32, eaten: u32, hungry: bool },
}

impl Party {
    /// The party at the map's party start, on the first morning, with
    /// `supplies`. `None` when the map has no party start.
    #[must_use]
    pub fn start(map: &Map, supplies: i32) -> Option<Self> {
        let at = map.starts.iter().find(|s| s.side == Some(Side::Party))?.at;
        let mut p = Self {
            at,
            revealed: BTreeSet::new(),
            clock: Clock {
                day: 1,
                portion: 0,
                night: false,
            },
            supplies,
            way: None,
        };
        p.reveal_around(map, at);
        Some(p)
    }

    /// See `at` and the hexes touching it.
    pub fn reveal_around(&mut self, map: &Map, at: Cell) {
        self.revealed.insert(at);
        for n in neighbours(Geometry::Hex, at) {
            if map.grid.contains(n) {
                self.revealed.insert(n);
            }
        }
    }

    /// Put the party on `at` (the GM's word), leaving any way.
    pub fn place(&mut self, map: &Map, at: Cell) {
        self.at = at;
        self.way = None;
        self.reveal_around(map, at);
    }

    /// Play one portion, or lift the camp after the night.
    ///
    /// `hold`: the party does not move this portion (lost on the way,
    /// resting, a scene that took the afternoon). `party_size` eats at
    /// dawn.
    pub fn advance(&mut self, guide: &Guide, map: &Map, hold: bool, party_size: u32) -> Advanced {
        if self.clock.night {
            self.clock = Clock {
                day: self.clock.day + 1,
                portion: 0,
                night: false,
            };
            let eaten = guide.supplies.per_person_per_day * party_size;
            self.supplies -= i32::try_from(eaten).unwrap_or(i32::MAX);
            return Advanced::Dawn {
                day: self.clock.day,
                eaten,
                hungry: self.supplies < 0,
            };
        }
        let name = guide
            .portions
            .get(self.clock.portion as usize)
            .cloned()
            .unwrap_or_default();
        let day = self.clock.day;
        let mut entered = Vec::new();
        let mut arrived = false;
        if let Some(way) = self.way.as_mut().filter(|w| !hold && !w.arrived()) {
            way.bank += guide.steps_per_portion;
            while let Some(&next) = way.hexes.get(way.step) {
                // A hex that became impassable (a map edited since) stops
                // the party where it is.
                let Some(cost) = guide.cost(map, next) else {
                    way.bank = 0;
                    break;
                };
                if way.bank < cost {
                    break;
                }
                way.bank -= cost;
                way.step += 1;
                entered.push(next);
            }
            if way.arrived() {
                way.bank = 0;
                arrived = true;
            }
        }
        for c in &entered {
            self.at = *c;
            self.reveal_around(map, *c);
        }
        self.clock.portion += 1;
        let night = self.clock.portion >= guide.portions_per_day();
        if night {
            self.clock.night = true;
            self.clock.portion = 0;
        }
        Advanced::Portion {
            day,
            name,
            entered,
            arrived,
            night,
        }
    }
}
