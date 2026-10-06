//! Levelling up between sessions (planche « Entre deux », moments 1
//! and 2): Borin reaches level 4, takes his hit points on the die or at
//! the average, and gets the card the rules unlock at that level.

use std::path::PathBuf;

use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::changes::{Field, Section, rule_changes};
use promptus_shared::rules::dice::ScriptedDice;
use promptus_shared::rules::load::ErrorCode;
use promptus_shared::rules::progression::{
    HitPointChoice, ProgressEvent, choose_level_hit_points, gain_xp, levels_to_choose, unlocked_at,
};
use promptus_shared::rules::sheet::{Combatant, SheetError};

fn text() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content/rules/corsaires/v1.yaml");
    std::fs::read_to_string(p).unwrap()
}

/// The Corsaires as Romain would edit them for D&D-style levels: a d10
/// plus Constitution a level, the Riposte learned at level 4, and a
/// sturdier Bretteur (CON 16, +3).
fn with_level_hit_points() -> RuleSystem {
    let t = text()
        .replacen(
            "  upgrade_points: 1\n",
            "  upgrade_points: 1\n  hit_points_per_level: { dice: 1d10, bonus: \"mod(CON)\" }\n",
            1,
        )
        .replacen(
            "abilities: { FOR: 13, DEX: 14, CON: 10,",
            "abilities: { FOR: 13, DEX: 14, CON: 16,",
            1,
        )
        .replacen("        level: 3\n", "        level: 4\n", 1);
    RuleSystem::from_yaml(&t).unwrap()
}

fn borin(system: &RuleSystem) -> Combatant {
    let mut b = Combatant::from_class(system, "borin", "Borin", "bretteur").unwrap();
    let mut progress = b.progress.unwrap();
    let events = gain_xp(system, &mut progress, 15);
    b.progress = Some(progress);
    assert!(events.contains(&ProgressEvent::LevelUp { level: 4 }));
    b
}

#[test]
fn borin_reaches_level_four_with_both_hit_point_options() {
    let system = with_level_hit_points();
    let mut b = borin(&system);
    assert_eq!(b.level(&system), Some(4));
    // Untaken levels count the average (6 + 3): 10 + 3 × 9.
    assert_eq!(levels_to_choose(&system, &b), vec![2, 3, 4]);
    assert_eq!(b.max_hit_points(&system).unwrap(), 37);

    // Level 4 on the die: a 7, one more than the average.
    let mut dice = ScriptedDice::new([7]);
    let before = b.hit_points;
    let took =
        choose_level_hit_points(&system, &mut b, 4, HitPointChoice::Roll, &mut dice).unwrap();
    assert_eq!(took.die, 7);
    assert_eq!(took.roll.as_ref().unwrap().faces, vec![7]);
    assert_eq!((took.max_before, took.max_after), (37, 38));
    assert_eq!(
        b.hit_points,
        before + 1,
        "the new hit point is there to use"
    );

    // A level is taken once: no second roll for a better number.
    assert_eq!(
        choose_level_hit_points(&system, &mut b, 4, HitPointChoice::Roll, &mut dice),
        Err(SheetError::LevelHitPointsChosen(4))
    );

    // Level 3 at the average: no risk, nothing changes but the choice.
    let took =
        choose_level_hit_points(&system, &mut b, 3, HitPointChoice::Average, &mut dice).unwrap();
    assert_eq!((took.die, took.max_after), (6, 38));
    assert_eq!(levels_to_choose(&system, &b), vec![2]);

    // A bad roll still gives at least one hit point a level.
    let mut weak = Combatant::from_class(&system, "w", "W", "chirurgien").unwrap();
    weak.abilities.insert("CON".into(), 1);
    let mut p = weak.progress.unwrap();
    gain_xp(&system, &mut p, 5);
    weak.progress = Some(p);
    let took = choose_level_hit_points(
        &system,
        &mut weak,
        2,
        HitPointChoice::Roll,
        &mut ScriptedDice::new([1]),
    )
    .unwrap();
    assert_eq!(took.max_after - 10, 1);

    // Not reached yet, or level 1: nothing to take.
    assert_eq!(
        choose_level_hit_points(&system, &mut b, 5, HitPointChoice::Average, &mut dice),
        Err(SheetError::LevelNotReached(5))
    );
    assert_eq!(
        choose_level_hit_points(&system, &mut b, 1, HitPointChoice::Average, &mut dice),
        Err(SheetError::LevelNotReached(1))
    );
}

#[test]
fn the_new_card_is_the_one_the_rules_unlock_at_that_level() {
    let system = with_level_hit_points();
    let ids = |level| -> Vec<String> {
        unlocked_at(&system, "bretteur", level)
            .iter()
            .map(|a| a.id.clone())
            .collect()
    };
    assert_eq!(ids(4), vec!["riposte_en_quarte"]);
    assert!(ids(3).is_empty());
    assert_eq!(ids(7), vec!["danse_des_lames"]);
}

#[test]
fn worlds_without_level_hit_points_keep_flat_hit_points() {
    let system = RuleSystem::from_yaml(&text()).unwrap();
    let mut b = borin(&system);
    assert_eq!(b.max_hit_points(&system).unwrap(), 10);
    assert!(levels_to_choose(&system, &b).is_empty());
    assert_eq!(
        choose_level_hit_points(
            &system,
            &mut b,
            2,
            HitPointChoice::Average,
            &mut ScriptedDice::new([])
        ),
        Err(SheetError::NoLevelHitPoints)
    );
}

#[test]
fn a_fight_saved_before_levels_had_hit_points_still_reads() {
    let system = with_level_hit_points();
    let b = borin(&system);
    let mut json = serde_json::to_value(&b).unwrap();
    json.as_object_mut().unwrap().remove("level_hit_dice");
    let back: Combatant = serde_json::from_value(json).unwrap();
    assert!(back.level_hit_dice.is_empty());
}

#[test]
fn a_flat_hit_die_is_refused_and_a_changed_one_is_listed() {
    let flat = text().replacen(
        "  upgrade_points: 1\n",
        "  upgrade_points: 1\n  hit_points_per_level: { dice: 6 }\n",
        1,
    );
    let err = RuleSystem::from_yaml(&flat).unwrap_err();
    assert_eq!(err.codes(), vec![ErrorCode::InvalidDice]);

    let old = RuleSystem::from_yaml(&text()).unwrap();
    let new = with_level_hit_points();
    let changes = rule_changes(&old, &new);
    let hp = changes
        .iter()
        .find(|c| c.field == Field::LevelHitPoints)
        .expect("the new level hit points are listed");
    assert_eq!(hp.section, Section::Progression);
    assert_eq!(
        serde_json::to_value(&hp.to).unwrap()["value"],
        "1d10 + mod(CON)"
    );
}
