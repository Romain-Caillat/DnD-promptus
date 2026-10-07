//! A ship fight: ships on an open grid (space or sea), each with a bow;
//! a crew of players acting together at their stations, the GM's ships
//! acting at their own initiative. Pure and deterministic: dice come
//! from a [`DiceSource`].
//!
//! The server stays authoritative: every order is checked here (the
//! crew member is at the station, the station is in service, two actions
//! and one attack a turn, the target in the weapon's arc and range)
//! before anything moves, and a refused turn changes nothing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::model::{
    Allocation, Arc, DamageKind, EnergyLevel, ShipActionDef, ShipCombat, ShipDef, ShipEffect,
    WeaponDef,
};
use crate::rules::dice::DiceSource;

/// Where a bow points. North is towards smaller `y`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    North,
    East,
    South,
    West,
}

impl Facing {
    pub const ALL: [Facing; 4] = [Facing::North, Facing::East, Facing::South, Facing::West];

    fn index(self) -> i32 {
        match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
        }
    }

    /// Quarter turns between two facings (0, 1 or 2).
    #[must_use]
    pub fn turns_to(self, other: Facing) -> u32 {
        let d = (other.index() - self.index()).rem_euclid(4);
        d.min(4 - d).unsigned_abs()
    }

    /// The unit step ahead.
    #[must_use]
    pub fn ahead(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }
}

/// Cells between two positions: diagonal steps count one (§4).
#[must_use]
pub fn distance(a: (i32, i32), b: (i32, i32)) -> u32 {
    (a.0 - b.0).unsigned_abs().max((a.1 - b.1).unsigned_abs())
}

/// The arc of `at` seen from a ship at `from` facing `facing`. A cell
/// on a diagonal belongs to the flank (INTERPRETATION: the bow and the
/// stern arcs are the strict quarters ahead and behind, so the blind
/// spot is narrow, as « le pur arrière » says).
#[must_use]
pub fn arc_of(from: (i32, i32), facing: Facing, at: (i32, i32)) -> Arc {
    let (dx, dy) = (at.0 - from.0, at.1 - from.1);
    let (ax, ay) = facing.ahead();
    // Forward component, and rightwards (starboard) component.
    let fwd = dx * ax + dy * ay;
    let right = -dx * ay + dy * ax;
    if (fwd == 0 && right == 0) || fwd > right.abs() {
        Arc::Front
    } else if -fwd > right.abs() {
        Arc::Rear
    } else if right > 0 {
        Arc::Starboard
    } else {
        Arc::Port
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipSide {
    Party,
    Opposition,
}

/// Why a ship left the fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Out {
    /// Hull at 0.
    Destroyed,
    /// Morale at 0: flees, surrenders, breaks off (§11).
    Broken,
}

/// A crew member and the station they hold.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CrewMember {
    pub id: String,
    pub name: String,
    /// Ability modifiers (`DEX` → +2).
    pub modifiers: BTreeMap<String, i32>,
    pub station: String,
}

/// An ongoing damage (§8).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActiveDamage {
    pub kind: DamageKind,
    pub name: String,
    pub hull_per_turn: i32,
    /// The station out of service (`SystemDown`).
    pub station: Option<String>,
}

/// A ship in the fight.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ship {
    /// This ship in this fight (`chasseur_2`).
    pub key: String,
    /// Its sheet in the rules.
    pub def: String,
    pub name: String,
    pub side: ShipSide,
    /// Ships of one group (a swarm) share an initiative (§9).
    pub group: Option<String>,
    pub at: (i32, i32),
    pub facing: Facing,
    pub hull: i32,
    pub shields: i32,
    pub morale: Option<i32>,
    pub energy: Allocation,
    /// Until its next turn: armour from evading, damage off from bracing,
    /// to-hit lost to jamming.
    pub evade: i32,
    pub brace: i32,
    pub jammed: i32,
    /// To-hit bonus the next shot at it gets (a lock).
    pub locked: i32,
    pub damages: Vec<ActiveDamage>,
    pub crew: Vec<CrewMember>,
    /// Crew members who lose an action next turn (a jolt).
    pub shaken: Vec<String>,
    pub out: Option<Out>,
}

impl Ship {
    /// A ship at its sheet's values, the energy at the rule's start.
    #[must_use]
    pub fn new(rules: &ShipCombat, def: &ShipDef, key: &str, side: ShipSide) -> Self {
        Self {
            key: key.to_string(),
            def: def.id.clone(),
            name: def.name.clone(),
            side,
            group: None,
            at: (0, 0),
            facing: Facing::North,
            hull: def.hull,
            shields: def.shields,
            morale: def.morale,
            energy: if def.crewed {
                rules.energy.start
            } else {
                Allocation::default()
            },
            evade: 0,
            brace: 0,
            jammed: 0,
            locked: 0,
            damages: Vec::new(),
            crew: Vec::new(),
            shaken: Vec::new(),
            out: None,
        }
    }

    #[must_use]
    pub fn in_play(&self) -> bool {
        self.out.is_none()
    }

    /// The station is out of service.
    #[must_use]
    pub fn station_down(&self, station: &str) -> bool {
        self.damages
            .iter()
            .any(|d| d.kind == DamageKind::SystemDown && d.station.as_deref() == Some(station))
    }
}

/// One gesture of a crew member.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Order {
    pub action: String,
    /// A ship (fire, lock, jam, break morale).
    #[serde(default)]
    pub target: Option<String>,
    /// Where to (manoeuvre).
    #[serde(default)]
    pub to: Option<(i32, i32)>,
    #[serde(default)]
    pub facing: Option<Facing>,
    /// The station to go to (move) or to repair (repair a system).
    #[serde(default)]
    pub station: Option<String>,
    /// The weapon, when the station has several.
    #[serde(default)]
    pub weapon: Option<String>,
    /// The new split (reroute).
    #[serde(default)]
    pub energy: Option<Allocation>,
}

/// What a crew member does this turn, in order.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CrewOrders {
    pub crew: String,
    pub orders: Vec<Order>,
}

/// An order the rules refuse; the turn changes nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipError {
    pub code: &'static str,
    pub detail: String,
}

fn refuse(code: &'static str, detail: impl Into<String>) -> ShipError {
    ShipError {
        code,
        detail: detail.into(),
    }
}

/// What happened, for the log and the screens.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ShipEvent {
    Initiative {
        ships: Vec<String>,
        roll: i32,
    },
    Maneuver {
        ship: String,
        to: (i32, i32),
        facing: Facing,
    },
    Shot {
        ship: String,
        weapon: String,
        target: String,
        roll: u32,
        total: i32,
        armor: i32,
        hit: bool,
        critical: bool,
        damage: i32,
    },
    Absorbed {
        ship: String,
        shields: i32,
        hull: i32,
    },
    Damage {
        ship: String,
        name: String,
        kind: DamageKind,
        station: Option<String>,
    },
    Burn {
        ship: String,
        hull: i32,
    },
    Check {
        ship: String,
        crew: String,
        action: String,
        roll: u32,
        total: i32,
        difficulty: i32,
        success: bool,
    },
    Effect {
        ship: String,
        crew: String,
        action: String,
    },
    Morale {
        ship: String,
        from: i32,
        to: i32,
    },
    Out {
        ship: String,
        why: Out,
    },
    /// The target left the fight earlier in the turn: the action is
    /// spent for nothing.
    Wasted {
        ship: String,
        crew: String,
        action: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipEnd {
    /// Every enemy ship destroyed or broken.
    Victory,
    /// The party's ship destroyed.
    Defeat,
}

/// The fight: ships, the initiative order (groups of ships acting
/// together), whose turn it is.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShipFight {
    pub ships: Vec<Ship>,
    /// Indexes into `ships`, one group per initiative rank.
    pub order: Vec<Vec<usize>>,
    pub round: u32,
    pub turn: usize,
    pub ended: Option<ShipEnd>,
    pub actions_per_turn: u32,
    pub attacks_per_turn: u32,
}

/// The effects of a channel's points (nominal for a ship without
/// energy).
fn level(levels: &[EnergyLevel], ship: &Ship, points: u32) -> EnergyLevel {
    if ship.crew.is_empty() {
        return EnergyLevel::default();
    }
    levels
        .get(points as usize)
        .or(levels.last())
        .copied()
        .unwrap_or_default()
}

/// The die a check rolls.
const D20: u32 = 20;

impl ShipFight {
    /// Roll initiative (§3): the helm's DEX for a crewed ship, the
    /// sheet's bonus otherwise; a group rolls once. Highest first, the
    /// party first on a tie.
    #[must_use]
    pub fn start(
        rules: &ShipCombat,
        ships: Vec<Ship>,
        dice: &mut dyn DiceSource,
        events: &mut Vec<ShipEvent>,
    ) -> Self {
        let mut ranked: Vec<(i32, bool, Vec<usize>)> = Vec::new();
        let mut grouped: BTreeMap<String, usize> = BTreeMap::new();
        for (i, s) in ships.iter().enumerate() {
            if let Some(g) = &s.group
                && let Some(&r) = grouped.get(g)
            {
                ranked[r].2.push(i);
                continue;
            }
            let bonus = if s.crew.is_empty() {
                rules.ship(&s.def).map_or(0, |d| d.initiative_bonus)
            } else {
                let helm = rules.stations.iter().find(|st| st.helm);
                helm.and_then(|h| s.crew.iter().find(|c| c.station == h.id))
                    .and_then(|c| c.modifiers.get("DEX").copied())
                    .unwrap_or(0)
            };
            let roll = i32::try_from(dice.roll(D20)).unwrap_or(0) + bonus;
            if let Some(g) = &s.group {
                grouped.insert(g.clone(), ranked.len());
            }
            ranked.push((roll, s.side == ShipSide::Party, vec![i]));
        }
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        for (roll, _, group) in &ranked {
            events.push(ShipEvent::Initiative {
                ships: group.iter().map(|&i| ships[i].key.clone()).collect(),
                roll: *roll,
            });
        }
        let mut fight = Self {
            ships,
            order: ranked.into_iter().map(|r| r.2).collect(),
            round: 1,
            turn: 0,
            ended: None,
            actions_per_turn: 2,
            attacks_per_turn: 1,
        };
        fight.begin_turn(rules, dice, events);
        fight
    }

    /// The ships whose turn it is.
    #[must_use]
    pub fn acting(&self) -> &[usize] {
        self.order.get(self.turn).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn ship(&self, key: &str) -> Option<usize> {
        self.ships.iter().position(|s| s.key == key)
    }

    /// The reactor's points now (§6).
    #[must_use]
    pub fn reactor(&self, rules: &ShipCombat, ship: usize) -> u32 {
        let s = &self.ships[ship];
        let hit = rules
            .stations
            .iter()
            .any(|st| st.reactor && s.station_down(&st.id));
        let lost = if hit { rules.energy.reactor_hit } else { 0 };
        rules.reactor(s.hull).saturating_sub(lost)
    }

    /// Start of a turn: what lasted « until its next turn » ends, fires
    /// burn (§3), the energy fits the reactor.
    fn begin_turn(
        &mut self,
        rules: &ShipCombat,
        dice: &mut dyn DiceSource,
        ev: &mut Vec<ShipEvent>,
    ) {
        for i in self.acting().to_vec() {
            let s = &mut self.ships[i];
            if !s.in_play() {
                continue;
            }
            s.evade = 0;
            s.brace = 0;
            let burn: i32 = s.damages.iter().map(|d| d.hull_per_turn).sum();
            if burn > 0 {
                ev.push(ShipEvent::Burn {
                    ship: s.key.clone(),
                    hull: burn,
                });
                self.lose_hull(rules, i, burn, false, dice, ev);
            }
            if !self.ships[i].crew.is_empty() {
                let cap = self.reactor(rules, i);
                let e = &mut self.ships[i].energy;
                // INTERPRETATION: a reactor that drops cuts navigation
                // first, then weapons, then shields.
                while e.total() > cap {
                    if e.navigation > 0 {
                        e.navigation -= 1;
                    } else if e.weapons > 0 {
                        e.weapons -= 1;
                    } else {
                        e.shields -= 1;
                    }
                }
            }
        }
        self.check_end();
    }

    /// Next group's turn (a new round after the last).
    pub fn end_turn(
        &mut self,
        rules: &ShipCombat,
        dice: &mut dyn DiceSource,
        ev: &mut Vec<ShipEvent>,
    ) {
        if self.ended.is_some() {
            return;
        }
        for i in self.acting().to_vec() {
            // Jamming lasts through the jammed ship's own turn.
            self.ships[i].shaken.clear();
            self.ships[i].jammed = 0;
        }
        self.turn += 1;
        if self.turn >= self.order.len() {
            self.turn = 0;
            self.round += 1;
        }
        self.begin_turn(rules, dice, ev);
    }

    fn check_end(&mut self) {
        if self.ended.is_some() {
            return;
        }
        let alive = |side| self.ships.iter().any(|s| s.side == side && s.in_play());
        if !alive(ShipSide::Party) {
            self.ended = Some(ShipEnd::Defeat);
        } else if !alive(ShipSide::Opposition) {
            self.ended = Some(ShipEnd::Victory);
        }
    }

    /// Hull lost (after shields): thresholds and destruction.
    fn lose_hull(
        &mut self,
        rules: &ShipCombat,
        i: usize,
        amount: i32,
        critical: bool,
        dice: &mut dyn DiceSource,
        ev: &mut Vec<ShipEvent>,
    ) {
        let s = &mut self.ships[i];
        let before = s.hull;
        s.hull = (s.hull - amount).max(0);
        let after = s.hull;
        let crewed = !s.crew.is_empty();
        if after == 0 && s.out.is_none() {
            s.out = Some(Out::Destroyed);
            ev.push(ShipEvent::Out {
                ship: s.key.clone(),
                why: Out::Destroyed,
            });
            self.check_end();
            return;
        }
        if !crewed {
            return;
        }
        let crossed = rules
            .damage_table
            .hull_thresholds
            .iter()
            .filter(|&&t| before > t && after <= t)
            .count();
        for _ in 0..crossed + usize::from(critical) {
            self.roll_damage(rules, i, dice, ev);
        }
    }

    /// One roll on the damage table (§8).
    fn roll_damage(
        &mut self,
        rules: &ShipCombat,
        i: usize,
        dice: &mut dyn DiceSource,
        ev: &mut Vec<ShipEvent>,
    ) {
        let face = dice.roll(rules.damage_table.die.max(1));
        let Some(r) = rules
            .damage_table
            .results
            .iter()
            .find(|r| r.faces.contains(&face))
        else {
            return;
        };
        let s = &mut self.ships[i];
        let mut station = None;
        match r.kind {
            DamageKind::SystemDown => {
                let n = u32::try_from(rules.stations.len()).unwrap_or(1).max(1);
                let pick = dice.roll(n) as usize - 1;
                station = rules.stations.get(pick).map(|st| st.id.clone());
            }
            DamageKind::Shake => {
                if !s.crew.is_empty() {
                    let n = u32::try_from(s.crew.len()).unwrap_or(1);
                    let pick = dice.roll(n) as usize - 1;
                    s.shaken.push(s.crew[pick].id.clone());
                }
            }
            DamageKind::Fire | DamageKind::Breach => {}
        }
        ev.push(ShipEvent::Damage {
            ship: s.key.clone(),
            name: r.name.clone(),
            kind: r.kind,
            station: station.clone(),
        });
        if r.kind != DamageKind::Shake {
            s.damages.push(ActiveDamage {
                kind: r.kind,
                name: r.name.clone(),
                hull_per_turn: r.hull_per_turn,
                station,
            });
        }
    }

    /// A weapon of `ship` that can reach `target` now.
    #[must_use]
    pub fn can_fire(
        &self,
        weapon: &WeaponDef,
        rules: &ShipCombat,
        ship: usize,
        target: usize,
    ) -> bool {
        let (s, t) = (&self.ships[ship], &self.ships[target]);
        t.in_play()
            && t.side != s.side
            && weapon.arcs.contains(&arc_of(s.at, s.facing, t.at))
            && distance(s.at, t.at) <= rules.ranges.reach(weapon.range)
    }

    /// One shot (§5): d20 + bonus (+ lock, − jamming) against the armour
    /// (+ evasion); a natural 20 always hits and is critical, a natural
    /// 1 always misses. Shields take the damage first.
    #[allow(clippy::too_many_arguments)]
    fn shoot(
        &mut self,
        rules: &ShipCombat,
        ship: usize,
        target: usize,
        weapon: &WeaponDef,
        bonus: i32,
        charged: bool,
        dice: &mut dyn DiceSource,
        ev: &mut Vec<ShipEvent>,
    ) {
        let roll = dice.roll(D20);
        let lock = std::mem::take(&mut self.ships[target].locked);
        let s = &self.ships[ship];
        let t = &self.ships[target];
        let total = i32::try_from(roll).unwrap_or(0) + bonus + lock - s.jammed;
        let nav = level(&rules.energy.navigation, t, t.energy.navigation);
        let armor = rules.ship(&t.def).map_or(10, |d| d.armor)
            + t.evade
            + if t.evade > 0 { nav.evade } else { 0 };
        let critical = roll == D20;
        let hit = critical || (roll != 1 && total >= armor);
        let weapons = level(&rules.energy.weapons, s, s.energy.weapons);
        let shields_level = level(&rules.energy.shields, t, t.energy.shields);
        let base = if charged {
            weapon.charged_damage.unwrap_or(weapon.damage)
        } else {
            weapon.damage
        };
        let damage = if hit {
            (base + weapons.damage - t.brace - shields_level.damage_taken).max(0)
        } else {
            0
        };
        ev.push(ShipEvent::Shot {
            ship: s.key.clone(),
            weapon: weapon.id.clone(),
            target: t.key.clone(),
            roll,
            total,
            armor,
            hit,
            critical,
            damage,
        });
        if !hit {
            return;
        }
        let t = &mut self.ships[target];
        let absorbed = if shields_level.offline {
            0
        } else {
            damage.min(t.shields)
        };
        t.shields -= absorbed;
        let rest = damage - absorbed;
        ev.push(ShipEvent::Absorbed {
            ship: t.key.clone(),
            shields: absorbed,
            hull: rest,
        });
        if rest > 0 || critical {
            self.lose_hull(rules, target, rest, critical, dice, ev);
        }
    }

    /// The crew's turn (§3): every member's orders, in the given order.
    /// All or nothing: a refused order leaves the fight as it was.
    ///
    /// # Errors
    ///
    /// `NOT_YOUR_TURN`, `FIGHT_OVER`, `UNKNOWN_CREW`, `UNKNOWN_ACTION`,
    /// `NOT_AT_STATION`, `STATION_DOWN`, `STATION_TAKEN`, `TOO_MANY_ACTIONS`,
    /// `TOO_MANY_ATTACKS`, `ENGINES_DEAD`, `TOO_FAR`, `TOO_SHARP`,
    /// `CELL_TAKEN`, `NO_WEAPON`, `NO_TARGET`, `OUT_OF_ARC`,
    /// `OUT_OF_RANGE`, `NO_CHARGED_SHOT`, `INVALID_ENERGY`,
    /// `NOTHING_TO_FIX`, `NO_MORALE`.
    pub fn crew_turn(
        &mut self,
        rules: &ShipCombat,
        ship: usize,
        orders: &[CrewOrders],
        dice: &mut dyn DiceSource,
    ) -> Result<Vec<ShipEvent>, ShipError> {
        if self.ended.is_some() {
            return Err(refuse("FIGHT_OVER", "the fight is over"));
        }
        if !self.acting().contains(&ship) || self.ships[ship].crew.is_empty() {
            return Err(refuse("NOT_YOUR_TURN", "not this crew's turn"));
        }
        let mut next = self.clone();
        let mut ev = Vec::new();
        for o in orders {
            next.member_turn(rules, ship, o, dice, &mut ev)?;
            if next.ended.is_some() {
                break;
            }
        }
        *self = next;
        Ok(ev)
    }

    fn member_turn(
        &mut self,
        rules: &ShipCombat,
        ship: usize,
        o: &CrewOrders,
        dice: &mut dyn DiceSource,
        ev: &mut Vec<ShipEvent>,
    ) -> Result<(), ShipError> {
        let member = self.ships[ship]
            .crew
            .iter()
            .position(|c| c.id == o.crew)
            .ok_or_else(|| refuse("UNKNOWN_CREW", o.crew.clone()))?;
        let shaken = self.ships[ship].shaken.contains(&o.crew);
        let budget = self.actions_per_turn - u32::from(shaken);
        let mut spent = 0;
        let mut attacks = 0;
        for order in &o.orders {
            if self.ended.is_some() {
                break;
            }
            let def = rules
                .action(&order.action)
                .ok_or_else(|| refuse("UNKNOWN_ACTION", order.action.clone()))?;
            spent += def.cost;
            if spent > budget {
                return Err(refuse("TOO_MANY_ACTIONS", o.crew.clone()));
            }
            if def.attack {
                attacks += 1;
                if attacks > self.attacks_per_turn {
                    return Err(refuse("TOO_MANY_ATTACKS", o.crew.clone()));
                }
            }
            self.order(rules, ship, member, def, order, dice, ev)?;
        }
        Ok(())
    }

    /// The modifier of crew member `m` of `ship` for `ability`.
    fn modifier(&self, ship: usize, m: usize, ability: &str) -> i32 {
        self.ships[ship].crew[m]
            .modifiers
            .get(ability)
            .copied()
            .unwrap_or(0)
    }

    fn target(&self, ship: usize, order: &Order) -> Result<usize, ShipError> {
        let key = order
            .target
            .as_deref()
            .ok_or_else(|| refuse("NO_TARGET", "a target is needed"))?;
        let t = self
            .ship(key)
            .filter(|&t| self.ships[t].in_play() && self.ships[t].side != self.ships[ship].side)
            .ok_or_else(|| refuse("NO_TARGET", key.to_string()))?;
        Ok(t)
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn order(
        &mut self,
        rules: &ShipCombat,
        ship: usize,
        m: usize,
        def: &ShipActionDef,
        order: &Order,
        dice: &mut dyn DiceSource,
        ev: &mut Vec<ShipEvent>,
    ) -> Result<(), ShipError> {
        let here = self.ships[ship].crew[m].station.clone();
        let crew_id = self.ships[ship].crew[m].id.clone();
        let key = self.ships[ship].key.clone();
        if def.effect == ShipEffect::Move {
            let to = order
                .station
                .clone()
                .filter(|s| rules.station(s).is_some())
                .ok_or_else(|| refuse("UNKNOWN_STATION", format!("{:?}", order.station)))?;
            if self.ships[ship].crew.iter().any(|c| c.station == to) {
                return Err(refuse("STATION_TAKEN", to));
            }
            self.ships[ship].crew[m].station = to;
            ev.push(ShipEvent::Effect {
                ship: key,
                crew: crew_id,
                action: def.id.clone(),
            });
            return Ok(());
        }
        if here != def.station {
            return Err(refuse("NOT_AT_STATION", format!("{crew_id} is at {here}")));
        }
        if let Some(t) = order.target.as_deref().and_then(|k| self.ship(k))
            && self.ships[t].side != self.ships[ship].side
            && !self.ships[t].in_play()
        {
            ev.push(ShipEvent::Wasted {
                ship: key,
                crew: crew_id,
                action: def.id.clone(),
            });
            return Ok(());
        }
        if self.ships[ship].station_down(&here) {
            return Err(refuse("STATION_DOWN", here));
        }
        let ability = rules
            .station(&def.station)
            .map_or_else(String::new, |s| s.ability.clone());
        let modifier = self.modifier(ship, m, &ability);
        let nav = level(
            &rules.energy.navigation,
            &self.ships[ship],
            self.ships[ship].energy.navigation,
        );

        // Validate what needs no dice before rolling.
        match def.effect {
            ShipEffect::Maneuver { cells, turns } => {
                if nav.engines_dead {
                    return Err(refuse("ENGINES_DEAD", key));
                }
                let to = order
                    .to
                    .ok_or_else(|| refuse("TOO_FAR", "no destination"))?;
                let facing = order.facing.unwrap_or(self.ships[ship].facing);
                let reach =
                    u32::try_from((i64::from(cells) + i64::from(nav.movement)).max(0)).unwrap_or(0);
                if distance(self.ships[ship].at, to) > reach {
                    return Err(refuse("TOO_FAR", format!("{reach} cells at most")));
                }
                if self.ships[ship].facing.turns_to(facing) > turns {
                    return Err(refuse(
                        "TOO_SHARP",
                        format!("{turns} quarter turns at most"),
                    ));
                }
                if self
                    .ships
                    .iter()
                    .enumerate()
                    .any(|(i, s)| i != ship && s.in_play() && s.at == to)
                {
                    return Err(refuse("CELL_TAKEN", format!("{to:?}")));
                }
            }
            ShipEffect::Evade { .. } if nav.engines_dead => {
                return Err(refuse("ENGINES_DEAD", key));
            }
            ShipEffect::Fire { charged } => {
                let weapons = level(
                    &rules.energy.weapons,
                    &self.ships[ship],
                    self.ships[ship].energy.weapons,
                );
                let weapon = self.weapon(rules, ship, &def.station, order.weapon.as_deref())?;
                if charged && (weapon.charged_damage.is_none() || weapons.no_charged) {
                    return Err(refuse("NO_CHARGED_SHOT", weapon.id.clone()));
                }
                let t = self.target(ship, order)?;
                let (s, ts) = (&self.ships[ship], &self.ships[t]);
                if !weapon.arcs.contains(&arc_of(s.at, s.facing, ts.at)) {
                    return Err(refuse("OUT_OF_ARC", ts.key.clone()));
                }
                if distance(s.at, ts.at) > rules.ranges.reach(weapon.range) {
                    return Err(refuse("OUT_OF_RANGE", ts.key.clone()));
                }
            }
            ShipEffect::Reroute => {
                let e = order
                    .energy
                    .ok_or_else(|| refuse("INVALID_ENERGY", "no split"))?;
                let caps = (
                    rules.energy.navigation.len(),
                    rules.energy.weapons.len(),
                    rules.energy.shields.len(),
                );
                if e.total() > self.reactor(rules, ship)
                    || e.navigation as usize >= caps.0
                    || e.weapons as usize >= caps.1
                    || e.shields as usize >= caps.2
                {
                    return Err(refuse("INVALID_ENERGY", format!("{e:?}")));
                }
            }
            ShipEffect::RepairSystem => {
                let st = order.station.as_deref().unwrap_or_default();
                if !self.ships[ship].station_down(st) {
                    return Err(refuse("NOTHING_TO_FIX", st.to_string()));
                }
            }
            ShipEffect::Extinguish | ShipEffect::PatchBreach => {
                let kind = if def.effect == ShipEffect::Extinguish {
                    DamageKind::Fire
                } else {
                    DamageKind::Breach
                };
                if !self.ships[ship].damages.iter().any(|d| d.kind == kind) {
                    return Err(refuse("NOTHING_TO_FIX", def.id.clone()));
                }
            }
            ShipEffect::Lock { .. } | ShipEffect::Jam { .. } => {
                self.target(ship, order)?;
            }
            ShipEffect::BreakMorale { .. } => {
                let t = self.target(ship, order)?;
                if self.ships[t].morale.is_none() {
                    return Err(refuse("NO_MORALE", self.ships[t].key.clone()));
                }
            }
            _ => {}
        }

        // The check, when the action has one.
        if let Some(difficulty) = def.difficulty {
            let roll = dice.roll(D20);
            let total = i32::try_from(roll).unwrap_or(0) + modifier;
            let success = total >= difficulty;
            ev.push(ShipEvent::Check {
                ship: key.clone(),
                crew: crew_id.clone(),
                action: def.id.clone(),
                roll,
                total,
                difficulty,
                success,
            });
            if !success {
                return Ok(());
            }
        }

        let max_hull = rules.ship(&self.ships[ship].def).map_or(0, |d| d.hull);
        let max_shields = rules.ship(&self.ships[ship].def).map_or(0, |d| d.shields);
        match def.effect {
            ShipEffect::Maneuver { .. } => {
                let s = &mut self.ships[ship];
                s.at = order.to.unwrap_or(s.at);
                s.facing = order.facing.unwrap_or(s.facing);
                ev.push(ShipEvent::Maneuver {
                    ship: key,
                    to: s.at,
                    facing: s.facing,
                });
                return Ok(());
            }
            ShipEffect::Evade { armor } => self.ships[ship].evade = armor,
            ShipEffect::Brace { reduction } => self.ships[ship].brace = reduction,
            ShipEffect::Fire { charged } => {
                let weapon = self
                    .weapon(rules, ship, &def.station, order.weapon.as_deref())?
                    .clone();
                let t = self.target(ship, order)?;
                self.shoot(rules, ship, t, &weapon, modifier, charged, dice, ev);
                return Ok(());
            }
            ShipEffect::RechargeShields => {
                let lv = level(
                    &rules.energy.shields,
                    &self.ships[ship],
                    self.ships[ship].energy.shields,
                );
                if !lv.offline {
                    let s = &mut self.ships[ship];
                    s.shields = (s.shields + lv.recharge).min(max_shields);
                }
            }
            ShipEffect::Reroute => {
                self.ships[ship].energy = order.energy.unwrap_or_default();
            }
            ShipEffect::RepairHull { amount } => {
                let s = &mut self.ships[ship];
                s.hull = (s.hull + amount).min(max_hull);
            }
            ShipEffect::RepairSystem => {
                let st = order.station.clone();
                let s = &mut self.ships[ship];
                if let Some(i) = s
                    .damages
                    .iter()
                    .position(|d| d.kind == DamageKind::SystemDown && d.station == st)
                {
                    s.damages.remove(i);
                }
            }
            ShipEffect::Extinguish | ShipEffect::PatchBreach => {
                let kind = if def.effect == ShipEffect::Extinguish {
                    DamageKind::Fire
                } else {
                    DamageKind::Breach
                };
                let s = &mut self.ships[ship];
                if let Some(i) = s.damages.iter().position(|d| d.kind == kind) {
                    s.damages.remove(i);
                }
            }
            ShipEffect::Lock { bonus } => {
                let t = self.target(ship, order)?;
                self.ships[t].locked = bonus;
            }
            ShipEffect::Jam { malus } => {
                let t = self.target(ship, order)?;
                self.ships[t].jammed = malus;
            }
            ShipEffect::BreakMorale { amount } => {
                let t = self.target(ship, order)?;
                let resolve = rules.ship(&self.ships[t].def).map_or(12, |d| d.resolve);
                let roll = dice.roll(D20);
                let total = i32::try_from(roll).unwrap_or(0) + modifier;
                let success = total >= resolve;
                ev.push(ShipEvent::Check {
                    ship: key.clone(),
                    crew: crew_id.clone(),
                    action: def.id.clone(),
                    roll,
                    total,
                    difficulty: resolve,
                    success,
                });
                if success {
                    let ts = &mut self.ships[t];
                    let from = ts.morale.unwrap_or(0);
                    let to = (from - amount).max(0);
                    ts.morale = Some(to);
                    ev.push(ShipEvent::Morale {
                        ship: ts.key.clone(),
                        from,
                        to,
                    });
                    if to == 0 {
                        ts.out = Some(Out::Broken);
                        ev.push(ShipEvent::Out {
                            ship: ts.key.clone(),
                            why: Out::Broken,
                        });
                        self.check_end();
                    }
                }
                return Ok(());
            }
            ShipEffect::Move => {}
        }
        ev.push(ShipEvent::Effect {
            ship: key,
            crew: crew_id,
            action: def.id.clone(),
        });
        Ok(())
    }

    fn weapon<'r>(
        &self,
        rules: &'r ShipCombat,
        ship: usize,
        station: &str,
        pick: Option<&str>,
    ) -> Result<&'r WeaponDef, ShipError> {
        let def = rules
            .ship(&self.ships[ship].def)
            .ok_or_else(|| refuse("NO_WEAPON", station.to_string()))?;
        def.weapons
            .iter()
            .find(|w| w.station.as_deref() == Some(station) && pick.is_none_or(|p| p == w.id))
            .ok_or_else(|| refuse("NO_WEAPON", station.to_string()))
    }

    /// The turn of a ship the GM runs, as the simulator and the GM's
    /// « play it for me » run it: it slips towards its target's blind
    /// spot while keeping it in its own arc and range, then fires its
    /// best weapon.
    pub fn npc_turn(
        &mut self,
        rules: &ShipCombat,
        ship: usize,
        dice: &mut dyn DiceSource,
    ) -> Vec<ShipEvent> {
        let mut ev = Vec::new();
        if self.ended.is_some() || !self.ships[ship].in_play() {
            return ev;
        }
        let Some(def) = rules.ship(&self.ships[ship].def) else {
            return ev;
        };
        let me = self.ships[ship].clone();
        let Some(target) = self
            .ships
            .iter()
            .enumerate()
            .filter(|(_, s)| s.in_play() && s.side != me.side)
            .min_by_key(|(_, s)| distance(s.at, me.at))
            .map(|(i, _)| i)
        else {
            return ev;
        };
        let t = self.ships[target].clone();
        let reach = def
            .weapons
            .iter()
            .map(|w| rules.ranges.reach(w.range))
            .max()
            .unwrap_or(0);
        let speed = i32::try_from(def.speed).unwrap_or(0);
        let mut best: Option<(i64, (i32, i32), Facing)> = None;
        for dx in -speed..=speed {
            for dy in -speed..=speed {
                let at = (me.at.0 + dx, me.at.1 + dy);
                if at == t.at
                    || self
                        .ships
                        .iter()
                        .enumerate()
                        .any(|(i, s)| i != ship && s.in_play() && s.at == at)
                {
                    continue;
                }
                for facing in Facing::ALL {
                    if me.facing.turns_to(facing) > 1 {
                        continue;
                    }
                    let d = distance(at, t.at);
                    let arc_here = arc_of(at, facing, t.at);
                    let fires = def
                        .weapons
                        .iter()
                        .any(|w| w.arcs.contains(&arc_here) && d <= rules.ranges.reach(w.range));
                    let seen = arc_of(t.at, t.facing, at);
                    let mut score: i64 = 0;
                    if fires {
                        score += 100;
                    }
                    score += match seen {
                        Arc::Rear => 40,
                        Arc::Front => -15,
                        _ => 0,
                    };
                    // Close to the weapons' reach, not further.
                    score -= i64::from(d.abs_diff(reach.min(3)));
                    if best.is_none_or(|b| score > b.0) {
                        best = Some((score, at, facing));
                    }
                }
            }
        }
        if let Some((_, at, facing)) = best
            && (at != me.at || facing != me.facing)
        {
            self.ships[ship].at = at;
            self.ships[ship].facing = facing;
            ev.push(ShipEvent::Maneuver {
                ship: me.key.clone(),
                to: at,
                facing,
            });
        }
        let weapon = def
            .weapons
            .iter()
            .filter(|w| self.can_fire(w, rules, ship, target))
            .max_by_key(|w| w.damage)
            .cloned();
        if let Some(w) = weapon {
            self.shoot(
                rules,
                ship,
                target,
                &w,
                def.attack_bonus,
                false,
                dice,
                &mut ev,
            );
        }
        ev
    }
}
