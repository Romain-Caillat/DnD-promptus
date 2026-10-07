//! Death saves (`engine/save-against-death`), replaying the planche
//! « Mourir » with the Corsaires rules switched to death saves: Borin
//! falls to 0, rolls on his turns, the chief could finish him, a natural
//! 1 brings the third failure, and the death waits for the GM's word.

use std::path::PathBuf;

use promptus_shared::combat::fight::CombatRefusal;
use promptus_shared::combat::{DeathCall, Fight, FightEvent, Play, Standing, Step};
use promptus_shared::maps::{Cell, Map};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::dice::ScriptedDice;
use promptus_shared::rules::events::Event;
use promptus_shared::rules::model::ZeroHpRule;
use promptus_shared::rules::sheet::{Combatant, Side};

const KNOCKED_OUT: &str = "  rule: knocked_out\n  condition: inconscient\n  out_after_turns: 3\n  out_condition: hors_combat\n";
const DEATH_SAVES: &str = "  rule: death_saves\n  condition: inconscient\n  difficulty: 10\n  successes: 3\n  failures: 3\n";

fn rules() -> RuleSystem {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content/rules/corsaires/v1.yaml");
    let t = std::fs::read_to_string(p).unwrap();
    assert!(t.contains(KNOCKED_OUT));
    let s = RuleSystem::from_yaml(&t.replacen(KNOCKED_OUT, DEATH_SAVES, 1)).unwrap();
    assert!(matches!(s.zero_hp, ZeroHpRule::DeathSaves { .. }));
    s
}

fn room() -> Map {
    Map::from_yaml(
        "version: 1\nid: t\nname: Test\nscale: encounter\ntheme: test\ngrid:\n  legend:\n    \".\": { terrain: sol }\n  rows:\n    - \"...\"\n    - \"...\"\n    - \"...\"\n",
    )
    .unwrap()
}

fn faces(f: &[u32]) -> ScriptedDice {
    ScriptedDice::new(f.iter().copied())
}

fn ok(r: Result<Step, CombatRefusal>) -> Step {
    r.unwrap_or_else(|e| panic!("refused: {e:?}"))
}

/// Lyra (surgeon), the chief (a sailor) and Borin at 1 HP; Lyra plays
/// first, then the chief, then Borin.
fn fight(s: &RuleSystem, borin_hp: i32) -> Fight {
    let mut borin = Combatant::from_class(s, "borin", "Borin", "bretteur").unwrap();
    borin.hit_points = borin_hp;
    let lyra = Combatant::from_class(s, "lyra", "Lyra", "chirurgien").unwrap();
    let chef = Combatant::from_adversary(s, "chef", "Chef", "marin_de_gueule_rouge").unwrap();
    assert_eq!(chef.side, Side::Opposition);
    let mut dice = faces(&[20, 10, 1]);
    let f = Fight::start(
        s,
        "sol",
        room(),
        vec![
            (lyra, Cell::new(0, 1)),
            (chef, Cell::new(1, 0)),
            (borin, Cell::new(0, 0)),
        ],
        &mut dice,
    )
    .unwrap()
    .fight;
    assert_eq!(f.order, ["lyra", "chef", "borin"]);
    f
}

/// The chief hits Borin for 3 (15 to hit, 2 on the 1d4+1).
fn chief_downs_borin(s: &RuleSystem, f: Fight) -> Fight {
    let f = ok(f.end_turn(s, "lyra", &mut faces(&[]))).fight;
    let step = ok(f.act(
        s,
        &Play::on("chef", "marin_coutelas", &["borin"]),
        &mut faces(&[15, 2]),
    ));
    assert!(step.events.contains(&FightEvent::Rules {
        event: Event::KnockedOut {
            target: "borin".into()
        }
    }));
    ok(step.fight.end_turn(s, "chef", &mut faces(&[]))).fight
}

fn save(e: &[FightEvent]) -> (u32, u32, u32) {
    e.iter()
        .find_map(|e| match e {
            FightEvent::DeathSave {
                natural,
                successes,
                failures,
                ..
            } => Some((*natural, *successes, *failures)),
            _ => None,
        })
        .expect("a death save")
}

#[test]
fn borins_last_evening_plays_out_and_waits_for_the_gm() {
    let s = rules();
    // 1. Down at 0 HP: unconscious, not dead, still in the fight.
    let f = chief_downs_borin(&s, fight(&s, 1));
    assert_eq!(f.combatant("borin").unwrap().hit_points, 0);
    assert!(f.in_fight("borin"));
    assert!(!f.is_over());

    // 2. His turn opens for the save, and only for it: a 7 is a failure.
    assert_eq!(f.active(), Some("borin"));
    assert!(f.save_due(&s, "borin"));
    assert_eq!(
        f.end_turn(&s, "borin", &mut faces(&[])).unwrap_err(),
        CombatRefusal::DeathSaveDue
    );
    let step = ok(f.death_save(&s, "borin", &mut faces(&[7])));
    assert_eq!(save(&step.events), (7, 0, 1));
    let f = step.fight;
    assert_eq!(f.active(), Some("lyra"), "the save ends his turn");

    // 3. Lyra cannot save him this time (no potion): she passes.
    let f = ok(f.end_turn(&s, "lyra", &mut faces(&[]))).fight;

    // 4. The chief's turn: the rules let him finish Borin, the GM
    //    chooses the axe and hits Lyra instead.
    assert!(
        f.check(&s, &Play::on("chef", "marin_coutelas", &["borin"]))
            .is_ok()
    );
    let f = ok(f.act(
        &s,
        &Play::on("chef", "marin_coutelas", &["lyra"]),
        &mut faces(&[15, 1]),
    ))
    .fight;
    let f = ok(f.end_turn(&s, "chef", &mut faces(&[]))).fight;

    // 5. The last roll: a natural 1 is two failures, the third. The
    //    engine proposes the death; nothing more happens without the GM.
    let step = ok(f.death_save(&s, "borin", &mut faces(&[1])));
    assert_eq!(save(&step.events), (1, 0, 3));
    assert!(step.events.contains(&FightEvent::DeathProposed {
        who: "borin".into()
    }));
    assert!(
        !step
            .events
            .iter()
            .any(|e| matches!(e, FightEvent::Died { .. }))
    );
    let f = step.fight;
    assert_eq!(f.standing["borin"], Standing::InFight);
    assert_eq!(f.proposed_deaths().collect::<Vec<_>>(), ["borin"]);
    // His turns now pass on their own: no more saves.
    let f = ok(f.end_turn(&s, "lyra", &mut faces(&[]))).fight;
    let step = ok(f.end_turn(&s, "chef", &mut faces(&[])));
    assert!(step.events.contains(&FightEvent::TurnEnded {
        who: "borin".into()
    }));
    let f = step.fight;
    assert_eq!(f.active(), Some("lyra"));
    assert_eq!(
        f.decide_death("lyra", DeathCall::Die).unwrap_err(),
        CombatRefusal::NoDeathProposed
    );

    // The GM confirms: Borin dies and leaves the board.
    let step = ok(f.decide_death("borin", DeathCall::Die));
    assert_eq!(
        step.events[0],
        FightEvent::Died {
            who: "borin".into()
        }
    );
    let f = step.fight;
    assert_eq!(f.standing["borin"], Standing::Dead);
    assert_eq!(f.position("borin"), None);
    assert!(!f.is_over(), "Lyra still stands");
    assert_eq!(f.proposed_deaths().count(), 0);
}

/// Plays Lyra's and the chief's turns without touching Borin.
fn round_passes(s: &RuleSystem, f: Fight) -> Fight {
    let f = ok(f.end_turn(s, "lyra", &mut faces(&[]))).fight;
    ok(f.end_turn(s, "chef", &mut faces(&[]))).fight
}

#[test]
fn three_successes_stabilise_and_the_saves_stop() {
    let s = rules();
    let mut f = chief_downs_borin(&s, fight(&s, 1));
    for (i, face) in [10, 19, 12].into_iter().enumerate() {
        let step = ok(f.death_save(&s, "borin", &mut faces(&[face])));
        let (_, successes, failures) = save(&step.events);
        assert_eq!((successes, failures), (u32::try_from(i).unwrap() + 1, 0));
        f = round_passes(&s, step.fight);
    }
    assert!(!f.save_due(&s, "borin"));
    assert!(f.dying["borin"].stable);
    assert_eq!(f.combatant("borin").unwrap().hit_points, 0);
    // A stable character's turn passes on its own.
    assert_eq!(f.active(), Some("lyra"));
}

#[test]
fn a_natural_20_brings_him_back_at_one() {
    let s = rules();
    let f = chief_downs_borin(&s, fight(&s, 1));
    let step = ok(f.death_save(&s, "borin", &mut faces(&[20])));
    assert!(step.events.contains(&FightEvent::Rules {
        event: Event::Revived {
            target: "borin".into()
        }
    }));
    let f = round_passes(&s, step.fight);
    assert_eq!(f.combatant("borin").unwrap().hit_points, 1);
    assert!(!f.dying.contains_key("borin"));
    assert_eq!(f.active(), Some("borin"));
    assert!(!f.save_due(&s, "borin"));
    ok(f.end_turn(&s, "borin", &mut faces(&[])));
}

#[test]
fn a_hit_while_down_is_a_failure_and_a_critical_two() {
    let s = rules();
    let f = chief_downs_borin(&s, fight(&s, 1));
    let f = ok(f.death_save(&s, "borin", &mut faces(&[12]))).fight;
    let f = ok(f.end_turn(&s, "lyra", &mut faces(&[]))).fight;
    let step = ok(f.act(
        &s,
        &Play::on("chef", "marin_coutelas", &["borin"]),
        &mut faces(&[15, 1]),
    ));
    assert!(step.events.contains(&FightEvent::DeathFailure {
        who: "borin".into(),
        failures: 1
    }));
    assert_eq!(step.fight.dying["borin"].successes, 1, "successes stay");
    let f = ok(step.fight.end_turn(&s, "chef", &mut faces(&[]))).fight;
    let f = ok(f.death_save(&s, "borin", &mut faces(&[12]))).fight;
    let f = ok(f.end_turn(&s, "lyra", &mut faces(&[]))).fight;
    // A natural 20 on the attack: two failures, the third proposes death.
    let step = ok(f.act(
        &s,
        &Play::on("chef", "marin_coutelas", &["borin"]),
        &mut faces(&[20, 1]),
    ));
    assert!(step.events.contains(&FightEvent::DeathFailure {
        who: "borin".into(),
        failures: 3
    }));
    assert!(step.events.contains(&FightEvent::DeathProposed {
        who: "borin".into()
    }));
}

#[test]
fn a_heal_lifts_him_and_clears_the_boxes() {
    let s = rules();
    let f = chief_downs_borin(&s, fight(&s, 1));
    let f = ok(f.death_save(&s, "borin", &mut faces(&[3]))).fight;
    assert_eq!(f.dying["borin"].failures, 1);
    let f = ok(f.act(
        &s,
        &Play::on("lyra", "premiers_soins", &["borin"]),
        &mut faces(&[]),
    ))
    .fight;
    assert_eq!(f.combatant("borin").unwrap().hit_points, 3);
    assert!(!f.dying.contains_key("borin"));
    assert!(!f.save_due(&s, "borin"));
}

#[test]
fn the_gm_may_decide_another_outcome() {
    let s = rules();
    let f = chief_downs_borin(&s, fight(&s, 1));
    let f = ok(f.death_save(&s, "borin", &mut faces(&[1]))).fight;
    let f = round_passes(&s, f);
    let f = ok(f.death_save(&s, "borin", &mut faces(&[4]))).fight;
    assert!(f.dying["borin"].proposed);
    let step = ok(f.decide_death("borin", DeathCall::Spare));
    assert_eq!(
        step.events,
        [FightEvent::Spared {
            who: "borin".into()
        }]
    );
    let f = step.fight;
    assert!(f.dying["borin"].stable);
    assert_eq!(f.standing["borin"], Standing::InFight);
    assert!(!f.save_due(&s, "borin"));
    assert_eq!(
        f.decide_death("borin", DeathCall::Die).unwrap_err(),
        CombatRefusal::NoDeathProposed
    );
}

#[test]
fn the_gm_still_decides_once_the_fight_is_lost() {
    let s = rules();
    // Borin alone against the chief.
    let mut borin = Combatant::from_class(&s, "borin", "Borin", "bretteur").unwrap();
    borin.hit_points = 1;
    let chef = Combatant::from_adversary(&s, "chef", "Chef", "marin_de_gueule_rouge").unwrap();
    let f = Fight::start(
        &s,
        "sol",
        room(),
        vec![(chef, Cell::new(1, 0)), (borin, Cell::new(0, 0))],
        &mut faces(&[20, 1]),
    )
    .unwrap()
    .fight;
    let step = ok(f.act(
        &s,
        &Play::on("chef", "marin_coutelas", &["borin"]),
        &mut faces(&[15, 1]),
    ));
    assert!(step.fight.is_over(), "nobody of the party stands");
    // Nobody is left to tend him: his death is proposed to the GM.
    let proposed = step
        .events
        .iter()
        .position(|e| {
            *e == FightEvent::DeathProposed {
                who: "borin".into(),
            }
        })
        .expect("death proposed");
    assert!(matches!(
        step.events[proposed + 1],
        FightEvent::Ended { .. }
    ));
    let after = ok(step.fight.decide_death("borin", DeathCall::Die)).fight;
    assert_eq!(after.standing["borin"], Standing::Dead);
    let spared = ok(step.fight.decide_death("borin", DeathCall::Spare)).fight;
    assert!(spared.dying["borin"].stable);
}

#[test]
fn a_won_fight_stabilises_the_dying() {
    let s = rules();
    let mut borin = Combatant::from_class(&s, "borin", "Borin", "bretteur").unwrap();
    borin.hit_points = 1;
    let anne = Combatant::from_class(&s, "anne", "Anne", "bretteur").unwrap();
    let mut chef = Combatant::from_adversary(&s, "chef", "Chef", "marin_de_gueule_rouge").unwrap();
    chef.hit_points = 1;
    let f = Fight::start(
        &s,
        "sol",
        room(),
        vec![
            (chef, Cell::new(1, 0)),
            (borin, Cell::new(0, 0)),
            (anne, Cell::new(2, 0)),
        ],
        &mut faces(&[20, 10, 1]),
    )
    .unwrap()
    .fight;
    let f = ok(f.act(
        &s,
        &Play::on("chef", "marin_coutelas", &["borin"]),
        &mut faces(&[15, 2]),
    ))
    .fight;
    let f = ok(f.end_turn(&s, "chef", &mut faces(&[]))).fight;
    let f = ok(f.death_save(&s, "borin", &mut faces(&[5]))).fight;
    assert_eq!(f.active(), Some("anne"));
    let step = ok(f.act(
        &s,
        &Play::on("anne", "estocade", &["chef"]),
        &mut faces(&[15]),
    ));
    assert!(step.fight.is_over());
    assert!(step.events.contains(&FightEvent::Stabilised {
        who: "borin".into()
    }));
    assert!(
        !step
            .events
            .iter()
            .any(|e| matches!(e, FightEvent::DeathProposed { .. }))
    );
    assert_eq!(step.fight.proposed_deaths().count(), 0);
}

#[test]
fn adversaries_never_roll_and_knocked_out_systems_are_unchanged() {
    let s = rules();
    let f = fight(&s, 10);
    assert!(!f.save_due(&s, "chef"));
    let corsaires = RuleSystem::from_yaml(
        &std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content/rules/corsaires/v1.yaml"),
        )
        .unwrap(),
    )
    .unwrap();
    let f = chief_downs_borin(&corsaires, fight(&corsaires, 1));
    assert!(!f.save_due(&corsaires, "borin"));
    assert_eq!(
        f.death_save(&corsaires, f.active().unwrap(), &mut faces(&[10]))
            .unwrap_err(),
        CombatRefusal::NoDeathSaveDue
    );
}
