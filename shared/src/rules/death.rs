//! engine/save-against-death — what 0 hit points does under the
//! `death_saves` rule: the character is dying, rolls the check die
//! against the rule's difficulty at each of their turns, and comes out
//! stable, back on their feet, or with their death **proposed**. The
//! engine never kills: a death waits for the GM, who confirms it or
//! decides otherwise (`MEMORY.md` §1, "the GM has the last word").
//!
//! Under `knocked_out` (both witness worlds today) none of this runs: the
//! character is knocked out, then out of the scene (`conditions`), and a
//! death is only ever the GM's own decision.
//!
//! INTERPRETATION, to confirm by Romain:
//! - a death save adds no modifier and grants no XP (it is not a check of
//!   the character's skill);
//! - only player characters die slowly; an adversary at 0 is defeated;
//! - a stable character hit again starts rolling again.

use super::check::{self, Advantage, OutcomeBand, RollTarget};
use super::dice::DiceSource;
use super::events::{Event, RollPurpose};
use super::model::{RollScope, RuleSystem, StabilizeRule, ZeroHpRule};
use super::progression::award_band;
use super::sheet::{Combatant, DeathSaves, Scene};

/// Why a save or a stabilisation is refused. Nothing changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeathRefusal {
    /// The rule system has no death saves (`knocked_out`).
    NoDeathSaves,
    /// The system says nothing about stabilising someone.
    NoStabilizeRule,
    UnknownCombatant(String),
    /// Not dying, or stable, or already waiting for the GM.
    NotRolling(String),
    Engine(String),
}

/// The thresholds of the `death_saves` rule, when the system has it.
struct Rule<'a> {
    difficulty: i32,
    successes: u32,
    failures: u32,
    on_hit: u32,
    on_critical_hit: u32,
    on_critical_failure: u32,
    critical_success_revives: bool,
    stabilize: Option<&'a StabilizeRule>,
}

fn rule(system: &RuleSystem) -> Option<Rule<'_>> {
    match &system.zero_hp {
        ZeroHpRule::DeathSaves {
            difficulty,
            successes,
            failures,
            failures_on_hit,
            failures_on_critical_hit,
            failures_on_critical_failure,
            critical_success_revives,
            stabilize,
            ..
        } => Some(Rule {
            difficulty: *difficulty,
            successes: *successes,
            failures: *failures,
            on_hit: *failures_on_hit,
            on_critical_hit: *failures_on_critical_hit,
            on_critical_failure: *failures_on_critical_failure,
            critical_success_revives: *critical_success_revives,
            stabilize: stabilize.as_ref(),
        }),
        ZeroHpRule::KnockedOut { .. } => None,
    }
}

/// Whether the system plays death saves.
pub fn has_death_saves(system: &RuleSystem) -> bool {
    rule(system).is_some()
}

/// A player character just dropped to 0: they start dying.
pub(crate) fn start_dying(system: &RuleSystem, bearer: &mut Combatant) {
    if rule(system).is_some() && bearer.progress.is_some() && bearer.death_saves.is_none() {
        bearer.death_saves = Some(DeathSaves::default());
    }
}

fn tally_event(who: &Combatant, t: &DeathSaves) -> Event {
    Event::DeathSaves {
        target: who.id.clone(),
        successes: t.successes,
        failures: t.failures,
    }
}

/// Adds `n` failures and proposes the death when they reach the rule's.
fn fail(rule: &Rule<'_>, who: &mut Combatant, n: u32, events: &mut Vec<Event>) {
    let Some(mut t) = who.death_saves else {
        return;
    };
    t.failures = (t.failures + n).min(rule.failures);
    events.push(tally_event(who, &t));
    if t.failures >= rule.failures {
        t.death_due = true;
        events.push(Event::DeathDue {
            target: who.id.clone(),
        });
    }
    who.death_saves = Some(t);
}

/// A dying character takes damage: failures, a critical counting more;
/// a stable one starts rolling again.
pub(crate) fn hit_while_down(
    system: &RuleSystem,
    bearer: &mut Combatant,
    critical: bool,
    events: &mut Vec<Event>,
) {
    let Some(rule) = rule(system) else {
        return;
    };
    let Some(t) = bearer.death_saves.as_mut() else {
        return;
    };
    if t.death_due {
        return;
    }
    t.stable = false;
    let n = if critical {
        rule.on_critical_hit
    } else {
        rule.on_hit
    };
    fail(&rule, bearer, n, events);
}

fn rolling<'s>(scene: &'s mut Scene, who: &str) -> Result<&'s mut Combatant, DeathRefusal> {
    let c = scene
        .get_mut(who)
        .ok_or_else(|| DeathRefusal::UnknownCombatant(who.into()))?;
    if c.hit_points > 0 || !c.death_saves.is_some_and(|t| t.rolling()) {
        return Err(DeathRefusal::NotRolling(who.into()));
    }
    Ok(c)
}

/// `who` rolls against death: the check die alone against the rule's
/// difficulty. A success counts one, a failure one, a critical failure
/// the rule's number; a critical success brings them back with 1 hit
/// point when the rule says so. Enough successes: stable. Enough
/// failures: the death is proposed to the GM ([`Event::DeathDue`]).
pub fn death_save(
    system: &RuleSystem,
    scene: &Scene,
    who: &str,
    dice: &mut dyn DiceSource,
) -> Result<(Scene, Vec<Event>), DeathRefusal> {
    let rule = rule(system).ok_or(DeathRefusal::NoDeathSaves)?;
    let mut next = scene.clone();
    let c = rolling(&mut next, who)?;
    let mut roll = check::roll(
        system,
        RollScope::Saves,
        Vec::new(),
        Advantage::Normal,
        Some(RollTarget::Difficulty {
            id: None,
            value: rule.difficulty,
        }),
        dice,
    )
    .map_err(|e| DeathRefusal::Engine(format!("{e:?}")))?;
    // The death-save rule reads the natural faces itself ("a 20 brings
    // back, a 1 counts twice"), even where the system's critical bands
    // only cover attacks (the SRD's `rolls: attacks`).
    let o = &system.outcomes;
    if o.critical_failure.natural.contains(&roll.natural) {
        roll.band = Some(OutcomeBand::CriticalFailure);
    } else if o.critical_success.natural.contains(&roll.natural) {
        roll.band = Some(OutcomeBand::CriticalSuccess);
    }
    let band = roll.band.unwrap_or(OutcomeBand::Failure);
    let mut events = vec![Event::Roll {
        roller: who.into(),
        against: None,
        purpose: RollPurpose::DeathSave,
        breakdown: roll,
    }];
    match band {
        OutcomeBand::CriticalSuccess if rule.critical_success_revives => {
            let ko = system.zero_hp.condition().to_string();
            c.death_saves = None;
            c.hit_points = 1;
            events.push(Event::Healed {
                target: who.into(),
                amount: 1,
                hp_before: 0,
                hp_after: 1,
            });
            c.conditions.retain(|x| {
                let gone = x.id.as_deref() == Some(ko.as_str());
                if gone {
                    events.push(Event::ConditionEnded {
                        target: who.into(),
                        name: x.name.clone(),
                    });
                }
                !gone
            });
            events.push(Event::Revived { target: who.into() });
        }
        OutcomeBand::CriticalSuccess | OutcomeBand::Success => {
            let mut t = c.death_saves.unwrap_or_default();
            t.successes = (t.successes + 1).min(rule.successes);
            events.push(tally_event(c, &t));
            if t.successes >= rule.successes {
                t.stable = true;
                events.push(Event::Stabilized { target: who.into() });
            }
            c.death_saves = Some(t);
        }
        OutcomeBand::CriticalFailure => fail(&rule, c, rule.on_critical_failure, &mut events),
        OutcomeBand::Failure => fail(&rule, c, 1, &mut events),
    }
    Ok((next, events))
}

/// `actor` tries to stabilise `target`, who is dying: the rule's ability
/// against its difficulty. The action spent and the reach are the
/// caller's (the fight checks both); the XP of the roll's band goes to
/// the actor, as for any check.
pub fn stabilize(
    system: &RuleSystem,
    scene: &Scene,
    actor: &str,
    target: &str,
    dice: &mut dyn DiceSource,
) -> Result<(Scene, Vec<Event>), DeathRefusal> {
    let rule = rule(system).ok_or(DeathRefusal::NoDeathSaves)?;
    let st = rule.stabilize.ok_or(DeathRefusal::NoStabilizeRule)?;
    let mut next = scene.clone();
    rolling(&mut next, target)?;
    let helper = next
        .get(actor)
        .ok_or_else(|| DeathRefusal::UnknownCombatant(actor.into()))?;
    let roll = check::ability_check(
        system,
        helper,
        &st.ability,
        RollScope::Checks,
        Some(RollTarget::Difficulty {
            id: None,
            value: st.difficulty,
        }),
        Advantage::Normal,
        dice,
    )
    .map_err(|e| DeathRefusal::Engine(format!("{e:?}")))?;
    let band = roll.band;
    let mut events = vec![Event::Roll {
        roller: actor.into(),
        against: Some(target.into()),
        purpose: RollPurpose::Stabilize,
        breakdown: roll,
    }];
    if let Some(helper) = next.get_mut(actor) {
        for change in award_band(system, helper, band) {
            events.push(Event::Progress {
                who: actor.into(),
                change,
            });
        }
    }
    if band.is_some_and(OutcomeBand::is_success) {
        let c = next.get_mut(target).expect("checked above");
        if let Some(t) = c.death_saves.as_mut() {
            t.stable = true;
        }
        events.push(Event::Stabilized {
            target: target.into(),
        });
    }
    Ok((next, events))
}

/// The GM decides another outcome than the death the dice proposed (or
/// stops the saves): `who` is stable, still down.
pub fn spare(scene: &Scene, who: &str) -> Result<(Scene, Vec<Event>), DeathRefusal> {
    let mut next = scene.clone();
    let c = next
        .get_mut(who)
        .ok_or_else(|| DeathRefusal::UnknownCombatant(who.into()))?;
    let Some(t) = c.death_saves.as_mut() else {
        return Err(DeathRefusal::NotRolling(who.into()));
    };
    t.death_due = false;
    t.stable = true;
    Ok((next, vec![Event::Stabilized { target: who.into() }]))
}

/// The stabilise rule, if the system has one (what the player's card
/// shows).
pub fn stabilize_rule(system: &RuleSystem) -> Option<&StabilizeRule> {
    match &system.zero_hp {
        ZeroHpRule::DeathSaves { stabilize, .. } => stabilize.as_ref(),
        ZeroHpRule::KnockedOut { .. } => None,
    }
}
