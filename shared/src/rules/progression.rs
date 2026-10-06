//! XP, upgrade points and levels, as the system's progression says.

use serde::{Deserialize, Serialize};

use super::dice::{DiceRoll, DiceSource};
use super::model::{ActionDef, RuleSystem};
use super::sheet::{Combatant, Progress, SheetError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ProgressEvent {
    XpGained { amount: u32, total: u32 },
    UpgradePoint { available: u32 },
    LevelUp { level: u32 },
}

/// Highest level whose threshold the total XP reaches.
pub fn level_for(system: &RuleSystem, total_xp: u32) -> u32 {
    system
        .progression
        .levels
        .iter()
        .filter(|l| l.xp <= total_xp)
        .map(|l| l.level)
        .max()
        .unwrap_or(1)
}

/// Adds XP: the bar empties into upgrade points every
/// `upgrade_every_xp`, and the total decides the level.
pub fn gain_xp(system: &RuleSystem, progress: &mut Progress, amount: u32) -> Vec<ProgressEvent> {
    if amount == 0 {
        return Vec::new();
    }
    let rule = &system.progression;
    let before = level_for(system, progress.total_xp);
    progress.total_xp += amount;
    progress.bar += amount;
    let mut events = vec![ProgressEvent::XpGained {
        amount,
        total: progress.total_xp,
    }];
    while progress.bar >= rule.upgrade_every_xp {
        progress.bar -= rule.upgrade_every_xp;
        progress.upgrade_points += rule.upgrade_points;
        events.push(ProgressEvent::UpgradePoint {
            available: progress.upgrade_points,
        });
    }
    let after = level_for(system, progress.total_xp);
    for level in before + 1..=after {
        events.push(ProgressEvent::LevelUp { level });
    }
    events
}

/// Spends one upgrade point: +1 to the chosen ability score.
pub fn spend_upgrade_point(
    system: &RuleSystem,
    sheet: &mut Combatant,
    ability: &str,
) -> Result<(), SheetError> {
    if system.ability(ability).is_none() {
        return Err(SheetError::UnknownAbility(ability.into()));
    }
    let progress = sheet.progress.as_mut().ok_or(SheetError::NoUpgradePoint)?;
    if progress.upgrade_points == 0 {
        return Err(SheetError::NoUpgradePoint);
    }
    progress.upgrade_points -= 1;
    *sheet.abilities.entry(ability.into()).or_insert(0) += 1;
    Ok(())
}

/// Grants what a roll's band gives (XP), to a combatant that progresses.
/// Adversaries have no progression and gain nothing.
pub fn award_band(
    system: &RuleSystem,
    sheet: &mut Combatant,
    band: Option<super::check::OutcomeBand>,
) -> Vec<ProgressEvent> {
    match (band, sheet.progress.as_mut()) {
        (Some(band), Some(progress)) => gain_xp(system, progress, band.xp(system)),
        _ => Vec::new(),
    }
}

/// How a player takes a new level's hit points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitPointChoice {
    /// The server rolls the die: risk for more.
    Roll,
    /// The die's average, no risk.
    Average,
}

/// What a level's hit point choice gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelHitPointsTaken {
    pub level: u32,
    /// The die's value kept for the level (rolled or average).
    pub die: i32,
    /// The roll, when the player rolled.
    pub roll: Option<DiceRoll>,
    /// Maximum hit points before and after.
    pub max_before: i32,
    pub max_after: i32,
}

/// Levels reached past the first whose hit points are still to choose.
pub fn levels_to_choose(system: &RuleSystem, sheet: &Combatant) -> Vec<u32> {
    if sheet.level_hit_die(system).is_none() {
        return Vec::new();
    }
    let level = sheet.level(system).unwrap_or(1);
    (2..=level)
        .filter(|l| !sheet.level_hit_dice.contains_key(l))
        .collect()
}

/// Takes a reached level's hit points, once: the die rolled by the
/// server, or its average. Current hit points grow with the maximum.
pub fn choose_level_hit_points(
    system: &RuleSystem,
    sheet: &mut Combatant,
    level: u32,
    choice: HitPointChoice,
    dice: &mut dyn DiceSource,
) -> Result<LevelHitPointsTaken, SheetError> {
    let die = sheet
        .level_hit_die(system)
        .ok_or(SheetError::NoLevelHitPoints)?;
    if level < 2 || sheet.level(system).unwrap_or(1) < level {
        return Err(SheetError::LevelNotReached(level));
    }
    if sheet.level_hit_dice.contains_key(&level) {
        return Err(SheetError::LevelHitPointsChosen(level));
    }
    let max_before = sheet.max_hit_points(system)?;
    let (value, roll) = match choice {
        HitPointChoice::Average => (die.average_up(), None),
        HitPointChoice::Roll => {
            let roll = die.roll(dice);
            (roll.total, Some(roll))
        }
    };
    sheet.level_hit_dice.insert(level, value);
    let max_after = sheet.max_hit_points(system)?;
    sheet.hit_points += max_after - max_before;
    Ok(LevelHitPointsTaken {
        level,
        die: value,
        roll,
        max_before,
        max_after,
    })
}

/// The class cards a level unlocks (actions whose `level` is exactly it).
pub fn unlocked_at<'s>(system: &'s RuleSystem, class_id: &str, level: u32) -> Vec<&'s ActionDef> {
    system
        .class(class_id)
        .map(|c| {
            c.actions
                .iter()
                .filter(|a| a.level == Some(level))
                .collect()
        })
        .unwrap_or_default()
}
