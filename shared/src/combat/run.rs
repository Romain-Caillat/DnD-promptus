//! Playing a whole fight without a table: each side is a [`Policy`] that
//! decides, one command at a time, what its active combatant does. The
//! fight simulator (`engine/simulate-fights`) and the engine's own tests
//! run on this; at a real table the commands come from the players and
//! the GM instead.

use std::cmp::Reverse;
use std::collections::HashSet;

use serde::Serialize;

use crate::maps::{Cell, Diagonal, shortest_path};
use crate::rules::action::{ActionRef, action_cards, affordable};
use crate::rules::dice::DiceSource;
use crate::rules::model::{ActionDef, AreaShape, RuleSystem, Tag, Targeting};
use crate::rules::sheet::{Combatant, Side};

use super::fight::{CombatRefusal, Fight, FightEvent, Play, Step, movement_rules};
use super::reach::sight;

/// One command for the active combatant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Move(Vec<Cell>),
    Act(Play),
    /// `difficulty`: the GM asks for a roll to get away.
    Flee {
        difficulty: Option<i32>,
    },
    EndTurn,
}

/// Decides for one side. Called again after every accepted command
/// until it ends the turn.
pub trait Policy {
    fn decide(&mut self, system: &RuleSystem, fight: &Fight, who: &str) -> Decision;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Limits {
    /// The GM stops the fight after this many rounds.
    pub max_rounds: u32,
    /// Commands one turn may take before it is ended.
    pub max_commands_per_turn: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_rounds: 50,
            max_commands_per_turn: 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "entry", rename_all = "snake_case")]
pub enum LogEntry {
    Events {
        events: Vec<FightEvent>,
    },
    /// A policy asked for something the engine refused; its turn ended.
    Refused {
        who: String,
        refusal: CombatRefusal,
    },
}

/// A fight played to its end, with everything that happened.
#[derive(Debug, Clone)]
pub struct FightLog {
    pub fight: Fight,
    pub log: Vec<LogEntry>,
}

impl FightLog {
    pub fn events(&self) -> impl Iterator<Item = &FightEvent> {
        self.log.iter().flat_map(|e| match e {
            LogEntry::Events { events } => events.as_slice(),
            LogEntry::Refused { .. } => &[],
        })
    }

    pub fn refusals(&self) -> impl Iterator<Item = (&str, &CombatRefusal)> {
        self.log.iter().filter_map(|e| match e {
            LogEntry::Refused { who, refusal } => Some((who.as_str(), refusal)),
            LogEntry::Events { .. } => None,
        })
    }
}

/// Plays `start` to its end: the party's policy decides for the party,
/// the opposition's for the opposition. A refused command ends that
/// combatant's turn (it is logged); past `limits.max_rounds` the GM
/// stops the fight.
pub fn run_fight(
    system: &RuleSystem,
    start: Step,
    party: &mut dyn Policy,
    opposition: &mut dyn Policy,
    dice: &mut dyn DiceSource,
    limits: Limits,
) -> FightLog {
    let mut log = vec![LogEntry::Events {
        events: start.events,
    }];
    let mut fight = start.fight;
    let mut commands = 0;
    while let Some(who) = fight.active().map(str::to_string) {
        if fight.round > limits.max_rounds {
            let step = fight.stop();
            log.push(LogEntry::Events {
                events: step.events,
            });
            fight = step.fight;
            break;
        }
        let side = fight.combatant(&who).map(|c| c.side);
        let policy: &mut dyn Policy = match side {
            Some(Side::Party) => party,
            _ => opposition,
        };
        let decision = if commands >= limits.max_commands_per_turn {
            Decision::EndTurn
        } else {
            policy.decide(system, &fight, &who)
        };
        let ending = decision == Decision::EndTurn;
        let result = match decision {
            Decision::Move(path) => fight.move_along(system, &who, &path),
            Decision::Act(play) => fight.act(system, &play, dice),
            Decision::Flee { difficulty } => fight.flee(system, &who, difficulty, dice),
            Decision::EndTurn => fight.end_turn(system, &who, dice),
        };
        let step = match result {
            Ok(step) => step,
            Err(refusal) => {
                log.push(LogEntry::Refused {
                    who: who.clone(),
                    refusal,
                });
                fight
                    .end_turn(system, &who, dice)
                    .expect("the active combatant can always end their turn")
            }
        };
        let turn_changed = ending || step.fight.active() != Some(who.as_str());
        commands = if turn_changed { 0 } else { commands + 1 };
        log.push(LogEntry::Events {
            events: step.events,
        });
        fight = step.fight;
    }
    FightLog { fight, log }
}

/// A plain fighter: heals a fallen or badly hurt ally in reach, else
/// hits the nearest enemy it can, else helps an ally, else walks
/// towards the nearest enemy; ends its turn when nothing is left. The
/// baseline both sides use in the engine's tests.
#[derive(Debug, Clone)]
pub struct Brawler {
    /// The difficulty the GM gives for saves the system leaves open.
    pub gm_difficulty: i32,
}

impl Default for Brawler {
    fn default() -> Self {
        Self { gm_difficulty: 10 }
    }
}

/// A slightly better baseline, still not an AI: heals like the
/// [`Brawler`], but hits the weakest enemy it can reach (fewest hit
/// points, then nearest) so the side focuses its fire; when it can hit
/// nobody, it takes the cheapest cell from which it could, and it never
/// stops in a doorway, so it does not cork a door its allies need.
#[derive(Debug, Clone)]
pub struct Focus {
    /// The difficulty the GM gives for saves the system leaves open.
    pub gm_difficulty: i32,
}

impl Default for Focus {
    fn default() -> Self {
        Self { gm_difficulty: 10 }
    }
}

fn deals_damage(tags: &[Tag]) -> bool {
    tags.iter().any(|t| match t {
        Tag::Damage(_) => true,
        Tag::Choice(options) => deals_damage(options),
        _ => false,
    })
}

fn heals(tags: &[Tag]) -> bool {
    tags.iter().any(|t| matches!(t, Tag::Heal(_)))
}

fn foe_of(side: Side) -> Side {
    match side {
        Side::Party => Side::Opposition,
        Side::Opposition => Side::Party,
    }
}

/// How a policy ranks the enemies it may hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pick {
    Nearest,
    Weakest,
}

/// Enemies still standing, in the order `pick` prefers them.
fn enemies(system: &RuleSystem, fight: &Fight, who: &str, pick: Pick) -> Vec<String> {
    let (Some(me), Some(from)) = (fight.combatant(who), fight.position(who)) else {
        return Vec::new();
    };
    let diagonal = fight.diagonal(system);
    let mut foes: Vec<(i32, u32, String)> = fight
        .side(foe_of(me.side))
        .filter(|id| fight.standing_up(id))
        .filter_map(|id| {
            let at = fight.position(id)?;
            let hp = match pick {
                Pick::Nearest => 0,
                Pick::Weakest => fight.combatant(id)?.hit_points,
            };
            Some((hp, fight.map.distance(from, at, diagonal), id.to_string()))
        })
        .collect();
    foes.sort();
    foes.into_iter().map(|(_, _, id)| id).collect()
}

/// Plays this action could make now, best first.
fn candidates(
    system: &RuleSystem,
    fight: &Fight,
    who: &str,
    action: &ActionDef,
    enemies: &[String],
) -> Vec<Play> {
    let me = fight.combatant(who).expect("active");
    let id = action.id.as_str();
    match (action.target, action.area) {
        (Targeting::Enemy, _) => enemies.iter().map(|e| Play::on(who, id, &[e])).collect(),
        (Targeting::Enemies, Some(AreaShape::MeleeBurst)) => vec![Play::alone(who, id)],
        (Targeting::Enemies, Some(_)) => enemies
            .iter()
            .filter_map(|e| fight.position(e))
            .map(|at| Play::at(who, id, at))
            .collect(),
        (Targeting::Ally | Targeting::AllyOrSelf, _) => {
            let mut allies: Vec<(i32, String)> = fight
                .side(me.side)
                .filter(|a| action.target == Targeting::AllyOrSelf || *a != who)
                .filter_map(|a| {
                    let c = fight.combatant(a)?;
                    Some((c.hit_points, a.to_string()))
                })
                .collect();
            if heals(&action.tags) {
                // Only those who need it: down, or at half or less.
                allies.retain(|(hp, a)| {
                    let max = fight
                        .combatant(a)
                        .and_then(|c| c.max_hit_points(system).ok())
                        .unwrap_or(0);
                    *hp * 2 <= max
                });
            }
            allies.sort();
            allies
                .into_iter()
                .map(|(_, a)| Play::on(who, id, &[&a]))
                .collect()
        }
        (Targeting::Myself | Targeting::AllAllies, _) => vec![Play::alone(who, id)],
        (Targeting::Enemies, None) => Vec::new(),
    }
}

/// What `me` can play now: unlocked cards, then carried items.
fn options<'s>(system: &'s RuleSystem, me: &'s Combatant) -> Vec<(ActionRef, &'s ActionDef)> {
    let mut options: Vec<(ActionRef, &ActionDef)> = action_cards(system, me)
        .into_iter()
        .filter(|c| c.locked.is_none())
        .map(|c| (ActionRef::Own(c.action.id.clone()), c.action))
        .collect();
    for (item, qty) in &me.inventory {
        if let Some(a) = system.item(item).and_then(|i| i.action.as_ref())
            && *qty > 0
        {
            options.push((ActionRef::Item(item.clone()), a));
        }
    }
    options
}

/// The first legal play among the actions `keep` selects.
fn first_play(
    gm_difficulty: i32,
    system: &RuleSystem,
    fight: &Fight,
    who: &str,
    pick: Pick,
    keep: impl Fn(&ActionDef) -> bool,
) -> Option<Play> {
    let me = fight.combatant(who)?;
    let enemies = enemies(system, fight, who, pick);
    for (r, action) in options(system, me).into_iter().filter(|(_, a)| keep(a)) {
        for mut play in candidates(system, fight, who, action, &enemies) {
            play.action = r.clone();
            play.save_difficulty = Some(gm_difficulty);
            play.choice = Some(0);
            if fight.check(system, &play).is_ok() {
                return Some(play);
            }
        }
    }
    None
}

/// Heal, else hurt, else help: the play a policy makes when it can act.
fn best_play(
    gm_difficulty: i32,
    system: &RuleSystem,
    fight: &Fight,
    who: &str,
    pick: Pick,
) -> Option<Play> {
    let first = |keep: &dyn Fn(&ActionDef) -> bool| {
        first_play(gm_difficulty, system, fight, who, pick, keep)
    };
    first(&|a| heals(&a.tags))
        .or_else(|| first(&|a| deals_damage(&a.tags)))
        .or_else(|| {
            first(&|a| {
                !deals_damage(&a.tags)
                    && !heals(&a.tags)
                    && matches!(a.target, Targeting::Ally | Targeting::AllyOrSelf)
            })
        })
}

/// Whether `me` can still move this turn.
fn can_move(system: &RuleSystem, fight: &Fight, me: &Combatant) -> bool {
    match &system.combat.move_kind {
        Some(kind) => affordable(system, &fight.scene, me, kind).is_ok(),
        None => !fight.moved,
    }
}

fn is_door(fight: &Fight, cell: Cell) -> bool {
    fight.map.doors.iter().any(|d| d.at == cell)
}

fn path_to(system: &RuleSystem, fight: &Fight, who: &str, dest: Cell) -> Option<Vec<Cell>> {
    let from = fight.position(who)?;
    let (path, _) = shortest_path(
        &fight.map,
        &movement_rules(system, &fight.map),
        &fight.occupancy(who),
        from,
        dest,
        fight.move_budget(system, who),
    )?;
    Some(path)
}

/// The path to the reachable cell nearest the nearest enemy, if it gets
/// closer than where `who` stands; door cells are skipped when
/// `avoid_doors`.
fn approach(system: &RuleSystem, fight: &Fight, who: &str, avoid_doors: bool) -> Option<Vec<Cell>> {
    let me = fight.combatant(who)?;
    let from = fight.position(who)?;
    if !can_move(system, fight, me) {
        return None;
    }
    let targets: Vec<Cell> = enemies(system, fight, who, Pick::Nearest)
        .iter()
        .filter_map(|e| fight.position(e))
        .collect();
    let gap = |c: Cell| {
        targets
            .iter()
            .map(|t| fight.map.distance(c, *t, Diagonal::Chebyshev))
            .min()
            .unwrap_or(u32::MAX)
    };
    let now = gap(from);
    let (dest, _) = fight
        .reachable(system, who)
        .into_iter()
        .filter(|(c, _)| *c != from && !(avoid_doors && is_door(fight, *c)))
        .min_by_key(|(c, cost)| (gap(*c), *cost, Reverse(c.y), c.x))?;
    if gap(dest) >= now {
        return None;
    }
    path_to(system, fight, who, dest)
}

/// The cheapest reachable cell, not a doorway, from which `who` could
/// hit a standing enemy with one of its damaging actions (range and
/// sight; cooldowns and the turn budget are checked when it acts).
fn firing_position(system: &RuleSystem, fight: &Fight, who: &str) -> Option<Vec<Cell>> {
    let me = fight.combatant(who)?;
    let from = fight.position(who)?;
    if !can_move(system, fight, me) {
        return None;
    }
    let reach = options(system, me)
        .into_iter()
        .filter(|(_, a)| deals_damage(&a.tags))
        .filter(|(_, a)| matches!(a.target, Targeting::Enemy | Targeting::Enemies))
        .map(|(_, a)| a.long_range.unwrap_or(0).max(a.reach()))
        .max()?;
    let diagonal = fight.diagonal(system);
    let foes: Vec<(&str, Cell)> = fight
        .side(foe_of(me.side))
        .filter(|id| fight.standing_up(id))
        .filter_map(|id| Some((id, fight.position(id)?)))
        .collect();
    let can_hit = |c: Cell| {
        foes.iter().any(|(id, at)| {
            if fight.map.distance(c, *at, diagonal) > reach {
                return false;
            }
            let bodies: HashSet<Cell> = fight
                .positions
                .iter()
                .filter(|(other, _)| other.as_str() != who && other.as_str() != *id)
                .map(|(_, cell)| *cell)
                .collect();
            sight(&fight.map, diagonal, c, *at, &bodies).is_some()
        })
    };
    let (dest, _) = fight
        .reachable(system, who)
        .into_iter()
        .filter(|(c, _)| *c != from && !is_door(fight, *c))
        .filter(|(c, _)| can_hit(*c))
        .min_by_key(|(c, cost)| (*cost, c.y, c.x))?;
    path_to(system, fight, who, dest)
}

impl Policy for Brawler {
    fn decide(&mut self, system: &RuleSystem, fight: &Fight, who: &str) -> Decision {
        let Some(me) = fight.combatant(who) else {
            return Decision::EndTurn;
        };
        if me.turn.actions_left == 0 {
            return Decision::EndTurn;
        }
        if let Some(play) = best_play(self.gm_difficulty, system, fight, who, Pick::Nearest) {
            return Decision::Act(play);
        }
        match approach(system, fight, who, false) {
            Some(path) => Decision::Move(path),
            None => Decision::EndTurn,
        }
    }
}

impl Policy for Focus {
    fn decide(&mut self, system: &RuleSystem, fight: &Fight, who: &str) -> Decision {
        let Some(me) = fight.combatant(who) else {
            return Decision::EndTurn;
        };
        if me.turn.actions_left == 0 {
            return Decision::EndTurn;
        }
        if let Some(play) = best_play(self.gm_difficulty, system, fight, who, Pick::Weakest) {
            return Decision::Act(play);
        }
        match firing_position(system, fight, who).or_else(|| approach(system, fight, who, true)) {
            Some(path) => Decision::Move(path),
            None => Decision::EndTurn,
        }
    }
}
