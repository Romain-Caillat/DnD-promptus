//! House rules the server judges (`engine/formalise-house-rules`).
//!
//! A GM writes a house rule in French (`model::HouseRule::text`); the
//! co-GM proposes a formal form of it, the GM validates it, and from
//! then on the engine applies it like any other rule. The formal form is
//! built on what the engine already sees and already does:
//!
//! - **when**: an attack that lands (`hit`, optionally only a critical,
//!   or only an action dealing one damage type) or one that misses
//!   (`miss`, optionally only a natural fumble);
//! - **who**: filters on the attacker and the target — side, traits
//!   they must carry, traits and class / stat block ids excepted;
//! - **effects**: a condition put on the target or the attacker (the
//!   `ApplySpec` of the action tags), damage, healing;
//! - **players**: whether players may read the rule itself, or only see
//!   its effect;
//! - **cases**: worked examples the server replays (`run_cases`), so the
//!   GM reads « Torche contre zombie : effrayé 1 tour » and not the data.
//!
//! One pass only: the effects of a house rule never trigger a house rule
//! (`action::resolve_action` calls [`triggered`] once per target, before
//! applying them).

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::action::{ActionRef, ActionRequest, resolve_action};
use super::check::OutcomeBand;
use super::dice::{DiceExpr, DiceSource};
use super::events::{Event, RollPurpose};
use super::model::*;
use super::sheet::{Combatant, Scene, Side, TurnBudget};

/// A house rule as the engine judges it.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FormalRule {
    pub when: Trigger,
    /// `true`: only a critical hit (`hit`) or a natural fumble (`miss`);
    /// `false`: never then; absent: either way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critical: Option<bool>,
    /// `hit` only: the action deals damage of this type (a
    /// `damage_types` id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_type: Option<String>,
    /// Who attacks.
    #[serde(default, skip_serializing_if = "Who::is_anyone")]
    pub actor: Who,
    /// Who is attacked.
    #[serde(default, skip_serializing_if = "Who::is_anyone")]
    pub target: Who,
    pub effects: Vec<HouseEffect>,
    #[serde(default)]
    pub players: PlayersSee,
    /// Worked examples the server replays on every save.
    #[serde(default)]
    pub cases: Vec<RuleCase>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// An attack (or any action that rolls against a foe) lands.
    Hit,
    /// It misses.
    Miss,
}

/// A filter on one side of the attack. Empty = anyone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Who {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
    /// Every one of these traits.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<String>,
    /// None of these traits (the rule's exceptions).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub except_traits: Vec<String>,
    /// Not these classes or stat blocks (by id).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub except: Vec<String>,
}

impl Who {
    pub fn is_anyone(&self) -> bool {
        *self == Self::default()
    }

    /// Whether `c` passes this filter.
    pub fn matches(&self, system: &RuleSystem, c: &Combatant) -> bool {
        let traits = c.traits(system);
        self.side.is_none_or(|s| s == c.side)
            && self.traits.iter().all(|t| traits.contains(t))
            && !self.except_traits.iter().any(|t| traits.contains(t))
            && !self.except.iter().any(|id| id == c.origin_id())
    }
}

/// What a house rule does once triggered.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HouseEffect {
    /// A condition, on the attack's target (`to: targets`) or on the
    /// attacker (`to: self`).
    Apply(ApplySpec),
    Damage(Amount),
    Heal(Amount),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Amount {
    pub amount: DiceExpr,
    pub to: Recipient,
}

/// What players may read of a formalised rule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayersSee {
    /// The rule (name and text) on their rules page, and its name in
    /// the fight log.
    #[default]
    Rule,
    /// Only what it does (the condition on the monster), never the rule.
    Effect,
}

/// A worked example: this attacker plays this action on this target,
/// with this kind of roll; the rule applies, or it does not.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleCase {
    pub name: String,
    pub actor: CaseFighter,
    /// An action the attacker has, or an item of the system.
    pub action: String,
    pub target: CaseFighter,
    #[serde(default)]
    pub roll: CaseRoll,
    pub expect: Expect,
}

/// A combatant of a case: a level-1 character of a class (raised to the
/// action's level if it needs one), or an adversary's stat block.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CaseFighter {
    Class(String),
    Adversary(String),
}

impl CaseFighter {
    pub fn id(&self) -> &str {
        match self {
            Self::Class(id) | Self::Adversary(id) => id,
        }
    }

    fn build(&self, system: &RuleSystem, sheet_id: &str) -> Result<Combatant, String> {
        let built = match self {
            Self::Class(id) => Combatant::from_class(system, sheet_id, id, id),
            Self::Adversary(id) => Combatant::from_adversary(system, sheet_id, id, id),
        };
        built.map_err(|e| format!("{e:?}"))
    }
}

/// The roll the attacker makes, read as its band.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseRoll {
    #[default]
    Hit,
    Critical,
    Miss,
    Fumble,
}

impl CaseRoll {
    fn band(self) -> OutcomeBand {
        match self {
            Self::Hit => OutcomeBand::Success,
            Self::Critical => OutcomeBand::CriticalSuccess,
            Self::Miss => OutcomeBand::Failure,
            Self::Fumble => OutcomeBand::CriticalFailure,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    /// The rule applies.
    Applies,
    /// It does not.
    Nothing,
}

/// What an attack was, for the triggers to read.
#[derive(Debug, Clone, Copy)]
pub struct Moment<'a> {
    pub trigger: Trigger,
    /// A critical hit (`Hit`) or a natural fumble (`Miss`).
    pub critical: bool,
    /// The damage types the action deals.
    pub damage_types: &'a [&'a str],
}

/// The house rules this moment triggers, in the system's order.
pub fn triggered<'s>(
    system: &'s RuleSystem,
    moment: &Moment<'_>,
    actor: &Combatant,
    target: &Combatant,
) -> Vec<(&'s HouseRule, &'s FormalRule)> {
    system
        .house_rules
        .iter()
        .filter_map(|h| h.formal.as_ref().map(|f| (h, f)))
        .filter(|(_, f)| {
            f.when == moment.trigger
                && f.critical.is_none_or(|c| c == moment.critical)
                && f.damage_type
                    .as_deref()
                    .is_none_or(|t| moment.damage_types.contains(&t))
                && f.actor.matches(system, actor)
                && f.target.matches(system, target)
        })
        .collect()
}

/// One case replayed: what the attack did, and whether the rule fired.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseResult {
    pub name: String,
    pub expect: Expect,
    /// The natural face the attacker rolled (none for an action that
    /// does not roll).
    pub natural: Option<u32>,
    /// Whether the rule fired.
    pub fired: bool,
    pub passed: bool,
    /// What the rule did, as engine events (conditions, damage…).
    pub effects: Vec<Event>,
    /// Why the case could not be played (an action that does not reach
    /// an enemy, a roll the die cannot give…).
    pub error: Option<String>,
}

/// The check die's rolls follow a script; every other die, and every
/// check die once the script is spent, shows 1 (so a save the rule
/// allows fails: the case shows the rule's full effect).
struct CaseDice {
    check_faces: u32,
    script: VecDeque<u32>,
}

impl DiceSource for CaseDice {
    fn roll(&mut self, faces: u32) -> u32 {
        if faces == self.check_faces
            && let Some(f) = self.script.pop_front()
        {
            return f;
        }
        1
    }
}

/// Faces to try first for a roll of this kind.
fn preferred_faces(system: &RuleSystem, roll: CaseRoll) -> Vec<u32> {
    let n = system.check.dice.faces;
    let o = &system.outcomes;
    let plain = |f: &u32| {
        !o.critical_success.natural.contains(f) && !o.critical_failure.natural.contains(f)
    };
    match roll {
        CaseRoll::Hit => (1..=n).rev().filter(plain).collect(),
        CaseRoll::Miss => (1..=n).filter(plain).collect(),
        CaseRoll::Critical => o.critical_success.natural.clone(),
        CaseRoll::Fumble => o.critical_failure.natural.clone(),
    }
}

/// Plays every case of house rule `rule` of `system` (a rule the system
/// holds, formalised).
pub fn run_cases(system: &RuleSystem, rule: &HouseRule) -> Vec<CaseResult> {
    let Some(formal) = &rule.formal else {
        return Vec::new();
    };
    formal
        .cases
        .iter()
        .map(|case| run_case(system, &rule.id, case))
        .collect()
}

fn run_case(system: &RuleSystem, rule: &str, case: &RuleCase) -> CaseResult {
    let mut result = CaseResult {
        name: case.name.clone(),
        expect: case.expect,
        natural: None,
        fired: false,
        passed: false,
        effects: Vec::new(),
        error: None,
    };
    match play_case(system, case) {
        Ok((natural, events)) => {
            let at = events
                .iter()
                .position(|e| matches!(e, Event::HouseRule { rule: r, .. } if r == rule));
            result.natural = natural;
            result.fired = at.is_some();
            if let Some(at) = at {
                result.effects = events[at + 1..]
                    .iter()
                    .take_while(|e| !matches!(e, Event::HouseRule { .. } | Event::Missed { .. }))
                    .filter(|e| {
                        matches!(
                            e,
                            Event::ConditionApplied { .. }
                                | Event::ConditionResisted { .. }
                                | Event::Damaged { .. }
                                | Event::Healed { .. }
                                | Event::KnockedOut { .. }
                        )
                    })
                    .cloned()
                    .collect();
            }
            result.passed = result.fired == (case.expect == Expect::Applies);
        }
        Err(e) => result.error = Some(e),
    }
    result
}

/// The scene of a case: the attacker on its own side, the target facing
/// it, the attacker's turn open with a full budget.
fn stage(system: &RuleSystem, case: &RuleCase) -> Result<(Scene, ActionRef), String> {
    let mut actor = case.actor.build(system, "actor")?;
    let mut target = case.target.build(system, "target")?;
    target.side = match actor.side {
        Side::Party => Side::Opposition,
        Side::Opposition => Side::Party,
    };
    let owned = actor
        .actions(system)
        .into_iter()
        .find(|a| a.id == case.action)
        .cloned();
    let (action, reference) = match owned {
        Some(a) => (a, ActionRef::Own(case.action.clone())),
        None => {
            let item = system
                .item(&case.action)
                .ok_or_else(|| format!("no action or item `{}`", case.action))?;
            let a = item
                .action
                .clone()
                .ok_or_else(|| format!("item `{}` has no use", case.action))?;
            actor.inventory.insert(item.id.clone(), 1);
            (a, ActionRef::Item(item.id.clone()))
        }
    };
    if !matches!(action.target, Targeting::Enemy | Targeting::Enemies) {
        return Err(format!("`{}` does not target an enemy", action.id));
    }
    if let (Some(level), Some(progress)) = (action.level, actor.progress.as_mut())
        && let Some(t) = system.progression.levels.iter().find(|l| l.level == level)
    {
        progress.total_xp = t.xp;
    }
    let context = system
        .turn_contexts
        .first()
        .ok_or("the system has no turn context")?;
    actor.turn = TurnBudget {
        actions_left: context.actions_per_turn,
        ..TurnBudget::default()
    };
    let mut scene = Scene::new(&context.id, [actor, target]);
    scene.active = Some("actor".into());
    Ok((scene, reference))
}

fn play_case(system: &RuleSystem, case: &RuleCase) -> Result<(Option<u32>, Vec<Event>), String> {
    let (scene, action) = stage(system, case)?;
    let req = ActionRequest {
        action,
        // An open save is the GM's to set: the middle difficulty here.
        save_difficulty: system
            .difficulties
            .get(system.difficulties.len() / 2)
            .map(|d| d.value),
        ..ActionRequest::new("actor", "", &["target"])
    };
    let faces = system.check.dice.faces;
    let firsts = preferred_faces(system, case.roll);
    let mut last_refusal = None;
    for first in firsts {
        for second in 1..=faces {
            let mut dice = CaseDice {
                check_faces: faces,
                script: VecDeque::from([first, second]),
            };
            let resolution = match resolve_action(system, &scene, &req, &mut dice) {
                Ok(r) => r,
                Err(refusal) => {
                    last_refusal = Some(format!("{refusal:?}"));
                    break;
                }
            };
            let roll = resolution.events.iter().find_map(|e| match e {
                Event::Roll {
                    roller,
                    purpose: RollPurpose::Attack | RollPurpose::Contest,
                    breakdown,
                    ..
                } if roller == "actor" => Some(breakdown),
                _ => None,
            });
            match roll {
                // An action that does not roll lands every time.
                None => {
                    let lands = matches!(case.roll, CaseRoll::Hit | CaseRoll::Critical);
                    return if lands {
                        Ok((None, resolution.events))
                    } else {
                        Err("this action lands without a roll".into())
                    };
                }
                Some(b) if b.band == Some(case.roll.band()) => {
                    return Ok((Some(b.natural), resolution.events));
                }
                Some(_) => {}
            }
        }
    }
    Err(last_refusal.unwrap_or_else(|| format!("no face gives a {:?}", case.roll)))
}
