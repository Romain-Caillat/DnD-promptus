//! The D&D 5e SRD 5.1 as a third rule system (`engine/add-srd-preset`):
//! it loads through the same format as the two witness worlds, with no
//! case of its own in the engine, and the V1 demo's goblin fight plays
//! to the end on it.

use std::path::PathBuf;

use promptus_shared::combat::fight::Standing;
use promptus_shared::combat::{EndReason, Limits, PolicyKind, Scenario, simulate::play};
use promptus_shared::issue::Severity;
use promptus_shared::maps::Map;
use promptus_shared::rules::action::action_cards;
use promptus_shared::rules::check::ModifierSource;
use promptus_shared::rules::events::Event;
use promptus_shared::rules::model::{RollSpec, ZeroHpRule};
use promptus_shared::rules::sheet::{Combatant, Side};
use promptus_shared::rules::{RuleSystem, lint};

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn srd() -> RuleSystem {
    RuleSystem::from_yaml(&read("content/rules/srd/v1.yaml")).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_preset_holds_what_the_board_promises() {
    let s = srd();
    assert_eq!((s.id.as_str(), s.version), ("srd", 1));
    assert!(s.check.advantage);
    assert_eq!(s.conditions.len(), 14);
    let classes: Vec<_> = s.classes.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(classes, ["guerrier", "rodeur", "roublard", "magicien"]);
    let peoples: Vec<_> = s.peoples.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(peoples, ["humain", "nain", "elfe", "halfelin"]);
    assert!(matches!(
        s.zero_hp,
        ZeroHpRule::DeathSaves {
            difficulty: 10,
            successes: 3,
            failures: 3,
            ..
        }
    ));
    assert!(s.sources.iter().any(|x| x.contains("CC-BY-4.0")));
    // The lint flags nothing the GM must fix (warnings may stand).
    let errors: Vec<_> = lint(&s)
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn each_class_has_its_own_hit_points_and_armour() {
    let s = srd();
    let at = |class: &str| {
        let c = Combatant::from_class(&s, "x", "X", class).unwrap();
        (c.max_hit_points(&s).unwrap(), c.armor_class(&s).unwrap())
    };
    // d10 + CON 14 (+2), chain mail 16.
    assert_eq!(at("guerrier"), (12, 16));
    // d10 + CON 13 (+1), leather 11 + DEX 15 (+2).
    assert_eq!(at("rodeur"), (11, 13));
    // The system's d8 + CON 14 (+2), leather.
    assert_eq!(at("roublard"), (10, 13));
    // d6 + CON 13 (+1), no armour: 10 + DEX 14 (+2).
    assert_eq!(at("magicien"), (7, 12));
}

#[test]
fn the_proficiency_bonus_grows_with_the_level_and_shows_on_the_card() {
    let s = srd();
    let mut borin = Combatant::from_class(&s, "b", "Borin", "guerrier").unwrap();
    let bonus = |c: &Combatant| {
        action_cards(&s, c)
            .into_iter()
            .find(|card| card.action.id == "epee_longue")
            .unwrap()
            .attack_bonus
    };
    // FOR 15 (+2) and Maîtrise +2.
    assert_eq!(bonus(&borin), Some(4));
    borin.progress.as_mut().unwrap().total_xp = 6500; // level 5
    assert_eq!(borin.level(&s), Some(5));
    assert_eq!(bonus(&borin), Some(5));
    // A goblin's scimitar is the stat block's +4: DEX +2 and Maîtrise +2.
    let gob = Combatant::from_adversary(&s, "g", "Gobelin", "gobelin").unwrap();
    let action = gob.actions(&s)[0].clone();
    assert_eq!(action.roll, RollSpec::Attack);
    let (mods, _, _) = promptus_shared::rules::action::attack_modifiers(&s, &gob, &action).unwrap();
    assert_eq!(mods.iter().map(|m| m.value).sum::<i32>(), 4);
    assert!(
        mods.iter()
            .any(|m| m.source == ModifierSource::AttackBonus("Maîtrise".into()))
    );
}

#[test]
fn the_goblin_ambush_of_the_v1_demo_is_played_to_the_end() {
    let scenario = Scenario::from_yaml(&read("content/scenarios/srd/embuscade-des-gobelins.yaml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let system = scenario
        .system(&read("content/rules/srd/v1.yaml"), None)
        .unwrap_or_else(|e| panic!("{e}"));
    let map = Map::from_yaml(&read("content/maps/srd/route-des-gobelins.yaml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut party_wins = 0;
    let mut knocked_out = 0;
    for seed in 1..=20 {
        for party in [PolicyKind::Brawler, PolicyKind::Focus] {
            let log = play(
                &system,
                &scenario,
                &map,
                party,
                PolicyKind::Brawler,
                seed,
                Limits::default(),
            )
            .unwrap();
            let end = log.fight.end.as_ref().expect("the fight ended");
            assert_eq!(end.reason, EndReason::SideDown, "seed {seed}: decided");
            assert!(log.refusals().next().is_none(), "seed {seed}");
            if end.winner == Some(Side::Party) {
                party_wins += 1;
                // Every goblin is down or ran.
                for id in log.fight.side(Side::Opposition) {
                    assert_ne!(log.fight.standing[id], Standing::InFight);
                }
            }
            // A hero at 0 HP is unconscious, making death saves (the
            // zero-HP rule), never "out of the scene".
            for e in log.events() {
                if let promptus_shared::combat::FightEvent::Rules {
                    event: Event::KnockedOut { target },
                } = e
                    && log.fight.side(Side::Party).any(|p| p == target)
                {
                    knocked_out += 1;
                    assert_ne!(log.fight.standing[target], Standing::OutOfScene);
                }
            }
        }
    }
    // By the SRD's own encounter arithmetic this ambush is deadly for
    // four level-1 heroes (4 × 50 + 200 XP, ×2 for five foes = 800,
    // against a 400 threshold): either side can win, and heroes drop.
    assert!(
        (5..=35).contains(&party_wins),
        "the party won {party_wins} of 40"
    );
    assert!(knocked_out > 0, "nobody ever dropped");
}
