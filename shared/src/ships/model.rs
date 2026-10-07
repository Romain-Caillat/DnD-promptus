//! Ship combat as data: the `ship_combat` section of a rule system
//! (`docs/rules-format.md`). The same model dresses the Brasier's
//! starship and the Corsaires' brig: hull, shields (or canvas), energy
//! (or crew) to share between channels, stations the players hold,
//! weapons with their arcs and range, the damage table.
//!
//! Source: `dnd-save/DnD_07-06-2026/Combat_Vaisseau.md`; where the text
//! leaves a choice, the field's doc says INTERPRETATION.

use serde::Deserialize;

/// Everything ship combat reads from the rule system.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShipCombat {
    /// Range bands, in cells between two ships (Chebyshev distance).
    pub ranges: Ranges,
    /// A ship within this many cells can be boarded (§10).
    #[serde(default = "two")]
    pub boarding_range: u32,
    pub energy: EnergyRule,
    pub stations: Vec<StationDef>,
    pub actions: Vec<ShipActionDef>,
    pub damage_table: DamageTable,
    pub ships: Vec<ShipDef>,
}

fn two() -> u32 {
    2
}

/// The last cell of each band; past `long`, out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ranges {
    pub short: u32,
    pub medium: u32,
    pub long: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RangeBand {
    Short,
    Medium,
    Long,
}

impl Ranges {
    /// The band of `distance`, or `None` past long range.
    #[must_use]
    pub fn band(&self, distance: u32) -> Option<RangeBand> {
        if distance <= self.short {
            Some(RangeBand::Short)
        } else if distance <= self.medium {
            Some(RangeBand::Medium)
        } else if distance <= self.long {
            Some(RangeBand::Long)
        } else {
            None
        }
    }

    /// The farthest cell a weapon of `band` reaches.
    #[must_use]
    pub fn reach(&self, band: RangeBand) -> u32 {
        match band {
            RangeBand::Short => self.short,
            RangeBand::Medium => self.medium,
            RangeBand::Long => self.long,
        }
    }
}

/// The reactor and its three channels (§6).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnergyRule {
    /// Points to share each turn, intact.
    pub reactor: u32,
    /// Less when the hull suffers: the lowest `hull_at_most` that holds.
    #[serde(default)]
    pub damaged: Vec<DamagedReactor>,
    /// Lost while the reactor station is out of service.
    #[serde(default)]
    pub reactor_hit: u32,
    /// The balanced split the fight starts on.
    pub start: Allocation,
    /// Effects of 0, 1, 2… points in each channel; the list's length
    /// caps the channel.
    pub navigation: Vec<EnergyLevel>,
    pub weapons: Vec<EnergyLevel>,
    pub shields: Vec<EnergyLevel>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamagedReactor {
    pub hull_at_most: i32,
    pub reactor: u32,
}

/// Points in each channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Allocation {
    pub navigation: u32,
    pub weapons: u32,
    pub shields: u32,
}

impl Allocation {
    #[must_use]
    pub fn total(&self) -> u32 {
        self.navigation + self.weapons + self.shields
    }
}

/// What a number of points in a channel does. Every field defaults to
/// "no effect".
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct EnergyLevel {
    /// Navigation: cells added to (or taken from) a manoeuvre.
    pub movement: i32,
    /// Navigation: the engines are dead, no manoeuvre nor evasion.
    pub engines_dead: bool,
    /// Navigation: added to the armour while evading.
    pub evade: i32,
    /// Weapons: added to each shot's damage.
    pub damage: i32,
    /// Weapons: no charged shot.
    pub no_charged: bool,
    /// Shields: what « recharge the shields » restores.
    pub recharge: i32,
    /// Shields: offline, they absorb nothing (damage goes to the hull).
    pub offline: bool,
    /// Shields: taken off every hit.
    pub damage_taken: i32,
}

/// A station a crew member holds (§7).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationDef {
    pub id: String,
    pub name: String,
    /// The ability its actions roll with.
    pub ability: String,
    /// The reactor is there: out of service, the reactor loses
    /// `energy.reactor_hit`.
    #[serde(default)]
    pub reactor: bool,
    /// The helm: the pilot's DEX gives the ship's initiative.
    #[serde(default)]
    pub helm: bool,
}

/// One action of a station.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShipActionDef {
    pub id: String,
    pub name: String,
    pub station: String,
    /// Actions it spends out of the turn's.
    #[serde(default = "one")]
    pub cost: u32,
    /// It is the turn's one attack.
    #[serde(default)]
    pub attack: bool,
    /// A check on the station's ability against this difficulty; failed,
    /// the action does nothing (the actions are spent).
    #[serde(default)]
    pub difficulty: Option<i32>,
    pub effect: ShipEffect,
}

fn one() -> u32 {
    1
}

/// What the engine does for an action. The numbers are data; the kinds
/// are what the engine knows how to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ShipEffect {
    /// Move up to `cells` (plus navigation) and turn up to `turns`
    /// quarter turns.
    Maneuver {
        cells: u32,
        turns: u32,
    },
    /// Armour bonus until the ship's next turn (plus navigation's).
    Evade {
        armor: i32,
    },
    /// Damage taken off every hit until the ship's next turn.
    Brace {
        reduction: i32,
    },
    /// Fire the station's weapon; `charged` uses its charged damage.
    Fire {
        charged: bool,
    },
    /// Shields back by the shields channel's `recharge`.
    RechargeShields,
    /// Share the reactor's points anew.
    Reroute,
    RepairHull {
        amount: i32,
    },
    /// Put a station out of service back in.
    RepairSystem,
    Extinguish,
    PatchBreach,
    /// The next allied shot at the target gains `bonus` to hit.
    Lock {
        bonus: i32,
    },
    /// The target's shots lose `malus` to hit until its next turn.
    Jam {
        malus: i32,
    },
    /// Roll against the target's `resolve`; success takes `amount` off
    /// its morale.
    BreakMorale {
        amount: i32,
    },
    /// Change station (enter or leave a turret, §2).
    Move,
}

/// The damage table rolled on a critical hit taken and when the hull
/// crosses a threshold (§8).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageTable {
    pub die: u32,
    /// Crossing one of these hull values (going down) rolls once.
    #[serde(default)]
    pub hull_thresholds: Vec<i32>,
    pub results: Vec<DamageResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageResult {
    pub faces: Vec<u32>,
    pub name: String,
    pub kind: DamageKind,
    /// Fire: hull lost at the start of each of the ship's turns.
    #[serde(default)]
    pub hull_per_turn: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageKind {
    Fire,
    Breach,
    /// A station out of service (drawn among the ship's stations).
    SystemDown,
    /// A crew member loses an action next turn.
    Shake,
}

/// The four arcs around a ship's bow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Arc {
    Front,
    Port,
    Starboard,
    Rear,
}

/// A ship's sheet (§6, §9).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShipDef {
    pub id: String,
    pub name: String,
    pub hull: i32,
    #[serde(default)]
    pub shields: i32,
    pub armor: i32,
    /// `None`: immune (a swarm, a crew of players).
    #[serde(default)]
    pub morale: Option<i32>,
    /// What « break their morale » rolls against.
    #[serde(default = "twelve")]
    pub resolve: i32,
    /// Cells per manoeuvre for a ship the GM runs.
    #[serde(default = "three")]
    pub speed: u32,
    /// To-hit bonus of a ship the GM runs (a crew uses its stations).
    #[serde(default)]
    pub attack_bonus: i32,
    /// Initiative bonus of a ship the GM runs.
    #[serde(default)]
    pub initiative_bonus: i32,
    /// A crew of players: stations, energy, damage table.
    #[serde(default)]
    pub crewed: bool,
    pub weapons: Vec<WeaponDef>,
    #[serde(default)]
    pub notes: String,
}

fn twelve() -> i32 {
    12
}

fn three() -> u32 {
    3
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponDef {
    pub id: String,
    pub name: String,
    /// The station it is fired from (a crewed ship).
    #[serde(default)]
    pub station: Option<String>,
    pub arcs: Vec<Arc>,
    pub range: RangeBand,
    pub damage: i32,
    #[serde(default)]
    pub charged_damage: Option<i32>,
}

impl ShipCombat {
    #[must_use]
    pub fn ship(&self, id: &str) -> Option<&ShipDef> {
        self.ships.iter().find(|s| s.id == id)
    }

    #[must_use]
    pub fn station(&self, id: &str) -> Option<&StationDef> {
        self.stations.iter().find(|s| s.id == id)
    }

    #[must_use]
    pub fn action(&self, id: &str) -> Option<&ShipActionDef> {
        self.actions.iter().find(|a| a.id == id)
    }

    /// The reactor's points for a hull of `hull`, before a hit reactor.
    #[must_use]
    pub fn reactor(&self, hull: i32) -> u32 {
        self.energy
            .damaged
            .iter()
            .filter(|d| hull <= d.hull_at_most)
            .map(|d| d.reactor)
            .min()
            .unwrap_or(self.energy.reactor)
    }
}
