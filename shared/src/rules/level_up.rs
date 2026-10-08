//! engine/level-up — what a new level brings, as the rule system says:
//! the hit points of each level above the first when the system has
//! `progression.hit_points_per_level` (the die rolled, or the average —
//! the player's choice, made once per level), and the class cards the
//! new level unlocks. Levels themselves come from total XP
//! (`progression::level_for`); upgrade points from the XP bar.
//!
//! The server holds the choices made (one gain per level) and gives the
//! sum back to the engine as [`Combatant::hit_point_bonus`]: only the
//! gains of levels the character still has count, so XP taken back by
//! the GM takes the hit points with it.

use serde::{Deserialize, Serialize};

use super::dice::DiceSource;
use super::model::{ActionDef, RuleSystem};
use super::progression::level_for;
use super::sheet::{Combatant, SheetError};

/// How the player takes a level's hit points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitPointMethod {
    /// The rule's die, plus the ability modifier.
    Roll,
    /// The rule's average, plus the ability modifier: no risk.
    Average,
}

/// The hit points one level added, as rolled or taken.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitPointGain {
    pub level: u32,
    pub method: HitPointMethod,
    /// The die's faces (empty for the average).
    #[serde(default)]
    pub faces: Vec<u32>,
    /// The die's total, or the average.
    pub base: i32,
    /// The ability modifier added.
    pub modifier: i32,
    /// What the maximum gains: at least 1.
    pub amount: i32,
}

/// What the player chooses between, as the screen shows it: « 1d10 +
/// Constitution » or « 6 + Constitution ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HitPointOptions {
    pub dice: String,
    pub average: i32,
    pub ability: Option<String>,
    pub modifier: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelUpError {
    /// The system adds no hit points per level.
    NoHitPointsPerLevel,
    /// Not a level the character has above the first.
    NotALevel(u32),
    /// That level's hit points were already taken.
    AlreadyChosen(u32),
    Sheet(SheetError),
}

impl From<SheetError> for LevelUpError {
    fn from(e: SheetError) -> Self {
        Self::Sheet(e)
    }
}

/// The choice offered for a new level's hit points, if the system has one.
pub fn hit_point_options(
    system: &RuleSystem,
    sheet: &Combatant,
) -> Result<Option<HitPointOptions>, SheetError> {
    let Some(rule) = &system.progression.hit_points_per_level else {
        return Ok(None);
    };
    let modifier = match &rule.ability {
        Some(a) => sheet.modifier(system, a)?,
        None => 0,
    };
    Ok(Some(HitPointOptions {
        dice: rule.dice.to_string(),
        average: rule.average,
        ability: rule.ability.clone(),
        modifier,
    }))
}

/// Takes the hit points of `level` by `method`. `chosen` are the levels
/// already taken: each level is chosen once, never rerolled.
pub fn take_hit_points(
    system: &RuleSystem,
    sheet: &Combatant,
    level: u32,
    method: HitPointMethod,
    chosen: &[u32],
    dice: &mut dyn DiceSource,
) -> Result<HitPointGain, LevelUpError> {
    let rule = system
        .progression
        .hit_points_per_level
        .as_ref()
        .ok_or(LevelUpError::NoHitPointsPerLevel)?;
    let current = sheet.level(system).unwrap_or(1);
    if level < 2 || level > current {
        return Err(LevelUpError::NotALevel(level));
    }
    if chosen.contains(&level) {
        return Err(LevelUpError::AlreadyChosen(level));
    }
    let modifier = match &rule.ability {
        Some(a) => sheet.modifier(system, a)?,
        None => 0,
    };
    let (faces, base) = match method {
        HitPointMethod::Roll => {
            let r = rule.dice.roll(dice);
            (r.faces, r.total)
        }
        HitPointMethod::Average => (Vec::new(), rule.average),
    };
    Ok(HitPointGain {
        level,
        method,
        faces,
        base,
        modifier,
        amount: (base + modifier).max(1),
    })
}

/// The maximum hit points the gains add for a character at `level`:
/// only the levels they still have.
pub fn hit_point_bonus(gains: &[HitPointGain], level: u32) -> i32 {
    gains
        .iter()
        .filter(|g| g.level <= level)
        .map(|g| g.amount)
        .sum()
}

/// The levels above the first whose hit points are still to choose.
pub fn hit_points_due(system: &RuleSystem, total_xp: u32, chosen: &[u32]) -> Vec<u32> {
    if system.progression.hit_points_per_level.is_none() {
        return Vec::new();
    }
    (2..=level_for(system, total_xp))
        .filter(|l| !chosen.contains(l))
        .collect()
}

/// The class cards a character gains going from level `from` to `to`.
pub fn cards_unlocked<'a>(
    sheet: &'a Combatant,
    system: &'a RuleSystem,
    from: u32,
    to: u32,
) -> Vec<&'a ActionDef> {
    sheet
        .class(system)
        .map(|c| {
            c.actions
                .iter()
                .filter(|a| a.level.is_some_and(|l| l > from && l <= to))
                .collect()
        })
        .unwrap_or_default()
}
