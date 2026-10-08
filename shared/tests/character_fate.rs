//! engine/level-up and engine/save-against-death on both witness worlds.
//!
//! Neither world has hit points per level nor death saves today (both:
//! 10 hit points, knocked out then out of the scene). So each test runs
//! on the world as written — what a level brings there, what 0 hit
//! points does there — and on a variant that adds the rule the boards
//! draw (« Entre deux », « Mourir »), built the way a GM edit would be
//! (`rules::variant`).

use std::path::PathBuf;

use promptus_shared::combat::fight::CombatRefusal;
use promptus_shared::combat::{Fight, FightEvent, Play, Standing, Step};
use promptus_shared::maps::{Cell, Map};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::dice::ScriptedDice;
use promptus_shared::rules::events::Event;
use promptus_shared::rules::level_up::{
    HitPointMethod, LevelUpError, cards_unlocked, hit_point_bonus, hit_point_options,
    hit_points_due, take_hit_points,
};
use promptus_shared::rules::model::{RollSpec, Tag};
use promptus_shared::rules::progression::gain_xp;
use promptus_shared::rules::sheet::{Combatant, DeathSaves, Side};
use promptus_shared::rules::variant::{Variant, build_system};

const WORLDS: [&str; 2] = ["corsaires", "brasier"];

fn text(id: &str) -> String {
    let p =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../content/rules/{id}/v1.yaml"));
    std::fs::read_to_string(p).unwrap()
}

fn world(id: &str) -> RuleSystem {
    RuleSystem::from_yaml(&text(id)).unwrap()
}

fn variant(world: &str, set: &str) -> RuleSystem {
    let v: Variant =
        serde_yaml_ng::from_str(&format!("id: test\nname: Test\nset:\n{set}")).unwrap();
    build_system(&text(world), &[], Some(&v)).unwrap_or_else(|e| panic!("{e}"))
}

fn dice(faces: &[u32]) -> ScriptedDice {
    ScriptedDice::new(faces.iter().copied())
}

fn ok(r: Result<Step, CombatRefusal>) -> Step {
    r.unwrap_or_else(|e| panic!("refused: {e:?}"))
}

/// Who plays whom in each world: a fighter for Borin, a healer for
/// Lyra, a class the GM plays as the chief.
struct Cast {
    borin: &'static str,
    lyra: &'static str,
    chief: &'static str,
}

fn cast(world: &str) -> Cast {
    match world {
        "corsaires" => Cast {
            borin: "bretteur",
            lyra: "chirurgien",
            chief: "boucanier",
        },
        _ => Cast {
            borin: "canonnier",
            lyra: "toubib",
            chief: "pilote",
        },
    }
}

fn with_xp(s: &RuleSystem, mut c: Combatant, xp: u32) -> Combatant {
    let p = c.progress.as_mut().unwrap();
    gain_xp(s, p, xp);
    c
}

// ------------------------------------------------------------ engine/level-up

/// « Entre deux », moment 2: Borin reaches level 4, chooses between the
/// die and the average for its hit points, and finds his new card.
#[test]
fn borin_reaches_level_4_with_his_two_hit_point_options_and_his_new_card() {
    for w in WORLDS {
        let c = cast(w);
        let card = world(w).class(c.borin).unwrap().actions[1].id.clone();
        let s = variant(
            w,
            &format!(
                "  progression.hit_points_per_level: {{ dice: 1d10, average: 6, ability: CON }}\n  classes[{}].actions[{card}].level: 4\n",
                c.borin
            ),
        );
        let mut borin = Combatant::from_class(&s, "borin", "Borin", c.borin).unwrap();
        borin.abilities.insert("CON".into(), 16);
        let borin = with_xp(&s, borin, 15);
        assert_eq!(borin.level(&s), Some(4), "{w}");

        let options = hit_point_options(&s, &borin).unwrap().unwrap();
        assert_eq!(
            (options.dice.as_str(), options.average, options.modifier),
            ("1d10", 6, 3),
            "{w}: « 1d10 + Constitution » or « 6 + Constitution »"
        );
        // Levels 2 and 3 were never taken: they wait too.
        assert_eq!(hit_points_due(&s, 15, &[]), [2, 3, 4], "{w}");
        let rolled = take_hit_points(
            &s,
            &borin,
            4,
            HitPointMethod::Roll,
            &[2, 3],
            &mut dice(&[7]),
        )
        .unwrap();
        assert_eq!(
            (rolled.faces.as_slice(), rolled.amount),
            (&[7][..], 10),
            "{w}: 7 + 3"
        );
        let average = take_hit_points(
            &s,
            &borin,
            4,
            HitPointMethod::Average,
            &[2, 3],
            &mut dice(&[]),
        )
        .unwrap();
        assert_eq!(average.amount, 9, "{w}: 6 + 3, no die");
        assert_eq!(
            take_hit_points(
                &s,
                &borin,
                4,
                HitPointMethod::Roll,
                &[2, 3, 4],
                &mut dice(&[1])
            ),
            Err(LevelUpError::AlreadyChosen(4)),
            "{w}: a level is taken once, never rerolled"
        );
        assert_eq!(
            take_hit_points(&s, &borin, 5, HitPointMethod::Average, &[], &mut dice(&[])),
            Err(LevelUpError::NotALevel(5)),
            "{w}: not a level Borin has"
        );

        let new: Vec<&str> = cards_unlocked(&borin, &s, 3, 4)
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(new, [card.as_str()], "{w}");

        // The maximum: the rules' 10, plus what each level added.
        let gains = [
            take_hit_points(&s, &borin, 2, HitPointMethod::Average, &[], &mut dice(&[])).unwrap(),
            take_hit_points(&s, &borin, 3, HitPointMethod::Average, &[2], &mut dice(&[])).unwrap(),
            rolled,
        ];
        let mut levelled = borin.clone();
        levelled.hit_point_bonus = hit_point_bonus(&gains, 4);
        assert_eq!(levelled.max_hit_points(&s).unwrap(), 10 + 9 + 9 + 10, "{w}");
        // XP taken back to level 3: the level-4 die goes with it.
        assert_eq!(hit_point_bonus(&gains, 3), 18, "{w}");
    }
}

/// On the worlds as written: no hit point choice, the level-3 card.
#[test]
fn on_both_worlds_a_level_brings_the_class_card_of_that_level_and_no_hit_points() {
    for w in WORLDS {
        let s = world(w);
        let c = cast(w);
        let pc = with_xp(
            &s,
            Combatant::from_class(&s, "p", "P", c.borin).unwrap(),
            10,
        );
        assert_eq!(pc.level(&s), Some(3), "{w}");
        assert!(hit_point_options(&s, &pc).unwrap().is_none(), "{w}");
        assert!(hit_points_due(&s, 10, &[]).is_empty(), "{w}");
        assert_eq!(pc.max_hit_points(&s).unwrap(), 10, "{w}");
        let at_3: Vec<&str> = s
            .class(c.borin)
            .unwrap()
            .actions
            .iter()
            .filter(|a| a.level == Some(3))
            .map(|a| a.id.as_str())
            .collect();
        let new: Vec<&str> = cards_unlocked(&pc, &s, 1, 3)
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(new, at_3, "{w}");
        assert!(cards_unlocked(&pc, &s, 3, 3).is_empty(), "{w}");
    }
}

// --------------------------------------------------- engine/save-against-death

const DEATH_SAVES: &str = "  zero_hp: { rule: death_saves, condition: inconscient, difficulty: 10, successes: 3, failures: 3, failures_on_hit: 2, stabilize: { kind: soin, ability: SAG, difficulty: 10 } }\n";

fn room() -> Map {
    let rows: String = (0..4).map(|_| "    - \"......\"\n").collect();
    Map::from_yaml(&format!(
        "version: 1\nid: t\nname: Test\nscale: encounter\ntheme: test\ngrid:\n  legend:\n    \".\": {{ terrain: sol }}\n  rows:\n{rows}"
    ))
    .unwrap()
}

/// An attack of the chief's class with fixed damage (no damage die to
/// script).
fn chief_attack(s: &RuleSystem, class: &str) -> String {
    s.class(class)
        .unwrap()
        .actions
        .iter()
        .find(|a| {
            a.level.unwrap_or(1) == 1
                && a.roll == RollSpec::Attack
                && a.tags
                    .iter()
                    .any(|t| matches!(t, Tag::Damage(d) if d.amount.count == 0))
        })
        .map(|a| a.id.clone())
        .unwrap()
}

/// The crypt, round 1: the chief (played by the GM) first, then Borin —
/// down to his last hit point —, then Lyra next to him.
fn crypt(s: &RuleSystem, w: &str) -> Fight {
    let c = cast(w);
    let mut borin = Combatant::from_class(s, "borin", "Borin", c.borin).unwrap();
    borin.hit_points = 1;
    let lyra = Combatant::from_class(s, "lyra", "Lyra", c.lyra).unwrap();
    let mut chief = Combatant::from_class(s, "chef", "Chef gobelin", c.chief).unwrap();
    chief.side = Side::Opposition;
    chief.progress = None;
    let placements = vec![
        (borin, Cell::new(1, 1)),
        (lyra, Cell::new(1, 2)),
        (chief, Cell::new(2, 1)),
    ];
    let step = Fight::start(s, "sol", room(), placements, &mut dice(&[10, 5, 20])).unwrap();
    assert_eq!(step.fight.active(), Some("chef"), "{w}");
    step.fight
}

fn saves(f: &Fight, id: &str) -> Option<DeathSaves> {
    f.combatant(id).unwrap().death_saves
}

fn rules_events(step: &Step) -> Vec<&Event> {
    step.events
        .iter()
        .filter_map(|e| match e {
            FightEvent::Rules { event } => Some(event),
            _ => None,
        })
        .collect()
}

/// The seven moments of the board « Mourir · la dernière soirée de
/// Borin », on both worlds with death saves, the GM's word included.
#[test]
fn the_seven_moments_of_borins_death_replay_in_the_engine() {
    for w in WORLDS {
        let s = variant(w, DEATH_SAVES);
        let attack = chief_attack(&s, cast(w).chief);
        let f = crypt(&s, w);

        // 1. Borin is down: dying, not dead; his turn is his save.
        let hit = ok(f.act(&s, &Play::on("chef", &attack, &["borin"]), &mut dice(&[19])));
        assert!(
            rules_events(&hit)
                .iter()
                .any(|e| matches!(e, Event::KnockedOut { target } if target == "borin")),
            "{w}"
        );
        let f = hit.fight;
        assert_eq!(f.combatant("borin").unwrap().hit_points, 0, "{w}");
        assert_eq!(saves(&f, "borin"), Some(DeathSaves::default()), "{w}");
        assert_eq!(
            f.standing["borin"],
            Standing::InFight,
            "{w}: down, not dead"
        );
        let f = ok(f.end_turn(&s, "chef", &mut dice(&[]))).fight;
        assert_eq!(
            f.active(),
            Some("borin"),
            "{w}: the turn opens for the save"
        );
        assert!(f.awaits_death_save("borin"), "{w}");
        assert_eq!(
            f.end_turn(&s, "borin", &mut dice(&[])).unwrap_err(),
            CombatRefusal::DeathSaveFirst,
            "{w}: nothing else in that turn"
        );

        // 2. First save: 7, under 10: a failure. Lyra's turn.
        let save = ok(f.death_save(&s, "borin", &mut dice(&[7])));
        assert!(
            rules_events(&save).iter().any(|e| matches!(
                e,
                Event::DeathSaves { target, successes: 0, failures: 1 } if target == "borin"
            )),
            "{w}"
        );
        let f = save.fight;
        assert_eq!(f.active(), Some("lyra"), "{w}");

        // 3. Lyra tries to stabilise him: 4, missed. Her action is spent.
        let care = ok(f.stabilize(&s, "lyra", "borin", &mut dice(&[4])));
        assert!(saves(&care.fight, "borin").unwrap().rolling(), "{w}");
        assert_eq!(
            care.fight.combatant("lyra").unwrap().turn.actions_left,
            1,
            "{w}"
        );
        let f = ok(care.fight.end_turn(&s, "lyra", &mut dice(&[]))).fight;
        assert_eq!(f.active(), Some("chef"), "{w}: round 2");

        // 4. The GM plays the chief. The rules allow finishing Borin: two
        //    failures, the death proposed — never applied by the engine.
        let finish = ok(f.act(&s, &Play::on("chef", &attack, &["borin"]), &mut dice(&[19])));
        assert!(saves(&finish.fight, "borin").unwrap().death_due, "{w}");
        assert!(
            rules_events(&finish)
                .iter()
                .any(|e| matches!(e, Event::DeathDue { target } if target == "borin")),
            "{w}"
        );
        assert_eq!(finish.fight.standing["borin"], Standing::InFight, "{w}");
        //    The GM chooses mercy: the chief strikes Lyra and misses.
        let f = ok(f.act(&s, &Play::on("chef", &attack, &["lyra"]), &mut dice(&[2]))).fight;
        let f = ok(f.end_turn(&s, "chef", &mut dice(&[]))).fight;
        assert_eq!(f.active(), Some("borin"), "{w}");

        // 5. The last save: a natural 1, two failures. The engine proposes
        //    the death and waits; the GM confirms it.
        let last = ok(f.death_save(&s, "borin", &mut dice(&[1])));
        let f = last.fight;
        assert_eq!(
            saves(&f, "borin"),
            Some(DeathSaves {
                successes: 0,
                failures: 3,
                stable: false,
                death_due: true
            }),
            "{w}"
        );
        assert_eq!(f.deaths_due().collect::<Vec<_>>(), ["borin"], "{w}");
        assert_eq!(
            f.standing["borin"],
            Standing::InFight,
            "{w}: not before the GM"
        );
        assert_eq!(f.active(), Some("lyra"), "{w}");
        //    Another outcome is the GM's to decide too.
        let spared = ok(f.spare(&s, "borin", &mut dice(&[])));
        assert!(saves(&spared.fight, "borin").unwrap().stable, "{w}");
        assert_eq!(spared.fight.standing["borin"], Standing::InFight, "{w}");
        let died = ok(f.confirm_death(&s, "borin", &mut dice(&[])));
        assert!(
            died.events
                .iter()
                .any(|e| matches!(e, FightEvent::Died { who } if who == "borin")),
            "{w}"
        );
        let f = died.fight;
        assert_eq!(f.standing["borin"], Standing::Dead, "{w}");
        assert_eq!(f.position("borin"), None, "{w}");

        // 6. Borin never plays again: the turns go round without him.
        let f = ok(f.end_turn(&s, "lyra", &mut dice(&[]))).fight;
        let f = ok(f.end_turn(&s, "chef", &mut dice(&[]))).fight;
        assert_eq!(f.active(), Some("lyra"), "{w}: Borin's turn is skipped");

        // 7. When the fight ends, Borin is among the dead.
        let end = f.stop().fight.end.unwrap();
        assert_eq!(end.dead, ["borin"], "{w}");
        assert!(!end.out_of_scene.contains(&"borin".to_string()), "{w}");
    }
}

#[test]
fn three_successes_stabilise_a_natural_20_stands_up_and_a_heal_lifts_it_all() {
    for w in WORLDS {
        let s = variant(w, DEATH_SAVES);
        let attack = chief_attack(&s, cast(w).chief);
        let f = crypt(&s, w);
        let f = ok(f.act(&s, &Play::on("chef", &attack, &["borin"]), &mut dice(&[19]))).fight;
        let down = ok(f.end_turn(&s, "chef", &mut dice(&[]))).fight;

        // A natural 20: back with 1 hit point, and the turn is his.
        let up = ok(down.death_save(&s, "borin", &mut dice(&[20]))).fight;
        let borin = up.combatant("borin").unwrap();
        assert_eq!((borin.hit_points, borin.death_saves), (1, None), "{w}");
        assert!(borin.incapacitated_by().is_none(), "{w}");
        assert_eq!(up.active(), Some("borin"), "{w}");
        assert!(borin.turn.actions_left > 0, "{w}");

        // A heal from Lyra: on his feet, the tally gone.
        let failed = ok(down.death_save(&s, "borin", &mut dice(&[7]))).fight;
        let heal = s
            .class(cast(w).lyra)
            .unwrap()
            .actions
            .iter()
            .find(|a| a.level == Some(1) && a.tags.iter().any(|t| matches!(t, Tag::Heal(_))))
            .unwrap()
            .id
            .clone();
        let healed = ok(failed.act(&s, &Play::on("lyra", &heal, &["borin"]), &mut dice(&[]))).fight;
        let borin = healed.combatant("borin").unwrap();
        assert!(borin.hit_points > 0 && borin.death_saves.is_none(), "{w}");
        assert!(borin.incapacitated_by().is_none(), "{w}");

        // Three successes, turn after turn: stable, still down, no more saves.
        let mut f = down;
        for (i, face) in [12, 15, 10].into_iter().enumerate() {
            f = ok(f.death_save(&s, "borin", &mut dice(&[face]))).fight;
            assert_eq!(saves(&f, "borin").unwrap().successes, i as u32 + 1, "{w}");
            if i < 2 {
                f = ok(f.end_turn(&s, "lyra", &mut dice(&[]))).fight;
                f = ok(f.end_turn(&s, "chef", &mut dice(&[]))).fight;
            }
        }
        assert!(saves(&f, "borin").unwrap().stable, "{w}");
        assert!(!f.awaits_death_save("borin"), "{w}");
        assert_eq!(f.combatant("borin").unwrap().hit_points, 0, "{w}");
        // A stable Borin's turn passes on its own.
        let f = ok(f.end_turn(&s, "lyra", &mut dice(&[]))).fight;
        let f = ok(f.end_turn(&s, "chef", &mut dice(&[]))).fight;
        assert_eq!(f.active(), Some("lyra"), "{w}");
    }
}

#[test]
fn under_knocked_out_no_death_is_ever_proposed_but_the_gm_may_still_decide_one() {
    for w in WORLDS {
        let s = world(w);
        let attack = chief_attack(&s, cast(w).chief);
        let f = crypt(&s, w);
        let hit = ok(f.act(&s, &Play::on("chef", &attack, &["borin"]), &mut dice(&[19])));
        assert_eq!(
            saves(&hit.fight, "borin"),
            None,
            "{w}: knocked out, no saves"
        );
        let f = ok(hit.fight.end_turn(&s, "chef", &mut dice(&[]))).fight;
        assert_eq!(f.active(), Some("lyra"), "{w}: a knocked-out turn passes");
        assert_eq!(f.deaths_due().count(), 0, "{w}");
        assert_eq!(
            f.death_save(&s, "lyra", &mut dice(&[10])).unwrap_err(),
            CombatRefusal::NotDying { who: "lyra".into() },
            "{w}"
        );
        assert_eq!(
            f.confirm_death(&s, "lyra", &mut dice(&[])).unwrap_err(),
            CombatRefusal::NotDown { who: "lyra".into() },
            "{w}: only someone down at 0 can die"
        );
        let died = ok(f.confirm_death(&s, "borin", &mut dice(&[]))).fight;
        assert_eq!(died.standing["borin"], Standing::Dead, "{w}: the GM's word");
    }
}

/// A fight played without a table (the simulator) rolls the dying
/// party's saves itself and still ends.
#[test]
fn a_fight_played_without_a_table_rolls_the_saves_and_ends() {
    use promptus_shared::combat::{Brawler, EndReason, Limits, run_fight};
    use promptus_shared::rules::dice::SeededDice;
    for w in WORLDS {
        let s = variant(w, DEATH_SAVES);
        for seed in 0..10 {
            let f = crypt(&s, w);
            let log = run_fight(
                &s,
                Step {
                    fight: f,
                    events: Vec::new(),
                },
                &mut Brawler::default(),
                &mut Brawler::default(),
                &mut SeededDice::new(seed),
                Limits::default(),
            );
            let end = log.fight.end.as_ref().expect("ended");
            assert_ne!(end.reason, EndReason::Stalemate, "{w} seed {seed}");
            assert_eq!(log.refusals().count(), 0, "{w} seed {seed}");
        }
    }
}
