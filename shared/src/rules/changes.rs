//! What changed between two versions of a rule system, as a player
//! reads it (`player/read-the-rules`). A change ships as a new locked
//! version (`MEMORY.md` §6); players see this list before the next
//! session, never mid-game.
//!
//! Only what the player rules page shows is compared: the roll, the
//! difficulties and the four outcomes, the attack, the turn and its
//! action kinds, cooldowns and durations, conditions, the class action
//! cards, zero hit points, progression and grid combat. Never the
//! adversaries, their tiers, nor any GM note: those are not the
//! players' to read, so their changes are not either.

use serde::Serialize;

use super::check::OutcomeBand;
use super::model::{
    ActionDef, AttackAbility, CooldownMeaning, HouseRule, LongRangeRule, NaturalBand,
    PrecisionRule, RuleSystem, Tag, ZeroHpRule,
};

/// The part of the rules page a change belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Check,
    Stats,
    Difficulties,
    Outcomes,
    Attack,
    Turn,
    ActionKinds,
    Cooldowns,
    Durations,
    Conditions,
    Cards,
    ZeroHp,
    Progression,
    Combat,
    HouseRules,
}

/// What about the subject changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    /// The subject is new in this version.
    Added,
    /// The subject is gone from this version.
    Removed,
    Die,
    Advantage,
    Modifier,
    Formula,
    Value,
    Natural,
    Xp,
    DamageMultiplier,
    Description,
    Ability,
    Precision,
    Cost,
    ActionsPerTurn,
    Limit,
    Meaning,
    ApplicationTurnCounts,
    Effects,
    Damage,
    Heal,
    Cooldown,
    Level,
    Range,
    Kind,
    Rule,
    OutAfterTurns,
    UpgradeEveryXp,
    UpgradePoints,
    LevelXp,
    /// Hit points a level adds (« 1d10 ou 6 + CON »).
    HitPointsPerLevel,
    /// Death saves: the roll to reach, how many successes and failures.
    SaveDifficulty,
    Successes,
    Failures,
    FailuresOnHit,
    CoverHalf,
    CoverThreeQuarters,
    LongRange,
    Flee,
}

/// One side of a change. A `code` is a rule's choice among fixed
/// readings (`skip_next_turns`); the player screen words it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ChangeValue {
    Number(i64),
    Text(String),
    Flag(bool),
    Code(String),
}

/// One thing a player must know changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleChange {
    pub section: Section,
    /// What changed, as the rules name it (« Moyen », « Bretteur ·
    /// Estocade »); empty for a rule that stands alone.
    pub subject: String,
    pub field: Field,
    /// `None` with [`Field::Added`].
    pub from: Option<ChangeValue>,
    /// `None` with [`Field::Removed`].
    pub to: Option<ChangeValue>,
}

fn num(n: impl Into<i64>) -> ChangeValue {
    ChangeValue::Number(n.into())
}

fn text(s: impl ToString) -> ChangeValue {
    ChangeValue::Text(s.to_string())
}

fn code(s: &str) -> ChangeValue {
    ChangeValue::Code(s.to_string())
}

#[derive(Default)]
struct Diff(Vec<RuleChange>);

impl Diff {
    fn push(
        &mut self,
        section: Section,
        subject: &str,
        field: Field,
        from: Option<ChangeValue>,
        to: Option<ChangeValue>,
    ) {
        self.0.push(RuleChange {
            section,
            subject: subject.to_string(),
            field,
            from,
            to,
        });
    }

    /// Records `field` when `a` and `b` differ.
    fn cmp(
        &mut self,
        section: Section,
        subject: &str,
        field: Field,
        a: ChangeValue,
        b: ChangeValue,
    ) {
        if a != b {
            self.push(section, subject, field, Some(a), Some(b));
        }
    }

    /// Pairs two lists by id: what is new, what is gone, and `each` on
    /// what both versions have.
    fn lists<T>(
        &mut self,
        section: Section,
        old: &[T],
        new: &[T],
        id: impl Fn(&T) -> &str,
        name: impl Fn(&T) -> String,
        mut each: impl FnMut(&mut Self, &T, &T),
    ) {
        for o in old {
            match new.iter().find(|n| id(n) == id(o)) {
                Some(n) => each(self, o, n),
                None => self.push(section, &name(o), Field::Removed, None, None),
            }
        }
        for n in new.iter().filter(|n| !old.iter().any(|o| id(o) == id(n))) {
            self.push(section, &name(n), Field::Added, None, None);
        }
    }
}

fn naturals(b: &NaturalBand) -> ChangeValue {
    let faces: Vec<String> = b.natural.iter().map(u32::to_string).collect();
    text(faces.join(", "))
}

fn attack_ability(a: AttackAbility) -> ChangeValue {
    code(match a {
        AttackAbility::FirstPrimary => "first_primary",
        AttackAbility::BestPrimary => "best_primary",
    })
}

fn precision(p: PrecisionRule) -> ChangeValue {
    code(match p {
        PrecisionRule::AddedToAttackRoll => "added_to_attack_roll",
        PrecisionRule::NotApplied => "not_applied",
    })
}

fn cooldown_meaning(m: CooldownMeaning) -> ChangeValue {
    code(match m {
        CooldownMeaning::SkipNextTurns => "skip_next_turns",
        CooldownMeaning::TurnOfUseCounts => "turn_of_use_counts",
    })
}

fn long_range(r: LongRangeRule) -> ChangeValue {
    match r {
        LongRangeRule::Disadvantage => code("disadvantage"),
        LongRangeRule::Modifier(n) => num(n),
    }
}

fn zero_hp_rule(r: &ZeroHpRule) -> ChangeValue {
    code(match r {
        ZeroHpRule::KnockedOut { .. } => "knocked_out",
        ZeroHpRule::DeathSaves { .. } => "death_saves",
    })
}

fn dice_of(a: &ActionDef, pick: impl Fn(&Tag) -> Option<String>) -> ChangeValue {
    text(a.tags.iter().find_map(pick).unwrap_or_default())
}

fn kind_name(s: &RuleSystem, id: &str) -> String {
    s.action_kind(id)
        .map_or_else(|| id.to_string(), |k| k.name.clone())
}

fn cards(d: &mut Diff, old: &RuleSystem, new: &RuleSystem) {
    for oc in &old.classes {
        let Some(nc) = new.classes.iter().find(|c| c.id == oc.id) else {
            continue;
        };
        let subject = |a: &ActionDef| format!("{} · {}", nc.name, a.name);
        d.lists(
            Section::Cards,
            &oc.actions,
            &nc.actions,
            |a| &a.id,
            subject,
            |d, o, n| {
                let s = &subject(n);
                let c = Section::Cards;
                let damage = |t: &Tag| match t {
                    Tag::Damage(x) => Some(x.amount.to_string()),
                    _ => None,
                };
                let heal = |t: &Tag| match t {
                    Tag::Heal(x) => Some(x.amount.to_string()),
                    _ => None,
                };
                d.cmp(c, s, Field::Damage, dice_of(o, damage), dice_of(n, damage));
                d.cmp(c, s, Field::Heal, dice_of(o, heal), dice_of(n, heal));
                d.cmp(c, s, Field::Cooldown, num(o.cooldown()), num(n.cooldown()));
                d.cmp(
                    c,
                    s,
                    Field::Kind,
                    text(kind_name(old, &o.kind)),
                    text(kind_name(new, &n.kind)),
                );
                let cost = |sys: &RuleSystem, a: &ActionDef| {
                    num(sys.action_kind(&a.kind).map_or(0, |k| k.cost))
                };
                d.cmp(c, s, Field::Cost, cost(old, o), cost(new, n));
                d.cmp(
                    c,
                    s,
                    Field::Level,
                    num(o.level.unwrap_or(1)),
                    num(n.level.unwrap_or(1)),
                );
                d.cmp(
                    c,
                    s,
                    Field::Precision,
                    num(o.precision()),
                    num(n.precision()),
                );
                d.cmp(c, s, Field::Range, num(o.reach()), num(n.reach()));
                d.cmp(
                    c,
                    s,
                    Field::Description,
                    text(&o.description),
                    text(&n.description),
                );
            },
        );
    }
    for nc in new.classes.iter().filter(|n| old.class(&n.id).is_none()) {
        d.push(Section::Cards, &nc.name, Field::Added, None, None);
    }
}

/// Everything a player reads on the rules page that differs from `old`
/// to `new`, section by section.
#[must_use]
pub fn rule_changes(old: &RuleSystem, new: &RuleSystem) -> Vec<RuleChange> {
    let mut d = Diff::default();

    // The roll.
    let s = Section::Check;
    d.cmp(
        s,
        "",
        Field::Die,
        text(old.check.dice),
        text(new.check.dice),
    );
    d.cmp(
        s,
        "",
        Field::Advantage,
        ChangeValue::Flag(old.check.advantage),
        ChangeValue::Flag(new.check.advantage),
    );
    d.cmp(
        s,
        "",
        Field::Modifier,
        text(&old.modifier),
        text(&new.modifier),
    );

    let s = Section::Stats;
    for (o, n) in [
        (&old.stats.armor_class, &new.stats.armor_class),
        (&old.stats.hit_points, &new.stats.hit_points),
    ] {
        d.cmp(
            s,
            &n.name,
            Field::Formula,
            text(&o.formula),
            text(&n.formula),
        );
    }
    // A class's own hit points or armour class, where both versions
    // have the class.
    for nc in &new.classes {
        let Some(oc) = old.class(&nc.id) else {
            continue;
        };
        for (stat, of, nf) in [
            (
                &new.stats.hit_points.name,
                old.hit_points_formula(Some(oc)),
                new.hit_points_formula(Some(nc)),
            ),
            (
                &new.stats.armor_class.name,
                old.armor_class_formula(Some(oc)),
                new.armor_class_formula(Some(nc)),
            ),
        ] {
            d.cmp(
                s,
                &format!("{} · {stat}", nc.name),
                Field::Formula,
                text(of),
                text(nf),
            );
        }
    }

    d.lists(
        Section::Difficulties,
        &old.difficulties,
        &new.difficulties,
        |x| &x.id,
        |x| x.name.clone(),
        |d, o, n| {
            d.cmp(
                Section::Difficulties,
                &n.name,
                Field::Value,
                num(o.value),
                num(n.value),
            )
        },
    );

    let s = Section::Outcomes;
    let (oo, no) = (&old.outcomes, &new.outcomes);
    for (o, n) in [
        (&oo.critical_failure, &no.critical_failure),
        (&oo.critical_success, &no.critical_success),
    ] {
        d.cmp(s, &n.name, Field::Natural, naturals(o), naturals(n));
        d.cmp(
            s,
            &n.name,
            Field::DamageMultiplier,
            num(o.damage_multiplier.unwrap_or(1)),
            num(n.damage_multiplier.unwrap_or(1)),
        );
    }
    for band in [
        OutcomeBand::CriticalFailure,
        OutcomeBand::Failure,
        OutcomeBand::Success,
        OutcomeBand::CriticalSuccess,
    ] {
        d.cmp(
            s,
            band.name(new),
            Field::Xp,
            num(band.xp(old)),
            num(band.xp(new)),
        );
    }
    for (o, n) in [
        (
            &oo.critical_failure.description,
            &no.critical_failure.description,
        ),
        (&oo.failure.description, &no.failure.description),
        (&oo.success.description, &no.success.description),
        (
            &oo.critical_success.description,
            &no.critical_success.description,
        ),
    ] {
        if o != n {
            d.push(s, "", Field::Description, Some(text(o)), Some(text(n)));
        }
    }

    let s = Section::Attack;
    d.cmp(
        s,
        "",
        Field::Ability,
        attack_ability(old.attack.ability),
        attack_ability(new.attack.ability),
    );
    d.cmp(
        s,
        "",
        Field::Precision,
        precision(old.attack.precision),
        precision(new.attack.precision),
    );
    let bonus = |sys: &RuleSystem| {
        text(
            sys.attack
                .bonus
                .as_ref()
                .map(|b| format!("{} : {}", b.name, b.formula))
                .unwrap_or_default(),
        )
    };
    d.cmp(s, "", Field::Formula, bonus(old), bonus(new));

    d.lists(
        Section::Turn,
        &old.turn_contexts,
        &new.turn_contexts,
        |x| &x.id,
        |x| x.name.clone(),
        |d, o, n| {
            let s = Section::Turn;
            d.cmp(
                s,
                &n.name,
                Field::ActionsPerTurn,
                num(o.actions_per_turn),
                num(n.actions_per_turn),
            );
            // No limit on a kind reads as an empty value.
            let limit = |c: &super::model::TurnContext, kind: &str| {
                c.limits
                    .iter()
                    .find(|l| l.kind == kind)
                    .map_or_else(|| text(""), |l| num(l.max_per_turn))
            };
            let kinds: std::collections::BTreeSet<&str> = o
                .limits
                .iter()
                .chain(&n.limits)
                .map(|l| l.kind.as_str())
                .collect();
            for k in kinds {
                let subject = format!("{} · {}", n.name, kind_name(new, k));
                d.cmp(s, &subject, Field::Limit, limit(o, k), limit(n, k));
            }
        },
    );

    d.lists(
        Section::ActionKinds,
        &old.action_kinds,
        &new.action_kinds,
        |x| &x.id,
        |x| x.name.clone(),
        |d, o, n| {
            d.cmp(
                Section::ActionKinds,
                &n.name,
                Field::Cost,
                num(o.cost),
                num(n.cost),
            )
        },
    );

    d.cmp(
        Section::Cooldowns,
        "",
        Field::Meaning,
        cooldown_meaning(old.cooldowns.meaning),
        cooldown_meaning(new.cooldowns.meaning),
    );
    d.cmp(
        Section::Durations,
        "",
        Field::ApplicationTurnCounts,
        ChangeValue::Flag(old.durations.application_turn_counts),
        ChangeValue::Flag(new.durations.application_turn_counts),
    );

    d.lists(
        Section::Conditions,
        &old.conditions,
        &new.conditions,
        |x| &x.id,
        |x| x.name.clone(),
        |d, o, n| {
            let s = Section::Conditions;
            if o.effects != n.effects {
                d.push(
                    s,
                    &n.name,
                    Field::Effects,
                    Some(text(&o.description)),
                    Some(text(&n.description)),
                );
            } else {
                d.cmp(
                    s,
                    &n.name,
                    Field::Description,
                    text(&o.description),
                    text(&n.description),
                );
            }
        },
    );

    cards(&mut d, old, new);

    let s = Section::ZeroHp;
    d.cmp(
        s,
        "",
        Field::Rule,
        zero_hp_rule(&old.zero_hp),
        zero_hp_rule(&new.zero_hp),
    );
    if let (
        ZeroHpRule::KnockedOut {
            out_after_turns: a, ..
        },
        ZeroHpRule::KnockedOut {
            out_after_turns: b, ..
        },
    ) = (&old.zero_hp, &new.zero_hp)
    {
        d.cmp(s, "", Field::OutAfterTurns, num(*a), num(*b));
    }
    if let (
        ZeroHpRule::DeathSaves {
            difficulty: od,
            successes: os,
            failures: of,
            failures_on_hit: oh,
            ..
        },
        ZeroHpRule::DeathSaves {
            difficulty: nd,
            successes: ns,
            failures: nf,
            failures_on_hit: nh,
            ..
        },
    ) = (&old.zero_hp, &new.zero_hp)
    {
        d.cmp(s, "", Field::SaveDifficulty, num(*od), num(*nd));
        d.cmp(s, "", Field::Successes, num(*os), num(*ns));
        d.cmp(s, "", Field::Failures, num(*of), num(*nf));
        d.cmp(s, "", Field::FailuresOnHit, num(*oh), num(*nh));
    }

    let s = Section::Progression;
    let (op, np) = (&old.progression, &new.progression);
    d.cmp(
        s,
        "",
        Field::UpgradeEveryXp,
        num(op.upgrade_every_xp),
        num(np.upgrade_every_xp),
    );
    d.cmp(
        s,
        "",
        Field::UpgradePoints,
        num(op.upgrade_points),
        num(np.upgrade_points),
    );
    let level_xp = |p: &super::model::Progression, level: u32| {
        p.levels
            .iter()
            .find(|l| l.level == level)
            .map_or_else(|| text(""), |l| num(l.xp))
    };
    let per_level = |p: &super::model::Progression| {
        text(
            p.hit_points_per_level
                .as_ref()
                .map_or_else(String::new, |h| {
                    let plus = h
                        .ability
                        .as_ref()
                        .map_or_else(String::new, |a| format!(" + {a}"));
                    format!("{}{plus} ou {}{plus}", h.dice, h.average)
                }),
        )
    };
    d.cmp(
        s,
        "",
        Field::HitPointsPerLevel,
        per_level(op),
        per_level(np),
    );
    let levels: std::collections::BTreeSet<u32> = op
        .levels
        .iter()
        .chain(&np.levels)
        .map(|l| l.level)
        .collect();
    for level in levels {
        d.cmp(
            s,
            &level.to_string(),
            Field::LevelXp,
            level_xp(op, level),
            level_xp(np, level),
        );
    }

    let s = Section::Combat;
    let (oc, nc) = (&old.combat, &new.combat);
    d.cmp(
        s,
        "",
        Field::CoverHalf,
        num(oc.cover.half),
        num(nc.cover.half),
    );
    d.cmp(
        s,
        "",
        Field::CoverThreeQuarters,
        num(oc.cover.three_quarters),
        num(nc.cover.three_quarters),
    );
    d.cmp(
        s,
        "",
        Field::LongRange,
        long_range(oc.long_range),
        long_range(nc.long_range),
    );
    let flee = |sys: &RuleSystem| {
        text(
            sys.combat
                .flee
                .as_ref()
                .map(|f| kind_name(sys, &f.kind))
                .unwrap_or_default(),
        )
    };
    d.cmp(s, "", Field::Flee, flee(old), flee(new));

    // Only the house rules players may read: one that shows its effect
    // alone is not named to them, nor are its changes.
    let shown = |sys: &RuleSystem| -> Vec<HouseRule> {
        sys.house_rules
            .iter()
            .filter(|h| h.shown_to_players())
            .cloned()
            .collect()
    };
    let judged = |h: &HouseRule| {
        code(if h.formal.is_some() {
            "house_judged"
        } else {
            "house_by_gm"
        })
    };
    d.lists(
        Section::HouseRules,
        &shown(old),
        &shown(new),
        |x| &x.id,
        |x| x.name.clone(),
        |d, o, n| {
            if o.text != n.text || o.name != n.name {
                d.push(
                    Section::HouseRules,
                    &n.name,
                    Field::Description,
                    Some(text(&o.text)),
                    Some(text(&n.text)),
                );
            }
            match (&o.formal, &n.formal) {
                (Some(a), Some(b)) if a != b => d.push(
                    Section::HouseRules,
                    &n.name,
                    Field::Rule,
                    Some(judged(o)),
                    Some(code("house_judged_new")),
                ),
                (a, b) if a.is_some() != b.is_some() => d.push(
                    Section::HouseRules,
                    &n.name,
                    Field::Rule,
                    Some(judged(o)),
                    Some(judged(n)),
                ),
                _ => {}
            }
        },
    );

    d.0
}

#[cfg(test)]
mod tests {
    use serde_yaml_ng::{Mapping, Value};

    use super::*;
    use crate::rules::variant::{Variant, build_system};

    const CORSAIRES: &str = include_str!("../../../content/rules/corsaires/v1.yaml");

    fn edited(set: &[(&str, Value)]) -> RuleSystem {
        let mut map = Mapping::new();
        for (k, v) in set {
            map.insert(Value::from(*k), v.clone());
        }
        let v = Variant {
            id: "v2".into(),
            name: "v2".into(),
            description: String::new(),
            set: map,
        };
        build_system(CORSAIRES, &[], Some(&v)).unwrap()
    }

    fn find<'a>(changes: &'a [RuleChange], field: Field, subject: &str) -> Option<&'a RuleChange> {
        changes
            .iter()
            .find(|c| c.field == field && c.subject == subject)
    }

    #[test]
    fn the_same_version_changes_nothing() {
        let s = edited(&[]);
        assert_eq!(rule_changes(&s, &s), []);
    }

    #[test]
    fn a_rule_change_is_named_with_both_sides() {
        let old = edited(&[]);
        let new = edited(&[
            ("attack.precision", Value::from("added_to_attack_roll")),
            ("cooldowns.meaning", Value::from("turn_of_use_counts")),
            ("difficulties[moyen].value", Value::from(12)),
            (
                "classes[bretteur].actions[estocade].tags[0].damage.amount",
                Value::from(4),
            ),
            ("action_kinds[fuite].cost", Value::from(1)),
        ]);
        let changes = rule_changes(&old, &new);

        let c = find(&changes, Field::Precision, "").unwrap();
        assert_eq!(c.section, Section::Attack);
        assert_eq!(c.from, Some(code("not_applied")));
        assert_eq!(c.to, Some(code("added_to_attack_roll")));
        let c = find(&changes, Field::Meaning, "").unwrap();
        assert_eq!(c.to, Some(code("turn_of_use_counts")));
        let c = find(&changes, Field::Value, "Moyen").unwrap();
        assert_eq!(
            (c.from.clone(), c.to.clone()),
            (Some(num(10)), Some(num(12)))
        );
        let c = find(&changes, Field::Damage, "Bretteur · Estocade").unwrap();
        assert_eq!(
            (c.from.clone(), c.to.clone()),
            (Some(text("3")), Some(text("4")))
        );
        let c = find(&changes, Field::Cost, "Fuir").unwrap();
        assert_eq!(c.section, Section::ActionKinds);
        assert_eq!(changes.len(), 5, "{changes:#?}");
    }

    #[test]
    fn what_comes_and_goes_is_listed_and_adversaries_are_not() {
        let old = edited(&[]);
        let mut new = edited(&[
            ("adversaries[gueule_rouge].armor_class", Value::from(18)),
            ("adversary_tiers[boss].armor_class", Value::from(17)),
        ]);
        new.difficulties.retain(|d| d.id != "tres_difficile");
        new.conditions[0].effects.clear();
        let changes = rule_changes(&old, &new);
        assert!(find(&changes, Field::Removed, "Très difficile").is_some());
        let c = find(&changes, Field::Effects, "Inconscient").unwrap();
        assert_eq!(c.section, Section::Conditions);
        assert_eq!(
            changes.len(),
            2,
            "GM-only stat blocks leaked into {changes:#?}"
        );
    }
}
