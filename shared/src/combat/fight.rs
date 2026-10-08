//! A fight on a grid map: who stands where, the initiative order, whose
//! turn it is, and every command a combatant may give — move, act,
//! flee, end the turn. Each command takes a fight and returns the next
//! fight plus what happened, or a refusal with nothing changed.
//!
//! The rules (`rules::action`, `rules::conditions`) decide what an action
//! does; this layer adds what the grid decides — reach, sight, cover,
//! areas, paths — and the fight's own life: initiative, rounds, who is
//! out of it, and when it is over.
//!
//! Who leaves the fight:
//! - an adversary brought to 0 hit points is **defeated** at once
//!   (INTERPRETATION: the systems' zero-HP rule is written for player
//!   characters; an adversary has no "out after N turns");
//! - a character the zero-HP rule puts out of the scene (`hors_combat`)
//!   is **out of the scene**; a knocked-out one stays, on the ground,
//!   until healed or out;
//! - whoever flees is **fled**;
//! - under the `death_saves` rule a character at 0 stays in the fight,
//!   dying: their turn opens for the save alone ([`Fight::death_save`]);
//!   when the failures add up, the engine proposes the death and the GM
//!   confirms it ([`Fight::confirm_death`]: **dead**) or decides
//!   otherwise ([`Fight::spare`]). The GM may also confirm the death of
//!   any character down at 0, under any rule (engine/save-against-death).
//!
//! The fight is over when a side has nobody left standing — in the fight,
//! above 0 hit points — or when the GM stops it.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::maps::{
    Cell, Diagonal, Map, MovementRules, Obstacle, Occupancy, PathError, Scale, check_path,
    reachable, standable,
};
use crate::rules::action::{
    self, ActionRef, ActionRequest, Positional, Refusal, affordable, find_action, resolve_action,
    spend,
};
use crate::rules::check::{self, Advantage, OutcomeBand, RollBreakdown, RollTarget};
use crate::rules::conditions::{self, TurnError};
use crate::rules::death::{self, DeathRefusal};
use crate::rules::dice::DiceSource;
use crate::rules::events::Event;
use crate::rules::model::{
    ActionDef, AreaShape, MapScale, RollScope, RollSpec, RuleSystem, Targeting, ZeroHpRule,
};
use crate::rules::progression::award_band;
use crate::rules::sheet::{Combatant, Scene, SheetError, Side};

use super::reach::{self, Reach, ReachRefusal};

/// Movement per move when the rule system leaves `cells_per_move` null:
/// 6 cells (9 m on an encounter map, the D&D 5e walking speed — the
/// same source as `maps::MovementRules`' defaults). INTERPRETATION for
/// both witness worlds, whose rules give no distance.
pub const DEFAULT_CELLS_PER_MOVE: u32 = 6;

/// How many turns may pass without a fight ending before it is called
/// off as a stalemate (only reachable with conditions that never end).
const MAX_SKIPPED_TURNS_PER_COMBATANT: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    InFight,
    /// An adversary at 0 hit points.
    Defeated,
    /// Put out of the scene by the zero-HP rule.
    OutOfScene,
    Fled,
    /// A character whose death the GM confirmed.
    Dead,
}

/// One initiative roll: the die faces, the bonus, the total, and the
/// tie-breaking rerolls when the system rerolls ties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitiativeRoll {
    pub who: String,
    pub faces: Vec<u32>,
    pub bonus: i32,
    pub total: i32,
    pub rerolls: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FightEnd {
    /// `None` when the GM stopped it or it stalled.
    pub winner: Option<Side>,
    pub reason: EndReason,
    pub rounds: u32,
    pub defeated: Vec<String>,
    pub fled: Vec<String>,
    pub out_of_scene: Vec<String>,
    /// Characters whose death the GM confirmed.
    #[serde(default)]
    pub dead: Vec<String>,
    /// XP each player character gained during the fight (per the bands
    /// of their rolls), by id.
    pub xp: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    /// One side has nobody standing.
    SideDown,
    StoppedByGm,
    Stalemate,
}

/// What a fight reports, in order: its own moments around the rules'
/// events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FightEvent {
    InitiativeRolled {
        roll: InitiativeRoll,
    },
    Order {
        order: Vec<String>,
    },
    RoundStarted {
        round: u32,
    },
    TurnStarted {
        who: String,
        round: u32,
    },
    Moved {
        who: String,
        path: Vec<Cell>,
        cost: u32,
        budget: u32,
    },
    Acted {
        who: String,
        action: String,
        targets: Vec<String>,
    },
    Rules {
        event: Event,
    },
    FleeRoll {
        who: String,
        roll: RollBreakdown,
    },
    FleeFailed {
        who: String,
    },
    Fled {
        who: String,
    },
    Defeated {
        who: String,
    },
    LeftTheScene {
        who: String,
    },
    /// The GM confirmed the death.
    Died {
        who: String,
    },
    TurnEnded {
        who: String,
    },
    Ended {
        end: FightEnd,
    },
}

/// Why a fight cannot start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupError {
    UnknownContext(String),
    /// A fight needs someone on each side.
    MissingSide(Side),
    DuplicateId(String),
    CannotStand {
        who: String,
        at: Cell,
        obstacle: Obstacle,
    },
    SameCell {
        a: String,
        b: String,
        at: Cell,
    },
    Sheet(SheetError),
    Turn(TurnError),
}

/// Why a command is refused. Nothing changes when one is returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatRefusal {
    FightOver,
    NotTheirTurn,
    NotInFight {
        who: String,
    },
    CannotMove,
    AlreadyMoved,
    Path {
        error: String,
    },
    /// The aim does not fit the action (targets for an area, a cell for
    /// a single target…).
    WrongAim,
    OutOfReach {
        target: String,
        why: ReachRefusal,
    },
    AimOutOfReach {
        why: ReachRefusal,
    },
    AreaEmpty,
    NoFleeRoll,
    /// Dying: the turn is the death save, nothing else.
    DeathSaveFirst,
    /// Not dying (or stable, or waiting for the GM).
    NotDying {
        who: String,
    },
    /// The system has no death saves, or no way to stabilise.
    NoDeathSaves,
    NoStabilizeRule,
    /// Stabilising takes being next to them.
    TooFar {
        target: String,
    },
    /// Only a character down at 0 hit points can die.
    NotDown {
        who: String,
    },
    Rules {
        refusal: Refusal,
    },
}

impl From<DeathRefusal> for CombatRefusal {
    fn from(r: DeathRefusal) -> Self {
        match r {
            DeathRefusal::NoDeathSaves => Self::NoDeathSaves,
            DeathRefusal::NoStabilizeRule => Self::NoStabilizeRule,
            DeathRefusal::UnknownCombatant(who) => Self::NotInFight { who },
            DeathRefusal::NotRolling(who) => Self::NotDying { who },
            DeathRefusal::Engine(detail) => Self::Rules {
                refusal: Refusal::Engine { detail },
            },
        }
    }
}

impl From<Refusal> for CombatRefusal {
    fn from(refusal: Refusal) -> Self {
        Self::Rules { refusal }
    }
}

impl From<PathError> for CombatRefusal {
    fn from(e: PathError) -> Self {
        Self::Path {
            error: e.to_string(),
        }
    }
}

/// What an action is aimed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Aim {
    /// Self, every ally, or a burst around the actor.
    Nothing,
    /// Chosen combatants (one enemy, one ally).
    Targets(Vec<String>),
    /// A zone's centre, or the cell a line points to.
    Cell(Cell),
}

/// An action played in a fight. The grid turns the aim into targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Play {
    pub actor: String,
    pub action: ActionRef,
    pub aim: Aim,
    pub situations: Vec<String>,
    pub save_difficulty: Option<i32>,
    pub choice: Option<usize>,
}

impl Play {
    fn new(actor: &str, action: ActionRef, aim: Aim) -> Self {
        Self {
            actor: actor.into(),
            action,
            aim,
            situations: Vec::new(),
            save_difficulty: None,
            choice: None,
        }
    }

    /// An own action on chosen combatants.
    pub fn on(actor: &str, action: &str, targets: &[&str]) -> Self {
        let targets = targets.iter().map(|t| t.to_string()).collect();
        Self::new(actor, ActionRef::Own(action.into()), Aim::Targets(targets))
    }

    /// An own action aimed at a cell (zone, line).
    pub fn at(actor: &str, action: &str, cell: Cell) -> Self {
        Self::new(actor, ActionRef::Own(action.into()), Aim::Cell(cell))
    }

    /// An own action with no aim (self, every ally, a burst).
    pub fn alone(actor: &str, action: &str) -> Self {
        Self::new(actor, ActionRef::Own(action.into()), Aim::Nothing)
    }

    /// Using a carried item on chosen combatants.
    pub fn item(actor: &str, item: &str, targets: &[&str]) -> Self {
        let targets = targets.iter().map(|t| t.to_string()).collect();
        Self::new(actor, ActionRef::Item(item.into()), Aim::Targets(targets))
    }
}

/// A play with its targets picked and checked on the grid.
#[derive(Debug, Clone)]
pub struct Aimed {
    pub action: ActionDef,
    pub targets: Vec<String>,
    /// What each target's position adds to the attack roll.
    pub positional: BTreeMap<String, Positional>,
}

/// The next fight and what happened on the way.
#[derive(Debug, Clone)]
pub struct Step {
    pub fight: Fight,
    pub events: Vec<FightEvent>,
}

/// The whole state of a fight. Pure data: the server stores it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fight {
    /// Everyone who took part, including those now out of it.
    pub scene: Scene,
    pub map: Map,
    /// Cells of those still in the fight.
    pub positions: BTreeMap<String, Cell>,
    /// Initiative rolls, in turn order.
    pub initiative: Vec<InitiativeRoll>,
    /// Turn order for the whole fight (it never changes).
    pub order: Vec<String>,
    pub standing: BTreeMap<String, Standing>,
    pub round: u32,
    /// Index in `order` of the current turn.
    pub turn: usize,
    /// The free move of this turn is spent (systems with no move kind).
    pub moved: bool,
    xp_at_start: BTreeMap<String, u32>,
    pub end: Option<FightEnd>,
}

fn rules_events(events: Vec<Event>) -> impl Iterator<Item = FightEvent> {
    events.into_iter().map(|event| FightEvent::Rules { event })
}

/// The movement costs of the system at the map's scale.
pub fn movement_rules(system: &RuleSystem, map: &Map) -> MovementRules {
    system
        .movement(map_scale(map.scale))
        .and_then(|m| m.grid)
        .unwrap_or_default()
}

fn map_scale(scale: Scale) -> MapScale {
    match scale {
        Scale::World => MapScale::World,
        Scale::Place => MapScale::Place,
        Scale::Encounter => MapScale::Encounter,
    }
}

impl Fight {
    /// Opens a fight: everyone placed, initiative rolled, the first turn
    /// begun. `context` is the rule system's turn context (`sol`).
    pub fn start(
        system: &RuleSystem,
        context: &str,
        map: Map,
        placements: Vec<(Combatant, Cell)>,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, SetupError> {
        if system.turn_context(context).is_none() {
            return Err(SetupError::UnknownContext(context.into()));
        }
        for side in [Side::Party, Side::Opposition] {
            if !placements.iter().any(|(c, _)| c.side == side) {
                return Err(SetupError::MissingSide(side));
            }
        }
        let rules = movement_rules(system, &map);
        let mut positions: BTreeMap<String, Cell> = BTreeMap::new();
        let mut taken: BTreeMap<Cell, String> = BTreeMap::new();
        for (c, at) in &placements {
            if positions.contains_key(&c.id) {
                return Err(SetupError::DuplicateId(c.id.clone()));
            }
            standable(&map, &rules, *at).map_err(|obstacle| SetupError::CannotStand {
                who: c.id.clone(),
                at: *at,
                obstacle,
            })?;
            if let Some(other) = taken.insert(*at, c.id.clone()) {
                return Err(SetupError::SameCell {
                    a: other,
                    b: c.id.clone(),
                    at: *at,
                });
            }
            positions.insert(c.id.clone(), *at);
        }

        let mut events = Vec::new();
        let initiative = roll_initiative(system, &placements, dice)?;
        for roll in &initiative {
            events.push(FightEvent::InitiativeRolled { roll: roll.clone() });
        }
        let order: Vec<String> = initiative.iter().map(|r| r.who.clone()).collect();
        events.push(FightEvent::Order {
            order: order.clone(),
        });

        let combatants: Vec<Combatant> = placements.into_iter().map(|(c, _)| c).collect();
        let xp_at_start = combatants
            .iter()
            .filter_map(|c| c.progress.map(|p| (c.id.clone(), p.total_xp)))
            .collect();
        let standing = combatants
            .iter()
            .map(|c| (c.id.clone(), Standing::InFight))
            .collect();
        let mut fight = Fight {
            scene: Scene::new(context, combatants),
            map,
            positions,
            initiative,
            order,
            standing,
            round: 1,
            turn: 0,
            moved: false,
            xp_at_start,
            end: None,
        };
        events.push(FightEvent::RoundStarted { round: 1 });
        fight
            .advance(system, None, dice, &mut events)
            .map_err(SetupError::Turn)?;
        Ok(Step { fight, events })
    }

    /// Whose turn it is (none once the fight is over).
    pub fn active(&self) -> Option<&str> {
        self.scene.active.as_deref()
    }

    pub fn is_over(&self) -> bool {
        self.end.is_some()
    }

    pub fn in_fight(&self, id: &str) -> bool {
        self.standing.get(id) == Some(&Standing::InFight)
    }

    pub fn combatant(&self, id: &str) -> Option<&Combatant> {
        self.scene.get(id)
    }

    pub fn position(&self, id: &str) -> Option<Cell> {
        self.positions.get(id).copied()
    }

    /// The system's diagonal rule at this map's scale.
    pub fn diagonal(&self, system: &RuleSystem) -> Diagonal {
        movement_rules(system, &self.map).diagonal
    }

    /// Dying and still rolling: `id`'s turn is their death save.
    pub fn awaits_death_save(&self, id: &str) -> bool {
        self.in_fight(id)
            && self
                .scene
                .get(id)
                .is_some_and(|c| c.hit_points == 0 && c.death_saves.is_some_and(|d| d.rolling()))
    }

    /// Characters whose death the dice propose, waiting for the GM.
    pub fn deaths_due(&self) -> impl Iterator<Item = &str> {
        self.scene
            .combatants
            .values()
            .filter(|c| {
                c.death_saves.is_some_and(|d| d.death_due)
                    && self.standing.get(&c.id) != Some(&Standing::Dead)
            })
            .map(|c| c.id.as_str())
    }

    /// Still in the fight and above 0 hit points.
    pub fn standing_up(&self, id: &str) -> bool {
        self.in_fight(id) && self.scene.get(id).is_some_and(|c| c.hit_points > 0)
    }

    /// Ids of the combatants of `side` still in the fight.
    pub fn side(&self, side: Side) -> impl Iterator<Item = &str> {
        self.scene
            .combatants
            .values()
            .filter(move |c| c.side == side && self.in_fight(&c.id))
            .map(|c| c.id.as_str())
    }

    /// Cells where bodies stand, except these.
    fn bodies_except(&self, except: &[&str]) -> HashSet<Cell> {
        self.positions
            .iter()
            .filter(|(id, _)| !except.contains(&id.as_str()))
            .map(|(_, c)| *c)
            .collect()
    }

    /// Who blocks `who`'s way: standing enemies block; allies and bodies
    /// on the ground can be crossed but not stopped on (INTERPRETATION:
    /// a knocked-out body is stepped over).
    pub fn occupancy(&self, who: &str) -> Occupancy {
        let side = self.scene.get(who).map(|c| c.side);
        let mut occ = Occupancy::default();
        for (id, cell) in &self.positions {
            if id == who {
                continue;
            }
            let enemy = self.scene.get(id).map(|c| c.side) != side;
            if enemy && self.standing_up(id) {
                occ.enemies.insert(*cell);
            } else {
                occ.allies.insert(*cell);
            }
        }
        occ
    }

    /// Cells one move lets `who` cover: the system's cells per move
    /// (or [`DEFAULT_CELLS_PER_MOVE`]), scaled by their conditions.
    pub fn move_budget(&self, system: &RuleSystem, who: &str) -> u32 {
        let base = system
            .movement(map_scale(self.map.scale))
            .and_then(|m| m.cells_per_move)
            .unwrap_or(DEFAULT_CELLS_PER_MOVE);
        let percent = self.scene.get(who).map_or(0, Combatant::movement_percent);
        base * percent / 100
    }

    /// Every cell `who` could end one move on, with its cost.
    pub fn reachable(&self, system: &RuleSystem, who: &str) -> BTreeMap<Cell, u32> {
        let Some(start) = self.position(who) else {
            return BTreeMap::new();
        };
        reachable(
            &self.map,
            &movement_rules(system, &self.map),
            &self.occupancy(who),
            start,
            self.move_budget(system, who),
        )
    }

    /// Range and sight of an action from its user to a target.
    pub fn reach(
        &self,
        system: &RuleSystem,
        actor: &str,
        action: &ActionDef,
        target: &str,
    ) -> Result<Reach, CombatRefusal> {
        let not_in = |who: &str| CombatRefusal::NotInFight { who: who.into() };
        let from = self.position(actor).ok_or_else(|| not_in(actor))?;
        let to = self.position(target).ok_or_else(|| not_in(target))?;
        reach::reach(
            &self.map,
            self.diagonal(system),
            action,
            from,
            to,
            &self.bodies_except(&[actor, target]),
        )
        .map_err(|why| CombatRefusal::OutOfReach {
            target: target.into(),
            why,
        })
    }

    fn open_turn(&self, who: &str) -> Result<(), CombatRefusal> {
        if self.is_over() {
            return Err(CombatRefusal::FightOver);
        }
        if !self.in_fight(who) {
            return Err(CombatRefusal::NotInFight { who: who.into() });
        }
        if self.active() != Some(who) {
            return Err(CombatRefusal::NotTheirTurn);
        }
        Ok(())
    }

    /// Moves the active combatant along `path` (the cells entered, in
    /// order), checked step by step by the grid's movement rules.
    pub fn move_along(
        &self,
        system: &RuleSystem,
        who: &str,
        path: &[Cell],
    ) -> Result<Step, CombatRefusal> {
        self.open_turn(who)?;
        let me = self.scene.get(who).expect("in fight");
        if let Some(c) = me.incapacitated_by() {
            return Err(Refusal::Incapacitated {
                because: c.name.clone(),
            }
            .into());
        }
        let budget = self.move_budget(system, who);
        if budget == 0 {
            return Err(CombatRefusal::CannotMove);
        }
        let kind = match &system.combat.move_kind {
            Some(k) => Some(affordable(system, &self.scene, me, k)?),
            None if self.moved => return Err(CombatRefusal::AlreadyMoved),
            None => None,
        };
        let start = self.position(who).expect("in fight");
        let cost = check_path(
            &self.map,
            &movement_rules(system, &self.map),
            &self.occupancy(who),
            start,
            path,
            budget,
        )?;
        let mut next = self.clone();
        match kind {
            Some(k) => spend(next.scene.get_mut(who).expect("in fight"), k),
            None => next.moved = true,
        }
        if let Some(&last) = path.last() {
            next.positions.insert(who.into(), last);
        }
        Ok(Step {
            fight: next,
            events: vec![FightEvent::Moved {
                who: who.into(),
                path: path.to_vec(),
                cost,
                budget,
            }],
        })
    }

    /// The targets a play reaches, and what each one's position adds to
    /// an attack roll — every grid rule checked, nothing resolved.
    pub fn aim(&self, system: &RuleSystem, play: &Play) -> Result<Aimed, CombatRefusal> {
        self.open_turn(&play.actor)?;
        let me = self.scene.get(&play.actor).expect("in fight");
        let action = find_action(system, me, &play.action)?;
        let from = self.position(&play.actor).expect("in fight");
        let diagonal = self.diagonal(system);
        let mut reaches: BTreeMap<String, Reach> = BTreeMap::new();
        let targets: Vec<String> = match (action.target, action.area, &play.aim) {
            (Targeting::Myself | Targeting::AllAllies, _, Aim::Nothing) => Vec::new(),
            (Targeting::Myself, _, Aim::Targets(t))
                if t.as_slice() == std::slice::from_ref(&play.actor) =>
            {
                Vec::new()
            }
            (Targeting::Ally | Targeting::AllyOrSelf | Targeting::Enemy, _, Aim::Targets(ts)) => {
                for t in ts {
                    if !self.in_fight(t) {
                        return Err(CombatRefusal::NotInFight { who: t.clone() });
                    }
                    if t != &play.actor {
                        reaches.insert(t.clone(), self.reach(system, &play.actor, &action, t)?);
                    }
                }
                ts.clone()
            }
            (Targeting::Enemies, Some(shape), aim) => {
                let origin = match (shape, aim) {
                    (AreaShape::MeleeBurst, Aim::Nothing) => from,
                    (AreaShape::Line, Aim::Cell(c)) if *c != from => from,
                    (AreaShape::Zone, Aim::Cell(c)) => {
                        let bodies = self.bodies_except(&[&play.actor]);
                        let to_centre = Reach {
                            distance: self.map.distance(from, *c, diagonal),
                            long_range: false,
                            cover: crate::maps::Cover::None,
                        };
                        if to_centre.distance > action.reach() {
                            return Err(CombatRefusal::AimOutOfReach {
                                why: ReachRefusal::OutOfRange {
                                    distance: to_centre.distance,
                                    range: action.reach(),
                                },
                            });
                        }
                        reach::sight(&self.map, diagonal, from, *c, &bodies).ok_or(
                            CombatRefusal::AimOutOfReach {
                                why: ReachRefusal::NoLineOfSight,
                            },
                        )?;
                        *c
                    }
                    _ => return Err(CombatRefusal::WrongAim),
                };
                let mut caught = Vec::new();
                for enemy in self.side(other(me.side)) {
                    if !self.standing_up(enemy) {
                        continue;
                    }
                    let at = self.position(enemy).expect("in fight");
                    let inside = match (shape, aim) {
                        (AreaShape::MeleeBurst, _) => self.map.distance(from, at, diagonal) <= 1,
                        (AreaShape::Line, Aim::Cell(c)) => {
                            reach::on_line(from, *c, at)
                                && self.map.distance(from, at, diagonal) <= action.reach()
                        }
                        (AreaShape::Zone, _) => {
                            self.map.distance(origin, at, Diagonal::Chebyshev)
                                <= system.combat.zone_radius
                        }
                        _ => false,
                    };
                    if !inside {
                        continue;
                    }
                    let bodies = self.bodies_except(&[&play.actor, enemy]);
                    let Some(cover) = (if origin == at {
                        Some(crate::maps::Cover::None)
                    } else {
                        reach::sight(&self.map, diagonal, origin, at, &bodies)
                    }) else {
                        continue;
                    };
                    reaches.insert(
                        enemy.to_string(),
                        Reach {
                            distance: self.map.distance(origin, at, diagonal),
                            long_range: false,
                            cover,
                        },
                    );
                    caught.push(enemy.to_string());
                }
                if caught.is_empty() {
                    return Err(CombatRefusal::AreaEmpty);
                }
                caught
            }
            _ => return Err(CombatRefusal::WrongAim),
        };
        let mut positional = BTreeMap::new();
        if action.roll == RollSpec::Attack {
            for (id, r) in &reaches {
                let (modifiers, disadvantage) = reach::roll_effects(system, r);
                if !modifiers.is_empty() || disadvantage {
                    positional.insert(
                        id.clone(),
                        Positional {
                            modifiers,
                            disadvantage,
                        },
                    );
                }
            }
        }
        Ok(Aimed {
            action,
            targets,
            positional,
        })
    }

    /// The scene as the rules see it: only those still in the fight.
    fn fighting_scene(&self) -> Scene {
        let mut s = self.scene.clone();
        s.combatants.retain(|id, _| self.in_fight(id));
        s
    }

    /// Checks a play against every grid and rule gate without resolving
    /// it; returns the targets it would hit.
    pub fn check(&self, system: &RuleSystem, play: &Play) -> Result<Vec<String>, CombatRefusal> {
        let aimed = self.aim(system, play)?;
        let req = request(play, &aimed.targets, aimed.positional);
        let (_, resolved, _) = action::check_playable(system, &self.fighting_scene(), &req)?;
        Ok(resolved)
    }

    /// Plays an action: the grid picks and checks the targets, the rules
    /// resolve it, then whoever falls leaves the fight.
    pub fn act(
        &self,
        system: &RuleSystem,
        play: &Play,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, CombatRefusal> {
        let aimed = self.aim(system, play)?;
        let req = request(play, &aimed.targets, aimed.positional);
        let resolution = resolve_action(system, &self.fighting_scene(), &req, dice)?;
        let mut next = self.clone();
        let mut events = vec![FightEvent::Acted {
            who: play.actor.clone(),
            action: aimed.action.id,
            targets: aimed.targets,
        }];
        events.extend(rules_events(resolution.events));
        for (id, c) in resolution.scene.combatants {
            next.scene.combatants.insert(id, c);
        }
        next.settle(system, &mut events);
        next.check_end(&mut events);
        Ok(Step {
            fight: next,
            events,
        })
    }

    /// The active combatant tries to leave the fight. The system's flight
    /// action is spent; when the GM says the flight is hard
    /// (`difficulty`), the system's flight ability is rolled against it.
    pub fn flee(
        &self,
        system: &RuleSystem,
        who: &str,
        difficulty: Option<i32>,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, CombatRefusal> {
        self.open_turn(who)?;
        let me = self.scene.get(who).expect("in fight");
        if let Some(c) = me.incapacitated_by() {
            return Err(Refusal::Incapacitated {
                because: c.name.clone(),
            }
            .into());
        }
        let rule = system.combat.flee.as_ref();
        let kind = match rule {
            Some(f) => Some(affordable(system, &self.scene, me, &f.kind)?),
            None if me.turn.actions_left == 0 => {
                return Err(Refusal::NotEnoughActions { cost: 1, left: 0 }.into());
            }
            None => None,
        };
        let ability = rule.and_then(|f| f.ability.as_deref());
        if difficulty.is_some() && ability.is_none() {
            return Err(CombatRefusal::NoFleeRoll);
        }
        let mut next = self.clone();
        let mut events = Vec::new();
        {
            let me = next.scene.get_mut(who).expect("in fight");
            match kind {
                Some(k) => spend(me, k),
                None => me.turn.actions_left = 0,
            }
        }
        if let (Some(ability), Some(value)) = (ability, difficulty) {
            let roll = check::ability_check(
                system,
                next.scene.get(who).expect("in fight"),
                ability,
                RollScope::Checks,
                Some(RollTarget::Difficulty { id: None, value }),
                Advantage::Normal,
                dice,
            )
            .map_err(|e| Refusal::Engine {
                detail: format!("{e:?}"),
            })?;
            let band = roll.band;
            events.push(FightEvent::FleeRoll {
                who: who.into(),
                roll,
            });
            let me = next.scene.get_mut(who).expect("in fight");
            for change in award_band(system, me, band) {
                events.push(FightEvent::Rules {
                    event: Event::Progress {
                        who: who.into(),
                        change,
                    },
                });
            }
            if !band.is_some_and(OutcomeBand::is_success) {
                events.push(FightEvent::FleeFailed { who: who.into() });
                return Ok(Step {
                    fight: next,
                    events,
                });
            }
        }
        next.leave(who, Standing::Fled);
        events.push(FightEvent::Fled { who: who.into() });
        next.scene.active = None;
        next.check_end(&mut events);
        let current = next.turn;
        next.advance(system, Some(current), dice, &mut events)
            .map_err(|e| Refusal::Engine {
                detail: format!("{e:?}"),
            })?;
        Ok(Step {
            fight: next,
            events,
        })
    }

    /// Ends the active combatant's turn (damage over time, durations),
    /// then begins the next one in the order — skipping those out of the
    /// fight and passing the turns of those who cannot act.
    pub fn end_turn(
        &self,
        system: &RuleSystem,
        who: &str,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, CombatRefusal> {
        self.open_turn(who)?;
        if self.awaits_death_save(who) {
            return Err(CombatRefusal::DeathSaveFirst);
        }
        let mut next = self.clone();
        let mut events = Vec::new();
        next.close_turn(system, who, dice, &mut events)
            .map_err(|e| Refusal::Engine {
                detail: format!("{e:?}"),
            })?;
        let current = next.turn;
        next.advance(system, Some(current), dice, &mut events)
            .map_err(|e| Refusal::Engine {
                detail: format!("{e:?}"),
            })?;
        Ok(Step {
            fight: next,
            events,
        })
    }

    /// The GM ends the fight, whatever its state (rule 1: the GM has the
    /// last word).
    pub fn stop(&self) -> Step {
        let mut next = self.clone();
        let mut events = Vec::new();
        if !next.is_over() {
            next.finish(None, EndReason::StoppedByGm, &mut events);
        }
        Step {
            fight: next,
            events,
        }
    }

    /// The dying active combatant rolls against death; the save is their
    /// whole turn, which ends — unless a critical success brings them
    /// back, and then the turn is theirs to play.
    pub fn death_save(
        &self,
        system: &RuleSystem,
        who: &str,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, CombatRefusal> {
        self.open_turn(who)?;
        if !self.awaits_death_save(who) {
            return Err(CombatRefusal::NotDying { who: who.into() });
        }
        let (scene, ev) = death::death_save(system, &self.scene, who, dice)?;
        let mut next = self.clone();
        next.scene = scene;
        let mut events: Vec<FightEvent> = rules_events(ev).collect();
        let back_up = next.scene.get(who).is_some_and(|c| c.hit_points > 0);
        if back_up {
            let per_turn = system
                .turn_context(&next.scene.context)
                .map(|c| c.actions_per_turn);
            if let (Some(n), Some(me)) = (per_turn, next.scene.get_mut(who)) {
                me.turn.spent_by_kind.clear();
                me.turn.actions_left = n;
            }
            next.check_end(&mut events);
            return Ok(Step {
                fight: next,
                events,
            });
        }
        next.close_turn(system, who, dice, &mut events)
            .map_err(|e| Refusal::Engine {
                detail: format!("{e:?}"),
            })?;
        let current = next.turn;
        next.advance(system, Some(current), dice, &mut events)
            .map_err(|e| Refusal::Engine {
                detail: format!("{e:?}"),
            })?;
        Ok(Step {
            fight: next,
            events,
        })
    }

    /// The active combatant tries to stabilise `target`, dying next to
    /// them: the system's stabilise action is spent and its ability
    /// rolled.
    pub fn stabilize(
        &self,
        system: &RuleSystem,
        who: &str,
        target: &str,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, CombatRefusal> {
        self.open_turn(who)?;
        let rule = death::stabilize_rule(system).ok_or(if death::has_death_saves(system) {
            CombatRefusal::NoStabilizeRule
        } else {
            CombatRefusal::NoDeathSaves
        })?;
        let me = self.scene.get(who).expect("in fight");
        if let Some(c) = me.incapacitated_by() {
            return Err(Refusal::Incapacitated {
                because: c.name.clone(),
            }
            .into());
        }
        if !self.awaits_death_save(target) {
            return Err(CombatRefusal::NotDying { who: target.into() });
        }
        let near = match (self.position(who), self.position(target)) {
            (Some(a), Some(b)) => a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1,
            _ => false,
        };
        if !near {
            return Err(CombatRefusal::TooFar {
                target: target.into(),
            });
        }
        let kind = affordable(system, &self.scene, me, &rule.kind)?;
        let mut next = self.clone();
        spend(next.scene.get_mut(who).expect("in fight"), kind);
        let (scene, ev) = death::stabilize(system, &next.scene, who, target, dice)?;
        next.scene = scene;
        let events = rules_events(ev).collect();
        Ok(Step {
            fight: next,
            events,
        })
    }

    /// The GM confirms the death of `who`, a character down at 0 — the
    /// one the dice proposed, or any other (rule 1). They leave the fight
    /// for good; if it was their turn, the next one begins.
    pub fn confirm_death(
        &self,
        system: &RuleSystem,
        who: &str,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, CombatRefusal> {
        let c = self
            .scene
            .get(who)
            .ok_or_else(|| CombatRefusal::NotInFight { who: who.into() })?;
        let standing = self.standing.get(who).copied();
        if c.progress.is_none()
            || c.hit_points > 0
            || matches!(standing, Some(Standing::Dead | Standing::Fled) | None)
        {
            return Err(CombatRefusal::NotDown { who: who.into() });
        }
        let mut next = self.clone();
        let mut events = vec![FightEvent::Died { who: who.into() }];
        next.leave(who, Standing::Dead);
        if let Some(end) = next.end.as_mut() {
            end.dead.push(who.into());
            end.out_of_scene.retain(|o| o != who);
            return Ok(Step {
                fight: next,
                events,
            });
        }
        next.check_end(&mut events);
        if next.active() == Some(who) {
            next.scene.active = None;
            let current = next.turn;
            next.advance(system, Some(current), dice, &mut events)
                .map_err(|e| Refusal::Engine {
                    detail: format!("{e:?}"),
                })?;
        }
        Ok(Step {
            fight: next,
            events,
        })
    }

    /// The GM decides another outcome than the death the dice proposed:
    /// `who` is stable, still down. If it was their turn, it ends.
    pub fn spare(
        &self,
        system: &RuleSystem,
        who: &str,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, CombatRefusal> {
        let (scene, ev) = death::spare(&self.scene, who)?;
        let mut next = self.clone();
        next.scene = scene;
        let mut events: Vec<FightEvent> = rules_events(ev).collect();
        if next.active() == Some(who) && !next.is_over() {
            next.close_turn(system, who, dice, &mut events)
                .map_err(|e| Refusal::Engine {
                    detail: format!("{e:?}"),
                })?;
            let current = next.turn;
            next.advance(system, Some(current), dice, &mut events)
                .map_err(|e| Refusal::Engine {
                    detail: format!("{e:?}"),
                })?;
        }
        Ok(Step {
            fight: next,
            events,
        })
    }

    fn close_turn(
        &mut self,
        system: &RuleSystem,
        who: &str,
        dice: &mut dyn DiceSource,
        events: &mut Vec<FightEvent>,
    ) -> Result<(), TurnError> {
        let (scene, ev) = conditions::end_turn(system, &self.scene, who, dice)?;
        self.scene = scene;
        events.extend(rules_events(ev));
        events.push(FightEvent::TurnEnded { who: who.into() });
        self.settle(system, events);
        self.check_end(events);
        Ok(())
    }

    /// Begins the next turn after `from` (or the first one), wrapping into
    /// a new round, skipping whoever left the fight and closing at once
    /// the turns of those a condition keeps from acting.
    fn advance(
        &mut self,
        system: &RuleSystem,
        from: Option<usize>,
        dice: &mut dyn DiceSource,
        events: &mut Vec<FightEvent>,
    ) -> Result<(), TurnError> {
        let n = self.order.len();
        let mut i = from.map_or(0, |f| f + 1);
        let mut passed = 0;
        loop {
            if self.is_over() {
                return Ok(());
            }
            if i >= n {
                i = 0;
                self.round += 1;
                events.push(FightEvent::RoundStarted { round: self.round });
            }
            let id = self.order[i].clone();
            if !self.in_fight(&id) {
                i += 1;
                continue;
            }
            let (scene, ev) = conditions::start_turn(system, &self.scene, &id)?;
            self.scene = scene;
            self.turn = i;
            self.moved = false;
            events.push(FightEvent::TurnStarted {
                who: id.clone(),
                round: self.round,
            });
            events.extend(rules_events(ev));
            let blocked = self
                .scene
                .get(&id)
                .is_some_and(|c| c.incapacitated_by().is_some());
            if !blocked || self.awaits_death_save(&id) {
                return Ok(());
            }
            self.close_turn(system, &id, dice, events)?;
            passed += 1;
            if passed > n * MAX_SKIPPED_TURNS_PER_COMBATANT {
                self.finish(None, EndReason::Stalemate, events);
                return Ok(());
            }
            i += 1;
        }
    }

    fn leave(&mut self, who: &str, standing: Standing) {
        self.standing.insert(who.into(), standing);
        self.positions.remove(who);
    }

    /// Takes out of the fight the adversaries at 0 hit points and the
    /// characters the zero-HP rule put out of the scene.
    fn settle(&mut self, system: &RuleSystem, events: &mut Vec<FightEvent>) {
        let out_condition = match &system.zero_hp {
            ZeroHpRule::KnockedOut { out_condition, .. } => Some(out_condition.as_str()),
            ZeroHpRule::DeathSaves { .. } => None,
        };
        let mut leaving = Vec::new();
        for c in self.scene.combatants.values() {
            if !self.in_fight(&c.id) {
                continue;
            }
            if c.progress.is_none() && c.hit_points == 0 {
                leaving.push((c.id.clone(), Standing::Defeated));
            } else if out_condition.is_some_and(|o| c.condition_named(o).is_some()) {
                leaving.push((c.id.clone(), Standing::OutOfScene));
            }
        }
        for (id, standing) in leaving {
            self.leave(&id, standing);
            events.push(match standing {
                Standing::Defeated => FightEvent::Defeated { who: id },
                _ => FightEvent::LeftTheScene { who: id },
            });
        }
    }

    fn check_end(&mut self, events: &mut Vec<FightEvent>) {
        if self.is_over() {
            return;
        }
        let up = |side| self.side(side).any(|id| self.standing_up(id));
        let (party, opposition) = (up(Side::Party), up(Side::Opposition));
        let winner = match (party, opposition) {
            (true, true) => return,
            (true, false) => Some(Side::Party),
            (false, true) => Some(Side::Opposition),
            (false, false) => None,
        };
        self.finish(winner, EndReason::SideDown, events);
    }

    fn finish(&mut self, winner: Option<Side>, reason: EndReason, events: &mut Vec<FightEvent>) {
        let with = |s: Standing| -> Vec<String> {
            self.standing
                .iter()
                .filter(|(_, st)| **st == s)
                .map(|(id, _)| id.clone())
                .collect()
        };
        let xp = self
            .xp_at_start
            .iter()
            .filter_map(|(id, start)| {
                let now = self.scene.get(id)?.progress?.total_xp;
                Some((id.clone(), now - start))
            })
            .collect();
        let end = FightEnd {
            winner,
            reason,
            rounds: self.round,
            defeated: with(Standing::Defeated),
            fled: with(Standing::Fled),
            out_of_scene: with(Standing::OutOfScene),
            dead: with(Standing::Dead),
            xp,
        };
        self.scene.active = None;
        events.push(FightEvent::Ended { end: end.clone() });
        self.end = Some(end);
    }
}

fn other(side: Side) -> Side {
    match side {
        Side::Party => Side::Opposition,
        Side::Opposition => Side::Party,
    }
}

fn request(
    play: &Play,
    targets: &[String],
    positional: BTreeMap<String, Positional>,
) -> ActionRequest {
    ActionRequest {
        actor: play.actor.clone(),
        action: play.action.clone(),
        targets: targets.to_vec(),
        situations: play.situations.clone(),
        save_difficulty: play.save_difficulty,
        choice: play.choice,
        positional,
    }
}

/// Initiative: the system's die plus its bonus formula for everyone,
/// highest first. Ties per the system: the party first (then the order
/// placements were given in), or tied combatants reroll among themselves
/// until they differ.
pub fn roll_initiative(
    system: &RuleSystem,
    placements: &[(Combatant, Cell)],
    dice: &mut dyn DiceSource,
) -> Result<Vec<InitiativeRoll>, SetupError> {
    use crate::rules::model::InitiativeTies;

    let mut rolls = Vec::new();
    for (c, _) in placements {
        let bonus = c.initiative_bonus(system).map_err(SetupError::Sheet)?;
        let r = system.initiative.dice.roll(dice);
        rolls.push((
            InitiativeRoll {
                who: c.id.clone(),
                faces: r.faces,
                bonus,
                total: r.total + bonus,
                rerolls: Vec::new(),
            },
            c.side,
        ));
    }
    if system.initiative.ties == InitiativeTies::Reroll {
        // Reroll every group still tied, a bounded number of times; what
        // stays tied after that keeps the placement order.
        for _ in 0..10 {
            let mut tied: BTreeMap<Vec<i32>, Vec<usize>> = BTreeMap::new();
            for (i, (r, _)) in rolls.iter().enumerate() {
                let mut key = vec![r.total];
                key.extend(&r.rerolls);
                tied.entry(key).or_default().push(i);
            }
            let groups: Vec<Vec<usize>> = tied.into_values().filter(|g| g.len() > 1).collect();
            if groups.is_empty() {
                break;
            }
            for g in groups {
                for i in g {
                    let r = system.initiative.dice.roll(dice).total;
                    rolls[i].0.rerolls.push(r);
                }
            }
        }
    }
    let side_rank = |s: Side| match (system.initiative.ties, s) {
        (InitiativeTies::PartyFirst, Side::Party) => 0,
        (InitiativeTies::PartyFirst, Side::Opposition) => 1,
        (InitiativeTies::Reroll, _) => 0,
    };
    let mut indexed: Vec<(usize, (InitiativeRoll, Side))> = rolls.into_iter().enumerate().collect();
    indexed.sort_by(|(ia, (a, sa)), (ib, (b, sb))| {
        b.total
            .cmp(&a.total)
            .then_with(|| b.rerolls.cmp(&a.rerolls))
            .then_with(|| side_rank(*sa).cmp(&side_rank(*sb)))
            .then_with(|| ia.cmp(ib))
    });
    Ok(indexed.into_iter().map(|(_, (r, _))| r).collect())
}
