//! House rules the server judges (`engine/formalise-house-rules`), on
//! the three systems: the board's « undead fear fire » on the SRD, and
//! the natural 1 in attack that both witness worlds leave to the GM —
//! « le pied qui glisse » on the Corsaires, « l'arme qui s'enraye » in
//! the Brasier.

use std::path::PathBuf;

use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::action::{ActionRequest, resolve_action};
use promptus_shared::rules::dice::ScriptedDice;
use promptus_shared::rules::events::Event;
use promptus_shared::rules::house::{Expect, run_cases};
use promptus_shared::rules::load::ErrorCode;
use promptus_shared::rules::sheet::{Combatant, Scene, Side, TurnBudget};

fn text(id: &str) -> String {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../content/rules/{id}/v1.yaml"));
    std::fs::read_to_string(path).unwrap()
}

/// `id`'s rules with `extra` YAML appended at the top level.
fn with(id: &str, extra: &str) -> RuleSystem {
    RuleSystem::from_yaml(&format!("{}\n{extra}", text(id))).unwrap_or_else(|e| panic!("{e}"))
}

/// Corsaires: a natural 1 in attack makes the attacker slip — down for
/// a turn. Party only: the GM keeps the NPCs' fumbles to the story.
const PIED_QUI_GLISSE: &str = r#"
house_rules:
  - id: pied_qui_glisse
    name: Le pied qui glisse
    text: "Sur un 1 naturel en attaque, le corsaire glisse et se retrouve renversé jusqu'à son prochain tour."
    formal:
      when: miss
      critical: true
      actor: { side: party }
      effects:
        - { apply: { condition: renverse, turns: 1, to: self } }
      players: rule
      cases:
        - { name: Estocade ratée sur un 1, actor: { class: bretteur }, action: estocade, target: { adversary: marin_de_gueule_rouge }, roll: fumble, expect: applies }
        - { name: Estocade simplement ratée, actor: { class: bretteur }, action: estocade, target: { adversary: marin_de_gueule_rouge }, roll: miss, expect: nothing }
        - { name: Un marin qui fait 1, actor: { adversary: marin_de_gueule_rouge }, action: marin_coutelas, target: { class: bretteur }, roll: fumble, expect: nothing }
"#;

/// Brasier: a natural 1 jams the weapon — the turn after goes to
/// clearing it.
const ARME_ENRAYEE: &str = r#"
house_rules:
  - id: arme_enrayee
    name: L'arme qui s'enraye
    text: "Sur un 1 naturel au tir, l'arme s'enraye : on passe son prochain tour à la décoincer."
    formal:
      when: miss
      critical: true
      effects:
        - { apply: { condition: etourdi, turns: 1, to: self } }
      players: rule
      cases:
        - { name: Tir réflexe sur un 1, actor: { class: pilote }, action: tir_reflexe, target: { class: mecano }, roll: fumble, expect: applies }
        - { name: Tir réflexe raté, actor: { class: pilote }, action: tir_reflexe, target: { class: mecano }, roll: miss, expect: nothing }
        - { name: Tir réflexe qui touche, actor: { class: pilote }, action: tir_reflexe, target: { class: mecano }, roll: hit, expect: nothing }
"#;

fn assert_cases_pass(system: &RuleSystem, rule: &str) {
    let rule = system.house_rule(rule).unwrap();
    let results = run_cases(system, rule);
    assert_eq!(results.len(), rule.formal.as_ref().unwrap().cases.len());
    for r in &results {
        assert!(r.passed, "{}: {r:?}", r.name);
        assert_eq!(r.fired, r.expect == Expect::Applies, "{}", r.name);
    }
}

#[test]
fn every_case_of_the_three_systems_rules_plays_as_written() {
    let srd = RuleSystem::from_yaml(&text("srd")).unwrap();
    assert_cases_pass(&srd, "morts_vivants_feu");
    assert_cases_pass(&with("corsaires", PIED_QUI_GLISSE), "pied_qui_glisse");
    assert_cases_pass(&with("brasier", ARME_ENRAYEE), "arme_enrayee");
}

#[test]
fn a_case_that_contradicts_the_rule_fails_and_says_what_happened() {
    let wrong = ARME_ENRAYEE.replace("roll: miss, expect: nothing", "roll: miss, expect: applies");
    let system = with("brasier", &wrong);
    let results = run_cases(&system, system.house_rule("arme_enrayee").unwrap());
    let bad = results
        .iter()
        .find(|r| r.name == "Tir réflexe raté")
        .unwrap();
    assert!(!bad.passed && !bad.fired, "{bad:?}");
    // The case that fires reports what the rule did.
    let good = &results[0];
    assert!(
        good.effects.iter().any(|e| matches!(e, Event::ConditionApplied { target, name, .. } if target == "actor" && name == "Étourdi")),
        "{good:?}"
    );
    assert_eq!(good.natural, Some(1));
}

fn duel(system: &RuleSystem, actor: Combatant, mut target: Combatant) -> Scene {
    target.side = if actor.side == Side::Party {
        Side::Opposition
    } else {
        Side::Party
    };
    let mut actor = actor;
    actor.turn = TurnBudget {
        actions_left: system.turn_contexts[0].actions_per_turn,
        ..TurnBudget::default()
    };
    let mut scene = Scene::new(&system.turn_contexts[0].id, [actor, target]);
    scene.active = Some("a".into());
    scene
}

#[test]
fn in_play_fire_frightens_the_undead_and_leaves_the_goblin_alone() {
    let srd = RuleSystem::from_yaml(&text("srd")).unwrap();
    let guerrier = Combatant::from_class(&srd, "a", "Borin", "guerrier").unwrap();
    let torch = ActionRequest::item("a", "torche", &["t"]);

    let zombie = Combatant::from_adversary(&srd, "t", "Zombie", "zombie").unwrap();
    let scene = duel(&srd, guerrier.clone(), zombie);
    // d20 15 to hit, then the torch's d4.
    let r = resolve_action(&srd, &scene, &torch, &mut ScriptedDice::new([15, 1])).unwrap();
    let fired = r.events.iter().position(|e| matches!(e, Event::HouseRule { rule, shown, target, .. } if rule == "morts_vivants_feu" && !shown && target == "t"));
    assert!(fired.is_some(), "{:#?}", r.events);
    let zombie = r.scene.get("t").unwrap();
    assert!(zombie.condition_named("effraye").is_some());

    let gobelin = Combatant::from_adversary(&srd, "t", "Gobelin", "gobelin").unwrap();
    let scene = duel(&srd, guerrier, gobelin);
    let r = resolve_action(&srd, &scene, &torch, &mut ScriptedDice::new([19, 1])).unwrap();
    assert!(
        !r.events
            .iter()
            .any(|e| matches!(e, Event::HouseRule { .. }))
    );
    assert!(
        r.scene
            .get("t")
            .unwrap()
            .condition_named("effraye")
            .is_none()
    );
}

#[test]
fn a_rule_whose_effect_deals_its_own_trigger_fires_once() {
    // Fire on the undead burns them again, with fire: a rule that would
    // loop if its effects triggered house rules.
    let srd = RuleSystem::from_yaml(&text("srd")).unwrap();
    let mut text = text("srd");
    text = text.replace(
        "        - { apply: { condition: effraye, turns: 1, to: targets } }\n",
        "        - { apply: { condition: effraye, turns: 1, to: targets } }\n        - { damage: { amount: 2, to: targets } }\n",
    );
    let looped = RuleSystem::from_yaml(&text).unwrap();
    assert_ne!(
        looped.house_rule("morts_vivants_feu").unwrap().formal,
        srd.house_rule("morts_vivants_feu").unwrap().formal
    );
    let guerrier = Combatant::from_class(&looped, "a", "Borin", "guerrier").unwrap();
    let zombie = Combatant::from_adversary(&looped, "t", "Zombie", "zombie").unwrap();
    let scene = duel(&looped, guerrier, zombie);
    let r = resolve_action(
        &looped,
        &scene,
        &ActionRequest::item("a", "torche", &["t"]),
        &mut ScriptedDice::new([15, 1]),
    )
    .unwrap();
    let fired = r
        .events
        .iter()
        .filter(|e| matches!(e, Event::HouseRule { .. }))
        .count();
    assert_eq!(fired, 1);
    // 22 HP − 1 (torch) − 2 (the rule).
    assert_eq!(r.scene.get("t").unwrap().hit_points, 19);
}

#[test]
fn a_formal_rule_naming_what_the_system_lacks_does_not_load() {
    let bad = |extra: &str| {
        RuleSystem::from_yaml(&format!("{}\n{extra}", text("corsaires")))
            .unwrap_err()
            .codes()
    };
    // The Corsaires have no traits and no damage types.
    let undead = r#"
house_rules:
  - id: r
    name: R
    text: T
    formal:
      when: hit
      damage_type: feu
      target: { traits: [mort_vivant] }
      effects: [{ apply: { condition: apeure, turns: 1, to: targets } }]
"#;
    let codes = bad(undead);
    assert!(codes.contains(&ErrorCode::UnknownTrait), "{codes:?}");
    assert!(codes.contains(&ErrorCode::UnknownDamageType), "{codes:?}");
    // A save in a house rule names its difficulty; a miss deals no
    // damage type; a case plays an action its attacker has.
    let open_save = r#"
house_rules:
  - id: r
    name: R
    text: T
    formal:
      when: miss
      damage_type: feu
      effects: [{ apply: { condition: apeure, turns: 1, to: targets, save: { ability: SAG } } }]
      cases:
        - { name: C, actor: { class: bretteur }, action: tir_de_silex, target: { adversary: gueule_rouge }, expect: applies }
"#;
    let codes = bad(open_save);
    for code in [
        ErrorCode::UnknownDifficulty,
        ErrorCode::InvalidValue,
        ErrorCode::UnknownDamageType,
        ErrorCode::UnknownAction,
    ] {
        assert!(codes.contains(&code), "{code:?} in {codes:?}");
    }
}
