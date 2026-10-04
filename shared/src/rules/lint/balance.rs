//! Balance: the numbers a rule system implies, computed from its data
//! alone — ability totals per class, expected damage per turn per class
//! and level against the reference armour classes, XP per session and
//! the level reached at the end of a campaign.
//!
//! These are **expectations**, not a simulation (`engine/simulate-fights`
//! plays real fights): one target, base ability scores (upgrade points
//! are not spent), no situational bonus, no buff. Each class plays the
//! best sustainable mix of its unlocked actions under the turn budget,
//! the kind limits and the cooldowns.

use std::fmt;

use serde::Serialize;

use super::super::check::{OutcomeBand, band_for};
use super::super::dice::DiceExpr;
use super::super::model::*;
use super::super::progression::level_for;
use super::super::sheet::modifier_for;

/// The campaign the numbers are computed for.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BalanceParams {
    /// Sessions the campaign lasts.
    pub sessions: u32,
    pub fights_per_session: u32,
    pub rounds_per_fight: u32,
    /// Ability checks each player rolls in a session, outside fights.
    pub checks_per_session: u32,
    /// Their target; `None` = the median difficulty of the system.
    pub check_difficulty: Option<i32>,
    /// The turn context fights are played in; `None` = the first one.
    pub context: Option<String>,
}

impl Default for BalanceParams {
    /// A four-act campaign (the Corsaires' plan), two fights of four
    /// rounds and six checks per player each session.
    fn default() -> Self {
        Self {
            sessions: 4,
            fights_per_session: 2,
            rounds_per_fight: 4,
            checks_per_session: 6,
            check_difficulty: None,
            context: None,
        }
    }
}

/// An armour class the damage is computed against.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Target {
    pub label: String,
    pub armor_class: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BalanceReport {
    pub system: String,
    pub params: BalanceParams,
    /// The turn context used: its id and actions per turn.
    pub context: String,
    pub actions_per_turn: u32,
    /// The adversary tiers, or the median class armour class when the
    /// system has no tier.
    pub targets: Vec<Target>,
    /// The armour class XP in fights is computed against (the median target).
    pub xp_target: i32,
    pub check_difficulty: i32,
    /// Median ability total over the classes.
    pub ability_total_median: i32,
    pub classes: Vec<ClassBalance>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClassBalance {
    pub id: String,
    pub name: String,
    pub ability_total: i32,
    /// Expected damage per turn at each level, one value per target.
    pub damage_per_turn: Vec<LevelDamage>,
    /// Expected XP earned in each session, and the total after it.
    pub sessions: Vec<SessionXp>,
    pub final_level: u32,
    /// First session at whose end the class sits at the maximum level.
    pub max_level_after_session: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LevelDamage {
    pub level: u32,
    pub per_target: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionXp {
    pub session: u32,
    /// Level the session is played at.
    pub level: u32,
    pub from_fights: f64,
    pub from_checks: f64,
    pub total_xp: f64,
}

/// The balance numbers of a rule system for a campaign.
pub fn balance_report(s: &RuleSystem, params: &BalanceParams) -> BalanceReport {
    let ctx = params
        .context
        .as_deref()
        .and_then(|id| s.turn_context(id))
        .or_else(|| s.turn_contexts.first());
    let targets = targets(s);
    let xp_target = lower_median(targets.iter().map(|t| t.armor_class).collect()).unwrap_or(10);
    let check_difficulty = params.check_difficulty.unwrap_or_else(|| {
        lower_median(s.difficulties.iter().map(|d| d.value).collect()).unwrap_or(10)
    });
    let totals: Vec<i32> = s.classes.iter().map(ability_total).collect();
    let ability_total_median = lower_median(totals).unwrap_or(0);
    let classes = s
        .classes
        .iter()
        .map(|c| class_balance(s, c, ctx, &targets, xp_target, check_difficulty, params))
        .collect();
    BalanceReport {
        system: format!("{} v{}", s.id, s.version),
        params: params.clone(),
        context: ctx.map(|c| c.id.clone()).unwrap_or_default(),
        actions_per_turn: ctx.map(|c| c.actions_per_turn).unwrap_or(0),
        targets,
        xp_target,
        check_difficulty,
        ability_total_median,
        classes,
    }
}

pub(super) fn ability_total(c: &ClassDef) -> i32 {
    c.abilities.values().sum()
}

/// The middle value, the lower one of the two for an even count.
pub(super) fn lower_median(mut values: Vec<i32>) -> Option<i32> {
    values.sort_unstable();
    values.get(values.len().saturating_sub(1) / 2).copied()
}

fn targets(s: &RuleSystem) -> Vec<Target> {
    if !s.adversary_tiers.is_empty() {
        return s
            .adversary_tiers
            .iter()
            .map(|t| Target {
                label: t.name.clone(),
                armor_class: t.armor_class,
            })
            .collect();
    }
    let acs: Vec<i32> = s
        .classes
        .iter()
        .filter_map(|c| class_stat(s, c, &s.stats.armor_class.formula, 1))
        .collect();
    vec![Target {
        label: "CA médiane des classes".into(),
        armor_class: lower_median(acs).unwrap_or(10),
    }]
}

/// A stat formula on a class's base scores at a level.
pub(super) fn class_stat(
    s: &RuleSystem,
    c: &ClassDef,
    f: &super::super::formula::Formula,
    level: u32,
) -> Option<i32> {
    stat(s, &c.abilities, f, level)
}

pub(super) fn stat(
    s: &RuleSystem,
    scores: &std::collections::BTreeMap<String, i32>,
    f: &super::super::formula::Formula,
    level: u32,
) -> Option<i32> {
    use super::super::formula::FormulaEnv;
    struct Env<'a> {
        s: &'a RuleSystem,
        scores: &'a std::collections::BTreeMap<String, i32>,
        level: u32,
    }
    impl FormulaEnv for Env<'_> {
        fn var(&self, name: &str) -> Option<i64> {
            (name == "level").then_some(self.level as i64)
        }
        fn ability_mod(&self, a: &str) -> Option<i64> {
            let score = *self.scores.get(a)?;
            modifier_for(self.s, score).ok().map(i64::from)
        }
        fn ability_score(&self, a: &str) -> Option<i64> {
            self.scores.get(a).map(|&v| v as i64)
        }
    }
    f.eval(&Env { s, scores, level }).ok().map(|v| v as i32)
}

fn modifier(s: &RuleSystem, c: &ClassDef, ability: &str) -> i32 {
    c.abilities
        .get(ability)
        .and_then(|&score| modifier_for(s, score).ok())
        .unwrap_or(0)
}

/// The attack bonus of a class action, as `action::action_cards` computes it.
fn attack_bonus(s: &RuleSystem, c: &ClassDef, a: &ActionDef) -> i32 {
    let ability = a.ability.clone().or_else(|| match s.attack.ability {
        AttackAbility::FirstPrimary => c.primary_abilities.first().cloned(),
        AttackAbility::BestPrimary => c
            .primary_abilities
            .iter()
            .max_by_key(|p| modifier(s, c, p))
            .cloned(),
    });
    let m = ability.map(|ab| modifier(s, c, &ab)).unwrap_or(0);
    m + match s.attack.precision {
        PrecisionRule::AddedToAttackRoll => a.precision(),
        PrecisionRule::NotApplied => 0,
    }
}

/// Chances of (success, critical success) of a check die + `bonus`
/// against `target`, from the system's bands.
fn odds(s: &RuleSystem, bonus: i32, target: i32) -> (f64, f64) {
    let faces = s.check.dice.faces.max(1);
    let (mut hit, mut crit) = (0u32, 0u32);
    for n in 1..=faces {
        match band_for(s, n, n as i32 + bonus, Some(target)) {
            Some(OutcomeBand::Success) => hit += 1,
            Some(OutcomeBand::CriticalSuccess) => crit += 1,
            _ => {}
        }
    }
    (hit as f64 / faces as f64, crit as f64 / faces as f64)
}

fn average(d: &DiceExpr) -> f64 {
    d.count as f64 * (d.faces as f64 + 1.0) / 2.0 + d.modifier as f64
}

/// Expected damage of one use against one target of armour class `ac`.
fn expected_damage(s: &RuleSystem, c: &ClassDef, a: &ActionDef, ac: i32) -> f64 {
    let base: f64 = a
        .tags
        .iter()
        .filter_map(|t| match t {
            Tag::Damage(d) => Some(average(&d.amount)),
            _ => None,
        })
        .sum();
    if base == 0.0 {
        return 0.0;
    }
    let mult = s.outcomes.critical_success.damage_multiplier.unwrap_or(1) as f64;
    match &a.roll {
        RollSpec::None | RollSpec::AutoHit => base,
        RollSpec::AutoCritical => base * mult,
        RollSpec::Attack => {
            let (hit, crit) = odds(s, attack_bonus(s, c, a), ac);
            base * (hit + crit * mult)
        }
        // Both sides roll the same die: even odds before modifiers.
        RollSpec::Contest { actor, target: _ } => {
            let (hit, crit) = odds(s, modifier(s, c, actor), 11);
            base * (hit + crit)
        }
    }
}

/// Expected XP of one use: rolls grant what their band grants.
fn expected_xp(s: &RuleSystem, c: &ClassDef, a: &ActionDef, ac: i32) -> f64 {
    let o = &s.outcomes;
    let (bonus, target) = match &a.roll {
        RollSpec::Attack => (attack_bonus(s, c, a), ac),
        RollSpec::Contest { actor, .. } => (modifier(s, c, actor), 11),
        _ => return 0.0,
    };
    let (hit, crit) = odds(s, bonus, target);
    hit * o.success.grants.xp as f64 + crit * o.critical_success.grants.xp as f64
}

/// How many times per turn an action can be used on average.
fn uses_per_turn(s: &RuleSystem, a: &ActionDef) -> f64 {
    match (a.cooldown(), s.cooldowns.meaning) {
        (0, _) => f64::INFINITY,
        (n, CooldownMeaning::SkipNextTurns) => 1.0 / (n as f64 + 1.0),
        (n, CooldownMeaning::TurnOfUseCounts) => 1.0 / n as f64,
    }
}

/// The best sustainable value per turn: fill the turn budget with the
/// actions worth most per action spent, within cooldowns and kind limits.
fn best_per_turn(s: &RuleSystem, ctx: Option<&TurnContext>, actions: &[(&ActionDef, f64)]) -> f64 {
    let Some(ctx) = ctx else { return 0.0 };
    let cost = |a: &ActionDef| s.action_kind(&a.kind).map(|k| k.cost).unwrap_or(1).max(1) as f64;
    let mut ranked: Vec<&(&ActionDef, f64)> = actions.iter().filter(|(_, v)| *v > 0.0).collect();
    ranked.sort_by(|x, y| (y.1 / cost(y.0)).total_cmp(&(x.1 / cost(x.0))));
    let mut budget = ctx.actions_per_turn as f64;
    let mut kind_left: Vec<(String, f64)> = ctx
        .limits
        .iter()
        .map(|l| (l.kind.clone(), l.max_per_turn as f64))
        .collect();
    let mut total = 0.0;
    for (a, value) in ranked {
        let c = cost(a);
        let mut uses = uses_per_turn(s, a).min(budget / c);
        if let Some((_, left)) = kind_left.iter().find(|(k, _)| *k == a.kind) {
            uses = uses.min(*left);
        }
        if uses <= 0.0 {
            continue;
        }
        budget -= uses * c;
        if let Some((_, left)) = kind_left.iter_mut().find(|(k, _)| *k == a.kind) {
            *left -= uses;
        }
        total += uses * value;
    }
    total
}

fn unlocked(c: &ClassDef, level: u32) -> impl Iterator<Item = &ActionDef> {
    c.actions
        .iter()
        .filter(move |a| a.level.unwrap_or(1) <= level)
}

fn class_balance(
    s: &RuleSystem,
    c: &ClassDef,
    ctx: Option<&TurnContext>,
    targets: &[Target],
    xp_target: i32,
    check_difficulty: i32,
    p: &BalanceParams,
) -> ClassBalance {
    let damage_per_turn = (1..=s.max_level())
        .map(|level| LevelDamage {
            level,
            per_target: targets
                .iter()
                .map(|t| {
                    let values: Vec<_> = unlocked(c, level)
                        .map(|a| (a, expected_damage(s, c, a, t.armor_class)))
                        .collect();
                    round2(best_per_turn(s, ctx, &values))
                })
                .collect(),
        })
        .collect();

    // Checks are rolled with the class's best primary ability.
    let check_bonus = c
        .primary_abilities
        .iter()
        .map(|a| modifier(s, c, a))
        .max()
        .unwrap_or(0);
    let xp_per_check = {
        let (hit, crit) = odds(s, check_bonus, check_difficulty);
        hit * s.outcomes.success.grants.xp as f64
            + crit * s.outcomes.critical_success.grants.xp as f64
    };
    let mut total = 0.0;
    let mut sessions = Vec::new();
    let mut max_level_after_session = None;
    for session in 1..=p.sessions {
        let level = level_for(s, total as u32);
        let values: Vec<_> = unlocked(c, level)
            .map(|a| (a, expected_xp(s, c, a, xp_target)))
            .collect();
        let per_turn = best_per_turn(s, ctx, &values);
        let from_fights = per_turn * (p.fights_per_session * p.rounds_per_fight) as f64;
        let from_checks = xp_per_check * p.checks_per_session as f64;
        total += from_fights + from_checks;
        if max_level_after_session.is_none() && level_for(s, total as u32) >= s.max_level() {
            max_level_after_session = Some(session);
        }
        sessions.push(SessionXp {
            session,
            level,
            from_fights: round2(from_fights),
            from_checks: round2(from_checks),
            total_xp: round2(total),
        });
    }
    ClassBalance {
        id: c.id.clone(),
        name: c.name.clone(),
        ability_total: ability_total(c),
        damage_per_turn,
        sessions,
        final_level: level_for(s, total as u32),
        max_level_after_session,
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

impl fmt::Display for BalanceReport {
    /// A plain-text table, for a terminal report.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} — context `{}` ({} actions/turn), {} sessions × {} fights × {} rounds, {} checks/session vs {}",
            self.system,
            self.context,
            self.actions_per_turn,
            self.params.sessions,
            self.params.fights_per_session,
            self.params.rounds_per_fight,
            self.params.checks_per_session,
            self.check_difficulty,
        )?;
        let acs: Vec<String> = self
            .targets
            .iter()
            .map(|t| format!("{} AC {}", t.label, t.armor_class))
            .collect();
        writeln!(f, "targets: {}", acs.join(" · "))?;
        writeln!(f, "ability total median: {}", self.ability_total_median)?;
        for c in &self.classes {
            writeln!(
                f,
                "- {} ({}): abilities {}, level {} after {} sessions{}",
                c.name,
                c.id,
                c.ability_total,
                c.final_level,
                self.params.sessions,
                c.max_level_after_session
                    .map(|n| format!(", max level after session {n}"))
                    .unwrap_or_default(),
            )?;
            for d in &c.damage_per_turn {
                let v: Vec<String> = d.per_target.iter().map(|x| format!("{x:.2}")).collect();
                writeln!(f, "    level {}: damage/turn {}", d.level, v.join(" / "))?;
            }
            let xp: Vec<String> = c
                .sessions
                .iter()
                .map(|x| {
                    format!(
                        "s{} L{} +{:.1}",
                        x.session,
                        x.level,
                        x.from_fights + x.from_checks
                    )
                })
                .collect();
            writeln!(f, "    xp: {}", xp.join(", "))?;
        }
        Ok(())
    }
}
