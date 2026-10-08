//! XP, upgrade points and levels, as the system's progression says.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::model::RuleSystem;
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

/// Upgrade points spent, by ability id: each point adds 1 to that score
/// (player/play-between-sessions).
pub type Upgrades = BTreeMap<String, u32>;

/// Applies the points `upgrades` already spent to a combatant whose XP
/// was gained: each score rises by its points, and the points left are
/// those earned minus those spent — never below zero, since the GM may
/// take XP back after a point was spent (the score keeps it). Abilities
/// the rules no longer have are ignored. A combatant without progression
/// (an adversary) is left as it is.
pub fn apply_upgrades(system: &RuleSystem, sheet: &mut Combatant, upgrades: &Upgrades) {
    let Some(progress) = sheet.progress.as_mut() else {
        return;
    };
    let mut spent = 0u32;
    for (ability, points) in upgrades {
        if system.ability(ability).is_none() || *points == 0 {
            continue;
        }
        *sheet.abilities.entry(ability.clone()).or_insert(0) +=
            i32::try_from(*points).unwrap_or(i32::MAX);
        spent = spent.saturating_add(*points);
    }
    progress.upgrade_points = progress.upgrade_points.saturating_sub(spent);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn system(world: &str) -> RuleSystem {
        let yaml = match world {
            "corsaires" => include_str!("../../../content/rules/corsaires/v1.yaml"),
            _ => include_str!("../../../content/rules/brasier/v1.yaml"),
        };
        RuleSystem::from_yaml(yaml).unwrap()
    }

    fn earned(system: &RuleSystem, class: &str, xp: u32) -> Combatant {
        let mut c = Combatant::from_class(system, "pc", "Borin", class).unwrap();
        let mut progress = Progress::default();
        gain_xp(system, &mut progress, xp);
        c.progress = Some(progress);
        c
    }

    #[test]
    fn a_spent_point_raises_the_score_and_leaves_the_others() {
        for (world, class) in [("corsaires", "bretteur"), ("brasier", "pilote")] {
            let s = system(world);
            let mut c = earned(&s, class, 12);
            assert_eq!(c.progress.unwrap().upgrade_points, 2, "{world}");
            let ability = s.abilities[0].id.clone();
            let before = c.score(&ability).unwrap();
            let modifier = c.modifier(&s, &ability).unwrap();
            apply_upgrades(&s, &mut c, &Upgrades::from([(ability.clone(), 2)]));
            assert_eq!(c.score(&ability), Some(before + 2), "{world}");
            assert_eq!(c.modifier(&s, &ability).unwrap(), modifier + 1, "{world}");
            assert_eq!(c.progress.unwrap().upgrade_points, 0, "{world}");
        }
    }

    #[test]
    fn xp_taken_back_leaves_no_negative_points_and_keeps_the_score() {
        let s = system("corsaires");
        let mut c = earned(&s, "bretteur", 4);
        let before = c.score("FOR").unwrap();
        apply_upgrades(&s, &mut c, &Upgrades::from([("FOR".into(), 1)]));
        assert_eq!(c.progress.unwrap().upgrade_points, 0);
        assert_eq!(c.score("FOR"), Some(before + 1));
    }

    #[test]
    fn an_ability_the_rules_dropped_is_ignored() {
        let s = system("corsaires");
        let mut c = earned(&s, "bretteur", 5);
        apply_upgrades(&s, &mut c, &Upgrades::from([("PSI".into(), 1)]));
        assert_eq!(c.progress.unwrap().upgrade_points, 1);
        assert_eq!(c.score("PSI"), None);
    }
}
