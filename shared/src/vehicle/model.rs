//! Vehicle combat as data: the `vehicles:` block of a rule system
//! (`docs/rules-format.md` § Vehicles). One model, two skins — the
//! Brasier's Cure-Dent (hull, shields, reactor energy) and the
//! Corsaires' brig (hull, sails, crew hands): only the names, numbers
//! and the role of the second gauge change.
//!
//! Nothing here decides anything; [`super::battle`] reads these values.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::rules::dice::DiceExpr;

use super::geometry::Arc;

/// Everything a rule system says about ships.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleRules {
    /// « Combat de vaisseau », « Combat naval ».
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The rule system's turn context each crew member's turn follows
    /// (actions per turn, at most one attack…).
    pub context: String,
    /// The action kind a station's attack counts as, for that context's
    /// limits (« attaque »).
    pub attack_kind: String,
    /// The gauge whose 0 sinks or wrecks a ship (« Coque »).
    pub hull: GaugeDef,
    /// A second gauge: shields that take hits first, or sails that move
    /// the ship. Absent: hull only.
    #[serde(default)]
    pub screen: Option<ScreenDef>,
    /// How hard a ship is to hit (« Blindage », « Bordé »).
    pub armor: GaugeDef,
    /// The second way to win: a ship whose morale reaches 0 strikes its
    /// colours, flees or disengages.
    pub morale: GaugeDef,
    /// Points the crew splits between channels each turn (reactor
    /// energy, hands on deck). Absent: no allocation.
    #[serde(default)]
    pub power: Option<PowerDef>,
    /// Distance bands, nearest first; beyond the last one, out of range.
    pub ranges: Vec<RangeBand>,
    /// Within this many cells, a ship may grapple another and board it.
    pub boarding_range: u32,
    /// Wind or gravity: a ship whose bow points with it moves further,
    /// against it less. The direction comes from the map (`ambience.wind`).
    #[serde(default)]
    pub current: Option<CurrentDef>,
    /// Which station's holder rolls the ship's initiative (the helm).
    pub initiative_station: String,
    /// What changing station costs, in actions.
    #[serde(default = "one")]
    pub station_change_cost: u32,
    pub stations: Vec<StationDef>,
    /// What a hard hit breaks aboard a player ship.
    #[serde(default)]
    pub damage: Option<DamageTableDef>,
    pub ships: Vec<ShipDef>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GaugeDef {
    pub name: String,
    #[serde(default)]
    pub abbr: String,
}

/// What the second gauge does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenRole {
    /// Takes damage before the hull (shields).
    Absorb,
    /// Moves the ship: empty, it cannot manoeuvre; under half, one cell
    /// less. Only weapons aimed at it damage it (chain shot).
    Propulsion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenDef {
    pub name: String,
    #[serde(default)]
    pub abbr: String,
    pub role: ScreenRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerDef {
    /// « Énergie », « Équipage ».
    pub name: String,
    #[serde(default)]
    pub abbr: String,
    /// Points per turn at full hull.
    pub points: u32,
    /// Fewer points as the hull suffers, tightest first applies.
    #[serde(default)]
    pub reduced: Vec<PowerReduction>,
    pub channels: Vec<ChannelDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerReduction {
    pub hull_at_most: i32,
    pub points: u32,
}

/// One channel and what each level of it does; the level is the index
/// in `levels` (0 to `levels.len() - 1`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelDef {
    pub id: String,
    pub name: String,
    /// Points at the start of a battle.
    pub start: u32,
    pub levels: Vec<LevelEffect>,
}

/// What a channel's level changes. Every field defaults to "nothing".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LevelEffect {
    /// Shown to the crew (« Ralenti : -1 case »).
    pub note: String,
    /// Cells added to (or taken from) each manoeuvre.
    pub speed: i32,
    /// The ship cannot manoeuvre at all.
    pub immobile: bool,
    /// Armour while the ship evades, on top of the action's.
    pub evade: i32,
    /// The evade action does nothing.
    pub no_evade: bool,
    /// Damage added to each of the ship's shots.
    pub damage: i32,
    /// Heavy shots (2 actions) are refused.
    pub no_heavy: bool,
    /// What the recharge action restores (None: the action's own amount).
    pub recharge: Option<u32>,
    /// The second gauge is offline: no recharge, absorbs nothing.
    pub screen_offline: bool,
    /// Damage the ship takes per hit (negative reduces).
    pub damage_taken: i32,
    /// Added to repairs of hull and screen.
    pub repair: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangeBand {
    pub id: String,
    pub name: String,
    /// Farthest distance in cells (Chebyshev) of this band.
    pub max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentDef {
    /// « Vent », « Gravité ».
    pub name: String,
    /// Cells added to a manoeuvre whose bow points where it blows.
    pub with: u32,
    /// Cells taken from a manoeuvre whose bow points into it.
    pub against: u32,
}

/// A post aboard, held by one crew member.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The ability its holder rolls.
    pub ability: String,
    /// Classes this post suits (shown to the GM when seating the crew).
    #[serde(default)]
    pub suits: Vec<String>,
    /// Held by someone of the ship when no crew member sits there (LUMEN
    /// at the sensors): the GM plays them.
    #[serde(default)]
    pub held_by: Option<HeldBy>,
    pub actions: Vec<StationAction>,
}

/// A non-player holder of a station.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeldBy {
    pub name: String,
    /// What they add to the station's rolls.
    pub modifier: i32,
}

/// One action at a station: what it costs, whether it is the turn's
/// attack, the roll it asks, and what it does.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationAction {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Actions of the turn it spends (heavy actions: 2).
    #[serde(default = "one")]
    pub cost: u32,
    /// Counts against the turn context's attack limit.
    #[serde(default)]
    pub attack: bool,
    /// A check of the station's ability against this difficulty; the
    /// effect happens only on a success.
    #[serde(default)]
    pub check: Option<i32>,
    pub effect: VehicleEffect,
}

/// The closed vocabulary of what a station action does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VehicleEffect {
    /// Move up to the ship's speed × `multiplier` cells, then turn the
    /// bow by up to `turns` quarters.
    Maneuver {
        #[serde(default = "one")]
        multiplier: u32,
        #[serde(default = "one")]
        turns: u32,
    },
    /// Armour bonus until the ship's next turn.
    Evade { armor: i32 },
    /// Damage taken per hit until the ship's next turn (negative).
    Brace { damage_taken: i32 },
    /// Shoot one of the station's weapons; `damage` replaces the
    /// weapon's (a charged shot).
    Fire {
        #[serde(default)]
        damage: Option<i32>,
    },
    /// Restore the second gauge.
    Recharge { amount: u32 },
    /// Restore the hull, or the second gauge with `screen: true`.
    Repair {
        amount: u32,
        #[serde(default)]
        screen: bool,
    },
    /// Split the power between channels anew.
    Reroute,
    /// The next allied attack on the target gains this bonus.
    Lock { bonus: i32 },
    /// The target's numbers are shown to the players.
    Scan,
    /// The target's attacks suffer this until the end of its next turn.
    Jam { malus: i32 },
    /// A contest on the target's resolve; a success takes `amount` morale.
    BreakMorale { amount: i32 },
    /// The next roll of a crew member gains this bonus.
    Rally { bonus: i32 },
    /// Repair one of these damages (ids of the damage table), with its
    /// own check.
    Fix { damages: Vec<String> },
    /// Break off: the battle ends with the ship gone, when every enemy
    /// is at least this far.
    Disengage { min_distance: u32 },
    /// Something the engine does not resolve (a bluff, a scan for
    /// reinforcements): the roll is made, the GM tells what it does.
    Narrative,
}

/// What a hard hit breaks: rolled when the ship takes a critical hit
/// or its hull falls to or below a threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageTableDef {
    pub dice: DiceExpr,
    #[serde(default)]
    pub on_critical: bool,
    #[serde(default)]
    pub hull_thresholds: Vec<i32>,
    pub entries: Vec<DamageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageEntry {
    /// Faces of the die, both included.
    pub from: u32,
    pub to: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub effect: DamageEffect,
    /// The check that repairs it; none for a passing jolt.
    #[serde(default)]
    pub fix: Option<FixDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageEffect {
    /// Hull lost at the start of each of the ship's turns.
    Burn { hull: i32 },
    /// Danger the GM plays (decompression, a hold taking water).
    Hazard,
    /// A station drawn at random is out of service.
    StationDown,
    /// A crew member drawn at random loses one action next turn; gone
    /// at once.
    Jolt,
    /// Fewer power points until repaired.
    PowerLoss { points: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixDef {
    pub ability: String,
    pub difficulty: i32,
}

/// One kind of ship: the party's, or an enemy's stat block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShipDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub hull: i32,
    /// The second gauge at full; 0 when it has none.
    #[serde(default)]
    pub screen: i32,
    pub armor: i32,
    /// Absent: immune (a swarm, a crew of players).
    #[serde(default)]
    pub morale: Option<i32>,
    /// The difficulty of the contest that breaks its morale.
    #[serde(default = "default_resolve")]
    pub resolve: i32,
    /// Cells per manoeuvre.
    pub speed: u32,
    /// Initiative bonus of an enemy ship (its pilot's DEX).
    #[serde(default)]
    pub initiative: i32,
    /// Ships of this kind in one battle share one initiative and act
    /// together (an escadrille).
    #[serde(default)]
    pub squad: bool,
    pub weapons: Vec<WeaponDef>,
    /// GM notes: how it fights.
    #[serde(default)]
    pub tactics: String,
}

fn default_resolve() -> i32 {
    12
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponDef {
    pub id: String,
    pub name: String,
    /// The station that fires it (player ships).
    #[serde(default)]
    pub station: Option<String>,
    pub arcs: Vec<Arc>,
    /// The farthest band it reaches.
    pub range: String,
    pub damage: i32,
    /// Added to the roll of an enemy ship's shot (its gunners); a crew
    /// member rolls their station's ability instead.
    #[serde(default)]
    pub attack_bonus: i32,
    /// Aimed at the second gauge (chain shot on the rigging).
    #[serde(default)]
    pub at_screen: bool,
}

impl VehicleRules {
    pub fn station(&self, id: &str) -> Option<&StationDef> {
        self.stations.iter().find(|s| s.id == id)
    }

    pub fn ship(&self, id: &str) -> Option<&ShipDef> {
        self.ships.iter().find(|s| s.id == id)
    }

    pub fn band(&self, id: &str) -> Option<&RangeBand> {
        self.ranges.iter().find(|b| b.id == id)
    }

    /// The band a distance falls in, `None` beyond the last.
    pub fn band_at(&self, distance: u32) -> Option<&RangeBand> {
        self.ranges.iter().find(|b| distance <= b.max)
    }

    /// A station action by id, with the station it belongs to.
    pub fn action(&self, id: &str) -> Option<(&StationDef, &StationAction)> {
        self.stations
            .iter()
            .find_map(|s| s.actions.iter().find(|a| a.id == id).map(|a| (s, a)))
    }

    /// Power points available at `hull` (before damage losses).
    pub fn power_points(&self, hull: i32) -> u32 {
        let Some(p) = &self.power else { return 0 };
        p.reduced
            .iter()
            .filter(|r| hull <= r.hull_at_most)
            .map(|r| r.points)
            .min()
            .unwrap_or(p.points)
    }

    /// The starting split of the power.
    pub fn start_power(&self) -> BTreeMap<String, u32> {
        self.power
            .iter()
            .flat_map(|p| p.channels.iter().map(|c| (c.id.clone(), c.start)))
            .collect()
    }
}
