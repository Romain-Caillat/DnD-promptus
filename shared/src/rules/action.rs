//! Resolving an action played by a combatant: every gate checked first
//! (turn, budget, level, cooldown, targets), then the rolls, damage,
//! healing, conditions, cooldown and XP — one pure function from a scene
//! to the next scene plus what happened.

use serde::Serialize;

use super::check::{
    self, Advantage, CheckError, Modifier, ModifierSource, OutcomeBand, RollBreakdown, RollTarget,
};
use super::conditions::{self, gain_hp, lose_hp, put_on};
use super::dice::DiceSource;
use super::events::{DamageBreakdown, Event, RollPurpose};
use super::model::*;
use super::progression::award_band;
use super::sheet::{Combatant, Scene, SheetError};

/// Which action: one the combatant owns, or the use of an item they carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionRef {
    Own(String),
    Item(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRequest {
    pub actor: String,
    pub action: ActionRef,
    pub targets: Vec<String>,
    /// Situations the GM declared (`furtif`, `en_hauteur`).
    pub situations: Vec<String>,
    /// The difficulty the GM sets for a save the system leaves open.
    pub save_difficulty: Option<i32>,
    /// The option picked when the action offers a choice.
    pub choice: Option<usize>,
}

impl ActionRequest {
    pub fn new(actor: &str, action: &str, targets: &[&str]) -> Self {
        Self {
            actor: actor.into(),
            action: ActionRef::Own(action.into()),
            targets: targets.iter().map(|t| t.to_string()).collect(),
            situations: Vec::new(),
            save_difficulty: None,
            choice: None,
        }
    }

    pub fn item(actor: &str, item: &str, targets: &[&str]) -> Self {
        Self {
            action: ActionRef::Item(item.into()),
            ..Self::new(actor, "", targets)
        }
    }
}

/// Why an action is refused. Nothing changes when one is returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "refusal", rename_all = "snake_case")]
pub enum Refusal {
    UnknownActor { actor: String },
    UnknownAction { action: String },
    NotTheirTurn,
    Incapacitated { because: String },
    LevelTooLow { required: u32, current: u32 },
    OnCooldown { counter: u32 },
    NotEnoughActions { cost: u32, left: u32 },
    KindLimitReached { kind: String, max: u32 },
    ItemMissing { item: String },
    TargetCount { expected: &'static str, got: usize },
    UnknownTarget { target: String },
    WrongSide { target: String },
    TargetDown { target: String },
    UnknownSituation { situation: String },
    ChoiceRequired { options: usize },
    DifficultyRequired,
    Engine { detail: String },
}

impl From<SheetError> for Refusal {
    fn from(e: SheetError) -> Self {
        Self::Engine {
            detail: format!("{e:?}"),
        }
    }
}

impl From<CheckError> for Refusal {
    fn from(e: CheckError) -> Self {
        Self::Engine {
            detail: format!("{e:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resolution {
    pub scene: Scene,
    pub events: Vec<Event>,
}

/// Level and cooldown: what locks a card regardless of whose turn it is.
pub fn gate(system: &RuleSystem, actor: &Combatant, action: &ActionDef) -> Result<(), Refusal> {
    if let (Some(required), Some(current)) = (action.level, actor.level(system))
        && current < required
    {
        return Err(Refusal::LevelTooLow { required, current });
    }
    if let Some(&counter) = actor.cooldowns.get(&action.id)
        && counter > 0
    {
        return Err(Refusal::OnCooldown { counter });
    }
    Ok(())
}

/// One card of a combatant's hand, with what the card displays.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionCard<'a> {
    pub action: &'a ActionDef,
    /// Why it cannot be played now (level, cooldown), if so.
    pub locked: Option<Refusal>,
    /// The attack roll's bonus, already computed (ability + precision if
    /// the system adds it), for actions that roll to hit.
    pub attack_bonus: Option<i32>,
}

/// The actions a combatant has, derived from class, level and learned
/// slots — the cards the player sees.
pub fn action_cards<'a>(system: &'a RuleSystem, actor: &'a Combatant) -> Vec<ActionCard<'a>> {
    actor
        .actions(system)
        .into_iter()
        .map(|action| {
            let attack_bonus = (action.roll == RollSpec::Attack)
                .then(|| attack_ability(system, actor, action).ok())
                .flatten()
                .and_then(|ab| actor.modifier(system, &ab).ok())
                .map(|m| {
                    m + match system.attack.precision {
                        PrecisionRule::AddedToAttackRoll => action.precision(),
                        PrecisionRule::NotApplied => 0,
                    }
                });
            ActionCard {
                action,
                locked: gate(system, actor, action).err(),
                attack_bonus,
            }
        })
        .collect()
}

/// The ability an attack adds: the action's own, else the system's rule
/// over the class's primary abilities.
pub fn attack_ability(
    system: &RuleSystem,
    actor: &Combatant,
    action: &ActionDef,
) -> Result<String, Refusal> {
    if let Some(a) = &action.ability {
        return Ok(a.clone());
    }
    let class = actor.class(system).ok_or_else(|| Refusal::Engine {
        detail: format!(
            "action `{}` names no ability and its user has no class",
            action.id
        ),
    })?;
    let primaries = &class.primary_abilities;
    match system.attack.ability {
        AttackAbility::FirstPrimary => primaries.first().cloned(),
        AttackAbility::BestPrimary => primaries
            .iter()
            .max_by_key(|a| actor.modifier(system, a).unwrap_or(i32::MIN))
            .cloned(),
    }
    .ok_or_else(|| Refusal::Engine {
        detail: format!("class `{}` has no primary ability", class.id),
    })
}

fn find_action(
    system: &RuleSystem,
    actor: &Combatant,
    r: &ActionRef,
) -> Result<ActionDef, Refusal> {
    match r {
        ActionRef::Own(id) => actor
            .actions(system)
            .into_iter()
            .find(|a| &a.id == id)
            .cloned()
            .ok_or_else(|| Refusal::UnknownAction { action: id.clone() }),
        ActionRef::Item(id) => {
            let item = system
                .item(id)
                .ok_or_else(|| Refusal::UnknownAction { action: id.clone() })?;
            if actor.inventory.get(id).copied().unwrap_or(0) == 0 {
                return Err(Refusal::ItemMissing { item: id.clone() });
            }
            item.action
                .clone()
                .ok_or_else(|| Refusal::UnknownAction { action: id.clone() })
        }
    }
}

fn resolve_targets(
    scene: &Scene,
    actor: &Combatant,
    action: &ActionDef,
    asked: &[String],
) -> Result<Vec<String>, Refusal> {
    let same_side = |id: &str| scene.get(id).map(|c| c.side == actor.side);
    for t in asked {
        if scene.get(t).is_none() {
            return Err(Refusal::UnknownTarget { target: t.clone() });
        }
    }
    let count = |expected: &'static str, ok: bool| {
        if ok {
            Ok(())
        } else {
            Err(Refusal::TargetCount {
                expected,
                got: asked.len(),
            })
        }
    };
    let side = |t: &String, want_same: bool| {
        if same_side(t) == Some(want_same) {
            Ok(())
        } else {
            Err(Refusal::WrongSide { target: t.clone() })
        }
    };
    let targets: Vec<String> = match action.target {
        Targeting::Myself => {
            count(
                "none or self",
                asked.is_empty() || asked == [actor.id.clone()],
            )?;
            vec![actor.id.clone()]
        }
        Targeting::Ally | Targeting::AllyOrSelf => {
            count("one ally", asked.len() == 1)?;
            side(&asked[0], true)?;
            if action.target == Targeting::Ally && asked[0] == actor.id {
                return Err(Refusal::WrongSide {
                    target: asked[0].clone(),
                });
            }
            asked.to_vec()
        }
        Targeting::Enemy | Targeting::Enemies => {
            let ok = if action.target == Targeting::Enemy {
                asked.len() == 1
            } else {
                !asked.is_empty()
            };
            count(
                if action.target == Targeting::Enemy {
                    "one enemy"
                } else {
                    "at least one enemy"
                },
                ok,
            )?;
            let mut seen = Vec::new();
            for t in asked {
                side(t, false)?;
                if scene.get(t).is_some_and(|c| c.hit_points == 0) {
                    return Err(Refusal::TargetDown { target: t.clone() });
                }
                if !seen.contains(t) {
                    seen.push(t.clone());
                }
            }
            seen
        }
        Targeting::AllAllies => {
            count("none (every ally)", asked.is_empty())?;
            scene
                .combatants
                .values()
                .filter(|c| c.side == actor.side)
                .map(|c| c.id.clone())
                .collect()
        }
    };
    Ok(targets)
}

/// Tags with the player's choice resolved.
fn effective_tags(action: &ActionDef, choice: Option<usize>) -> Result<Vec<Tag>, Refusal> {
    let mut out = Vec::new();
    for t in &action.tags {
        match t {
            Tag::Choice(options) => {
                let i = choice
                    .filter(|i| *i < options.len())
                    .ok_or(Refusal::ChoiceRequired {
                        options: options.len(),
                    })?;
                out.push(options[i].clone());
            }
            other => out.push(other.clone()),
        }
    }
    Ok(out)
}

fn is_attack(roll: &RollSpec) -> bool {
    matches!(
        roll,
        RollSpec::Attack | RollSpec::AutoHit | RollSpec::AutoCritical
    )
}

/// Checks every gate of an action without resolving it.
pub fn check_playable(
    system: &RuleSystem,
    scene: &Scene,
    req: &ActionRequest,
) -> Result<(ActionDef, Vec<String>, Vec<Tag>), Refusal> {
    let actor = scene.get(&req.actor).ok_or_else(|| Refusal::UnknownActor {
        actor: req.actor.clone(),
    })?;
    if scene.active.as_deref() != Some(actor.id.as_str()) {
        return Err(Refusal::NotTheirTurn);
    }
    if let Some(c) = actor.incapacitated_by() {
        return Err(Refusal::Incapacitated {
            because: c.name.clone(),
        });
    }
    let action = find_action(system, actor, &req.action)?;
    gate(system, actor, &action)?;
    let ctx = system
        .turn_context(&scene.context)
        .ok_or_else(|| Refusal::Engine {
            detail: format!("no turn context `{}`", scene.context),
        })?;
    let kind = system
        .action_kind(&action.kind)
        .ok_or_else(|| Refusal::Engine {
            detail: format!("no action kind `{}`", action.kind),
        })?;
    if actor.turn.actions_left < kind.cost {
        return Err(Refusal::NotEnoughActions {
            cost: kind.cost,
            left: actor.turn.actions_left,
        });
    }
    if let Some(limit) = ctx.limits.iter().find(|l| l.kind == kind.id)
        && actor.turn.spent_by_kind.get(&kind.id).copied().unwrap_or(0) >= limit.max_per_turn
    {
        return Err(Refusal::KindLimitReached {
            kind: kind.id.clone(),
            max: limit.max_per_turn,
        });
    }
    for s in &req.situations {
        if system.situation(s).is_none() {
            return Err(Refusal::UnknownSituation {
                situation: s.clone(),
            });
        }
    }
    let targets = resolve_targets(scene, actor, &action, &req.targets)?;
    let tags = effective_tags(&action, req.choice)?;
    let open_save = |spec: &ApplySpec| spec.save.as_ref().is_some_and(|s| s.difficulty.is_none());
    let needs_difficulty = tags.iter().any(|t| match t {
        Tag::Buff(s) | Tag::Control(s) => open_save(s),
        _ => false,
    }) || (is_attack(&action.roll)
        && actor
            .effects()
            .any(|(_, e)| matches!(e, ConditionEffect::OnHit(s) if open_save(s))));
    if needs_difficulty && req.save_difficulty.is_none() {
        return Err(Refusal::DifficultyRequired);
    }
    Ok((action, targets, tags))
}

fn award(
    system: &RuleSystem,
    scene: &mut Scene,
    who: &str,
    band: Option<OutcomeBand>,
    events: &mut Vec<Event>,
) {
    let Some(c) = scene.get_mut(who) else { return };
    for change in award_band(system, c, band) {
        events.push(Event::Progress {
            who: who.into(),
            change,
        });
    }
}

/// Puts a condition on `target`, after the save it allows.
#[allow(clippy::too_many_arguments)]
fn deliver(
    system: &RuleSystem,
    scene: &mut Scene,
    spec: &ApplySpec,
    action_name: &str,
    source: &str,
    target: &str,
    gm_difficulty: Option<i32>,
    dice: &mut dyn DiceSource,
    events: &mut Vec<Event>,
) -> Result<(), Refusal> {
    let cond = conditions::instantiate(system, spec, action_name, source).map_err(|e| {
        Refusal::Engine {
            detail: format!("{e:?}"),
        }
    })?;
    if let Some(save) = &spec.save {
        let target_value = match &save.difficulty {
            Some(id) => check::difficulty(system, id)?,
            None => RollTarget::Difficulty {
                id: None,
                value: gm_difficulty.ok_or(Refusal::DifficultyRequired)?,
            },
        };
        let saver = scene.get(target).ok_or_else(|| Refusal::UnknownTarget {
            target: target.into(),
        })?;
        let roll = check::ability_check(
            system,
            saver,
            &save.ability,
            RollScope::Saves,
            Some(target_value),
            Advantage::Normal,
            dice,
        )?;
        let band = roll.band;
        events.push(Event::Roll {
            roller: target.into(),
            against: Some(source.into()),
            purpose: RollPurpose::Save,
            breakdown: roll,
        });
        award(system, scene, target, band, events);
        if band.is_some_and(OutcomeBand::is_success) {
            events.push(Event::ConditionResisted {
                target: target.into(),
                name: cond.name,
            });
            return Ok(());
        }
    }
    put_on(system, scene, target, cond, events);
    Ok(())
}

fn attack_roll(
    system: &RuleSystem,
    actor: &Combatant,
    action: &ActionDef,
    target: &Combatant,
    situations: &[&SituationalBonus],
    dice: &mut dyn DiceSource,
) -> Result<RollBreakdown, Refusal> {
    let ability = attack_ability(system, actor, action)?;
    let (mut mods, mut adv, mut dis) =
        check::ability_modifiers(system, actor, &ability, RollScope::Attacks)?;
    if system.attack.precision == PrecisionRule::AddedToAttackRoll {
        if action.precision() != 0 {
            mods.push(Modifier {
                source: ModifierSource::Precision(action.id.clone()),
                value: action.precision(),
            });
        }
        for s in situations.iter().filter(|s| s.precision != 0) {
            mods.push(Modifier {
                source: ModifierSource::Situation(s.situation.clone()),
                value: s.precision,
            });
        }
        for (c, e) in actor.effects() {
            if let ConditionEffect::Precision(v) = e {
                mods.push(Modifier {
                    source: ModifierSource::Condition(c.name.clone()),
                    value: *v,
                });
            }
        }
        for (c, e) in target.effects() {
            if let ConditionEffect::PrecisionAgainst(v) = e {
                mods.push(Modifier {
                    source: ModifierSource::Condition(c.name.clone()),
                    value: *v,
                });
            }
        }
    }
    for (_, e) in target.effects() {
        match e {
            ConditionEffect::AdvantageAgainst => adv = true,
            ConditionEffect::DisadvantageAgainst => dis = true,
            _ => {}
        }
    }
    Ok(check::roll(
        system,
        mods,
        Advantage::combine(adv, dis),
        Some(RollTarget::ArmorClass {
            value: target.armor_class(system)?,
        }),
        dice,
    )?)
}

/// Resolves one action. On a refusal nothing has happened; on success the
/// returned scene is the next state and the events say what changed.
pub fn resolve_action(
    system: &RuleSystem,
    scene: &Scene,
    req: &ActionRequest,
    dice: &mut dyn DiceSource,
) -> Result<Resolution, Refusal> {
    let (action, targets, tags) = check_playable(system, scene, req)?;
    let mut next = scene.clone();
    let mut events = Vec::new();
    let actor_id = req.actor.as_str();

    // Spend: budget, cooldown, item.
    {
        let kind = system.action_kind(&action.kind).expect("checked");
        let actor = next.get_mut(actor_id).expect("checked");
        actor.turn.actions_left -= kind.cost;
        *actor.turn.spent_by_kind.entry(kind.id.clone()).or_insert(0) += 1;
        let cd = action.cooldown();
        if cd > 0 {
            let counter = cd
                + match system.cooldowns.meaning {
                    CooldownMeaning::SkipNextTurns => 1,
                    CooldownMeaning::TurnOfUseCounts => 0,
                };
            actor.cooldowns.insert(action.id.clone(), counter);
            events.push(Event::CooldownStarted {
                who: actor_id.into(),
                action: action.id.clone(),
                counter,
            });
        }
        if let ActionRef::Item(item) = &req.action
            && system.item(item).is_some_and(|i| i.consumable)
        {
            let left = actor.inventory.get(item).copied().unwrap_or(1) - 1;
            if left == 0 {
                actor.inventory.remove(item);
            } else {
                actor.inventory.insert(item.clone(), left);
            }
            events.push(Event::ItemUsed {
                who: actor_id.into(),
                item: item.clone(),
                left,
            });
        }
    }

    let situational: Vec<&SituationalBonus> = tags
        .iter()
        .filter_map(|t| match t {
            Tag::Situational(b) if req.situations.contains(&b.situation) => Some(b),
            _ => None,
        })
        .collect();

    for target_id in &targets {
        let actor = next.get(actor_id).expect("checked").clone();
        let Some(target) = next.get(target_id).cloned() else {
            continue;
        };

        let (landed, critical) = match &action.roll {
            RollSpec::None | RollSpec::AutoHit => (true, false),
            RollSpec::AutoCritical => (true, true),
            RollSpec::Attack => {
                let roll = attack_roll(system, &actor, &action, &target, &situational, dice)?;
                let band = roll.band;
                events.push(Event::Roll {
                    roller: actor_id.into(),
                    against: Some(target_id.clone()),
                    purpose: RollPurpose::Attack,
                    breakdown: roll,
                });
                award(system, &mut next, actor_id, band, &mut events);
                (
                    band.is_some_and(OutcomeBand::is_success),
                    band == Some(OutcomeBand::CriticalSuccess),
                )
            }
            RollSpec::Contest {
                actor: a_ab,
                target: t_ab,
            } => {
                let defence = check::ability_check(
                    system,
                    &target,
                    t_ab,
                    RollScope::Checks,
                    None,
                    Advantage::Normal,
                    dice,
                )?;
                let mut attack = check::ability_check(
                    system,
                    &actor,
                    a_ab,
                    RollScope::Checks,
                    None,
                    Advantage::Normal,
                    dice,
                )?;
                attack.target = Some(RollTarget::Opposed {
                    value: defence.total,
                });
                attack.band =
                    check::band_for(system, attack.natural, attack.total, Some(defence.total));
                let band = attack.band;
                events.push(Event::Roll {
                    roller: target_id.clone(),
                    against: Some(actor_id.into()),
                    purpose: RollPurpose::Contest,
                    breakdown: defence,
                });
                events.push(Event::Roll {
                    roller: actor_id.into(),
                    against: Some(target_id.clone()),
                    purpose: RollPurpose::Contest,
                    breakdown: attack,
                });
                award(system, &mut next, actor_id, band, &mut events);
                (band.is_some_and(OutcomeBand::is_success), false)
            }
        };

        if landed {
            for tag in &tags {
                match tag {
                    Tag::Damage(d) => {
                        let r = d.amount.roll(dice);
                        let bonus: i32 = situational.iter().map(|s| s.damage).sum::<i32>()
                            + actor
                                .effects()
                                .filter_map(|(_, e)| match e {
                                    ConditionEffect::DamageDealt(v) => Some(*v),
                                    _ => None,
                                })
                                .sum::<i32>();
                        let multiplier = if critical {
                            system
                                .outcomes
                                .critical_success
                                .damage_multiplier
                                .unwrap_or(1)
                        } else {
                            1
                        };
                        let current = next.get(target_id).expect("exists");
                        let taken: i32 = current
                            .effects()
                            .filter_map(|(_, e)| match e {
                                ConditionEffect::DamageTaken(v) => Some(*v),
                                _ => None,
                            })
                            .sum();
                        let ignored = current
                            .effects()
                            .any(|(_, e)| *e == ConditionEffect::IgnoreDamage);
                        let total = if ignored {
                            0
                        } else {
                            ((r.total + bonus) * multiplier + taken).max(0)
                        };
                        let breakdown = DamageBreakdown {
                            amount: d.amount.to_string(),
                            faces: r.faces,
                            rolled: r.total,
                            bonus,
                            multiplier,
                            taken_modifier: taken,
                            ignored,
                            total,
                        };
                        lose_hp(system, &mut next, target_id, breakdown, &mut events);
                    }
                    Tag::Heal(h) => {
                        let r = h.amount.roll(dice);
                        gain_hp(system, &mut next, target_id, r.total, &mut events);
                    }
                    Tag::Buff(spec) | Tag::Control(spec) if spec.to == Recipient::Targets => {
                        deliver(
                            system,
                            &mut next,
                            spec,
                            &action.name,
                            actor_id,
                            target_id,
                            req.save_difficulty,
                            dice,
                            &mut events,
                        )?;
                    }
                    _ => {}
                }
            }
            if is_attack(&action.roll) {
                let riders: Vec<ApplySpec> = actor
                    .effects()
                    .filter_map(|(_, e)| match e {
                        ConditionEffect::OnHit(spec) => Some((**spec).clone()),
                        _ => None,
                    })
                    .collect();
                for spec in riders {
                    deliver(
                        system,
                        &mut next,
                        &spec,
                        &action.name,
                        actor_id,
                        target_id,
                        req.save_difficulty,
                        dice,
                        &mut events,
                    )?;
                }
            }
        } else {
            events.push(Event::Missed {
                target: target_id.clone(),
            });
        }

        if is_attack(&action.roll)
            && let Some(t) = next.get_mut(target_id)
        {
            let name = t.id.clone();
            t.conditions.retain(|c| {
                let consumed = c.has(&ConditionEffect::EndsWhenAttacked);
                if consumed {
                    events.push(Event::ConditionEnded {
                        target: name.clone(),
                        name: c.name.clone(),
                    });
                }
                !consumed
            });
        }
    }

    // Effects on the actor apply once, whatever the rolls.
    for tag in &tags {
        match tag {
            Tag::Buff(spec) | Tag::Control(spec) => {
                if spec.to == Recipient::Myself {
                    deliver(
                        system,
                        &mut next,
                        spec,
                        &action.name,
                        actor_id,
                        actor_id,
                        req.save_difficulty,
                        dice,
                        &mut events,
                    )?;
                }
                if !spec.note.is_empty() {
                    events.push(Event::ForTheGm {
                        text: spec.note.clone(),
                    });
                }
            }
            Tag::Note(text) => events.push(Event::ForTheGm { text: text.clone() }),
            Tag::Damage(DamageTag { note, .. }) | Tag::Heal(HealTag { note, .. })
                if !note.is_empty() =>
            {
                events.push(Event::ForTheGm { text: note.clone() })
            }
            _ => {}
        }
    }

    Ok(Resolution {
        scene: next,
        events,
    })
}
