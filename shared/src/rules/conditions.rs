//! Conditions defined by the rule system: putting them on, their effects
//! on hit points, and the turn boundaries where durations, cooldowns and
//! damage over time move.
//!
//! Durations count the **bearer's** turns and tick at the end of them.
//! Cooldowns tick at the start of their owner's turn. Both functions take
//! a scene and return the next one: no hidden state.

use super::dice::DiceSource;
use super::events::{DamageBreakdown, Event};
use super::model::{ApplySpec, ConditionEffect, ConditionKind, RuleSystem, ZeroHpRule};
use super::sheet::{ActiveCondition, Combatant, Scene};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnError {
    UnknownCombatant(String),
    UnknownContext(String),
    /// Another combatant's turn is still open.
    TurnInProgress(String),
    NotTheirTurn(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionError {
    UnknownCondition(String),
    UnknownCombatant(String),
}

/// Turns an `ApplySpec` into the condition the bearer will carry.
pub fn instantiate(
    system: &RuleSystem,
    spec: &ApplySpec,
    action_name: &str,
    source: &str,
) -> Result<ActiveCondition, ConditionError> {
    let (id, name, kind, effects) = match &spec.condition {
        Some(cid) => {
            let def = system
                .condition(cid)
                .ok_or_else(|| ConditionError::UnknownCondition(cid.clone()))?;
            (
                Some(def.id.clone()),
                def.name.clone(),
                def.kind,
                def.effects.clone(),
            )
        }
        None => (
            None,
            action_name.to_string(),
            spec.kind.unwrap_or(ConditionKind::Bane),
            spec.effects.clone(),
        ),
    };
    Ok(ActiveCondition {
        id,
        name,
        kind,
        effects,
        remaining: Some(spec.turns),
        source: source.into(),
        skip_next_tick: false,
        turns_elapsed: 0,
    })
}

/// A named condition of the system with a duration (`None` = until
/// removed).
pub fn named(
    system: &RuleSystem,
    id: &str,
    turns: Option<u32>,
    source: &str,
) -> Result<ActiveCondition, ConditionError> {
    let def = system
        .condition(id)
        .ok_or_else(|| ConditionError::UnknownCondition(id.into()))?;
    Ok(ActiveCondition {
        id: Some(def.id.clone()),
        name: def.name.clone(),
        kind: def.kind,
        effects: def.effects.clone(),
        remaining: turns,
        source: source.into(),
        skip_next_tick: false,
        turns_elapsed: 0,
    })
}

/// Puts a condition on a combatant. The same condition again refreshes
/// the duration (the longer one wins) instead of stacking.
pub(crate) fn put_on(
    system: &RuleSystem,
    scene: &mut Scene,
    target: &str,
    mut cond: ActiveCondition,
    events: &mut Vec<Event>,
) {
    cond.skip_next_tick =
        !system.durations.application_turn_counts && scene.active.as_deref() == Some(target);
    let Some(bearer) = scene.get_mut(target) else {
        return;
    };
    events.push(Event::ConditionApplied {
        target: target.into(),
        name: cond.name.clone(),
        kind: cond.kind,
        turns: cond.remaining,
    });
    let same = bearer
        .conditions
        .iter_mut()
        .find(|c| c.name == cond.name && c.id == cond.id);
    match same {
        Some(existing) => {
            existing.remaining = match (existing.remaining, cond.remaining) {
                (Some(a), Some(b)) => Some(a.max(b)),
                _ => None,
            };
            existing.skip_next_tick |= cond.skip_next_tick;
        }
        None => bearer.conditions.push(cond),
    }
}

/// Public, pure form of [`put_on`].
pub fn apply_condition(
    system: &RuleSystem,
    scene: &Scene,
    target: &str,
    cond: ActiveCondition,
) -> Result<(Scene, Vec<Event>), ConditionError> {
    if scene.get(target).is_none() {
        return Err(ConditionError::UnknownCombatant(target.into()));
    }
    let mut next = scene.clone();
    let mut events = Vec::new();
    put_on(system, &mut next, target, cond, &mut events);
    Ok((next, events))
}

fn remove_where(
    bearer: &mut Combatant,
    events: &mut Vec<Event>,
    pred: impl Fn(&ActiveCondition) -> bool,
) {
    let target = bearer.id.clone();
    bearer.conditions.retain(|c| {
        let gone = pred(c);
        if gone {
            events.push(Event::ConditionEnded {
                target: target.clone(),
                name: c.name.clone(),
            });
        }
        !gone
    });
}

/// Takes hit points away; at 0 the system's zero-HP condition goes on.
pub(crate) fn lose_hp(
    system: &RuleSystem,
    scene: &mut Scene,
    target: &str,
    breakdown: DamageBreakdown,
    events: &mut Vec<Event>,
) {
    let Some(bearer) = scene.get_mut(target) else {
        return;
    };
    let before = bearer.hit_points;
    let after = (before - breakdown.total).max(0);
    bearer.hit_points = after;
    let critical = breakdown.multiplier > 1;
    let hurt = breakdown.total > 0;
    events.push(Event::Damaged {
        target: target.into(),
        breakdown,
        hp_before: before,
        hp_after: after,
    });
    if before == 0 && hurt {
        super::death::hit_while_down(system, bearer, critical, events);
        return;
    }
    let ko = system.zero_hp.condition();
    if after == 0 && before > 0 && bearer.condition_named(ko).is_none() {
        let out = match &system.zero_hp {
            ZeroHpRule::KnockedOut { out_condition, .. } => Some(out_condition.as_str()),
            ZeroHpRule::DeathSaves { .. } => None,
        };
        if out.is_some_and(|o| bearer.condition_named(o).is_some()) {
            return;
        }
        if let Ok(cond) = named(system, ko, None, target) {
            events.push(Event::KnockedOut {
                target: target.into(),
            });
            put_on(system, scene, target, cond, events);
        }
        if let Some(bearer) = scene.get_mut(target) {
            super::death::start_dying(system, bearer);
        }
    }
}

/// Gives hit points back up to the maximum; a heal lifts the zero-HP
/// condition (but not "out of the scene").
pub(crate) fn gain_hp(
    system: &RuleSystem,
    scene: &mut Scene,
    target: &str,
    amount: i32,
    events: &mut Vec<Event>,
) {
    let Some(bearer) = scene.get_mut(target) else {
        return;
    };
    let max = bearer.max_hit_points(system).unwrap_or(bearer.hit_points);
    let before = bearer.hit_points;
    let after = (before + amount.max(0)).min(max).max(before);
    bearer.hit_points = after;
    events.push(Event::Healed {
        target: target.into(),
        amount: after - before,
        hp_before: before,
        hp_after: after,
    });
    let ko = system.zero_hp.condition();
    if after > 0 {
        bearer.death_saves = None;
    }
    if after > 0 && bearer.condition_named(ko).is_some() {
        remove_where(bearer, events, |c| c.id.as_deref() == Some(ko));
        events.push(Event::Revived {
            target: target.into(),
        });
    }
}

/// Opens a combatant's turn: cooldowns tick, the action budget of the
/// scene's context is refilled — or emptied if a condition makes them
/// lose the turn.
pub fn start_turn(
    system: &RuleSystem,
    scene: &Scene,
    who: &str,
) -> Result<(Scene, Vec<Event>), TurnError> {
    if let Some(active) = &scene.active {
        return Err(TurnError::TurnInProgress(active.clone()));
    }
    let ctx = system
        .turn_context(&scene.context)
        .ok_or_else(|| TurnError::UnknownContext(scene.context.clone()))?;
    let mut next = scene.clone();
    let mut events = Vec::new();
    let c = next
        .get_mut(who)
        .ok_or_else(|| TurnError::UnknownCombatant(who.into()))?;
    for counter in c.cooldowns.values_mut() {
        *counter = counter.saturating_sub(1);
    }
    c.cooldowns.retain(|_, n| *n > 0);
    c.turn.spent_by_kind.clear();
    c.turn.actions_left = ctx.actions_per_turn;
    if let Some(cond) = c.incapacitated_by() {
        events.push(Event::TurnLost {
            who: who.into(),
            because: cond.name.clone(),
        });
        c.turn.actions_left = 0;
    }
    next.active = Some(who.into());
    Ok((next, events))
}

/// Closes a combatant's turn: damage over time, then every duration
/// counts down; a knocked-out bearer counts a turn without a heal.
pub fn end_turn(
    system: &RuleSystem,
    scene: &Scene,
    who: &str,
    dice: &mut dyn DiceSource,
) -> Result<(Scene, Vec<Event>), TurnError> {
    if scene.active.as_deref() != Some(who) {
        return Err(TurnError::NotTheirTurn(who.into()));
    }
    let mut next = scene.clone();
    let mut events = Vec::new();
    let bearer = next
        .get(who)
        .ok_or_else(|| TurnError::UnknownCombatant(who.into()))?;

    let dots: Vec<(String, super::dice::DiceExpr)> = bearer
        .effects()
        .filter_map(|(c, e)| match e {
            ConditionEffect::DamagePerTurn(d) => Some((c.name.clone(), *d)),
            _ => None,
        })
        .collect();
    for (_, amount) in dots {
        if next.get(who).is_some_and(|b| b.hit_points == 0) {
            break;
        }
        let r = amount.roll(dice);
        let total = r.total.max(0);
        let breakdown = DamageBreakdown {
            amount: amount.to_string(),
            faces: r.faces,
            rolled: r.total,
            bonus: 0,
            multiplier: 1,
            taken_modifier: 0,
            ignored: false,
            total,
        };
        lose_hp(system, &mut next, who, breakdown, &mut events);
    }

    let bearer = next.get_mut(who).expect("checked above");
    for c in &mut bearer.conditions {
        if c.skip_next_tick {
            c.skip_next_tick = false;
            continue;
        }
        c.turns_elapsed += 1;
        if let Some(n) = c.remaining.as_mut() {
            *n = n.saturating_sub(1);
        }
    }
    remove_where(bearer, &mut events, |c| c.remaining == Some(0));

    if let ZeroHpRule::KnockedOut {
        condition,
        out_after_turns,
        out_condition,
        ..
    } = &system.zero_hp
    {
        let down_long_enough = bearer
            .condition_named(condition)
            .is_some_and(|c| c.turns_elapsed >= *out_after_turns);
        if down_long_enough {
            remove_where(bearer, &mut events, |c| {
                c.id.as_deref() == Some(condition.as_str())
            });
            events.push(Event::OutOfScene { target: who.into() });
            if let Ok(out) = named(system, out_condition, None, who) {
                put_on(system, &mut next, who, out, &mut events);
            }
        }
    }
    next.active = None;
    Ok((next, events))
}

/// Removes a condition by id (the GM lifts it, an escape succeeds).
pub fn remove_condition(
    scene: &Scene,
    target: &str,
    id: &str,
) -> Result<(Scene, Vec<Event>), ConditionError> {
    let mut next = scene.clone();
    let mut events = Vec::new();
    let bearer = next
        .get_mut(target)
        .ok_or_else(|| ConditionError::UnknownCombatant(target.into()))?;
    remove_where(bearer, &mut events, |c| c.id.as_deref() == Some(id));
    Ok((next, events))
}
