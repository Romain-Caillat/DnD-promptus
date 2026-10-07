//! Ship fights played N times with seeded dice, as `engine/simulate-fights`
//! plays ground fights: a [`ShipScenario`] (`content/ship-scenarios/
//! <world>/<id>.yaml`), a crew that plays its stations sensibly, the GM's
//! ships run by [`ShipFight::npc_turn`]. What comes out: who won, how
//! many rounds, how long at a remote table, the hull left.
//!
//! The crew policy is a reasonable table, not an optimal one: the helm
//! keeps the nearest enemy ahead and out of the stern, the heavy gun
//! fires ahead, a free hand takes a turret, the engineer keeps the
//! shields up and fixes what breaks, integrity fights fires and braces,
//! sensors lock the gunners' target, the link breaks morale when the
//! enemy has any.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::fight::{
    CrewMember, CrewOrders, Facing, Order, Ship, ShipEnd, ShipFight, ShipSide, arc_of, distance,
};
use super::model::{Allocation, Arc, DamageKind, ShipCombat, ShipEffect};
use crate::rules::RuleSystem;
use crate::rules::dice::SeededDice;
use crate::rules::sheet::modifier_for;

/// A ship fight to rehearse.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShipScenario {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
    pub rules: String,
    pub version: u32,
    pub party: ScenarioShip,
    pub opposition: Vec<ScenarioShip>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioShip {
    pub ship: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    pub at: (i32, i32),
    pub facing: Facing,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub crew: Vec<ScenarioCrew>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioCrew {
    pub id: String,
    /// A class of the rules: its starting abilities give the modifiers.
    pub class: String,
    pub station: String,
}

/// A scenario that does not fit its rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipScenarioError(pub String);

impl std::fmt::Display for ShipScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ShipScenarioError {}

impl ShipScenario {
    /// # Errors
    ///
    /// The YAML does not parse.
    pub fn from_yaml(text: &str) -> Result<Self, ShipScenarioError> {
        serde_yaml_ng::from_str(text).map_err(|e| ShipScenarioError(e.to_string()))
    }

    /// The ships, in place, crews seated.
    ///
    /// # Errors
    ///
    /// The rules have no ship combat, or the scenario names a ship,
    /// class or station they lack.
    pub fn ships(&self, system: &RuleSystem) -> Result<Vec<Ship>, ShipScenarioError> {
        let rules = system
            .ship_combat
            .as_ref()
            .ok_or_else(|| ShipScenarioError(format!("{} has no ship_combat", system.id)))?;
        let mut out = Vec::new();
        for (n, (s, side)) in std::iter::once((&self.party, ShipSide::Party))
            .chain(self.opposition.iter().map(|s| (s, ShipSide::Opposition)))
            .enumerate()
        {
            let n = n + 1;
            let def = rules
                .ship(&s.ship)
                .ok_or_else(|| ShipScenarioError(format!("unknown ship {}", s.ship)))?;
            let key = s.key.clone().unwrap_or_else(|| format!("{}_{n}", s.ship));
            let mut ship = Ship::new(rules, def, &key, side);
            if let Some(name) = &s.name {
                ship.name.clone_from(name);
            }
            ship.at = s.at;
            ship.facing = s.facing;
            ship.group.clone_from(&s.group);
            for c in &s.crew {
                let class = system
                    .classes
                    .iter()
                    .find(|k| k.id == c.class)
                    .ok_or_else(|| ShipScenarioError(format!("unknown class {}", c.class)))?;
                if rules.station(&c.station).is_none() {
                    return Err(ShipScenarioError(format!("unknown station {}", c.station)));
                }
                let modifiers = class
                    .abilities
                    .iter()
                    .map(|(a, score)| (a.clone(), modifier_for(system, *score).unwrap_or(0)))
                    .collect::<BTreeMap<_, _>>();
                ship.crew.push(CrewMember {
                    id: c.id.clone(),
                    name: class.name.clone(),
                    modifiers,
                    station: c.station.clone(),
                });
            }
            out.push(ship);
        }
        Ok(out)
    }
}

/// Table time (INTERPRETATION, like the ground fights' `TimeModel`):
/// two minutes to set up; the crew's turn is played together, six
/// players at once, in two minutes; the GM runs a ship or a swarm in
/// thirty seconds.
pub const SETUP_SECONDS: u32 = 120;
pub const CREW_TURN_SECONDS: u32 = 120;
pub const GM_TURN_SECONDS: u32 = 30;

/// No fight goes on forever.
pub const MAX_ROUNDS: u32 = 30;

/// What N runs of a scenario gave.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShipReport {
    pub runs: u32,
    pub victories: u32,
    pub defeats: u32,
    /// Neither side out after [`MAX_ROUNDS`].
    pub stalled: u32,
    /// Enemy ships broken (morale) rather than destroyed, all runs.
    pub broken: u32,
    pub destroyed: u32,
    pub average_rounds: f64,
    pub average_minutes: f64,
    pub longest_minutes: f64,
    /// The party ship's hull at the end, on average.
    pub average_hull_left: f64,
    /// Damage-table results the party suffered, by kind, all runs.
    pub damages: BTreeMap<String, u32>,
}

/// One run, its log kept: rounds, seconds at the table, the end.
#[derive(Debug, Clone)]
pub struct ShipRun {
    pub fight: ShipFight,
    pub seconds: u32,
    pub events: Vec<super::fight::ShipEvent>,
}

/// Play `scenario` once on `seed`.
///
/// # Errors
///
/// As [`ShipScenario::ships`].
pub fn run_once(
    system: &RuleSystem,
    scenario: &ShipScenario,
    seed: u64,
) -> Result<ShipRun, ShipScenarioError> {
    let rules = system
        .ship_combat
        .as_ref()
        .ok_or_else(|| ShipScenarioError(format!("{} has no ship_combat", system.id)))?;
    let mut dice = SeededDice::new(seed);
    let mut events = Vec::new();
    let mut fight = ShipFight::start(rules, scenario.ships(system)?, &mut dice, &mut events);
    let mut seconds = SETUP_SECONDS;
    while fight.ended.is_none() && fight.round <= MAX_ROUNDS {
        let acting = fight.acting().to_vec();
        if acting.iter().any(|&i| !fight.ships[i].crew.is_empty()) {
            for &i in &acting {
                if fight.ships[i].in_play() && fight.ended.is_none() {
                    let orders = crew_policy(rules, &fight, i);
                    match fight.crew_turn(rules, i, &orders, &mut dice) {
                        Ok(ev) => events.extend(ev),
                        // A policy order the rules refuse is the
                        // policy's bug: play the turn without it.
                        Err(e) => panic!("crew policy refused: {} {}", e.code, e.detail),
                    }
                }
            }
            seconds += CREW_TURN_SECONDS;
        } else if acting.iter().any(|&i| fight.ships[i].in_play()) {
            for &i in &acting {
                events.extend(fight.npc_turn(rules, i, &mut dice));
            }
            seconds += GM_TURN_SECONDS;
        }
        fight.end_turn(rules, &mut dice, &mut events);
    }
    Ok(ShipRun {
        fight,
        seconds,
        events,
    })
}

/// Play `scenario` on seeds `0..runs`.
///
/// # Errors
///
/// As [`ShipScenario::ships`].
#[allow(clippy::cast_precision_loss)]
pub fn simulate(
    system: &RuleSystem,
    scenario: &ShipScenario,
    runs: u32,
) -> Result<ShipReport, ShipScenarioError> {
    let mut r = ShipReport {
        runs,
        victories: 0,
        defeats: 0,
        stalled: 0,
        broken: 0,
        destroyed: 0,
        average_rounds: 0.0,
        average_minutes: 0.0,
        longest_minutes: 0.0,
        average_hull_left: 0.0,
        damages: BTreeMap::new(),
    };
    for seed in 0..u64::from(runs) {
        let run = run_once(system, scenario, seed)?;
        let f = &run.fight;
        match f.ended {
            Some(ShipEnd::Victory) => r.victories += 1,
            Some(ShipEnd::Defeat) => r.defeats += 1,
            None => r.stalled += 1,
        }
        for s in f.ships.iter().filter(|s| s.side == ShipSide::Opposition) {
            match s.out {
                Some(super::fight::Out::Broken) => r.broken += 1,
                Some(super::fight::Out::Destroyed) => r.destroyed += 1,
                None => {}
            }
        }
        for e in &run.events {
            if let super::fight::ShipEvent::Damage { ship, kind, .. } = e
                && f.ships
                    .iter()
                    .any(|s| &s.key == ship && s.side == ShipSide::Party)
            {
                *r.damages.entry(format!("{kind:?}")).or_default() += 1;
            }
        }
        let minutes = f64::from(run.seconds) / 60.0;
        r.average_rounds += f64::from(f.round.min(MAX_ROUNDS));
        r.average_minutes += minutes;
        r.longest_minutes = r.longest_minutes.max(minutes);
        r.average_hull_left += f64::from(
            f.ships
                .iter()
                .find(|s| s.side == ShipSide::Party)
                .map_or(0, |s| s.hull),
        );
    }
    let n = f64::from(runs.max(1));
    r.average_rounds /= n;
    r.average_minutes /= n;
    r.average_hull_left /= n;
    Ok(r)
}

/// The id of the first action of `station` with this kind of effect.
fn action_of(
    rules: &ShipCombat,
    station: &str,
    pred: impl Fn(&ShipEffect) -> bool,
) -> Option<String> {
    rules
        .actions
        .iter()
        .find(|a| a.station == station && pred(&a.effect))
        .map(|a| a.id.clone())
}

fn move_action(rules: &ShipCombat) -> Option<String> {
    rules
        .actions
        .iter()
        .find(|a| a.effect == ShipEffect::Move)
        .map(|a| a.id.clone())
}

/// What the crew of ship `i` does this turn.
#[allow(clippy::too_many_lines)]
fn crew_policy(rules: &ShipCombat, fight: &ShipFight, i: usize) -> Vec<CrewOrders> {
    let me = &fight.ships[i];
    let def = rules.ship(&me.def);
    let enemies: Vec<usize> = fight
        .ships
        .iter()
        .enumerate()
        .filter(|(_, s)| s.in_play() && s.side != me.side)
        .map(|(k, _)| k)
        .collect();
    if enemies.is_empty() {
        return Vec::new();
    }
    // The helm moves first: everyone else reads the new position.
    let mut sim = fight.clone();
    let mut out = Vec::new();
    let mut taken: Vec<String> = me.crew.iter().map(|c| c.station.clone()).collect();
    let nearest = |f: &ShipFight| {
        enemies
            .iter()
            .copied()
            .filter(|&e| f.ships[e].in_play())
            .min_by_key(|&e| distance(f.ships[e].at, f.ships[i].at))
    };

    for c in &me.crew {
        let station = c.station.clone();
        let st = rules.station(&station);
        let mut orders = Vec::new();
        if st.is_some_and(|s| s.helm) && !me.station_down(&station) {
            if let (Some(a), Some(e)) = (
                action_of(rules, &station, |e| {
                    matches!(e, ShipEffect::Maneuver { .. })
                }),
                nearest(&sim),
            ) && let Some((to, facing)) = helm_move(rules, &sim, i, e, &a)
            {
                sim.ships[i].at = to;
                sim.ships[i].facing = facing;
                orders.push(Order {
                    action: a,
                    to: Some(to),
                    facing: Some(facing),
                    ..Order::default()
                });
            }
            let engines_dead = rules
                .energy
                .navigation
                .get(me.energy.navigation as usize)
                .is_some_and(|l| l.engines_dead);
            if !engines_dead
                && let Some(a) =
                    action_of(rules, &station, |e| matches!(e, ShipEffect::Evade { .. }))
            {
                orders.push(Order {
                    action: a,
                    ..Order::default()
                });
            }
        }
        out.push(CrewOrders {
            crew: c.id.clone(),
            orders,
        });
    }

    for (k, c) in me.crew.iter().enumerate() {
        let station = c.station.clone();
        if rules.station(&station).is_some_and(|s| s.helm) {
            continue;
        }
        let mut orders = Vec::new();
        let down = me.station_down(&station);
        let targets_for = |f: &ShipFight, st: &str| -> Vec<usize> {
            def.map(|d| {
                d.weapons
                    .iter()
                    .filter(|w| w.station.as_deref() == Some(st))
                    .flat_map(|w| {
                        enemies
                            .iter()
                            .copied()
                            .filter(move |&e| f.can_fire(w, rules, i, e))
                    })
                    .collect()
            })
            .unwrap_or_default()
        };
        let pick = |f: &ShipFight, ts: &[usize]| -> Option<usize> {
            ts.iter().copied().min_by_key(|&e| {
                let s = &f.ships[e];
                (i32::from(s.locked == 0), s.hull + s.shields)
            })
        };
        let fire = |st: &str, t: usize| -> Option<Order> {
            action_of(rules, st, |e| *e == ShipEffect::Fire { charged: false }).map(|a| Order {
                action: a,
                target: Some(fight.ships[t].key.clone()),
                ..Order::default()
            })
        };
        let has_weapon = def.is_some_and(|d| {
            d.weapons
                .iter()
                .any(|w| w.station.as_deref() == Some(station.as_str()))
        });
        let effects: Vec<ShipEffect> = rules
            .actions
            .iter()
            .filter(|a| a.station == station)
            .map(|a| a.effect)
            .collect();

        if !down && has_weapon {
            if let Some(t) = pick(&sim, &targets_for(&sim, &station))
                && let Some(o) = fire(&station, t)
            {
                orders.push(o);
            }
        } else if !down && effects.contains(&ShipEffect::RechargeShields) {
            // The engineer: fix a station, keep the shields up, patch the hull.
            let max_shields = def.map_or(0, |d| d.shields);
            let max_hull = def.map_or(0, |d| d.hull);
            let broken: Vec<String> = me
                .damages
                .iter()
                .filter(|d| d.kind == DamageKind::SystemDown)
                .filter_map(|d| d.station.clone())
                .collect();
            let mut shields = me.shields;
            // A reactor that dropped: share what is left, shields last.
            let cap = fight.reactor(rules, i);
            let want = split(cap, rules.energy.start);
            if want != me.energy
                && let Some(a) = action_of(rules, &station, |e| *e == ShipEffect::Reroute)
            {
                orders.push(Order {
                    action: a,
                    energy: Some(want),
                    ..Order::default()
                });
            }
            while orders.len() < 2 {
                let before = orders.len();
                if let Some(st) = broken.first()
                    && !orders.iter().any(|o| o.station.is_some())
                    && let Some(a) = action_of(rules, &station, |e| *e == ShipEffect::RepairSystem)
                {
                    orders.push(Order {
                        action: a,
                        station: Some(st.clone()),
                        ..Order::default()
                    });
                } else if shields < max_shields
                    && let Some(a) =
                        action_of(rules, &station, |e| *e == ShipEffect::RechargeShields)
                {
                    shields += 4;
                    orders.push(Order {
                        action: a,
                        ..Order::default()
                    });
                } else if me.hull < max_hull
                    && let Some(a) = action_of(rules, &station, |e| {
                        matches!(e, ShipEffect::RepairHull { .. })
                    })
                {
                    orders.push(Order {
                        action: a,
                        ..Order::default()
                    });
                }
                if orders.len() == before {
                    break;
                }
            }
        } else if !down && effects.contains(&ShipEffect::Extinguish) {
            let mut fires = me
                .damages
                .iter()
                .filter(|d| d.kind == DamageKind::Fire)
                .count();
            let mut breaches = me
                .damages
                .iter()
                .filter(|d| d.kind == DamageKind::Breach)
                .count();
            for _ in 0..2 {
                if fires > 0
                    && let Some(a) = action_of(rules, &station, |e| *e == ShipEffect::Extinguish)
                {
                    fires -= 1;
                    orders.push(Order {
                        action: a,
                        ..Order::default()
                    });
                } else if breaches > 0
                    && let Some(a) = action_of(rules, &station, |e| *e == ShipEffect::PatchBreach)
                {
                    breaches -= 1;
                    orders.push(Order {
                        action: a,
                        ..Order::default()
                    });
                } else if !orders.iter().any(|o| {
                    rules
                        .action(&o.action)
                        .is_some_and(|a| matches!(a.effect, ShipEffect::Brace { .. }))
                }) && let Some(a) =
                    action_of(rules, &station, |e| matches!(e, ShipEffect::Brace { .. }))
                {
                    orders.push(Order {
                        action: a,
                        ..Order::default()
                    });
                }
            }
        } else if !down && effects.iter().any(|e| matches!(e, ShipEffect::Lock { .. })) {
            // Sensors: lock what the guns will shoot, jam what shoots us.
            let ahead: Vec<usize> = enemies
                .iter()
                .copied()
                .filter(|&e| {
                    arc_of(sim.ships[i].at, sim.ships[i].facing, sim.ships[e].at) != Arc::Rear
                })
                .collect();
            let t = pick(&sim, &ahead).or_else(|| nearest(&sim));
            if let Some(t) = t
                && let Some(a) =
                    action_of(rules, &station, |e| matches!(e, ShipEffect::Lock { .. }))
            {
                orders.push(Order {
                    action: a,
                    target: Some(fight.ships[t].key.clone()),
                    ..Order::default()
                });
            }
            if let Some(t) = nearest(&sim)
                && let Some(a) = action_of(rules, &station, |e| matches!(e, ShipEffect::Jam { .. }))
            {
                orders.push(Order {
                    action: a,
                    target: Some(fight.ships[t].key.clone()),
                    ..Order::default()
                });
            }
        } else if !down
            && effects
                .iter()
                .any(|e| matches!(e, ShipEffect::BreakMorale { .. }))
            && let Some(t) = enemies
                .iter()
                .copied()
                .find(|&e| fight.ships[e].morale.is_some())
            && let Some(a) = action_of(rules, &station, |e| {
                matches!(e, ShipEffect::BreakMorale { .. })
            })
        {
            orders.push(Order {
                action: a,
                target: Some(fight.ships[t].key.clone()),
                ..Order::default()
            });
        } else if let Some(mv) = move_action(rules) {
            // A free hand: to a turret that has a target, and fire.
            let free = def
                .map(|d| {
                    d.weapons
                        .iter()
                        .filter_map(|w| w.station.clone())
                        .filter(|s| !taken.contains(s) && !me.station_down(s))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if let Some((st, t)) = free
                .iter()
                .find_map(|s| pick(&sim, &targets_for(&sim, s)).map(|t| (s.clone(), t)))
            {
                taken.retain(|x| x != &station);
                taken.push(st.clone());
                orders.push(Order {
                    action: mv,
                    station: Some(st.clone()),
                    ..Order::default()
                });
                if let Some(o) = fire(&st, t) {
                    orders.push(o);
                }
            }
        }
        out[k].orders.extend(orders);
    }
    out.retain(|o| !o.orders.is_empty());
    // The helm first: the guns read where it took the ship.
    out.sort_by_key(|o| {
        !me.crew
            .iter()
            .any(|c| c.id == o.crew && rules.station(&c.station).is_some_and(|s| s.helm))
    });
    // A crew member shaken by a jolt keeps only their first action.
    for o in &mut out {
        if me.shaken.contains(&o.crew) {
            let mut spent = 0;
            o.orders.retain(|ord| {
                spent += rules.action(&ord.action).map_or(1, |a| a.cost);
                spent <= 1
            });
        }
    }
    out
}

/// The split the engineer keeps for a reactor of `cap` points: the
/// start's, losing navigation first, then weapons.
fn split(cap: u32, start: Allocation) -> Allocation {
    let mut a = start;
    while a.total() > cap {
        if a.navigation > 1 {
            a.navigation -= 1;
        } else if a.weapons > 1 {
            a.weapons -= 1;
        } else if a.navigation > 0 {
            a.navigation -= 1;
        } else if a.weapons > 0 {
            a.weapons -= 1;
        } else {
            a.shields -= 1;
        }
    }
    a
}

/// Where the helm takes the ship: the nearest enemy ahead, in the heavy
/// gun's reach but not on top of it, and never sitting in the stern.
fn helm_move(
    rules: &ShipCombat,
    fight: &ShipFight,
    i: usize,
    target: usize,
    action: &str,
) -> Option<((i32, i32), Facing)> {
    let a = rules.action(action)?;
    let ShipEffect::Maneuver { cells, turns } = a.effect else {
        return None;
    };
    let me = &fight.ships[i];
    let nav = rules
        .energy
        .navigation
        .get(me.energy.navigation as usize)
        .copied()
        .unwrap_or_default();
    if nav.engines_dead {
        return None;
    }
    let reach = i32::try_from((i64::from(cells) + i64::from(nav.movement)).max(0)).unwrap_or(0);
    let long = rules.ranges.long;
    let medium = rules.ranges.medium;
    let mut best: Option<(i64, (i32, i32), Facing)> = None;
    for dx in -reach..=reach {
        for dy in -reach..=reach {
            let at = (me.at.0 + dx, me.at.1 + dy);
            if fight
                .ships
                .iter()
                .enumerate()
                .any(|(k, s)| k != i && s.in_play() && s.at == at)
            {
                continue;
            }
            for facing in Facing::ALL {
                if me.facing.turns_to(facing) > turns {
                    continue;
                }
                let mut score: i64 = 0;
                for (k, s) in fight.ships.iter().enumerate() {
                    if !s.in_play() || s.side == me.side {
                        continue;
                    }
                    let d = distance(at, s.at);
                    let arc = arc_of(at, facing, s.at);
                    let weight = if k == target { 3 } else { 1 };
                    score += match arc {
                        Arc::Front if d <= long => 30 * weight,
                        Arc::Port | Arc::Starboard if d <= medium => 10 * weight,
                        Arc::Rear => -40 * weight,
                        _ => 0,
                    };
                }
                let d = i64::from(distance(at, fight.ships[target].at));
                // Out of boarding range, inside the medium band.
                score -= (d - i64::from(medium)).abs() * 2;
                if best.is_none_or(|b| score > b.0) {
                    best = Some((score, at, facing));
                }
            }
        }
    }
    best.and_then(|(_, at, facing)| (at != me.at || facing != me.facing).then_some((at, facing)))
}
