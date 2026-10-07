//! Checks: the system's die plus modifiers against a target, landing in
//! one of the four outcome bands. Every roll returns its breakdown —
//! die faces, each modifier and where it comes from, total, target, band
//! — which is exactly what the player screen shows.

use serde::{Deserialize, Serialize};

use super::dice::DiceSource;
use super::model::{ConditionEffect, GroupThreshold, RollScope, RuleSystem};
use super::sheet::{Combatant, SheetError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeBand {
    CriticalFailure,
    Failure,
    Success,
    CriticalSuccess,
}

impl OutcomeBand {
    pub fn is_success(self) -> bool {
        matches!(self, Self::Success | Self::CriticalSuccess)
    }

    /// XP this band grants, per the system.
    pub fn xp(self, system: &RuleSystem) -> u32 {
        let o = &system.outcomes;
        match self {
            Self::CriticalFailure => o.critical_failure.grants.xp,
            Self::Failure => o.failure.grants.xp,
            Self::Success => o.success.grants.xp,
            Self::CriticalSuccess => o.critical_success.grants.xp,
        }
    }

    pub fn name(self, system: &RuleSystem) -> &str {
        let o = &system.outcomes;
        match self {
            Self::CriticalFailure => &o.critical_failure.name,
            Self::Failure => &o.failure.name,
            Self::Success => &o.success.name,
            Self::CriticalSuccess => &o.critical_success.name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Advantage {
    #[default]
    Normal,
    Advantage,
    Disadvantage,
}

impl Advantage {
    /// Advantage and disadvantage cancel out.
    pub fn combine(has_adv: bool, has_dis: bool) -> Self {
        match (has_adv, has_dis) {
            (true, false) => Self::Advantage,
            (false, true) => Self::Disadvantage,
            _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "from", content = "id", rename_all = "snake_case")]
pub enum ModifierSource {
    Ability(String),
    /// An action's precision.
    Precision(String),
    /// A condition (its name, as the badge shows it).
    Condition(String),
    Situation(String),
    /// The target's cover on the grid.
    Cover(crate::maps::Cover),
    /// Beyond the action's range, within its long range.
    LongRange,
    /// The system's attack bonus (`attack.bonus`), by its name.
    AttackBonus(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifier {
    pub source: ModifierSource,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "against", rename_all = "snake_case")]
pub enum RollTarget {
    /// A difficulty: a named one (`id`) or a number the GM gave.
    Difficulty {
        id: Option<String>,
        value: i32,
    },
    ArmorClass {
        value: i32,
    },
    /// The other side's total in a contest.
    Opposed {
        value: i32,
    },
}

impl RollTarget {
    pub fn value(&self) -> i32 {
        match self {
            Self::Difficulty { value, .. }
            | Self::ArmorClass { value }
            | Self::Opposed { value } => *value,
        }
    }
}

/// One roll, everything the table needs to see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollBreakdown {
    /// The die as written (`1d20`).
    pub die: String,
    /// Every face rolled — two with advantage or disadvantage.
    pub faces: Vec<u32>,
    /// The face kept.
    pub natural: u32,
    pub advantage: Advantage,
    pub modifiers: Vec<Modifier>,
    pub total: i32,
    pub target: Option<RollTarget>,
    /// `None` when no target is known yet and the natural face alone
    /// does not decide.
    pub band: Option<OutcomeBand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// The system has no advantage and one was asked for.
    AdvantageNotInSystem,
    UnknownDifficulty(String),
    NoGroupCheckRule,
    EmptyGroup,
    Sheet(SheetError),
}

impl From<SheetError> for CheckError {
    fn from(e: SheetError) -> Self {
        Self::Sheet(e)
    }
}

/// The band of a roll: natural faces first, then the target.
pub fn band_for(
    system: &RuleSystem,
    natural: u32,
    total: i32,
    target: Option<i32>,
) -> Option<OutcomeBand> {
    let o = &system.outcomes;
    if o.critical_failure.natural.contains(&natural) {
        return Some(OutcomeBand::CriticalFailure);
    }
    if o.critical_success.natural.contains(&natural) {
        return Some(OutcomeBand::CriticalSuccess);
    }
    target.map(|t| {
        if total >= t {
            OutcomeBand::Success
        } else {
            OutcomeBand::Failure
        }
    })
}

/// Rolls the system's check die with these modifiers.
pub fn roll(
    system: &RuleSystem,
    modifiers: Vec<Modifier>,
    advantage: Advantage,
    target: Option<RollTarget>,
    dice: &mut dyn DiceSource,
) -> Result<RollBreakdown, CheckError> {
    if advantage != Advantage::Normal && !system.check.advantage {
        return Err(CheckError::AdvantageNotInSystem);
    }
    let faces_of = system.check.dice.faces;
    let mut faces = vec![dice.roll(faces_of)];
    if advantage != Advantage::Normal {
        faces.push(dice.roll(faces_of));
    }
    let natural = match advantage {
        Advantage::Advantage => *faces.iter().max().unwrap_or(&1),
        Advantage::Disadvantage => *faces.iter().min().unwrap_or(&1),
        Advantage::Normal => faces[0],
    };
    let total = natural as i32 + modifiers.iter().map(|m| m.value).sum::<i32>();
    let band = band_for(
        system,
        natural,
        total,
        target.as_ref().map(RollTarget::value),
    );
    Ok(RollBreakdown {
        die: system.check.dice.to_string(),
        faces,
        natural,
        advantage,
        modifiers,
        total,
        target,
        band,
    })
}

/// Modifiers and advantage a roller's conditions bring to a kind of roll.
pub fn condition_modifiers(sheet: &Combatant, scope: RollScope) -> (Vec<Modifier>, bool, bool) {
    let mut mods = Vec::new();
    let (mut adv, mut dis) = (false, false);
    for (c, e) in sheet.effects() {
        match e {
            ConditionEffect::RollModifier { rolls, value } if rolls.covers(scope) => {
                mods.push(Modifier {
                    source: ModifierSource::Condition(c.name.clone()),
                    value: *value,
                })
            }
            ConditionEffect::Advantage { rolls } if rolls.covers(scope) => adv = true,
            ConditionEffect::Disadvantage { rolls } if rolls.covers(scope) => dis = true,
            _ => {}
        }
    }
    (mods, adv, dis)
}

/// What a roll of one ability by one combatant adds: the ability's
/// modifier (with condition bonuses to the score) and condition modifiers.
pub fn ability_modifiers(
    system: &RuleSystem,
    sheet: &Combatant,
    ability: &str,
    scope: RollScope,
) -> Result<(Vec<Modifier>, bool, bool), CheckError> {
    let mut mods = vec![Modifier {
        source: ModifierSource::Ability(ability.into()),
        value: sheet.modifier(system, ability)?,
    }];
    let (cond, adv, dis) = condition_modifiers(sheet, scope);
    mods.extend(cond);
    Ok((mods, adv, dis))
}

/// A difficulty by its id (`moyen`) as a roll target.
pub fn difficulty(system: &RuleSystem, id: &str) -> Result<RollTarget, CheckError> {
    system
        .difficulty(id)
        .map(|d| RollTarget::Difficulty {
            id: Some(d.id.clone()),
            value: d.value,
        })
        .ok_or_else(|| CheckError::UnknownDifficulty(id.into()))
}

/// An ability check (or save, by `scope`) by one combatant. `requested`
/// is advantage the GM grants on top of conditions. Pure: XP is awarded
/// separately, by whoever applies the result.
pub fn ability_check(
    system: &RuleSystem,
    sheet: &Combatant,
    ability: &str,
    scope: RollScope,
    target: Option<RollTarget>,
    requested: Advantage,
    dice: &mut dyn DiceSource,
) -> Result<RollBreakdown, CheckError> {
    let (mods, adv, dis) = ability_modifiers(system, sheet, ability, scope)?;
    if requested != Advantage::Normal && !system.check.advantage {
        return Err(CheckError::AdvantageNotInSystem);
    }
    let advantage = Advantage::combine(
        adv || requested == Advantage::Advantage,
        dis || requested == Advantage::Disadvantage,
    );
    roll(system, mods, advantage, target, dice)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupCheck {
    /// (combatant id, its roll)
    pub rolls: Vec<(String, RollBreakdown)>,
    pub successes: usize,
    pub needed: usize,
    pub success: bool,
}

/// Everyone rolls the same check; the group succeeds per the system's
/// threshold (e.g. at least half).
pub fn group_check(
    system: &RuleSystem,
    members: &[&Combatant],
    ability: &str,
    target: RollTarget,
    dice: &mut dyn DiceSource,
) -> Result<GroupCheck, CheckError> {
    let rule = system
        .group_check
        .as_ref()
        .ok_or(CheckError::NoGroupCheckRule)?;
    if members.is_empty() {
        return Err(CheckError::EmptyGroup);
    }
    let mut rolls = Vec::new();
    for m in members {
        let r = ability_check(
            system,
            m,
            ability,
            RollScope::Checks,
            Some(target.clone()),
            Advantage::Normal,
            dice,
        )?;
        rolls.push((m.id.clone(), r));
    }
    let n = members.len();
    let successes = rolls
        .iter()
        .filter(|(_, r)| r.band.is_some_and(OutcomeBand::is_success))
        .count();
    let needed = match rule.succeeds_when {
        GroupThreshold::AtLeastHalf => n.div_ceil(2),
        GroupThreshold::Majority => n / 2 + 1,
        GroupThreshold::All => n,
        GroupThreshold::Any => 1,
    };
    Ok(GroupCheck {
        rolls,
        successes,
        needed,
        success: successes >= needed,
    })
}
