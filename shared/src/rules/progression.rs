//! XP, upgrade points and levels, as the system's progression says.

use serde::Serialize;

use super::model::RuleSystem;
use super::sheet::{Combatant, Progress, SheetError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
