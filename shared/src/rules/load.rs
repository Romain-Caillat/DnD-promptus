//! Loading a rule system: YAML → typed data → validation.
//!
//! Validation refuses what the engine cannot run (a reference to an
//! ability, condition or kind that does not exist; a formula that does
//! not evaluate; levels out of order). It does **not** judge whether the
//! rules are good — that is `engine/lint-rule-system`, which flags and
//! never blocks. Errors carry a stable code, a path into the file and an
//! English detail; the UI translates the code.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::formula::{Formula, FormulaEnv};
use super::model::*;

/// What went wrong, as a stable machine code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorCode {
    Syntax,
    EmptyField,
    DuplicateId,
    UnknownAbility,
    UnknownCondition,
    UnknownActionKind,
    UnknownDifficulty,
    UnknownSituation,
    UnknownItem,
    UnknownTier,
    InvalidFormula,
    InvalidDice,
    InvalidValue,
    MissingAbilityScore,
    AdvantageNotInSystem,
    InvalidLevels,
    LevelOutOfRange,
    InvalidTargeting,
    InvalidTag,
    InvalidApply,
    MissingAttackAbility,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::EmptyField => "empty_field",
            Self::DuplicateId => "duplicate_id",
            Self::UnknownAbility => "unknown_ability",
            Self::UnknownCondition => "unknown_condition",
            Self::UnknownActionKind => "unknown_action_kind",
            Self::UnknownDifficulty => "unknown_difficulty",
            Self::UnknownSituation => "unknown_situation",
            Self::UnknownItem => "unknown_item",
            Self::UnknownTier => "unknown_tier",
            Self::InvalidFormula => "invalid_formula",
            Self::InvalidDice => "invalid_dice",
            Self::InvalidValue => "invalid_value",
            Self::MissingAbilityScore => "missing_ability_score",
            Self::AdvantageNotInSystem => "advantage_not_in_system",
            Self::InvalidLevels => "invalid_levels",
            Self::LevelOutOfRange => "level_out_of_range",
            Self::InvalidTargeting => "invalid_targeting",
            Self::InvalidTag => "invalid_tag",
            Self::InvalidApply => "invalid_apply",
            Self::MissingAttackAbility => "missing_attack_ability",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleError {
    pub code: ErrorCode,
    /// Where in the file, as `classes[bretteur].actions[estocade].tags[0]`.
    pub path: String,
    pub detail: String,
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}: {}",
            self.code.as_str(),
            self.path,
            self.detail
        )
    }
}

#[derive(Debug, Clone)]
pub struct LoadError(pub Vec<RuleError>);

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{e}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LoadError {}

impl LoadError {
    pub fn codes(&self) -> Vec<ErrorCode> {
        self.0.iter().map(|e| e.code).collect()
    }
}

impl RuleSystem {
    /// Parses and validates a rule system written in YAML.
    pub fn from_yaml(text: &str) -> Result<Self, LoadError> {
        // Variants with data are written as one-key maps (`damage: {…}`),
        // not YAML tags (`!damage`): read every enum that way.
        let de = serde_yaml_ng::Deserializer::from_str(text);
        let parsed: Result<RuleSystem, serde_yaml_ng::Error> =
            serde_yaml_ng::with::singleton_map_recursive::deserialize(de);
        let system = parsed.map_err(|e| {
            let path = e
                .location()
                .map(|l| format!("line {}, column {}", l.line(), l.column()))
                .unwrap_or_else(|| "file".into());
            LoadError(vec![RuleError {
                code: ErrorCode::Syntax,
                path,
                detail: e.to_string(),
            }])
        })?;
        let errors = validate(&system);
        if errors.is_empty() {
            Ok(system)
        } else {
            Err(LoadError(errors))
        }
    }
}

struct V<'a> {
    s: &'a RuleSystem,
    errors: Vec<RuleError>,
    abilities: BTreeSet<&'a str>,
}

impl<'a> V<'a> {
    fn err(&mut self, code: ErrorCode, path: impl Into<String>, detail: impl Into<String>) {
        self.errors.push(RuleError {
            code,
            path: path.into(),
            detail: detail.into(),
        });
    }

    fn non_empty(&mut self, value: &str, path: &str) {
        if value.trim().is_empty() {
            self.err(ErrorCode::EmptyField, path, "must not be empty");
        }
    }

    fn unique<'b>(&mut self, ids: impl IntoIterator<Item = &'b str>, path: &str) {
        let mut seen = BTreeSet::new();
        for id in ids {
            if id.trim().is_empty() {
                self.err(ErrorCode::EmptyField, path, "an id is empty");
            } else if !seen.insert(id) {
                self.err(
                    ErrorCode::DuplicateId,
                    format!("{path}[{id}]"),
                    format!("`{id}` is defined twice"),
                );
            }
        }
    }

    fn ability(&mut self, id: &str, path: &str) {
        if !self.abilities.contains(id) {
            self.err(
                ErrorCode::UnknownAbility,
                path,
                format!("no ability `{id}` in this system"),
            );
        }
    }

    fn condition(&mut self, id: &str, path: &str) {
        if self.s.condition(id).is_none() {
            self.err(
                ErrorCode::UnknownCondition,
                path,
                format!("no condition `{id}` in this system"),
            );
        }
    }

    fn score_table(&mut self, scores: &BTreeMap<String, i32>, path: &str) {
        for a in &self.s.abilities {
            if !scores.contains_key(&a.id) {
                self.err(
                    ErrorCode::MissingAbilityScore,
                    path,
                    format!("no score for `{}`", a.id),
                );
            }
        }
        for id in scores.keys() {
            self.ability(id, &format!("{path}.{id}"));
        }
    }

    /// Checks names a formula reads and that it evaluates on a sample.
    fn formula(&mut self, f: &Formula, vars: &[&str], path: &str) {
        let refs = f.references();
        for v in &refs.vars {
            if !vars.contains(&v.as_str()) {
                self.err(
                    ErrorCode::InvalidFormula,
                    path,
                    format!(
                        "`{}` reads `{v}`, which is not available here (allowed: {})",
                        f,
                        vars.join(", ")
                    ),
                );
            }
        }
        for a in &refs.abilities {
            self.ability(a, path);
        }
        struct Sample;
        impl FormulaEnv for Sample {
            fn var(&self, _: &str) -> Option<i64> {
                Some(10)
            }
            fn ability_mod(&self, _: &str) -> Option<i64> {
                Some(0)
            }
            fn ability_score(&self, _: &str) -> Option<i64> {
                Some(10)
            }
        }
        if let Err(e) = f.eval(&Sample) {
            self.err(ErrorCode::InvalidFormula, path, format!("`{f}`: {e}"));
        }
    }

    fn effects(&mut self, effects: &[ConditionEffect], path: &str) {
        for (i, e) in effects.iter().enumerate() {
            let p = format!("{path}[{i}]");
            match e {
                ConditionEffect::Advantage { .. }
                | ConditionEffect::Disadvantage { .. }
                | ConditionEffect::AdvantageAgainst
                | ConditionEffect::DisadvantageAgainst
                    if !self.s.check.advantage =>
                {
                    self.err(
                        ErrorCode::AdvantageNotInSystem,
                        p,
                        "this system has no advantage/disadvantage (check.advantage is false)",
                    );
                }
                ConditionEffect::AbilityBonus {
                    ability: Some(a), ..
                } => self.ability(a, &p),
                ConditionEffect::OnHit(spec) => self.apply(spec, &p, None),
                ConditionEffect::Narrative(t) => self.non_empty(t, &p),
                _ => {}
            }
        }
    }

    fn apply(&mut self, spec: &ApplySpec, path: &str, targeting: Option<Targeting>) {
        match (&spec.condition, spec.effects.is_empty()) {
            (Some(c), true) => {
                self.condition(c, &format!("{path}.condition"));
                if spec.kind.is_some() {
                    self.err(
                        ErrorCode::InvalidApply,
                        path,
                        "`kind` belongs to the named condition, not here",
                    );
                }
            }
            (None, false) => {
                self.effects(&spec.effects, &format!("{path}.effects"));
                if spec.kind.is_none() {
                    self.err(
                        ErrorCode::InvalidApply,
                        path,
                        "inline effects need a `kind` (boon or bane)",
                    );
                }
            }
            (Some(_), false) => self.err(
                ErrorCode::InvalidApply,
                path,
                "give either `condition` or `effects`, not both",
            ),
            (None, true) => self.err(
                ErrorCode::InvalidApply,
                path,
                "give a `condition` or `effects`",
            ),
        }
        if spec.turns == 0 {
            self.err(
                ErrorCode::InvalidValue,
                format!("{path}.turns"),
                "a duration lasts at least 1 turn",
            );
        }
        if let Some(save) = &spec.save {
            self.ability(&save.ability, &format!("{path}.save.ability"));
            if let Some(d) = &save.difficulty
                && self.s.difficulty(d).is_none()
            {
                self.err(
                    ErrorCode::UnknownDifficulty,
                    format!("{path}.save.difficulty"),
                    format!("no difficulty `{d}`"),
                );
            }
        }
        if spec.to == Recipient::Targets && targeting == Some(Targeting::Myself) {
            self.err(
                ErrorCode::InvalidApply,
                path,
                "a self-targeted action applies `to: self`",
            );
        }
    }

    fn tags(&mut self, tags: &[Tag], path: &str, targeting: Targeting, in_choice: bool) {
        let (mut cooldowns, mut precisions) = (0, 0);
        for (i, t) in tags.iter().enumerate() {
            let p = format!("{path}[{i}]");
            match t {
                Tag::Damage(d) => {
                    if d.amount.range().0 < 0 {
                        self.err(
                            ErrorCode::InvalidDice,
                            p,
                            format!("damage `{}` can be negative", d.amount),
                        );
                    }
                }
                Tag::Heal(h) => {
                    if h.amount.range().0 < 0 {
                        self.err(
                            ErrorCode::InvalidDice,
                            p,
                            format!("healing `{}` can be negative", h.amount),
                        );
                    }
                }
                Tag::Buff(a) | Tag::Control(a) => self.apply(a, &p, Some(targeting)),
                Tag::Precision(_) => precisions += 1,
                Tag::Cooldown(_) => cooldowns += 1,
                Tag::Situational(b) => {
                    if self.s.situation(&b.situation).is_none() {
                        self.err(
                            ErrorCode::UnknownSituation,
                            format!("{p}.situation"),
                            format!("no situation `{}` in this system", b.situation),
                        );
                    }
                }
                Tag::Choice(options) => {
                    if in_choice {
                        self.err(
                            ErrorCode::InvalidTag,
                            &p,
                            "a choice cannot contain another choice",
                        );
                    }
                    if options.len() < 2 {
                        self.err(
                            ErrorCode::InvalidTag,
                            &p,
                            "a choice offers at least two options",
                        );
                    }
                    if options
                        .iter()
                        .any(|o| matches!(o, Tag::Cooldown(_) | Tag::Precision(_)))
                    {
                        self.err(
                            ErrorCode::InvalidTag,
                            &p,
                            "cooldown and precision belong to the action, not to an option",
                        );
                    }
                    self.tags(options, &p, targeting, true);
                }
                Tag::Note(n) => self.non_empty(n, &p),
            }
        }
        if cooldowns > 1 || precisions > 1 {
            self.err(
                ErrorCode::InvalidTag,
                path,
                "an action has at most one cooldown and one precision tag",
            );
        }
    }

    fn action(&mut self, a: &ActionDef, path: &str, has_class: bool, levelled: bool) {
        self.non_empty(&a.name, &format!("{path}.name"));
        if self.s.action_kind(&a.kind).is_none() {
            self.err(
                ErrorCode::UnknownActionKind,
                format!("{path}.kind"),
                format!("no action kind `{}`", a.kind),
            );
        }
        if let Some(level) = a.level {
            if !levelled {
                self.err(
                    ErrorCode::LevelOutOfRange,
                    format!("{path}.level"),
                    "only class actions are gated by level",
                );
            } else if level < 1 || level > self.s.max_level() {
                self.err(
                    ErrorCode::LevelOutOfRange,
                    format!("{path}.level"),
                    format!("level {level} is outside 1..={}", self.s.max_level()),
                );
            }
        }
        if let Some(ab) = &a.ability {
            self.ability(ab, &format!("{path}.ability"));
        }
        match &a.roll {
            RollSpec::Attack if a.ability.is_none() && !has_class => self.err(
                ErrorCode::MissingAttackAbility,
                format!("{path}.ability"),
                "an attack outside a class must name the ability it rolls",
            ),
            RollSpec::Contest { actor, target } => {
                self.ability(actor, &format!("{path}.roll.contest.actor"));
                self.ability(target, &format!("{path}.roll.contest.target"));
            }
            _ => {}
        }
        let rolls_against_foes = !matches!(a.roll, RollSpec::None);
        let friendly = matches!(
            a.target,
            Targeting::Myself | Targeting::Ally | Targeting::AllyOrSelf | Targeting::AllAllies
        );
        if rolls_against_foes && friendly {
            self.err(
                ErrorCode::InvalidTargeting,
                format!("{path}.roll"),
                "attacks and contests target enemies",
            );
        }
        if a.range == Some(0) {
            self.err(
                ErrorCode::InvalidValue,
                format!("{path}.range"),
                "a range is at least 1 cell",
            );
        }
        if let Some(long) = a.long_range
            && long <= a.reach()
        {
            self.err(
                ErrorCode::InvalidValue,
                format!("{path}.long_range"),
                format!("{long} must exceed the range ({})", a.reach()),
            );
        }
        if a.target == Targeting::Enemies && a.area.is_none() {
            self.err(
                ErrorCode::InvalidTargeting,
                format!("{path}.area"),
                "several enemies means an area shape",
            );
        }
        self.tags(&a.tags, &format!("{path}.tags"), a.target, false);
    }
}

fn validate(s: &RuleSystem) -> Vec<RuleError> {
    let mut v = V {
        s,
        errors: Vec::new(),
        abilities: s.abilities.iter().map(|a| a.id.as_str()).collect(),
    };
    v.non_empty(&s.id, "id");
    v.non_empty(&s.name, "name");
    if s.version == 0 {
        v.err(ErrorCode::InvalidValue, "version", "versions start at 1");
    }
    if s.abilities.is_empty() {
        v.err(
            ErrorCode::EmptyField,
            "abilities",
            "a system needs at least one ability",
        );
    }
    v.unique(s.abilities.iter().map(|a| a.id.as_str()), "abilities");
    v.formula(&s.modifier, &["score"], "modifier");
    v.formula(
        &s.stats.armor_class.formula,
        &["level"],
        "stats.armor_class.formula",
    );
    v.formula(
        &s.stats.hit_points.formula,
        &["level"],
        "stats.hit_points.formula",
    );
    v.formula(&s.initiative.bonus, &["level"], "initiative.bonus");

    let die = s.check.dice;
    if die.count != 1 || die.modifier != 0 {
        v.err(
            ErrorCode::InvalidDice,
            "check.dice",
            format!("a check rolls one die, as 1d20 (got `{die}`)"),
        );
    }
    for (band, naturals) in [
        ("critical_failure", &s.outcomes.critical_failure.natural),
        ("critical_success", &s.outcomes.critical_success.natural),
    ] {
        for n in naturals {
            if *n < 1 || *n > die.faces {
                v.err(
                    ErrorCode::InvalidValue,
                    format!("outcomes.{band}.natural"),
                    format!("{n} cannot come up on a d{}", die.faces),
                );
            }
        }
    }
    let overlap: Vec<_> = s
        .outcomes
        .critical_failure
        .natural
        .iter()
        .filter(|n| s.outcomes.critical_success.natural.contains(n))
        .collect();
    if !overlap.is_empty() {
        v.err(
            ErrorCode::InvalidValue,
            "outcomes",
            format!("natural {overlap:?} is both a critical failure and success"),
        );
    }
    if let Some(m) = s.outcomes.critical_success.damage_multiplier
        && m < 1
    {
        v.err(
            ErrorCode::InvalidValue,
            "outcomes.critical_success.damage_multiplier",
            "at least 1",
        );
    }

    v.unique(s.difficulties.iter().map(|d| d.id.as_str()), "difficulties");
    v.unique(s.action_kinds.iter().map(|k| k.id.as_str()), "action_kinds");
    for k in &s.action_kinds {
        if k.cost == 0 {
            v.err(
                ErrorCode::InvalidValue,
                format!("action_kinds[{}].cost", k.id),
                "an action costs at least 1",
            );
        }
    }
    if s.turn_contexts.is_empty() {
        v.err(
            ErrorCode::EmptyField,
            "turn_contexts",
            "at least one turn context",
        );
    }
    v.unique(
        s.turn_contexts.iter().map(|c| c.id.as_str()),
        "turn_contexts",
    );
    for c in &s.turn_contexts {
        if c.actions_per_turn == 0 {
            v.err(
                ErrorCode::InvalidValue,
                format!("turn_contexts[{}].actions_per_turn", c.id),
                "at least 1",
            );
        }
        for l in &c.limits {
            if s.action_kind(&l.kind).is_none() {
                v.err(
                    ErrorCode::UnknownActionKind,
                    format!("turn_contexts[{}].limits", c.id),
                    format!("no action kind `{}`", l.kind),
                );
            }
        }
    }

    let levels = &s.progression.levels;
    let ordered = levels.first().is_some_and(|l| l.level == 1 && l.xp == 0)
        && levels
            .windows(2)
            .all(|w| w[1].level == w[0].level + 1 && w[1].xp > w[0].xp);
    if !ordered {
        v.err(
            ErrorCode::InvalidLevels,
            "progression.levels",
            "levels start at level 1 with 0 XP and go up one level at a time with increasing XP",
        );
    }
    if let Some(hp) = &s.progression.hit_points_per_level {
        if hp.dice.is_flat() {
            v.err(
                ErrorCode::InvalidDice,
                "progression.hit_points_per_level.dice",
                "a die to roll (1d10), not a flat number",
            );
        }
        if let Some(bonus) = &hp.bonus {
            v.formula(bonus, &["level"], "progression.hit_points_per_level.bonus");
        }
    }
    for c in &s.classes {
        if c.hit_dice.is_some_and(|d| d.is_flat()) {
            v.err(
                ErrorCode::InvalidDice,
                format!("classes[{}].hit_dice", c.id),
                "a die to roll (1d10), not a flat number",
            );
        }
    }
    if s.progression.upgrade_every_xp == 0 {
        v.err(
            ErrorCode::InvalidValue,
            "progression.upgrade_every_xp",
            "at least 1",
        );
    }

    v.unique(s.conditions.iter().map(|c| c.id.as_str()), "conditions");
    for c in &s.conditions {
        v.non_empty(&c.name, &format!("conditions[{}].name", c.id));
        v.effects(&c.effects, &format!("conditions[{}].effects", c.id));
    }
    match &s.zero_hp {
        ZeroHpRule::KnockedOut {
            condition,
            out_condition,
            out_after_turns,
            ..
        } => {
            v.condition(condition, "zero_hp.condition");
            v.condition(out_condition, "zero_hp.out_condition");
            if *out_after_turns == 0 {
                v.err(
                    ErrorCode::InvalidValue,
                    "zero_hp.out_after_turns",
                    "at least 1",
                );
            }
        }
        ZeroHpRule::DeathSaves {
            condition,
            successes,
            failures,
            ..
        } => {
            v.condition(condition, "zero_hp.condition");
            if *successes == 0 || *failures == 0 {
                v.err(
                    ErrorCode::InvalidValue,
                    "zero_hp",
                    "death saves need at least one success and one failure",
                );
            }
        }
    }

    let combat = &s.combat;
    let kinds = combat
        .move_kind
        .iter()
        .map(|k| ("combat.move_kind", k))
        .chain(combat.flee.iter().map(|f| ("combat.flee.kind", &f.kind)));
    for (path, kind) in kinds {
        if s.action_kind(kind).is_none() {
            v.err(
                ErrorCode::UnknownActionKind,
                path,
                format!("no action kind `{kind}`"),
            );
        }
    }
    if let Some(ability) = combat.flee.as_ref().and_then(|f| f.ability.as_ref()) {
        v.ability(ability, "combat.flee.ability");
    }
    if combat.long_range == LongRangeRule::Disadvantage && !s.check.advantage {
        v.err(
            ErrorCode::AdvantageNotInSystem,
            "combat.long_range",
            "long range cannot impose disadvantage in a system without it",
        );
    }

    v.unique(s.situations.iter().map(|x| x.id.as_str()), "situations");
    v.unique(s.house_rules.iter().map(|x| x.id.as_str()), "house_rules");
    for (i, h) in s.house_rules.iter().enumerate() {
        v.non_empty(&h.name, &format!("house_rules[{i}].name"));
        v.non_empty(&h.text, &format!("house_rules[{i}].text"));
    }
    v.unique(s.resources.iter().map(|x| x.id.as_str()), "resources");
    v.unique(s.peoples.iter().map(|x| x.id.as_str()), "peoples");
    v.unique(s.items.iter().map(|x| x.id.as_str()), "items");
    v.unique(
        s.adversary_tiers.iter().map(|x| x.id.as_str()),
        "adversary_tiers",
    );
    v.unique(s.adversaries.iter().map(|x| x.id.as_str()), "adversaries");
    let mut scales = BTreeSet::new();
    for m in &s.movement {
        if !scales.insert(m.scale) {
            v.err(
                ErrorCode::DuplicateId,
                "movement",
                format!("{:?} is defined twice", m.scale),
            );
        }
    }

    // Action ids key cooldowns on a sheet: one id, one action, system-wide.
    let mut action_ids: Vec<&str> = Vec::new();

    if s.classes.is_empty() {
        v.err(ErrorCode::EmptyField, "classes", "at least one class");
    }
    v.unique(s.classes.iter().map(|c| c.id.as_str()), "classes");
    for c in &s.classes {
        let path = format!("classes[{}]", c.id);
        v.non_empty(&c.name, &format!("{path}.name"));
        if c.primary_abilities.is_empty() {
            v.err(
                ErrorCode::EmptyField,
                format!("{path}.primary_abilities"),
                "at least one primary ability",
            );
        }
        for a in c.primary_abilities.iter().chain(&c.secondary_abilities) {
            v.ability(a, &format!("{path}.primary_abilities"));
        }
        v.score_table(&c.abilities, &format!("{path}.abilities"));
        for it in &c.items {
            if s.item(&it.item).is_none() {
                v.err(
                    ErrorCode::UnknownItem,
                    format!("{path}.items"),
                    format!("no item `{}`", it.item),
                );
            }
        }
        for a in &c.actions {
            v.action(a, &format!("{path}.actions[{}]", a.id), true, true);
            action_ids.push(&a.id);
        }
    }
    for it in &s.items {
        if let Some(a) = &it.action {
            v.action(a, &format!("items[{}].action", it.id), false, false);
            action_ids.push(&a.id);
        }
    }
    for t in &s.adversary_tiers {
        v.non_empty(&t.name, &format!("adversary_tiers[{}].name", t.id));
    }
    for a in &s.adversaries {
        let path = format!("adversaries[{}]", a.id);
        if let Some(t) = &a.tier
            && !s.adversary_tiers.iter().any(|x| &x.id == t)
        {
            v.err(
                ErrorCode::UnknownTier,
                format!("{path}.tier"),
                format!("no adversary tier `{t}`"),
            );
        }
        v.score_table(&a.abilities, &format!("{path}.abilities"));
        if let Some(reason) = &a.exception {
            v.non_empty(reason, &format!("{path}.exception"));
        }
        if a.hit_points <= 0 {
            v.err(
                ErrorCode::InvalidValue,
                format!("{path}.hit_points"),
                "at least 1",
            );
        }
        for act in &a.actions {
            v.action(act, &format!("{path}.actions[{}]", act.id), false, false);
            action_ids.push(&act.id);
        }
    }
    v.unique(action_ids, "actions");
    v.errors
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI: &str = r#"
id: mini
version: 1
name: Mini
abilities: [{ id: FOR, name: Force }, { id: DEX, name: Dextérité }]
modifier: "(score - 10) / 2"
stats:
  armor_class: { name: Classe d'armure, abbr: CA, formula: "10 + mod(DEX)" }
  hit_points: { name: Points de vie, abbr: PV, formula: 10 }
check: { dice: 1d20, advantage: false }
difficulties: [{ id: moyen, name: Moyen, value: 10 }]
outcomes:
  critical_failure: { name: Échec critique, natural: [1] }
  failure: { name: Échec }
  success: { name: Réussite, grants: { xp: 1 } }
  critical_success: { name: Réussite critique, natural: [20], grants: { xp: 1 }, damage_multiplier: 2 }
attack: { ability: first_primary, precision: added_to_attack_roll }
initiative: { dice: 1d20, bonus: "mod(DEX)", ties: party_first }
action_kinds: [{ id: attaque, name: Attaquer, cost: 1 }]
turn_contexts: [{ id: sol, name: Au sol, actions_per_turn: 2 }]
cooldowns: { meaning: skip_next_turns }
durations: { application_turn_counts: false }
progression: { upgrade_every_xp: 5, upgrade_points: 1, levels: [{ level: 1, xp: 0 }, { level: 2, xp: 5 }] }
zero_hp: { rule: knocked_out, condition: ko, out_after_turns: 3, out_condition: ko }
creation: { abilities: from_class, free_action_slots: 1 }
conditions:
  - { id: ko, name: KO, kind: bane, effects: [skips_turn] }
classes:
  - id: brute
    name: Brute
    primary_abilities: [FOR]
    abilities: { FOR: 14, DEX: 10 }
    actions:
      - id: coup
        name: Coup
        kind: attaque
        level: 1
        target: enemy
        roll: attack
        tags: [{ damage: { amount: 3 } }, { cooldown: 1 }]
"#;

    fn load_with(from: &str, to: &str) -> Result<RuleSystem, LoadError> {
        assert!(MINI.contains(from), "test edit `{from}` does not apply");
        RuleSystem::from_yaml(&MINI.replacen(from, to, 1))
    }

    #[test]
    fn a_minimal_system_loads() {
        let s = RuleSystem::from_yaml(MINI).unwrap();
        assert_eq!(s.classes[0].actions[0].cooldown(), 1);
    }

    #[test]
    fn a_typo_in_a_field_is_a_syntax_error_with_its_line() {
        let err = load_with("actions_per_turn: 2", "actions_per_tour: 2").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::Syntax]);
        assert!(err.0[0].path.starts_with("line "), "{}", err.0[0].path);
        assert!(
            err.0[0].detail.contains("actions_per_tour"),
            "{}",
            err.0[0].detail
        );
    }

    #[test]
    fn invalid_dice_are_refused_at_parse() {
        let err = load_with("amount: 3", "amount: \"d6\"").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::Syntax]);
        assert!(err.to_string().contains("d6"));
    }

    #[test]
    fn unknown_references_are_named_with_their_path() {
        let err = load_with("primary_abilities: [FOR]", "primary_abilities: [LUCK]").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::UnknownAbility]);
        assert_eq!(err.0[0].path, "classes[brute].primary_abilities");
        assert!(err.0[0].detail.contains("LUCK"));

        let err =
            load_with("condition: ko, out_after", "condition: assomme, out_after").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::UnknownCondition]);
        assert!(err.0[0].detail.contains("assomme"));

        let err = load_with("kind: attaque\n", "kind: frapper\n").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::UnknownActionKind]);
    }

    #[test]
    fn formulas_may_only_read_what_exists() {
        let err = load_with("10 + mod(DEX)", "10 + mod(SAG)").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::UnknownAbility]);
        let err = load_with("10 + mod(DEX)", "10 + score").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::InvalidFormula]);
    }

    #[test]
    fn a_class_without_every_score_is_refused() {
        let err = load_with("{ FOR: 14, DEX: 10 }", "{ FOR: 14 }").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::MissingAbilityScore]);
    }

    #[test]
    fn levels_must_climb_from_one() {
        let err = load_with("{ level: 2, xp: 5 }", "{ level: 3, xp: 5 }").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::InvalidLevels]);
        let err = load_with("level: 1\n", "level: 4\n").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::LevelOutOfRange]);
    }

    #[test]
    fn advantage_needs_a_system_that_has_it() {
        let err = load_with(
            "effects: [skips_turn]",
            "effects: [{ advantage: { rolls: all } }]",
        )
        .unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::AdvantageNotInSystem]);
        assert!(
            load_with("advantage: false", "advantage: true")
                .and_then(|_| {
                    RuleSystem::from_yaml(
                        &MINI
                            .replacen("advantage: false", "advantage: true", 1)
                            .replacen(
                                "effects: [skips_turn]",
                                "effects: [{ advantage: { rolls: all } }]",
                                1,
                            ),
                    )
                })
                .is_ok()
        );
    }

    #[test]
    fn combat_fields_must_point_at_what_exists() {
        let err = load_with("roll: attack\n", "roll: attack\n        range: 0\n").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::InvalidValue]);
        let err = load_with("roll: attack\n", "roll: attack\n        long_range: 1\n").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::InvalidValue]);
        let err =
            load_with("conditions:", "combat: { move_kind: marcher }\nconditions:").unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::UnknownActionKind]);
        let err = load_with(
            "conditions:",
            "combat: { flee: { kind: attaque, ability: CHA } }\nconditions:",
        )
        .unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::UnknownAbility]);
        let err = load_with(
            "conditions:",
            "combat: { long_range: disadvantage }\nconditions:",
        )
        .unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::AdvantageNotInSystem]);
        let s = load_with(
            "conditions:",
            "combat: { long_range: { modifier: -3 } }\nconditions:",
        )
        .unwrap();
        assert_eq!(s.combat.long_range, LongRangeRule::Modifier(-3));
        assert_eq!(s.combat.cover, CoverRule::default());
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let err = load_with(
            "  - { id: ko, name: KO, kind: bane, effects: [skips_turn] }",
            "  - { id: ko, name: KO, kind: bane, effects: [skips_turn] }\n  - { id: ko, name: KO 2, kind: bane, effects: [] }",
        )
        .unwrap_err();
        assert_eq!(err.codes(), vec![ErrorCode::DuplicateId]);
    }

    #[test]
    fn several_errors_are_reported_together() {
        let text = MINI
            .replacen("primary_abilities: [FOR]", "primary_abilities: [LUCK]", 1)
            .replacen("kind: attaque\n", "kind: frapper\n", 1);
        let err = RuleSystem::from_yaml(&text).unwrap_err();
        assert_eq!(err.0.len(), 2);
    }
}
