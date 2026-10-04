//! Playing a whole fight without a table: each side is a [`Policy`] that
//! decides, one command at a time, what its active combatant does. The
//! fight simulator (`engine/simulate-fights`) and the engine's own tests
//! run on this; at a real table the commands come from the players and
//! the GM instead.

use std::cmp::Reverse;

use serde::Serialize;

use crate::maps::{Cell, Diagonal, shortest_path};
use crate::rules::action::{ActionRef, action_cards, affordable};
use crate::rules::dice::DiceSource;
use crate::rules::model::{AreaShape, RuleSystem, Tag, Targeting};
use crate::rules::sheet::Side;

use super::fight::{CombatRefusal, Fight, FightEvent, Play, Step, movement_rules};

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

#[derive(Debug, Clone, Copy)]
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

impl Brawler {
    fn play(&self, mut play: Play) -> Play {
        play.save_difficulty = Some(self.gm_difficulty);
        play.choice = Some(0);
        play
    }

    /// Enemies still standing, nearest first.
    fn enemies_by_distance(&self, system: &RuleSystem, fight: &Fight, who: &str) -> Vec<String> {
        let (Some(me), Some(from)) = (fight.combatant(who), fight.position(who)) else {
            return Vec::new();
        };
        let diagonal = fight.diagonal(system);
        let foe = match me.side {
            Side::Party => Side::Opposition,
            Side::Opposition => Side::Party,
        };
        let mut foes: Vec<(u32, String)> = fight
            .side(foe)
            .filter(|id| fight.standing_up(id))
            .filter_map(|id| {
                let at = fight.position(id)?;
                Some((fight.map.distance(from, at, diagonal), id.to_string()))
            })
            .collect();
        foes.sort();
        foes.into_iter().map(|(_, id)| id).collect()
    }

    /// Plays this action could make now, best first.
    fn candidates(
        &self,
        system: &RuleSystem,
        fight: &Fight,
        who: &str,
        action: &crate::rules::model::ActionDef,
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

    /// The first legal play among the actions `keep` selects.
    fn first_play(
        &self,
        system: &RuleSystem,
        fight: &Fight,
        who: &str,
        keep: impl Fn(&crate::rules::model::ActionDef) -> bool,
    ) -> Option<Play> {
        let me = fight.combatant(who)?;
        let enemies = self.enemies_by_distance(system, fight, who);
        let mut options: Vec<(ActionRef, &crate::rules::model::ActionDef)> =
            action_cards(system, me)
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
        for (r, action) in options.into_iter().filter(|(_, a)| keep(a)) {
            for mut play in self.candidates(system, fight, who, action, &enemies) {
                play.action = r.clone();
                let play = self.play(play);
                if fight.check(system, &play).is_ok() {
                    return Some(play);
                }
            }
        }
        None
    }

    /// The path to the reachable cell nearest the nearest enemy, if it
    /// gets closer than where `who` stands.
    fn approach(&self, system: &RuleSystem, fight: &Fight, who: &str) -> Option<Vec<Cell>> {
        let me = fight.combatant(who)?;
        let from = fight.position(who)?;
        if let Some(kind) = &system.combat.move_kind {
            affordable(system, &fight.scene, me, kind).ok()?;
        } else if fight.moved {
            return None;
        }
        let targets: Vec<Cell> = self
            .enemies_by_distance(system, fight, who)
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
            .filter(|(c, _)| *c != from)
            .min_by_key(|(c, cost)| (gap(*c), *cost, Reverse(c.y), c.x))?;
        if gap(dest) >= now {
            return None;
        }
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
}

impl Policy for Brawler {
    fn decide(&mut self, system: &RuleSystem, fight: &Fight, who: &str) -> Decision {
        let Some(me) = fight.combatant(who) else {
            return Decision::EndTurn;
        };
        if me.turn.actions_left == 0 {
            return Decision::EndTurn;
        }
        let picks = [
            self.first_play(system, fight, who, |a| heals(&a.tags)),
            self.first_play(system, fight, who, |a| deals_damage(&a.tags)),
            self.first_play(system, fight, who, |a| {
                !deals_damage(&a.tags)
                    && !heals(&a.tags)
                    && matches!(a.target, Targeting::Ally | Targeting::AllyOrSelf)
            }),
        ];
        if let Some(play) = picks.into_iter().flatten().next() {
            return Decision::Act(play);
        }
        match self.approach(system, fight, who) {
            Some(path) => Decision::Move(path),
            None => Decision::EndTurn,
        }
    }
}
