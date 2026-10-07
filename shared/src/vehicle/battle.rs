//! A ship battle on a grid: ships with a bow, a crew at their stations,
//! enemy ships at their initiative. Each command takes a battle and
//! returns the next battle plus what happened, or a refusal with nothing
//! changed — the same contract as `combat::fight`.
//!
//! The turn, from `Combat_Vaisseau.md`:
//! - every **ship** rolls initiative once (d20 + the helm's DEX; an enemy
//!   its own bonus); ships of a squad share one roll and act together;
//! - on the party ship's turn **the whole crew acts**, each member with
//!   the turn context's budget (2 actions, at most 1 attack), in the order
//!   the table likes; the turn ends when every member is done or the GM
//!   ends it;
//! - an enemy ship manoeuvres once and fires once per turn.
//!
//! What lasts: an evade or a brace until the ship's next turn; a jam
//! until the end of the jammed ship's next turn; a lock until the next
//! allied shot on that ship; a rally until the next crew roll or the end
//! of the crew's turn.
//!
//! A ship leaves the battle **destroyed** (hull 0), **struck** (morale
//! 0), **captured** (boarded and taken) or **fled**. The battle ends when
//! no enemy ship is left afloat, when the party ship is wrecked or
//! taken, when the party ship disengages, or when the GM stops it.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

use crate::combat::fight::InitiativeRoll;
use crate::maps::{Cell, Direction, Map, MovementRules, Occupancy, line_of_sight, reachable};
use crate::rules::RuleSystem;
use crate::rules::check::{
    self, Advantage, Modifier, ModifierSource, OutcomeBand, RollBreakdown, RollTarget,
};
use crate::rules::dice::DiceSource;
use crate::rules::model::InitiativeTies;
use crate::rules::sheet::{Combatant, Side};

use super::geometry::{Facing, arc_of, distance};
use super::model::{
    DamageEffect, LevelEffect, ScreenRole, ShipDef, StationAction, VehicleEffect, VehicleRules,
    WeaponDef,
};

/// After this many rounds a battle is called off as a stalemate.
pub const MAX_ROUNDS: u32 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipStanding {
    Afloat,
    Destroyed,
    Struck,
    Captured,
    Fled,
}

/// A damage aboard, waiting for its repair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveDamage {
    /// The damage table's entry.
    pub id: String,
    pub name: String,
    /// The station it put out of service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub station: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ship {
    pub id: String,
    /// The rules' ship id (`ShipDef`).
    pub kind: String,
    pub name: String,
    pub side: Side,
    pub at: Cell,
    pub facing: Facing,
    pub hull: i32,
    pub max_hull: i32,
    pub screen: i32,
    pub max_screen: i32,
    pub armor: i32,
    pub morale: Option<i32>,
    pub max_morale: Option<i32>,
    pub resolve: i32,
    /// Points per channel (the party ship).
    pub power: BTreeMap<String, u32>,
    /// Armour bonus while evading.
    pub evade: i32,
    /// Damage taken per hit while braced (negative).
    pub brace: i32,
    /// Malus on its shots, and how many of its turn ends it lasts.
    pub jam: i32,
    pub jam_turns: u32,
    /// Bonus of the next allied shot on it.
    pub locked: i32,
    pub damages: Vec<ActiveDamage>,
    /// Hull thresholds already crossed.
    pub thresholds: Vec<i32>,
    /// The players see its numbers.
    pub scanned: bool,
    pub standing: ShipStanding,
    /// An enemy ship's manoeuvre and shot this turn.
    pub moved: bool,
    pub fired: bool,
}

impl Ship {
    pub fn afloat(&self) -> bool {
        self.standing == ShipStanding::Afloat
    }

    fn station_down(&self, station: &str) -> bool {
        self.damages
            .iter()
            .any(|d| d.station.as_deref() == Some(station))
    }
}

/// A crew member of the party ship.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Crew {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub class: Option<String>,
    /// Ability modifiers, by ability id.
    pub mods: BTreeMap<String, i32>,
    pub station: Option<String>,
    /// Played by the GM (LUMEN), not a player.
    pub npc: bool,
    pub actions: u32,
    pub attacks: u32,
    pub done: bool,
    /// Actions lost at the start of the next turn (a jolt).
    pub shaken: u32,
}

impl Crew {
    /// A player character as a crew member: its modifiers from the rules.
    pub fn from_combatant(system: &RuleSystem, c: &Combatant, class: Option<String>) -> Self {
        let mods = system
            .abilities
            .iter()
            .filter_map(|a| c.modifier(system, &a.id).ok().map(|m| (a.id.clone(), m)))
            .collect();
        Self {
            id: c.id.clone(),
            name: c.name.clone(),
            class,
            mods,
            station: None,
            npc: false,
            actions: 0,
            attacks: 0,
            done: true,
            shaken: 0,
        }
    }

    fn modifier(&self, ability: &str) -> i32 {
        self.mods.get(ability).copied().unwrap_or(0)
    }
}

/// One rank of the initiative: a ship, or a squad of ships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub id: String,
    pub name: String,
    pub side: Side,
    pub ships: Vec<String>,
    pub initiative: InitiativeRoll,
}

/// A boarding in progress: the battle waits for the fight on the deck.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Boarding {
    pub attacker: String,
    pub defender: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleEndReason {
    /// No enemy ship left afloat.
    Victory,
    /// The party ship is wrecked or taken.
    Defeat,
    /// The party ship broke off.
    Disengaged,
    StoppedByGm,
    Stalemate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleEnd {
    pub winner: Option<Side>,
    pub reason: BattleEndReason,
    pub rounds: u32,
    /// XP each crew member's rolls earned, by id.
    pub xp: BTreeMap<String, u32>,
}

/// What a battle reports, in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BattleEvent {
    InitiativeRolled {
        unit: String,
        roll: InitiativeRoll,
    },
    RoundStarted {
        round: u32,
    },
    TurnStarted {
        unit: String,
        round: u32,
    },
    Seated {
        crew: String,
        station: Option<String>,
    },
    Maneuvered {
        ship: String,
        from: Cell,
        to: Cell,
        facing: Facing,
    },
    Fired {
        ship: String,
        weapon: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        by: Option<String>,
        target: String,
        roll: RollBreakdown,
        hit: bool,
        damage: i32,
        /// The target's gauges after the shot; the player projection
        /// hides an unscanned enemy's.
        screen_after: Option<i32>,
        hull_after: Option<i32>,
    },
    Evading {
        ship: String,
        armor: i32,
    },
    Braced {
        ship: String,
        damage_taken: i32,
    },
    Recharged {
        ship: String,
        screen: i32,
    },
    Repaired {
        ship: String,
        by: String,
        amount: i32,
        screen: bool,
    },
    Rerouted {
        ship: String,
        power: BTreeMap<String, u32>,
    },
    Locked {
        by: String,
        target: String,
        bonus: i32,
    },
    Scanned {
        by: String,
        target: String,
    },
    Jammed {
        by: String,
        target: String,
        malus: i32,
    },
    MoraleRoll {
        by: String,
        target: String,
        roll: RollBreakdown,
        loss: i32,
        morale_after: Option<i32>,
    },
    Rallied {
        by: String,
        bonus: i32,
    },
    /// A station action's own check.
    Check {
        by: String,
        action: String,
        roll: RollBreakdown,
    },
    /// An action the GM resolves.
    ForTheGm {
        by: String,
        action: String,
    },
    DamageRolled {
        ship: String,
        face: u32,
        damage: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        station: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        crew: Option<String>,
    },
    DamageFixed {
        ship: String,
        damage: String,
        by: String,
    },
    Burned {
        ship: String,
        damage: String,
        amount: i32,
        hull_after: i32,
    },
    PowerShort {
        ship: String,
        points: u32,
        power: BTreeMap<String, u32>,
    },
    ShipOut {
        ship: String,
        standing: ShipStanding,
    },
    Adjusted {
        ship: String,
    },
    CurrentChanged {
        from: Option<Direction>,
    },
    BoardingStarted {
        attacker: String,
        defender: String,
    },
    BoardingEnded {
        attacker: String,
        defender: String,
        attacker_won: bool,
    },
    TurnEnded {
        unit: String,
    },
    Ended {
        end: BattleEnd,
    },
}

/// Why a command is refused. Nothing changes when one is returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BattleRefusal {
    BattleOver,
    Boarding,
    NotBoarding,
    NotTheirTurn,
    UnknownShip { ship: String },
    UnknownCrew { crew: String },
    UnknownStation { station: String },
    UnknownAction { action: String },
    UnknownWeapon { weapon: String },
    StationTaken { station: String },
    NotAtStation { station: String },
    StationDown { station: String },
    NoActionsLeft,
    NoAttackLeft,
    AlreadyMoved,
    AlreadyFired,
    CannotMove,
    NotReachable,
    TooSharpATurn,
    WrongTarget,
    OutOfArc,
    OutOfRange,
    NoLineOfFire,
    HeavyRefused,
    ScreenOffline,
    Immune,
    WrongPower,
    UnknownDamage,
    NotFixableHere,
    TooClose,
    NoVehicleRules,
}

/// The next battle and what happened on the way.
#[derive(Debug, Clone)]
pub struct Step {
    pub battle: Battle,
    pub events: Vec<BattleEvent>,
}

/// What a crew member's action is aimed at.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Aim {
    /// A ship (enemy or own).
    pub target: Option<String>,
    /// The weapon fired (when the station has several).
    pub weapon: Option<String>,
    /// Where a manoeuvre ends, and the bow then.
    pub to: Option<Cell>,
    pub facing: Option<Facing>,
    /// A new split of the power.
    pub power: Option<BTreeMap<String, u32>>,
    /// Index of the damage repaired in the ship's list.
    pub damage: Option<usize>,
}

/// How a battle opens: the ships and where they stand, the crew and
/// where they sit.
#[derive(Debug, Clone)]
pub struct Setup {
    pub party_ship: Placed,
    pub crew: Vec<Crew>,
    pub enemies: Vec<Placed>,
}

/// One ship placed on the map.
#[derive(Debug, Clone)]
pub struct Placed {
    pub id: String,
    /// `ShipDef` id.
    pub kind: String,
    pub name: Option<String>,
    pub at: Cell,
    pub facing: Facing,
}

/// The whole state of a battle. Pure data: the server stores it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Battle {
    pub map: Map,
    pub ships: Vec<Ship>,
    pub crew: Vec<Crew>,
    /// Initiative order for the whole battle.
    pub units: Vec<Unit>,
    pub round: u32,
    /// Index in `units` of the current turn.
    pub turn: usize,
    /// Where the wind (or the pull) comes from.
    pub current: Option<Direction>,
    /// The next crew roll's bonus.
    pub rally: i32,
    pub boarding: Option<Boarding>,
    pub xp: BTreeMap<String, u32>,
    pub end: Option<BattleEnd>,
}

/// Ships move like a body over open water: deep water floats them,
/// walls (reefs, rocks, asteroids) stop them.
fn sailing() -> MovementRules {
    MovementRules {
        swim_factor: Some(1),
        difficult_factor: 1,
        max_step: u8::MAX,
        ..MovementRules::default()
    }
}

fn vehicles(system: &RuleSystem) -> Result<&VehicleRules, BattleRefusal> {
    system
        .vehicles
        .as_ref()
        .ok_or(BattleRefusal::NoVehicleRules)
}

impl Battle {
    /// Opens a battle: ships placed, crew seated, initiative rolled, the
    /// first turn begun.
    pub fn start(
        system: &RuleSystem,
        map: Map,
        setup: Setup,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, BattleRefusal> {
        let v = vehicles(system)?;
        let ship_of = |p: &Placed, side: Side| -> Result<Ship, BattleRefusal> {
            let def = v.ship(&p.kind).ok_or_else(|| BattleRefusal::UnknownShip {
                ship: p.kind.clone(),
            })?;
            Ok(new_ship(v, def, p, side))
        };
        let mut ships = vec![ship_of(&setup.party_ship, Side::Party)?];
        for e in &setup.enemies {
            ships.push(ship_of(e, Side::Opposition)?);
        }
        let mut crew = setup.crew;
        // Ship-held stations nobody sits at get their holder (LUMEN).
        for s in &v.stations {
            if let Some(h) = &s.held_by
                && !crew.iter().any(|c| c.station.as_deref() == Some(&s.id))
            {
                crew.push(Crew {
                    id: format!("held:{}", s.id),
                    name: h.name.clone(),
                    class: None,
                    mods: system
                        .abilities
                        .iter()
                        .map(|a| (a.id.clone(), h.modifier))
                        .collect(),
                    station: Some(s.id.clone()),
                    npc: true,
                    actions: 0,
                    attacks: 0,
                    done: true,
                    shaken: 0,
                });
            }
        }
        let current = map.ambience.wind.map(|w| w.from);
        let mut battle = Battle {
            map,
            ships,
            crew,
            units: Vec::new(),
            round: 1,
            turn: 0,
            current,
            rally: 0,
            boarding: None,
            xp: BTreeMap::new(),
            end: None,
        };
        let mut events = Vec::new();
        battle.units = battle.roll_initiative(system, v, dice);
        for u in &battle.units {
            events.push(BattleEvent::InitiativeRolled {
                unit: u.id.clone(),
                roll: u.initiative.clone(),
            });
        }
        events.push(BattleEvent::RoundStarted { round: 1 });
        battle.begin_turn(system, v, dice, &mut events);
        Ok(Step { battle, events })
    }

    fn roll_initiative(
        &self,
        system: &RuleSystem,
        v: &VehicleRules,
        dice: &mut dyn DiceSource,
    ) -> Vec<Unit> {
        let mut units: Vec<Unit> = Vec::new();
        for ship in &self.ships {
            let def = v.ship(&ship.kind);
            let squad = def.is_some_and(|d| d.squad) && ship.side == Side::Opposition;
            if squad
                && let Some(u) = units
                    .iter_mut()
                    .find(|u| u.id == format!("squad:{}", ship.kind))
            {
                u.ships.push(ship.id.clone());
                continue;
            }
            let bonus = match ship.side {
                Side::Party => self.helm_modifier(v),
                Side::Opposition => def.map_or(0, |d| d.initiative),
            };
            let r = system.initiative.dice.roll(dice);
            let (id, name) = if squad {
                let name = def.map_or_else(|| ship.kind.clone(), |d| d.name.clone());
                (format!("squad:{}", ship.kind), name)
            } else {
                (ship.id.clone(), ship.name.clone())
            };
            units.push(Unit {
                id: id.clone(),
                name,
                side: ship.side,
                ships: vec![ship.id.clone()],
                initiative: InitiativeRoll {
                    who: id,
                    faces: r.faces,
                    bonus,
                    total: r.total + bonus,
                    rerolls: Vec::new(),
                },
            });
        }
        let party_first = system.initiative.ties == InitiativeTies::PartyFirst;
        let rank = |s: Side| match (party_first, s) {
            (true, Side::Opposition) => 1,
            _ => 0,
        };
        let mut indexed: Vec<(usize, Unit)> = units.into_iter().enumerate().collect();
        indexed.sort_by(|(ia, a), (ib, b)| {
            b.initiative
                .total
                .cmp(&a.initiative.total)
                .then_with(|| rank(a.side).cmp(&rank(b.side)))
                .then_with(|| ia.cmp(ib))
        });
        indexed.into_iter().map(|(_, u)| u).collect()
    }

    /// The helm holder's modifier in the helm's ability.
    fn helm_modifier(&self, v: &VehicleRules) -> i32 {
        let Some(helm) = v.station(&v.initiative_station) else {
            return 0;
        };
        self.crew
            .iter()
            .find(|c| c.station.as_deref() == Some(&helm.id))
            .map_or(0, |c| c.modifier(&helm.ability))
    }

    pub fn is_over(&self) -> bool {
        self.end.is_some()
    }

    pub fn ship(&self, id: &str) -> Option<&Ship> {
        self.ships.iter().find(|s| s.id == id)
    }

    fn ship_index(&self, id: &str) -> Result<usize, BattleRefusal> {
        self.ships
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| BattleRefusal::UnknownShip { ship: id.into() })
    }

    /// The party's ship.
    pub fn party_ship(&self) -> &Ship {
        self.ships
            .iter()
            .find(|s| s.side == Side::Party)
            .expect("a battle has a party ship")
    }

    fn party_index(&self) -> usize {
        self.ships
            .iter()
            .position(|s| s.side == Side::Party)
            .expect("a battle has a party ship")
    }

    pub fn crew_member(&self, id: &str) -> Option<&Crew> {
        self.crew.iter().find(|c| c.id == id)
    }

    /// The unit whose turn it is.
    pub fn active(&self) -> Option<&Unit> {
        if self.is_over() {
            return None;
        }
        self.units.get(self.turn)
    }

    /// Whether it is the party ship's turn.
    pub fn crew_turn(&self) -> bool {
        self.active().is_some_and(|u| u.side == Side::Party)
    }

    fn power_level(
        &self,
        v: &VehicleRules,
        ship: &Ship,
        channel_effect: impl Fn(&LevelEffect) -> i32,
    ) -> i32 {
        let Some(p) = &v.power else { return 0 };
        if ship.side != Side::Party {
            return 0;
        }
        p.channels
            .iter()
            .filter_map(|c| {
                let level = *ship.power.get(&c.id).unwrap_or(&0) as usize;
                c.levels.get(level.min(c.levels.len().saturating_sub(1)))
            })
            .map(channel_effect)
            .sum()
    }

    fn power_flag(
        &self,
        v: &VehicleRules,
        ship: &Ship,
        flag: impl Fn(&LevelEffect) -> bool,
    ) -> bool {
        self.power_level(v, ship, |e| i32::from(flag(e))) > 0
    }

    /// The recharge a channel level sets, if any.
    fn power_recharge(&self, v: &VehicleRules, ship: &Ship) -> Option<u32> {
        let p = v.power.as_ref()?;
        if ship.side != Side::Party {
            return None;
        }
        p.channels.iter().find_map(|c| {
            let level = *ship.power.get(&c.id).unwrap_or(&0) as usize;
            c.levels
                .get(level.min(c.levels.len().saturating_sub(1)))
                .and_then(|e| e.recharge)
        })
    }

    /// Power points the ship has now: by its hull, less damages.
    pub fn power_points(&self, v: &VehicleRules, ship: &Ship) -> u32 {
        let lost: u32 = ship
            .damages
            .iter()
            .filter_map(|d| {
                v.damage
                    .as_ref()?
                    .entries
                    .iter()
                    .find(|e| e.id == d.id)
                    .and_then(|e| match e.effect {
                        DamageEffect::PowerLoss { points } => Some(points),
                        _ => None,
                    })
            })
            .sum();
        v.power_points(ship.hull).saturating_sub(lost)
    }

    /// Cells one manoeuvre lets `ship` cover, with `multiplier`.
    pub fn maneuver_budget(&self, v: &VehicleRules, ship: &Ship, multiplier: u32) -> Option<u32> {
        let def = v.ship(&ship.kind)?;
        if self.power_flag(v, ship, |e| e.immobile) {
            return None;
        }
        let mut cells = (def.speed * multiplier) as i32 + self.power_level(v, ship, |e| e.speed);
        if let Some(s) = &v.screen
            && s.role == ScreenRole::Propulsion
            && ship.max_screen > 0
        {
            if ship.screen <= 0 {
                return None;
            }
            if ship.screen * 2 < ship.max_screen {
                cells -= 1;
            }
        }
        if let (Some(c), Some(from)) = (&v.current, self.current) {
            match ship.facing.to_current(from) {
                1 => cells += c.with as i32,
                -1 => cells -= c.against as i32,
                _ => {}
            }
        }
        Some(cells.max(0) as u32)
    }

    /// Every cell `ship` could end a manoeuvre on.
    pub fn reachable(&self, v: &VehicleRules, ship: &str, multiplier: u32) -> BTreeMap<Cell, u32> {
        let Some(s) = self.ship(ship) else {
            return BTreeMap::new();
        };
        let Some(budget) = self.maneuver_budget(v, s, multiplier) else {
            return BTreeMap::new();
        };
        let occ = Occupancy {
            enemies: self
                .ships
                .iter()
                .filter(|o| o.id != s.id && o.afloat())
                .map(|o| o.at)
                .collect(),
            allies: HashSet::new(),
        };
        reachable(&self.map, &sailing(), &occ, s.at, budget)
    }

    #[allow(clippy::too_many_arguments)]
    fn move_ship(
        &mut self,
        v: &VehicleRules,
        index: usize,
        multiplier: u32,
        turns: u32,
        to: Cell,
        facing: Option<Facing>,
        events: &mut Vec<BattleEvent>,
    ) -> Result<(), BattleRefusal> {
        let ship = &self.ships[index];
        let facing = facing.unwrap_or(ship.facing);
        if ship.facing.quarters_to(facing) > turns {
            return Err(BattleRefusal::TooSharpATurn);
        }
        if self.maneuver_budget(v, ship, multiplier).is_none() {
            return Err(BattleRefusal::CannotMove);
        }
        if to != ship.at && !self.reachable(v, &ship.id, multiplier).contains_key(&to) {
            return Err(BattleRefusal::NotReachable);
        }
        let from = ship.at;
        let ship = &mut self.ships[index];
        ship.at = to;
        ship.facing = facing;
        events.push(BattleEvent::Maneuvered {
            ship: ship.id.clone(),
            from,
            to,
            facing,
        });
        Ok(())
    }

    /// Whether `weapon` of `shooter` can reach `target` now.
    pub fn can_fire(
        &self,
        v: &VehicleRules,
        shooter: &Ship,
        weapon: &WeaponDef,
        target: &Ship,
    ) -> Result<(), BattleRefusal> {
        if !target.afloat() || target.side == shooter.side {
            return Err(BattleRefusal::WrongTarget);
        }
        let arc =
            arc_of(shooter.at, shooter.facing, target.at).ok_or(BattleRefusal::WrongTarget)?;
        if !weapon.arcs.contains(&arc) {
            return Err(BattleRefusal::OutOfArc);
        }
        let reach = v.band(&weapon.range).map_or(0, |b| b.max);
        if distance(shooter.at, target.at) > reach {
            return Err(BattleRefusal::OutOfRange);
        }
        if !line_of_sight(&self.map, shooter.at, target.at, &HashSet::new()).visible {
            return Err(BattleRefusal::NoLineOfFire);
        }
        Ok(())
    }

    /// A shot: the roll against the target's armour, the damage through
    /// its gauges, and what a hard hit breaks.
    #[allow(clippy::too_many_arguments)]
    fn shoot(
        &mut self,
        system: &RuleSystem,
        v: &VehicleRules,
        shooter: usize,
        weapon: &WeaponDef,
        by: Option<&Crew>,
        damage_override: Option<i32>,
        target: usize,
        dice: &mut dyn DiceSource,
        events: &mut Vec<BattleEvent>,
    ) -> Option<OutcomeBand> {
        let (s, t) = (&self.ships[shooter], &self.ships[target]);
        let mut mods = Vec::new();
        match by {
            Some(c) => {
                let ability = c
                    .station
                    .as_deref()
                    .and_then(|st| v.station(st))
                    .map_or("", |st| st.ability.as_str());
                mods.push(Modifier {
                    source: ModifierSource::Ability(ability.into()),
                    value: c.modifier(ability),
                });
            }
            None => mods.push(Modifier {
                source: ModifierSource::Situation(weapon.name.clone()),
                value: weapon.attack_bonus,
            }),
        }
        if t.locked != 0 {
            mods.push(Modifier {
                source: ModifierSource::Situation("lock".into()),
                value: t.locked,
            });
        }
        if s.jam != 0 {
            mods.push(Modifier {
                source: ModifierSource::Situation("jam".into()),
                value: s.jam,
            });
        }
        if by.is_some() && self.rally != 0 {
            mods.push(Modifier {
                source: ModifierSource::Situation("rally".into()),
                value: self.rally,
            });
        }
        let evading = if t.evade != 0 && !self.power_flag(v, t, |e| e.no_evade) {
            t.evade + self.power_level(v, t, |e| e.evade)
        } else {
            0
        };
        let armor = t.armor + evading;
        let roll = check::roll(
            system,
            mods,
            Advantage::Normal,
            Some(RollTarget::ArmorClass { value: armor }),
            dice,
        )
        .expect("a plain roll never fails");
        let band = roll.band;
        let hit = band.is_some_and(OutcomeBand::is_success);
        let critical = band == Some(OutcomeBand::CriticalSuccess);
        if by.is_some() {
            self.rally = 0;
        }
        self.ships[target].locked = 0;
        let mut damage = 0;
        let (t_side, t_id) = (self.ships[target].side, self.ships[target].id.clone());
        let hull_before = self.ships[target].hull;
        if hit {
            let base = damage_override.unwrap_or(weapon.damage)
                + self.power_level(v, &self.ships[shooter], |e| e.damage);
            let multiplier = if critical {
                system
                    .outcomes
                    .critical_success
                    .damage_multiplier
                    .unwrap_or(1)
            } else {
                1
            };
            let t = &self.ships[target];
            damage =
                (base * multiplier + t.brace + self.power_level(v, t, |e| e.damage_taken)).max(0);
            let role = v.screen.as_ref().map(|s| s.role);
            let offline = self.power_flag(v, t, |e| e.screen_offline);
            let t = &mut self.ships[target];
            match role {
                Some(ScreenRole::Propulsion) if weapon.at_screen => {
                    t.screen = (t.screen - damage).max(0);
                }
                Some(ScreenRole::Absorb) if !offline && t.screen > 0 => {
                    let absorbed = t.screen.min(damage);
                    t.screen -= absorbed;
                    t.hull -= damage - absorbed;
                }
                _ => t.hull -= damage,
            }
            t.hull = t.hull.max(0);
        }
        let t = &self.ships[target];
        events.push(BattleEvent::Fired {
            ship: self.ships[shooter].id.clone(),
            weapon: weapon.id.clone(),
            by: by.map(|c| c.id.clone()),
            target: t_id.clone(),
            roll,
            hit,
            damage,
            screen_after: (t.max_screen > 0).then_some(t.screen),
            hull_after: Some(t.hull),
        });
        if t_side == Side::Party {
            self.hard_hits(v, target, hull_before, critical, dice, events);
        }
        self.check_out(target, events);
        band
    }

    /// Rolls the damage table for a critical hit and each hull threshold
    /// crossed.
    fn hard_hits(
        &mut self,
        v: &VehicleRules,
        index: usize,
        hull_before: i32,
        critical: bool,
        dice: &mut dyn DiceSource,
        events: &mut Vec<BattleEvent>,
    ) {
        let Some(table) = &v.damage else { return };
        let mut rolls = usize::from(critical && table.on_critical);
        let hull = self.ships[index].hull;
        for t in &table.hull_thresholds {
            if hull_before > *t && hull <= *t && !self.ships[index].thresholds.contains(t) {
                self.ships[index].thresholds.push(*t);
                rolls += 1;
            }
        }
        if hull <= 0 {
            return;
        }
        for _ in 0..rolls {
            self.roll_damage(v, index, dice, events);
        }
    }

    fn roll_damage(
        &mut self,
        v: &VehicleRules,
        index: usize,
        dice: &mut dyn DiceSource,
        events: &mut Vec<BattleEvent>,
    ) {
        let Some(table) = &v.damage else { return };
        let face = table.dice.roll(dice).total.max(1) as u32;
        let Some(entry) = table
            .entries
            .iter()
            .find(|e| face >= e.from && face <= e.to)
        else {
            return;
        };
        let ship_id = self.ships[index].id.clone();
        let (mut station, mut crew) = (None, None);
        match entry.effect {
            DamageEffect::StationDown => {
                let up: Vec<&str> = v
                    .stations
                    .iter()
                    .map(|s| s.id.as_str())
                    .filter(|s| !self.ships[index].station_down(s))
                    .collect();
                if !up.is_empty() {
                    let i = dice.roll(up.len() as u32) as usize - 1;
                    station = Some(up[i].to_string());
                }
            }
            DamageEffect::Jolt => {
                if !self.crew.is_empty() {
                    let i = dice.roll(self.crew.len() as u32) as usize - 1;
                    self.crew[i].shaken += 1;
                    crew = Some(self.crew[i].id.clone());
                }
            }
            _ => {}
        }
        if entry.effect != DamageEffect::Jolt
            && (entry.effect != DamageEffect::StationDown || station.is_some())
        {
            self.ships[index].damages.push(ActiveDamage {
                id: entry.id.clone(),
                name: entry.name.clone(),
                station: station.clone(),
            });
        }
        events.push(BattleEvent::DamageRolled {
            ship: ship_id,
            face,
            damage: entry.id.clone(),
            name: entry.name.clone(),
            station,
            crew,
        });
    }

    /// A ship at 0 hull or 0 morale leaves the battle.
    fn check_out(&mut self, index: usize, events: &mut Vec<BattleEvent>) {
        let s = &mut self.ships[index];
        if !s.afloat() {
            return;
        }
        let standing = if s.hull <= 0 {
            ShipStanding::Destroyed
        } else if s.morale.is_some_and(|m| m <= 0) {
            ShipStanding::Struck
        } else {
            return;
        };
        s.standing = standing;
        events.push(BattleEvent::ShipOut {
            ship: s.id.clone(),
            standing,
        });
    }

    /// Ends the battle if a side is done.
    fn check_end(&mut self, events: &mut Vec<BattleEvent>) -> bool {
        if self.is_over() {
            return true;
        }
        let party_out = !self.party_ship().afloat();
        let enemies_out = self
            .ships
            .iter()
            .filter(|s| s.side == Side::Opposition)
            .all(|s| !s.afloat());
        let (winner, reason) = if party_out {
            (Some(Side::Opposition), BattleEndReason::Defeat)
        } else if enemies_out {
            (Some(Side::Party), BattleEndReason::Victory)
        } else {
            return false;
        };
        self.finish(winner, reason, events);
        true
    }

    fn finish(
        &mut self,
        winner: Option<Side>,
        reason: BattleEndReason,
        events: &mut Vec<BattleEvent>,
    ) {
        let end = BattleEnd {
            winner,
            reason,
            rounds: self.round,
            xp: self.xp.clone(),
        };
        events.push(BattleEvent::Ended { end: end.clone() });
        self.end = Some(end);
    }

    fn award(&mut self, system: &RuleSystem, crew: &str, band: Option<OutcomeBand>) {
        if let Some(b) = band {
            let xp = b.xp(system);
            if xp > 0 && self.crew_member(crew).is_some_and(|c| !c.npc) {
                *self.xp.entry(crew.to_string()).or_default() += xp;
            }
        }
    }

    /// Opens the current unit's turn: what lasted "until its next turn"
    /// ends, fires burn, the crew gets its actions.
    fn begin_turn(
        &mut self,
        system: &RuleSystem,
        v: &VehicleRules,
        dice: &mut dyn DiceSource,
        events: &mut Vec<BattleEvent>,
    ) {
        let Some(unit) = self.units.get(self.turn).cloned() else {
            return;
        };
        events.push(BattleEvent::TurnStarted {
            unit: unit.id.clone(),
            round: self.round,
        });
        for id in &unit.ships {
            let Ok(i) = self.ship_index(id) else { continue };
            let s = &mut self.ships[i];
            s.evade = 0;
            s.brace = 0;
            s.moved = false;
            s.fired = false;
        }
        if unit.side != Side::Party {
            return;
        }
        let p = self.party_index();
        // Fires and the like burn first.
        let burns: Vec<(String, i32)> = self.ships[p]
            .damages
            .iter()
            .filter_map(|d| {
                let e = v.damage.as_ref()?.entries.iter().find(|e| e.id == d.id)?;
                match e.effect {
                    DamageEffect::Burn { hull } => Some((d.id.clone(), hull)),
                    _ => None,
                }
            })
            .collect();
        for (damage, amount) in burns {
            let before = self.ships[p].hull;
            self.ships[p].hull = (before - amount).max(0);
            events.push(BattleEvent::Burned {
                ship: self.ships[p].id.clone(),
                damage,
                amount,
                hull_after: self.ships[p].hull,
            });
            self.hard_hits(v, p, before, false, dice, events);
            self.check_out(p, events);
        }
        if self.check_end(events) {
            return;
        }
        // Less power than allocated: the highest channels give back first.
        let points = self.power_points(v, &self.ships[p]);
        let mut total: u32 = self.ships[p].power.values().sum();
        if total > points {
            while total > points {
                let Some(top) = self.ships[p]
                    .power
                    .iter()
                    .max_by_key(|(_, n)| **n)
                    .map(|(k, _)| k.clone())
                else {
                    break;
                };
                *self.ships[p].power.get_mut(&top).expect("present") -= 1;
                total -= 1;
            }
            events.push(BattleEvent::PowerShort {
                ship: self.ships[p].id.clone(),
                points,
                power: self.ships[p].power.clone(),
            });
        }
        let ctx = system.turn_context(&v.context);
        let per_turn = ctx.map_or(2, |c| c.actions_per_turn);
        let attacks = ctx
            .and_then(|c| c.limits.iter().find(|l| l.kind == v.attack_kind))
            .map_or(per_turn, |l| l.max_per_turn);
        for c in &mut self.crew {
            c.actions = per_turn.saturating_sub(c.shaken);
            c.shaken = 0;
            c.attacks = attacks;
            c.done = c.actions == 0;
        }
    }

    /// Ends the current unit's turn and opens the next one.
    fn next_turn(
        &mut self,
        system: &RuleSystem,
        v: &VehicleRules,
        dice: &mut dyn DiceSource,
        events: &mut Vec<BattleEvent>,
    ) {
        let Some(unit) = self.units.get(self.turn).cloned() else {
            return;
        };
        for id in &unit.ships {
            if let Ok(i) = self.ship_index(id) {
                let s = &mut self.ships[i];
                if s.jam_turns > 0 {
                    s.jam_turns -= 1;
                    if s.jam_turns == 0 {
                        s.jam = 0;
                    }
                }
            }
        }
        if unit.side == Side::Party {
            self.rally = 0;
            for c in &mut self.crew {
                c.actions = 0;
                c.done = true;
            }
        }
        events.push(BattleEvent::TurnEnded { unit: unit.id });
        if self.check_end(events) {
            return;
        }
        // The next unit with a ship still afloat.
        for _ in 0..=self.units.len() {
            self.turn += 1;
            if self.turn >= self.units.len() {
                self.turn = 0;
                self.round += 1;
                if self.round > MAX_ROUNDS {
                    self.finish(None, BattleEndReason::Stalemate, events);
                    return;
                }
                events.push(BattleEvent::RoundStarted { round: self.round });
            }
            let alive = self.units[self.turn]
                .ships
                .iter()
                .any(|id| self.ship(id).is_some_and(Ship::afloat));
            if alive {
                break;
            }
        }
        self.begin_turn(system, v, dice, events);
        // A crew with nothing to do passes at once.
        if self.crew_turn() && !self.is_over() && self.crew.iter().all(|c| c.done) {
            self.next_turn(system, v, dice, events);
        }
    }

    fn open(&self) -> Result<(), BattleRefusal> {
        if self.is_over() {
            return Err(BattleRefusal::BattleOver);
        }
        if self.boarding.is_some() {
            return Err(BattleRefusal::Boarding);
        }
        Ok(())
    }

    /// A crew member changes station (paid in actions on the crew's
    /// turn).
    pub fn take_station(
        &self,
        system: &RuleSystem,
        crew: &str,
        station: Option<&str>,
    ) -> Result<Step, BattleRefusal> {
        self.open()?;
        let v = vehicles(system)?;
        if !self.crew_turn() {
            return Err(BattleRefusal::NotTheirTurn);
        }
        let ci = self.crew_index(crew)?;
        let cost = v.station_change_cost;
        if self.crew[ci].actions < cost {
            return Err(BattleRefusal::NoActionsLeft);
        }
        let mut b = self.clone();
        let mut events = Vec::new();
        b.seat_unchecked(v, ci, station, &mut events)?;
        b.crew[ci].actions -= cost;
        b.after_crew_action(system, v, ci, &mut events);
        Ok(Step { battle: b, events })
    }

    /// The GM seats a crew member, at no cost.
    pub fn seat(
        &self,
        system: &RuleSystem,
        crew: &str,
        station: Option<&str>,
    ) -> Result<Step, BattleRefusal> {
        if self.is_over() {
            return Err(BattleRefusal::BattleOver);
        }
        let v = vehicles(system)?;
        let ci = self.crew_index(crew)?;
        let mut b = self.clone();
        let mut events = Vec::new();
        b.seat_unchecked(v, ci, station, &mut events)?;
        Ok(Step { battle: b, events })
    }

    fn seat_unchecked(
        &mut self,
        v: &VehicleRules,
        ci: usize,
        station: Option<&str>,
        events: &mut Vec<BattleEvent>,
    ) -> Result<(), BattleRefusal> {
        if let Some(st) = station {
            v.station(st)
                .ok_or_else(|| BattleRefusal::UnknownStation { station: st.into() })?;
            if let Some(other) = self
                .crew
                .iter()
                .position(|c| c.station.as_deref() == Some(st))
                && other != ci
            {
                // A ship's own holder steps aside for a player.
                if self.crew[other].npc && !self.crew[ci].npc {
                    self.crew[other].station = None;
                } else {
                    return Err(BattleRefusal::StationTaken { station: st.into() });
                }
            }
        }
        self.crew[ci].station = station.map(str::to_string);
        events.push(BattleEvent::Seated {
            crew: self.crew[ci].id.clone(),
            station: self.crew[ci].station.clone(),
        });
        Ok(())
    }

    fn crew_index(&self, crew: &str) -> Result<usize, BattleRefusal> {
        self.crew
            .iter()
            .position(|c| c.id == crew)
            .ok_or_else(|| BattleRefusal::UnknownCrew { crew: crew.into() })
    }

    /// A crew member is done for this turn.
    pub fn pass(
        &self,
        system: &RuleSystem,
        crew: &str,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, BattleRefusal> {
        self.open()?;
        let v = vehicles(system)?;
        if !self.crew_turn() {
            return Err(BattleRefusal::NotTheirTurn);
        }
        let ci = self.crew_index(crew)?;
        let mut b = self.clone();
        let mut events = Vec::new();
        b.crew[ci].actions = 0;
        b.crew[ci].done = true;
        b.after_crew_action(system, v, ci, &mut events);
        if b.crew.iter().all(|c| c.done) && !b.is_over() {
            b.next_turn(system, v, dice, &mut events);
        }
        Ok(Step { battle: b, events })
    }

    fn after_crew_action(
        &mut self,
        _system: &RuleSystem,
        _v: &VehicleRules,
        ci: usize,
        _events: &mut [BattleEvent],
    ) {
        if self.crew[ci].actions == 0 {
            self.crew[ci].done = true;
        }
    }

    /// A crew member's station action, on the party ship's turn.
    pub fn crew_act(
        &self,
        system: &RuleSystem,
        crew: &str,
        action: &str,
        aim: &Aim,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, BattleRefusal> {
        self.open()?;
        let v = vehicles(system)?;
        if !self.crew_turn() {
            return Err(BattleRefusal::NotTheirTurn);
        }
        let ci = self.crew_index(crew)?;
        let member = &self.crew[ci];
        let (station, act) = v
            .action(action)
            .ok_or_else(|| BattleRefusal::UnknownAction {
                action: action.into(),
            })?;
        if member.station.as_deref() != Some(station.id.as_str()) {
            return Err(BattleRefusal::NotAtStation {
                station: station.id.clone(),
            });
        }
        let p = self.party_index();
        if self.ships[p].station_down(&station.id) {
            return Err(BattleRefusal::StationDown {
                station: station.id.clone(),
            });
        }
        if member.actions < act.cost {
            return Err(BattleRefusal::NoActionsLeft);
        }
        if act.attack && member.attacks == 0 {
            return Err(BattleRefusal::NoAttackLeft);
        }
        if act.cost > 1
            && matches!(act.effect, VehicleEffect::Fire { .. })
            && self.power_flag(v, &self.ships[p], |e| e.no_heavy)
        {
            return Err(BattleRefusal::HeavyRefused);
        }
        let mut b = self.clone();
        let mut events = Vec::new();
        b.resolve(
            system,
            v,
            ci,
            station.ability.as_str(),
            act,
            aim,
            dice,
            &mut events,
        )?;
        let m = &mut b.crew[ci];
        m.actions -= act.cost;
        if act.attack {
            m.attacks -= 1;
        }
        b.after_crew_action(system, v, ci, &mut events);
        b.check_end(&mut events);
        if !b.is_over() && b.crew.iter().all(|c| c.done) {
            b.next_turn(system, v, dice, &mut events);
        }
        Ok(Step { battle: b, events })
    }

    /// A roll of the crew member in `ability` against `target`: rally
    /// included and spent, XP earned.
    fn crew_roll(
        &mut self,
        system: &RuleSystem,
        ci: usize,
        ability: &str,
        target: RollTarget,
        dice: &mut dyn DiceSource,
    ) -> RollBreakdown {
        let mut mods = vec![Modifier {
            source: ModifierSource::Ability(ability.into()),
            value: self.crew[ci].modifier(ability),
        }];
        if self.rally != 0 {
            mods.push(Modifier {
                source: ModifierSource::Situation("rally".into()),
                value: self.rally,
            });
            self.rally = 0;
        }
        let roll = check::roll(system, mods, Advantage::Normal, Some(target), dice)
            .expect("a plain roll never fails");
        let id = self.crew[ci].id.clone();
        self.award(system, &id, roll.band);
        roll
    }

    fn enemy_target(&self, aim: &Aim) -> Result<usize, BattleRefusal> {
        let id = aim.target.as_deref().ok_or(BattleRefusal::WrongTarget)?;
        let i = self.ship_index(id)?;
        if self.ships[i].side == Side::Party || !self.ships[i].afloat() {
            return Err(BattleRefusal::WrongTarget);
        }
        Ok(i)
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve(
        &mut self,
        system: &RuleSystem,
        v: &VehicleRules,
        ci: usize,
        ability: &str,
        act: &StationAction,
        aim: &Aim,
        dice: &mut dyn DiceSource,
        events: &mut Vec<BattleEvent>,
    ) -> Result<(), BattleRefusal> {
        let p = self.party_index();
        let by = self.crew[ci].id.clone();
        let party_id = self.ships[p].id.clone();
        // Check first what needs no roll to be refused.
        match &act.effect {
            VehicleEffect::Maneuver { multiplier, turns } => {
                let to = aim.to.unwrap_or(self.ships[p].at);
                // Validate on a copy before any roll.
                let mut probe = self.clone();
                probe.move_ship(v, p, *multiplier, *turns, to, aim.facing, &mut Vec::new())?;
            }
            VehicleEffect::Fire { .. } => {
                let w = self.station_weapon(v, act, aim, &self.ships[p].kind)?;
                let t = self.enemy_target(aim)?;
                self.can_fire(v, &self.ships[p], &w, &self.ships[t])?;
            }
            VehicleEffect::Lock { .. } | VehicleEffect::Scan | VehicleEffect::Jam { .. } => {
                self.enemy_target(aim)?;
            }
            VehicleEffect::BreakMorale { .. } => {
                let t = self.enemy_target(aim)?;
                if self.ships[t].morale.is_none() {
                    return Err(BattleRefusal::Immune);
                }
            }
            VehicleEffect::Recharge { .. } => {
                if self.power_flag(v, &self.ships[p], |e| e.screen_offline)
                    || self.ships[p].max_screen == 0
                {
                    return Err(BattleRefusal::ScreenOffline);
                }
            }
            VehicleEffect::Reroute => {
                let power = aim.power.as_ref().ok_or(BattleRefusal::WrongPower)?;
                let def = v.power.as_ref().ok_or(BattleRefusal::WrongPower)?;
                let total: u32 = power.values().sum();
                let fits = def.channels.iter().all(|c| {
                    power
                        .get(&c.id)
                        .is_some_and(|n| (*n as usize) < c.levels.len())
                }) && power.len() == def.channels.len();
                if !fits || total > self.power_points(v, &self.ships[p]) {
                    return Err(BattleRefusal::WrongPower);
                }
            }
            VehicleEffect::Fix { damages } => {
                let d = aim
                    .damage
                    .and_then(|i| self.ships[p].damages.get(i))
                    .ok_or(BattleRefusal::UnknownDamage)?;
                if !damages.contains(&d.id) {
                    return Err(BattleRefusal::NotFixableHere);
                }
            }
            VehicleEffect::Disengage { min_distance } => {
                let close = self
                    .ships
                    .iter()
                    .filter(|s| s.side == Side::Opposition && s.afloat())
                    .any(|s| distance(s.at, self.ships[p].at) < *min_distance);
                if close {
                    return Err(BattleRefusal::TooClose);
                }
            }
            _ => {}
        }
        // The action's own check.
        if let Some(dc) = act.check {
            let roll = self.crew_roll(
                system,
                ci,
                ability,
                RollTarget::Difficulty {
                    id: None,
                    value: dc,
                },
                dice,
            );
            let ok = roll.band.is_some_and(OutcomeBand::is_success);
            events.push(BattleEvent::Check {
                by: by.clone(),
                action: act.id.clone(),
                roll,
            });
            if !ok {
                return Ok(());
            }
        }
        match &act.effect {
            VehicleEffect::Maneuver { multiplier, turns } => {
                let to = aim.to.unwrap_or(self.ships[p].at);
                self.move_ship(v, p, *multiplier, *turns, to, aim.facing, events)?;
            }
            VehicleEffect::Evade { armor } => {
                if self.power_flag(v, &self.ships[p], |e| e.no_evade) {
                    return Err(BattleRefusal::CannotMove);
                }
                self.ships[p].evade = *armor;
                events.push(BattleEvent::Evading {
                    ship: party_id,
                    armor: *armor + self.power_level(v, &self.ships[p], |e| e.evade),
                });
            }
            VehicleEffect::Brace { damage_taken } => {
                self.ships[p].brace = *damage_taken;
                events.push(BattleEvent::Braced {
                    ship: party_id,
                    damage_taken: *damage_taken,
                });
            }
            VehicleEffect::Fire { damage } => {
                let w = self.station_weapon(v, act, aim, &self.ships[p].kind)?;
                let t = self.enemy_target(aim)?;
                let member = self.crew[ci].clone();
                let band = self.shoot(system, v, p, &w, Some(&member), *damage, t, dice, events);
                self.award(system, &by, band);
            }
            VehicleEffect::Recharge { amount } => {
                let amount = self.power_recharge(v, &self.ships[p]).unwrap_or(*amount) as i32;
                let s = &mut self.ships[p];
                s.screen = (s.screen + amount).min(s.max_screen);
                events.push(BattleEvent::Recharged {
                    ship: party_id,
                    screen: s.screen,
                });
            }
            VehicleEffect::Repair { amount, screen } => {
                let amount = *amount as i32 + self.power_level(v, &self.ships[p], |e| e.repair);
                let s = &mut self.ships[p];
                if *screen {
                    s.screen = (s.screen + amount).min(s.max_screen);
                } else {
                    s.hull = (s.hull + amount).min(s.max_hull);
                }
                events.push(BattleEvent::Repaired {
                    ship: party_id,
                    by,
                    amount,
                    screen: *screen,
                });
            }
            VehicleEffect::Reroute => {
                let power = aim.power.clone().unwrap_or_default();
                self.ships[p].power = power.clone();
                events.push(BattleEvent::Rerouted {
                    ship: party_id,
                    power,
                });
            }
            VehicleEffect::Lock { bonus } => {
                let t = self.enemy_target(aim)?;
                self.ships[t].locked = *bonus;
                events.push(BattleEvent::Locked {
                    by,
                    target: self.ships[t].id.clone(),
                    bonus: *bonus,
                });
            }
            VehicleEffect::Scan => {
                let t = self.enemy_target(aim)?;
                self.ships[t].scanned = true;
                events.push(BattleEvent::Scanned {
                    by,
                    target: self.ships[t].id.clone(),
                });
            }
            VehicleEffect::Jam { malus } => {
                let t = self.enemy_target(aim)?;
                self.ships[t].jam = *malus;
                self.ships[t].jam_turns = 1;
                events.push(BattleEvent::Jammed {
                    by,
                    target: self.ships[t].id.clone(),
                    malus: *malus,
                });
            }
            VehicleEffect::BreakMorale { amount } => {
                let t = self.enemy_target(aim)?;
                let resolve = self.ships[t].resolve;
                let roll = self.crew_roll(
                    system,
                    ci,
                    ability,
                    RollTarget::Opposed { value: resolve },
                    dice,
                );
                let loss = match roll.band {
                    Some(OutcomeBand::CriticalSuccess) => {
                        amount
                            * system
                                .outcomes
                                .critical_success
                                .damage_multiplier
                                .unwrap_or(1)
                    }
                    Some(OutcomeBand::Success) => *amount,
                    _ => 0,
                };
                let s = &mut self.ships[t];
                if let Some(m) = s.morale.as_mut() {
                    *m = (*m - loss).max(0);
                }
                events.push(BattleEvent::MoraleRoll {
                    by,
                    target: s.id.clone(),
                    roll,
                    loss,
                    morale_after: s.morale,
                });
                self.check_out(t, events);
            }
            VehicleEffect::Rally { bonus } => {
                self.rally = *bonus;
                events.push(BattleEvent::Rallied { by, bonus: *bonus });
            }
            VehicleEffect::Fix { .. } => {
                let i = aim.damage.ok_or(BattleRefusal::UnknownDamage)?;
                let d = self.ships[p].damages[i].clone();
                let fix = v
                    .damage
                    .as_ref()
                    .and_then(|t| t.entries.iter().find(|e| e.id == d.id))
                    .and_then(|e| e.fix.clone());
                let ok = match fix {
                    Some(f) => {
                        let roll = self.crew_roll(
                            system,
                            ci,
                            &f.ability,
                            RollTarget::Difficulty {
                                id: None,
                                value: f.difficulty,
                            },
                            dice,
                        );
                        let ok = roll.band.is_some_and(OutcomeBand::is_success);
                        events.push(BattleEvent::Check {
                            by: by.clone(),
                            action: act.id.clone(),
                            roll,
                        });
                        ok
                    }
                    None => true,
                };
                if ok {
                    self.ships[p].damages.remove(i);
                    events.push(BattleEvent::DamageFixed {
                        ship: party_id,
                        damage: d.id,
                        by,
                    });
                }
            }
            VehicleEffect::Disengage { .. } => {
                let s = &mut self.ships[p];
                s.standing = ShipStanding::Fled;
                events.push(BattleEvent::ShipOut {
                    ship: party_id,
                    standing: ShipStanding::Fled,
                });
                self.finish(None, BattleEndReason::Disengaged, events);
            }
            VehicleEffect::Narrative => {
                events.push(BattleEvent::ForTheGm {
                    by,
                    action: act.id.clone(),
                });
            }
        }
        Ok(())
    }

    /// The weapon a fire action shoots: the one aimed, else the
    /// station's only one.
    fn station_weapon(
        &self,
        v: &VehicleRules,
        act: &StationAction,
        aim: &Aim,
        ship_kind: &str,
    ) -> Result<WeaponDef, BattleRefusal> {
        let def = v
            .ship(ship_kind)
            .ok_or_else(|| BattleRefusal::UnknownShip {
                ship: ship_kind.into(),
            })?;
        let (station, _) = v
            .action(&act.id)
            .ok_or_else(|| BattleRefusal::UnknownAction {
                action: act.id.clone(),
            })?;
        let mine: Vec<&WeaponDef> = def
            .weapons
            .iter()
            .filter(|w| w.station.as_deref() == Some(station.id.as_str()))
            .collect();
        let w = match &aim.weapon {
            Some(id) => mine.into_iter().find(|w| &w.id == id),
            None if mine.len() == 1 => mine.into_iter().next(),
            None => None,
        };
        w.cloned().ok_or_else(|| BattleRefusal::UnknownWeapon {
            weapon: aim.weapon.clone().unwrap_or_default(),
        })
    }

    fn enemy_turn(&self, ship: &str) -> Result<usize, BattleRefusal> {
        self.open()?;
        let unit = self.active().ok_or(BattleRefusal::BattleOver)?;
        if unit.side != Side::Opposition || !unit.ships.iter().any(|s| s == ship) {
            return Err(BattleRefusal::NotTheirTurn);
        }
        let i = self.ship_index(ship)?;
        if !self.ships[i].afloat() {
            return Err(BattleRefusal::NotTheirTurn);
        }
        Ok(i)
    }

    /// An enemy ship's manoeuvre (once a turn): its speed in cells, a
    /// quarter turn of the bow.
    pub fn enemy_maneuver(
        &self,
        system: &RuleSystem,
        ship: &str,
        to: Cell,
        facing: Facing,
    ) -> Result<Step, BattleRefusal> {
        let v = vehicles(system)?;
        let i = self.enemy_turn(ship)?;
        if self.ships[i].moved {
            return Err(BattleRefusal::AlreadyMoved);
        }
        let mut b = self.clone();
        let mut events = Vec::new();
        b.move_ship(v, i, 1, 1, to, Some(facing), &mut events)?;
        b.ships[i].moved = true;
        Ok(Step { battle: b, events })
    }

    /// An enemy ship's shot (once a turn).
    pub fn enemy_fire(
        &self,
        system: &RuleSystem,
        ship: &str,
        weapon: &str,
        target: &str,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, BattleRefusal> {
        let v = vehicles(system)?;
        let i = self.enemy_turn(ship)?;
        if self.ships[i].fired {
            return Err(BattleRefusal::AlreadyFired);
        }
        let def = v
            .ship(&self.ships[i].kind)
            .ok_or_else(|| BattleRefusal::UnknownShip {
                ship: self.ships[i].kind.clone(),
            })?;
        let w = def
            .weapons
            .iter()
            .find(|w| w.id == weapon)
            .ok_or_else(|| BattleRefusal::UnknownWeapon {
                weapon: weapon.into(),
            })?
            .clone();
        let t = self.ship_index(target)?;
        self.can_fire(v, &self.ships[i], &w, &self.ships[t])?;
        let mut b = self.clone();
        let mut events = Vec::new();
        b.shoot(system, v, i, &w, None, None, t, dice, &mut events);
        b.ships[i].fired = true;
        b.check_end(&mut events);
        Ok(Step { battle: b, events })
    }

    /// Ends the current turn (the GM, or an enemy ship done).
    pub fn end_turn(
        &self,
        system: &RuleSystem,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, BattleRefusal> {
        self.open()?;
        let v = vehicles(system)?;
        let mut b = self.clone();
        let mut events = Vec::new();
        b.next_turn(system, v, dice, &mut events);
        Ok(Step { battle: b, events })
    }

    /// The GM sets a ship's gauges.
    pub fn adjust(
        &self,
        ship: &str,
        hull: Option<i32>,
        screen: Option<i32>,
        morale: Option<i32>,
    ) -> Result<Step, BattleRefusal> {
        if self.is_over() {
            return Err(BattleRefusal::BattleOver);
        }
        let i = self.ship_index(ship)?;
        let mut b = self.clone();
        let s = &mut b.ships[i];
        if let Some(h) = hull {
            s.hull = h.clamp(0, s.max_hull);
        }
        if let Some(x) = screen {
            s.screen = x.clamp(0, s.max_screen);
        }
        if let (Some(m), Some(max)) = (morale, s.max_morale) {
            s.morale = Some(m.clamp(0, max));
        }
        let mut events = vec![BattleEvent::Adjusted { ship: ship.into() }];
        b.check_out(i, &mut events);
        b.check_end(&mut events);
        Ok(Step { battle: b, events })
    }

    /// The GM takes a ship out (it strikes, flees, sinks).
    pub fn strike(&self, ship: &str, standing: ShipStanding) -> Result<Step, BattleRefusal> {
        if self.is_over() {
            return Err(BattleRefusal::BattleOver);
        }
        let i = self.ship_index(ship)?;
        let mut b = self.clone();
        b.ships[i].standing = standing;
        let mut events = vec![BattleEvent::ShipOut {
            ship: ship.into(),
            standing,
        }];
        b.check_end(&mut events);
        Ok(Step { battle: b, events })
    }

    /// The GM turns the wind (or the pull).
    pub fn set_current(&self, from: Option<Direction>) -> Result<Step, BattleRefusal> {
        if self.is_over() {
            return Err(BattleRefusal::BattleOver);
        }
        let mut b = self.clone();
        b.current = from;
        Ok(Step {
            battle: b,
            events: vec![BattleEvent::CurrentChanged { from }],
        })
    }

    /// Grapples: `attacker` boards `defender`, within boarding range.
    /// The battle waits for the fight on the deck.
    pub fn board(
        &self,
        system: &RuleSystem,
        attacker: &str,
        defender: &str,
    ) -> Result<Step, BattleRefusal> {
        self.open()?;
        let v = vehicles(system)?;
        let (a, d) = (self.ship_index(attacker)?, self.ship_index(defender)?);
        let (sa, sd) = (&self.ships[a], &self.ships[d]);
        if sa.side == sd.side || !sa.afloat() || !sd.afloat() {
            return Err(BattleRefusal::WrongTarget);
        }
        if distance(sa.at, sd.at) > v.boarding_range {
            return Err(BattleRefusal::OutOfRange);
        }
        let mut b = self.clone();
        b.boarding = Some(Boarding {
            attacker: attacker.into(),
            defender: defender.into(),
        });
        Ok(Step {
            battle: b,
            events: vec![BattleEvent::BoardingStarted {
                attacker: attacker.into(),
                defender: defender.into(),
            }],
        })
    }

    /// The fight on the deck is over: the loser's ship is taken when the
    /// boarders won, the boarders are thrown back otherwise.
    pub fn end_boarding(&self, attacker_won: bool) -> Result<Step, BattleRefusal> {
        if self.is_over() {
            return Err(BattleRefusal::BattleOver);
        }
        let boarding = self.boarding.clone().ok_or(BattleRefusal::NotBoarding)?;
        let mut b = self.clone();
        b.boarding = None;
        let mut events = vec![BattleEvent::BoardingEnded {
            attacker: boarding.attacker.clone(),
            defender: boarding.defender.clone(),
            attacker_won,
        }];
        if attacker_won {
            let d = b.ship_index(&boarding.defender)?;
            b.ships[d].standing = ShipStanding::Captured;
            events.push(BattleEvent::ShipOut {
                ship: boarding.defender,
                standing: ShipStanding::Captured,
            });
        }
        b.check_end(&mut events);
        Ok(Step { battle: b, events })
    }

    /// The GM ends the battle now.
    pub fn stop(&self) -> Step {
        let mut b = self.clone();
        let mut events = Vec::new();
        if !b.is_over() {
            b.boarding = None;
            b.finish(None, BattleEndReason::StoppedByGm, &mut events);
        }
        Step { battle: b, events }
    }

    /// Weapons of `ship` that can hit `target` now.
    pub fn weapons_on<'a>(
        &self,
        v: &'a VehicleRules,
        ship: &Ship,
        target: &Ship,
    ) -> Vec<&'a WeaponDef> {
        v.ship(&ship.kind)
            .map(|d| {
                d.weapons
                    .iter()
                    .filter(|w| self.can_fire(v, ship, w, target).is_ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether the party ship's power allows heavy shots now.
    pub fn heavy_allowed(&self, v: &VehicleRules) -> bool {
        !self.power_flag(v, self.party_ship(), |e| e.no_heavy)
    }

    /// The crew members, seated or not, that are players.
    pub fn players(&self) -> impl Iterator<Item = &Crew> {
        self.crew.iter().filter(|c| !c.npc)
    }

    /// Stations of the party ship that are out of service.
    pub fn stations_down(&self) -> BTreeSet<String> {
        self.party_ship()
            .damages
            .iter()
            .filter_map(|d| d.station.clone())
            .collect()
    }
}

fn new_ship(v: &VehicleRules, def: &ShipDef, p: &Placed, side: Side) -> Ship {
    Ship {
        id: p.id.clone(),
        kind: def.id.clone(),
        name: p.name.clone().unwrap_or_else(|| def.name.clone()),
        side,
        at: p.at,
        facing: p.facing,
        hull: def.hull,
        max_hull: def.hull,
        screen: def.screen,
        max_screen: def.screen,
        armor: def.armor,
        morale: def.morale,
        max_morale: def.morale,
        resolve: def.resolve,
        power: if side == Side::Party {
            v.start_power()
        } else {
            BTreeMap::new()
        },
        evade: 0,
        brace: 0,
        jam: 0,
        jam_turns: 0,
        locked: 0,
        damages: Vec::new(),
        thresholds: Vec::new(),
        scanned: false,
        standing: ShipStanding::Afloat,
        moved: false,
        fired: false,
    }
}
