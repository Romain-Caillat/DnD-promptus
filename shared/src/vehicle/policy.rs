//! How a crew and an enemy ship play, so that a battle can be played
//! without a table: the fight simulator runs whole battles with them,
//! and the co-GM proposes an enemy ship's turn with the same rules
//! (`copilot/propose-adversary-turns`). Reference tactics, not an AI:
//!
//! - **the crew** focuses the enemy ship with the least hull. The
//!   sensors lock it, the helm brings the most guns to bear on it while
//!   staying out of the enemy's arcs, gunners fire (a charged shot when
//!   they have the actions), a gunner without a shot runs to a turret
//!   that has one, the engineer recharges the screen under half, then
//!   repairs; damage control puts out fires, then braces; the liaison
//!   breaks the morale of a ship that has one, else rallies;
//! - **an enemy ship** goes where its own weapons bear on the party ship
//!   and the party's do not (a swarm slips into the blind spot), then
//!   fires its best weapon.

use std::collections::BTreeMap;

use crate::maps::Cell;
use crate::rules::RuleSystem;
use crate::rules::dice::DiceSource;
use crate::rules::sheet::Side;

use super::battle::{Aim, Battle, BattleEvent, BattleRefusal, Ship, Step};
use super::geometry::{Facing, distance};
use super::model::{ScreenRole, StationAction, VehicleEffect, VehicleRules};

/// One command of a crew member or an enemy ship.
#[derive(Debug, Clone, PartialEq)]
pub enum Order {
    Station {
        crew: String,
        station: String,
    },
    Act {
        crew: String,
        action: String,
        aim: Aim,
    },
    Pass {
        crew: String,
    },
    Maneuver {
        ship: String,
        to: Cell,
        facing: Facing,
    },
    Fire {
        ship: String,
        weapon: String,
        target: String,
    },
    EndTurn,
}

impl Order {
    /// Plays the order on `battle`.
    pub fn apply(
        &self,
        system: &RuleSystem,
        battle: &Battle,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, BattleRefusal> {
        match self {
            Order::Station { crew, station } => battle.take_station(system, crew, Some(station)),
            Order::Act { crew, action, aim } => battle.crew_act(system, crew, action, aim, dice),
            Order::Pass { crew } => battle.pass(system, crew, dice),
            Order::Maneuver { ship, to, facing } => {
                battle.enemy_maneuver(system, ship, *to, *facing)
            }
            Order::Fire {
                ship,
                weapon,
                target,
            } => battle.enemy_fire(system, ship, weapon, target, dice),
            Order::EndTurn => battle.end_turn(system, dice),
        }
    }
}

/// The enemy ship the crew focuses: afloat, least hull, then nearest.
fn focus(b: &Battle) -> Option<&Ship> {
    let me = b.party_ship();
    b.ships
        .iter()
        .filter(|s| s.side == Side::Opposition && s.afloat())
        .min_by_key(|s| (s.hull, distance(s.at, me.at)))
}

/// Damage `shooter` at (`at`, `facing`) can bring on `target`.
fn bearing(
    b: &Battle,
    v: &VehicleRules,
    shooter: &Ship,
    at: Cell,
    facing: Facing,
    target: &Ship,
) -> i32 {
    let mut s = shooter.clone();
    s.at = at;
    s.facing = facing;
    v.ship(&s.kind)
        .map(|d| {
            d.weapons
                .iter()
                .filter(|w| b.can_fire(v, &s, w, target).is_ok())
                .map(|w| w.damage)
                .sum()
        })
        .unwrap_or(0)
}

/// The best place for `ship` to end a manoeuvre against `target`: its
/// guns on the target, the target's off itself; `None` when staying put
/// is as good.
fn best_pose(
    b: &Battle,
    v: &VehicleRules,
    ship: &Ship,
    target: &Ship,
    multiplier: u32,
    turns: u32,
) -> Option<(Cell, Facing)> {
    // Dismasted or engines dead: it cannot even turn.
    b.maneuver_budget(v, ship, multiplier)?;
    let threats: Vec<&Ship> = b
        .ships
        .iter()
        .filter(|s| s.side != ship.side && s.afloat())
        .collect();
    let score = |at: Cell, facing: Facing| -> i32 {
        let mine = bearing(b, v, ship, at, facing, target) * 2;
        let theirs: i32 = threats
            .iter()
            .map(|t| {
                let mut me = ship.clone();
                me.at = at;
                bearing(b, v, t, t.at, t.facing, &me)
            })
            .sum();
        // Closing in matters when nothing bears yet.
        let near = -(distance(at, target.at) as i32);
        mine * 10 - theirs * 5 + near
    };
    let here = score(ship.at, ship.facing);
    let mut best: Option<(i32, Cell, Facing)> = None;
    for at in b
        .reachable(v, &ship.id, multiplier)
        .into_keys()
        .chain([ship.at])
    {
        for f in Facing::ALL {
            if ship.facing.quarters_to(f) > turns {
                continue;
            }
            let s = score(at, f);
            if best.is_none_or(|(x, _, _)| s > x) {
                best = Some((s, at, f));
            }
        }
    }
    best.filter(|(s, _, _)| *s > here).map(|(_, at, f)| (at, f))
}

/// The next order of the crew on their turn: the first member who has
/// something useful to do, or a pass.
pub fn crew_order(system: &RuleSystem, b: &Battle) -> Order {
    let Some(v) = system.vehicles.as_ref() else {
        return Order::EndTurn;
    };
    // Sensors first (a lock helps the guns), then the helm, the guns,
    // the rest — the order a table would pick.
    let mut members: Vec<_> = b.crew.iter().filter(|c| !c.done).collect();
    members.sort_by_key(|c| {
        let effects: Vec<&VehicleEffect> = c
            .station
            .as_deref()
            .and_then(|s| v.station(s))
            .map(|s| s.actions.iter().map(|a| &a.effect).collect())
            .unwrap_or_default();
        if effects
            .iter()
            .any(|e| matches!(e, VehicleEffect::Lock { .. }))
        {
            0
        } else if effects
            .iter()
            .any(|e| matches!(e, VehicleEffect::Maneuver { .. }))
        {
            1
        } else if effects
            .iter()
            .any(|e| matches!(e, VehicleEffect::Fire { .. }))
        {
            2
        } else {
            3
        }
    });
    match members.first() {
        Some(c) => member_order(v, b, &c.id).unwrap_or_else(|| Order::Pass { crew: c.id.clone() }),
        None => Order::EndTurn,
    }
}

fn usable<'a>(b: &Battle, v: &'a VehicleRules, crew: &str) -> Vec<&'a StationAction> {
    let Some(c) = b.crew_member(crew) else {
        return Vec::new();
    };
    let Some(st) = c.station.as_deref().and_then(|s| v.station(s)) else {
        return Vec::new();
    };
    if b.stations_down().contains(&st.id) {
        return Vec::new();
    }
    st.actions
        .iter()
        .filter(|a| a.cost <= c.actions && (!a.attack || c.attacks > 0))
        .collect()
}

fn act(crew: &str, a: &StationAction, aim: Aim) -> Option<Order> {
    Some(Order::Act {
        crew: crew.into(),
        action: a.id.clone(),
        aim,
    })
}

fn member_order(v: &VehicleRules, b: &Battle, crew: &str) -> Option<Order> {
    let me = b.party_ship();
    let target = focus(b)?;
    let actions = usable(b, v, crew);
    let c = b.crew_member(crew)?;
    let on = |t: &Ship| Aim {
        target: Some(t.id.clone()),
        ..Aim::default()
    };
    // Shots first: the heaviest that fits.
    let mut fires: Vec<&&StationAction> = actions
        .iter()
        .filter(|a| matches!(a.effect, VehicleEffect::Fire { .. }))
        .collect();
    fires.sort_by_key(|a| std::cmp::Reverse(a.cost));
    for a in &fires {
        if a.cost > 1 && !b.heavy_allowed(v) {
            continue;
        }
        let Some(station) = c.station.as_deref() else {
            break;
        };
        let weapons: Vec<_> = v
            .ship(&me.kind)?
            .weapons
            .iter()
            .filter(|w| w.station.as_deref() == Some(station))
            .collect();
        let targets = std::iter::once(target).chain(
            b.ships
                .iter()
                .filter(|s| s.side == Side::Opposition && s.afloat()),
        );
        for t in targets {
            if let Some(w) = weapons.iter().find(|w| b.can_fire(v, me, w, t).is_ok()) {
                return act(
                    crew,
                    a,
                    Aim {
                        target: Some(t.id.clone()),
                        weapon: Some(w.id.clone()),
                        ..Aim::default()
                    },
                );
            }
        }
    }
    let attacked = c.attacks == 0;
    for a in &actions {
        let order = match &a.effect {
            VehicleEffect::Lock { .. } if target.locked == 0 && !attacked => {
                act(crew, a, on(target))
            }
            VehicleEffect::Maneuver { multiplier, turns } if *multiplier == 1 => {
                best_pose(b, v, me, target, *multiplier, *turns).and_then(|(to, facing)| {
                    act(
                        crew,
                        a,
                        Aim {
                            to: Some(to),
                            facing: Some(facing),
                            ..Aim::default()
                        },
                    )
                })
            }
            VehicleEffect::Evade { .. } if me.evade == 0 => act(crew, a, Aim::default()),
            VehicleEffect::Recharge { .. }
                if v.screen
                    .as_ref()
                    .is_some_and(|s| s.role == ScreenRole::Absorb)
                    && me.screen * 2 < me.max_screen =>
            {
                act(crew, a, Aim::default())
            }
            VehicleEffect::Repair { screen: false, .. } if me.hull < me.max_hull => {
                act(crew, a, Aim::default())
            }
            VehicleEffect::Repair { screen: true, .. } if me.screen < me.max_screen => {
                act(crew, a, Aim::default())
            }
            VehicleEffect::Fix { damages } => me
                .damages
                .iter()
                .position(|d| damages.contains(&d.id))
                .and_then(|i| {
                    act(
                        crew,
                        a,
                        Aim {
                            damage: Some(i),
                            ..Aim::default()
                        },
                    )
                }),
            VehicleEffect::BreakMorale { .. } if !attacked => b
                .ships
                .iter()
                .filter(|s| s.side == Side::Opposition && s.afloat() && s.morale.is_some())
                .min_by_key(|s| s.morale)
                .and_then(|t| act(crew, a, on(t))),
            VehicleEffect::Brace { .. } if me.brace == 0 => act(crew, a, Aim::default()),
            VehicleEffect::Scan if !target.scanned => act(crew, a, on(target)),
            VehicleEffect::Jam { .. } => b
                .ships
                .iter()
                .filter(|s| s.side == Side::Opposition && s.afloat() && s.jam == 0)
                .min_by_key(|s| distance(s.at, me.at))
                .and_then(|t| act(crew, a, on(t))),
            VehicleEffect::Rally { .. } if b.rally == 0 => act(crew, a, Aim::default()),
            _ => None,
        };
        if order.is_some() {
            return order;
        }
    }
    // A gunner with nothing to shoot runs to a free station whose weapon
    // bears.
    if !fires.is_empty() || c.station.is_none() {
        let def = v.ship(&me.kind)?;
        let taken: Vec<&str> = b.crew.iter().filter_map(|x| x.station.as_deref()).collect();
        if c.actions > v.station_change_cost {
            for w in &def.weapons {
                let Some(st) = w.station.as_deref() else {
                    continue;
                };
                if taken.contains(&st) || b.stations_down().contains(st) {
                    continue;
                }
                if b.ships
                    .iter()
                    .any(|t| t.side == Side::Opposition && b.can_fire(v, me, w, t).is_ok())
                {
                    return Some(Order::Station {
                        crew: crew.into(),
                        station: st.into(),
                    });
                }
            }
        }
    }
    None
}

/// The next order of the enemy unit whose turn it is.
pub fn enemy_order(system: &RuleSystem, b: &Battle) -> Order {
    let Some(v) = system.vehicles.as_ref() else {
        return Order::EndTurn;
    };
    let Some(unit) = b.active() else {
        return Order::EndTurn;
    };
    let target = b.party_ship();
    for id in &unit.ships {
        let Some(ship) = b.ship(id).filter(|s| s.afloat()) else {
            continue;
        };
        if !ship.fired {
            // Fire first when a weapon already bears and moving would not
            // do better.
            let best_weapon = b
                .weapons_on(v, ship, target)
                .into_iter()
                .max_by_key(|w| w.damage);
            if let Some(w) = best_weapon
                && (ship.moved || best_pose(b, v, ship, target, 1, 1).is_none())
            {
                return Order::Fire {
                    ship: ship.id.clone(),
                    weapon: w.id.clone(),
                    target: target.id.clone(),
                };
            }
        }
        if !ship.moved
            && let Some((to, facing)) = best_pose(b, v, ship, target, 1, 1)
        {
            return Order::Maneuver {
                ship: ship.id.clone(),
                to,
                facing,
            };
        }
        if !ship.fired
            && let Some(w) = b
                .weapons_on(v, ship, target)
                .into_iter()
                .max_by_key(|w| w.damage)
        {
            return Order::Fire {
                ship: ship.id.clone(),
                weapon: w.id.clone(),
                target: target.id.clone(),
            };
        }
    }
    Order::EndTurn
}

/// How many orders a battle may take before the runner gives up.
const MAX_ORDERS: usize = 5000;

/// A whole battle played by the reference tactics.
#[derive(Debug, Clone)]
pub struct BattleLog {
    pub battle: Battle,
    pub events: Vec<BattleEvent>,
    /// Orders the engine refused (the policies only ask for what it
    /// accepts: always 0 unless something is wrong).
    pub refusals: Vec<(Order, BattleRefusal)>,
}

/// Plays `battle` to its end with the crew and enemy tactics.
pub fn run_battle(system: &RuleSystem, battle: Battle, dice: &mut dyn DiceSource) -> BattleLog {
    let mut b = battle;
    let mut events = Vec::new();
    let mut refusals = Vec::new();
    for _ in 0..MAX_ORDERS {
        if b.is_over() {
            break;
        }
        let order = if b.crew_turn() {
            crew_order(system, &b)
        } else {
            enemy_order(system, &b)
        };
        match order.apply(system, &b, dice) {
            Ok(s) => {
                events.extend(s.events);
                b = s.battle;
            }
            Err(r) => {
                refusals.push((order.clone(), r));
                // Never stall: whoever was asked passes.
                let fallback = match order {
                    Order::Act { crew, .. } | Order::Station { crew, .. } => Order::Pass { crew },
                    _ => Order::EndTurn,
                };
                match fallback.apply(system, &b, dice) {
                    Ok(s) => {
                        events.extend(s.events);
                        b = s.battle;
                    }
                    Err(_) => break,
                }
            }
        }
    }
    BattleLog {
        battle: b,
        events,
        refusals,
    }
}

/// Plays the active enemy unit's whole turn on a copy, for the GM to
/// accept (the co-GM's proposal): the orders and where they lead.
pub fn propose_enemy_turn(
    system: &RuleSystem,
    battle: &Battle,
    dice: &mut dyn DiceSource,
) -> (Vec<Order>, Vec<BattleEvent>, Battle) {
    let mut b = battle.clone();
    let mut orders = Vec::new();
    let mut events = Vec::new();
    let unit = battle.active().map(|u| u.id.clone());
    for _ in 0..32 {
        if b.is_over() || b.crew_turn() || b.active().map(|u| u.id.clone()) != unit {
            break;
        }
        let order = enemy_order(system, &b);
        let ending = order == Order::EndTurn;
        match order.apply(system, &b, dice) {
            Ok(s) => {
                orders.push(order);
                events.extend(s.events);
                b = s.battle;
            }
            Err(_) => {
                if let Ok(s) = Order::EndTurn.apply(system, &b, dice) {
                    orders.push(Order::EndTurn);
                    events.extend(s.events);
                    b = s.battle;
                }
                break;
            }
        }
        if ending {
            break;
        }
    }
    (orders, events, b)
}

/// Each crew member's station by class: the first free station that
/// suits them, in the order of the rules' stations.
pub fn seat_by_class(
    v: &VehicleRules,
    classes: &[(String, Option<String>)],
) -> BTreeMap<String, String> {
    let mut seats = BTreeMap::new();
    let mut taken: Vec<&str> = Vec::new();
    for (crew, class) in classes {
        let Some(class) = class else { continue };
        if let Some(st) = v
            .stations
            .iter()
            .find(|s| s.suits.contains(class) && !taken.contains(&s.id.as_str()))
        {
            taken.push(&st.id);
            seats.insert(crew.clone(), st.id.clone());
        }
    }
    seats
}
