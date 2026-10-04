//! The engine on the two witness worlds, loaded from `content/rules/`.
//! Every behaviour is pinned on both systems where both have it: a
//! ticket is done only when it works on the Corsaires and the Brasier.

use std::path::PathBuf;

use promptus_shared::rules::action::{
    ActionRequest, Refusal, Resolution, action_cards, resolve_action,
};
use promptus_shared::rules::check::{
    self, Advantage, Modifier, ModifierSource, OutcomeBand, RollTarget, ability_check, group_check,
};
use promptus_shared::rules::conditions::{end_turn, start_turn};
use promptus_shared::rules::dice::{ScriptedDice, SeededDice};
use promptus_shared::rules::events::{Event, RollPurpose};
use promptus_shared::rules::model::{ConditionEffect, RollScope};
use promptus_shared::rules::progression::{
    ProgressEvent, award_band, gain_xp, spend_upgrade_point,
};
use promptus_shared::rules::sheet::{Combatant, Scene, Side};
use promptus_shared::rules::{ErrorCode, RuleSystem};

fn path(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../content/rules/{id}/v1.yaml"))
}

fn text(id: &str) -> String {
    std::fs::read_to_string(path(id)).unwrap()
}

fn system(id: &str) -> RuleSystem {
    RuleSystem::from_yaml(&text(id)).unwrap_or_else(|e| panic!("{id} does not load:\n{e}"))
}

/// The same system with one edit to its file — what Romain does.
fn edited(id: &str, from: &str, to: &str) -> RuleSystem {
    let t = text(id);
    assert!(t.contains(from), "`{from}` is not in {id}");
    RuleSystem::from_yaml(&t.replacen(from, to, 1)).unwrap()
}

fn pc(s: &RuleSystem, id: &str, class: &str) -> Combatant {
    Combatant::from_class(s, id, id, class).unwrap()
}

/// A class character fighting on the other side (the Brasier has no
/// ground stat block yet).
fn rival(s: &RuleSystem, id: &str, class: &str) -> Combatant {
    let mut c = pc(s, id, class);
    c.side = Side::Opposition;
    c
}

fn foe(s: &RuleSystem, id: &str, adversary: &str) -> Combatant {
    Combatant::from_adversary(s, id, id, adversary).unwrap()
}

fn with_xp(s: &RuleSystem, mut c: Combatant, xp: u32) -> Combatant {
    gain_xp(s, c.progress.as_mut().unwrap(), xp);
    c
}

fn begin(s: &RuleSystem, scene: &Scene, who: &str) -> Scene {
    start_turn(s, scene, who).unwrap().0
}

fn finish(s: &RuleSystem, scene: &Scene, who: &str) -> Scene {
    end_turn(s, scene, who, &mut ScriptedDice::new([]))
        .unwrap()
        .0
}

/// Resolves with exactly these faces: an extra or a missing roll fails.
fn play(s: &RuleSystem, scene: &Scene, req: &ActionRequest, faces: &[u32]) -> Resolution {
    let mut dice = ScriptedDice::new(faces.iter().copied());
    let r = resolve_action(s, scene, req, &mut dice).unwrap_or_else(|e| panic!("refused: {e:?}"));
    assert_eq!(dice.remaining(), 0, "faces left unrolled");
    r
}

fn refused(s: &RuleSystem, scene: &Scene, req: &ActionRequest) -> Refusal {
    let before = scene.clone();
    let r =
        resolve_action(s, scene, req, &mut ScriptedDice::new([])).expect_err("should be refused");
    assert_eq!(scene, &before);
    r
}

fn hp(scene: &Scene, id: &str) -> i32 {
    scene.get(id).unwrap().hit_points
}

fn xp(scene: &Scene, id: &str) -> u32 {
    scene.get(id).unwrap().progress.unwrap().total_xp
}

fn has(scene: &Scene, id: &str, condition: &str) -> bool {
    scene.get(id).unwrap().condition_named(condition).is_some()
}

fn attack_rolls(r: &Resolution) -> Vec<&check::RollBreakdown> {
    r.events
        .iter()
        .filter_map(|e| match e {
            Event::Roll {
                purpose: RollPurpose::Attack,
                breakdown,
                ..
            } => Some(breakdown),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------- loading

#[test]
fn both_witness_systems_load_with_their_classes() {
    let c = system("corsaires");
    assert_eq!((c.id.as_str(), c.version), ("corsaires", 1));
    assert_eq!(c.classes.len(), 8);
    let b = system("brasier");
    assert_eq!(b.classes.len(), 6);
    // Armour class comes from the formula: 10 + floor((DEX - 10) / 2).
    for (s, class, ac) in [
        (&c, "bretteur", 12),
        (&c, "canonnier", 10),
        (&c, "quartier_maitre", 9),
        (&b, "mecano", 11),
        (&b, "pilote", 12),
    ] {
        let sheet = pc(s, "x", class);
        assert_eq!(sheet.armor_class(s).unwrap(), ac, "{class}");
        assert_eq!(sheet.hit_points, 10);
    }
    for s in [&c, &b] {
        for class in &s.classes {
            let levels: Vec<_> = class.actions.iter().map(|a| a.level).collect();
            assert_eq!(levels, vec![Some(1), Some(3), Some(7)], "{}", class.id);
        }
    }
}

#[test]
fn a_broken_reference_in_a_world_file_is_named() {
    let t = text("corsaires").replacen("condition: renverse", "condition: renverser", 1);
    let err = RuleSystem::from_yaml(&t).unwrap_err();
    assert_eq!(err.codes(), vec![ErrorCode::UnknownCondition]);
    assert_eq!(
        err.0[0].path,
        "classes[canonnier].actions[salve_de_bordee].tags[2].condition"
    );
}

// ----------------------------------------------------------------- checks

#[test]
fn a_check_lands_in_its_band_with_its_breakdown_on_both_worlds() {
    for (id, class) in [("corsaires", "navigateur"), ("brasier", "xenologue")] {
        let s = system(id);
        let sheet = pc(&s, "p", class); // SAG 14 → +2
        let moyen = check::difficulty(&s, "moyen").unwrap();
        let roll = |face, target: Option<RollTarget>| {
            ability_check(
                &s,
                &sheet,
                "SAG",
                RollScope::Checks,
                target,
                Advantage::Normal,
                &mut ScriptedDice::new([face]),
            )
            .unwrap()
        };

        let r = roll(8, Some(moyen.clone()));
        assert_eq!(r.faces, vec![8]);
        assert_eq!(
            r.modifiers,
            vec![Modifier {
                source: ModifierSource::Ability("SAG".into()),
                value: 2
            }]
        );
        assert_eq!(r.total, 10);
        assert_eq!(
            r.target,
            Some(RollTarget::Difficulty {
                id: Some("moyen".into()),
                value: 10
            })
        );
        assert_eq!(
            r.band,
            Some(OutcomeBand::Success),
            "{id}: meeting the threshold succeeds"
        );
        assert_eq!(roll(7, Some(moyen)).band, Some(OutcomeBand::Failure));

        let easy = Some(RollTarget::Difficulty { id: None, value: 3 });
        assert_eq!(
            roll(1, easy).band,
            Some(OutcomeBand::CriticalFailure),
            "a natural 1 fails whatever the total"
        );
        let impossible = Some(RollTarget::Difficulty {
            id: None,
            value: 30,
        });
        assert_eq!(
            roll(20, impossible).band,
            Some(OutcomeBand::CriticalSuccess)
        );
        assert_eq!(roll(12, None).band, None, "no target, no band yet");
    }
}

#[test]
fn successful_rolls_grant_the_xp_their_band_says() {
    for id in ["corsaires", "brasier"] {
        let s = system(id);
        let mut sheet = pc(&s, "p", &s.classes[0].id);
        for band in [
            OutcomeBand::Success,
            OutcomeBand::CriticalSuccess,
            OutcomeBand::Failure,
            OutcomeBand::CriticalFailure,
        ] {
            award_band(&s, &mut sheet, Some(band));
        }
        assert_eq!(
            sheet.progress.unwrap().total_xp,
            2,
            "{id}: +1 per success, critical included"
        );
    }
}

#[test]
fn group_check_succeeds_when_half_the_group_does() {
    for (id, class) in [("corsaires", "navigateur"), ("brasier", "xenologue")] {
        let s = system(id);
        let crew: Vec<Combatant> = (0..6).map(|i| pc(&s, &format!("p{i}"), class)).collect();
        let members: Vec<&Combatant> = crew.iter().collect();
        let moyen = check::difficulty(&s, "moyen").unwrap();
        let run = |faces: [u32; 6]| {
            group_check(
                &s,
                &members,
                "SAG",
                moyen.clone(),
                &mut ScriptedDice::new(faces),
            )
            .unwrap()
        };

        let half = run([8, 9, 10, 2, 3, 4]);
        assert_eq!(
            (half.successes, half.needed, half.success),
            (3, 3, true),
            "{id}"
        );
        assert!(!run([8, 9, 2, 2, 3, 4]).success);
        assert_eq!(half.rolls.len(), 6);

        let five = &members[..5];
        let r = group_check(
            &s,
            five,
            "SAG",
            moyen.clone(),
            &mut ScriptedDice::new([8, 9, 2, 2, 3]),
        )
        .unwrap();
        assert!(!r.success, "2 of 5 is less than half");
    }
}

#[test]
fn advantage_keeps_the_best_die_only_in_a_system_that_has_it() {
    let s = system("corsaires");
    let sheet = pc(&s, "p", "navigateur");
    let refusal = ability_check(
        &s,
        &sheet,
        "SAG",
        RollScope::Checks,
        None,
        Advantage::Advantage,
        &mut ScriptedDice::new([3, 17]),
    );
    assert_eq!(refusal, Err(check::CheckError::AdvantageNotInSystem));

    let s = edited(
        "corsaires",
        "check: { dice: 1d20, advantage: false }",
        "check: { dice: 1d20, advantage: true }",
    );
    let roll = |adv| check::roll(&s, vec![], adv, None, &mut ScriptedDice::new([3, 17])).unwrap();
    let best = roll(Advantage::Advantage);
    assert_eq!((best.faces.clone(), best.natural), (vec![3, 17], 17));
    assert_eq!(roll(Advantage::Disadvantage).natural, 3);
    assert_eq!(Advantage::combine(true, true), Advantage::Normal);
    // One die only when they cancel out.
    let mut dice = ScriptedDice::new([11]);
    let normal = check::roll(&s, vec![], Advantage::combine(true, true), None, &mut dice).unwrap();
    assert_eq!(normal.faces, vec![11]);
}

#[test]
fn a_condition_can_impose_disadvantage_which_granted_advantage_cancels() {
    let s = RuleSystem::from_yaml(
        &text("corsaires")
            .replacen(
                "check: { dice: 1d20, advantage: false }",
                "check: { dice: 1d20, advantage: true }",
                1,
            )
            .replacen(
                "effects: [{ roll_modifier: { rolls: all, value: -1 } }]",
                "effects: [{ disadvantage: { rolls: checks } }]",
                1,
            ),
    )
    .unwrap();
    let mut sheet = pc(&s, "p", "navigateur");
    let poison = promptus_shared::rules::conditions::named(&s, "empoisonne", Some(2), "x").unwrap();
    sheet.conditions.push(poison);
    let roll = |adv| {
        ability_check(
            &s,
            &sheet,
            "SAG",
            RollScope::Checks,
            None,
            adv,
            &mut ScriptedDice::new([17, 3]),
        )
        .unwrap()
    };
    let r = roll(Advantage::Normal);
    assert_eq!((r.advantage, r.natural), (Advantage::Disadvantage, 3));
    let r = roll(Advantage::Advantage);
    assert_eq!((r.advantage, r.faces), (Advantage::Normal, vec![17]));
    // Attacks are not checks: the same condition leaves them alone.
    let attack = ability_check(
        &s,
        &sheet,
        "SAG",
        RollScope::Attacks,
        None,
        Advantage::Normal,
        &mut ScriptedDice::new([17]),
    )
    .unwrap();
    assert_eq!(attack.advantage, Advantage::Normal);
}

// ---------------------------------------------------------------- attacks

#[test]
fn an_attack_hits_misses_and_crits_on_both_worlds() {
    // (world, attacker class, action, ability bonus, target, its AC, its HP, fixed damage)
    let cases = [
        (
            "corsaires",
            "bretteur",
            "estocade",
            1,
            Some("marin_de_gueule_rouge"),
            10,
            4,
            3,
        ),
        ("brasier", "pilote", "tir_reflexe", 2, None, 10, 10, 3),
    ];
    for (id, class, action, bonus, adversary, ac, hp0, dmg) in cases {
        let s = system(id);
        let target = match adversary {
            Some(a) => foe(&s, "t", a),
            None => rival(&s, "t", "canonnier"),
        };
        assert_eq!(target.armor_class(&s).unwrap(), ac);
        let scene = begin(&s, &Scene::new("sol", [pc(&s, "a", class), target]), "a");
        let req = ActionRequest::new("a", action, &["t"]);

        let face_to_hit = (ac - bonus) as u32;
        let hit = play(&s, &scene, &req, &[face_to_hit]);
        let roll = attack_rolls(&hit)[0];
        // Precision is not part of the roll in either draft.
        assert_eq!(roll.modifiers.len(), 1, "{id}: {:?}", roll.modifiers);
        assert_eq!(roll.total, ac);
        assert_eq!(roll.band, Some(OutcomeBand::Success));
        assert_eq!(hp(&hit.scene, "t"), hp0 - dmg);
        assert_eq!(xp(&hit.scene, "a"), 1);

        let miss = play(&s, &scene, &req, &[face_to_hit - 1]);
        assert_eq!(hp(&miss.scene, "t"), hp0);
        assert_eq!(xp(&miss.scene, "a"), 0);
        assert!(miss.events.contains(&Event::Missed { target: "t".into() }));

        let crit = play(&s, &scene, &req, &[20]);
        assert_eq!(
            hp(&crit.scene, "t"),
            (hp0 - 2 * dmg).max(0),
            "{id}: a natural 20 doubles"
        );
        assert_eq!(xp(&crit.scene, "a"), 1);
    }
}

#[test]
fn a_natural_one_misses_even_against_a_low_armour_class() {
    let s = edited(
        "corsaires",
        "armor_class: 10\n    hit_points: 4",
        "armor_class: 3\n    hit_points: 4",
    );
    let scene = begin(
        &s,
        &Scene::new(
            "sol",
            [
                pc(&s, "a", "flibustier"),
                foe(&s, "t", "marin_de_gueule_rouge"),
            ],
        ),
        "a",
    );
    // FOR 14 → +2: a 1 totals 3, enough for AC 3, but it is a critical failure.
    let r = play(
        &s,
        &scene,
        &ActionRequest::new("a", "coup_de_hache", &["t"]),
        &[1],
    );
    let roll = attack_rolls(&r)[0];
    assert_eq!(roll.total, 3);
    assert_eq!(roll.band, Some(OutcomeBand::CriticalFailure));
    assert_eq!(hp(&r.scene, "t"), 4);
    assert_eq!(xp(&r.scene, "a"), 0);
}

#[test]
fn adversaries_roll_dice_damage_from_their_stat_block() {
    let s = system("corsaires");
    let scene = begin(
        &s,
        &Scene::new(
            "sol",
            [foe(&s, "gr", "gueule_rouge"), pc(&s, "p", "canonnier")],
        ),
        "gr",
    );
    // Sabre d'abordage 1d6+2 with FOR +2: 8 + 2 = 10 vs AC 10, then a 4.
    let r = play(
        &s,
        &scene,
        &ActionRequest::new("gr", "gueule_rouge_sabre", &["p"]),
        &[8, 4],
    );
    assert_eq!(hp(&r.scene, "p"), 10 - 6);
    let damaged = r.events.iter().find_map(|e| match e {
        Event::Damaged { breakdown, .. } => Some(breakdown),
        _ => None,
    });
    assert_eq!(damaged.unwrap().amount, "1d6+2");
    assert_eq!(damaged.unwrap().faces, vec![4]);
}

#[test]
fn situational_bonus_only_when_declared() {
    let s = system("corsaires");
    let scene = begin(
        &s,
        &Scene::new(
            "sol",
            [pc(&s, "b", "boucanier"), foe(&s, "gr", "gueule_rouge")],
        ),
        "b",
    );
    let mut req = ActionRequest::new("b", "tir_furtif", &["gr"]);
    assert_eq!(hp(&play(&s, &scene, &req, &[15]).scene, "gr"), 7);
    req.situations = vec!["furtif".into()];
    assert_eq!(hp(&play(&s, &scene, &req, &[15]).scene, "gr"), 5);
    req.situations = vec!["dans_le_noir".into()];
    assert_eq!(
        refused(&s, &scene, &req),
        Refusal::UnknownSituation {
            situation: "dans_le_noir".into()
        }
    );
}

#[test]
fn an_automatic_critical_doubles_without_a_roll_or_xp() {
    let s = system("corsaires");
    let b = with_xp(&s, pc(&s, "b", "boucanier"), 30);
    let scene = begin(
        &s,
        &Scene::new("sol", [b, foe(&s, "gr", "gueule_rouge")]),
        "b",
    );
    let r = play(
        &s,
        &scene,
        &ActionRequest::new("b", "embuscade_mortelle", &["gr"]),
        &[],
    );
    assert!(attack_rolls(&r).is_empty());
    assert_eq!(hp(&r.scene, "gr"), 0, "8 doubled is 16");
    assert!(has(&r.scene, "gr", "inconscient"));
    assert_eq!(xp(&r.scene, "b"), 30);
}

// -------------------------------------------------------------- cooldowns

#[test]
fn a_cooldown_blocks_then_frees_the_action_on_both_worlds() {
    for (id, class, action, target_class) in [
        ("corsaires", "canonnier", "tir_de_silex", "vigie"),
        ("brasier", "canonnier", "rafale_lourde", "pilote"),
    ] {
        let s = system(id);
        let start = Scene::new("sol", [pc(&s, "a", class), rival(&s, "t", target_class)]);
        let req = ActionRequest::new("a", action, &["t"]);

        let turn1 = begin(&s, &start, "a");
        let shot = play(&s, &turn1, &req, &[2]);
        assert!(
            shot.events
                .iter()
                .any(|e| matches!(e, Event::CooldownStarted { counter: 2, .. }))
        );
        assert_eq!(
            refused(&s, &shot.scene, &req),
            Refusal::OnCooldown { counter: 2 },
            "{id}: not twice in a turn"
        );

        let turn2 = begin(&s, &finish(&s, &shot.scene, "a"), "a");
        assert_eq!(
            refused(&s, &turn2, &req),
            Refusal::OnCooldown { counter: 1 },
            "{id}: CD 1 skips the next turn"
        );

        let turn3 = begin(&s, &finish(&s, &turn2, "a"), "a");
        play(&s, &turn3, &req, &[2]);
    }
}

#[test]
fn an_action_without_cooldown_can_be_played_twice_in_a_turn() {
    let s = system("corsaires");
    let scene = begin(
        &s,
        &Scene::new(
            "sol",
            [pc(&s, "a", "bretteur"), foe(&s, "gr", "gueule_rouge")],
        ),
        "a",
    );
    let req = ActionRequest::new("a", "estocade", &["gr"]);
    let once = play(&s, &scene, &req, &[2]);
    let twice = play(&s, &once.scene, &req, &[2]);
    assert_eq!(
        refused(&s, &twice.scene, &req),
        Refusal::NotEnoughActions { cost: 1, left: 0 },
        "two actions per turn"
    );
}

// ------------------------------------------------------------ level gates

#[test]
fn a_level_gated_action_is_refused_then_allowed_on_both_worlds() {
    for (id, class, action, target, ally) in [
        (
            "corsaires",
            "bretteur",
            "riposte_en_quarte",
            Some("gr"),
            false,
        ),
        ("brasier", "toubib", "garde_du_corps", Some("ally"), true),
    ] {
        let s = system(id);
        let other = if ally {
            pc(&s, "ally", "pilote")
        } else {
            rival(&s, "gr", "canonnier")
        };
        let targets: Vec<&str> = target.into_iter().collect();
        let req = ActionRequest::new("a", action, &targets);
        let at = |xp_total| {
            begin(
                &s,
                &Scene::new(
                    "sol",
                    [with_xp(&s, pc(&s, "a", class), xp_total), other.clone()],
                ),
                "a",
            )
        };

        assert_eq!(
            refused(&s, &at(0), &req),
            Refusal::LevelTooLow {
                required: 3,
                current: 1
            },
            "{id}"
        );
        assert_eq!(
            refused(&s, &at(9), &req),
            Refusal::LevelTooLow {
                required: 3,
                current: 2
            }
        );
        let faces: &[u32] = if ally { &[] } else { &[2] };
        play(&s, &at(10), &req, faces);
    }
}

#[test]
fn the_cards_offered_derive_from_class_and_level() {
    for (id, class, first, bonus) in [
        ("corsaires", "bretteur", "estocade", 1),
        ("brasier", "pilote", "tir_reflexe", 2),
    ] {
        let s = system(id);
        let sheet = pc(&s, "a", class);
        let cards = action_cards(&s, &sheet);
        assert_eq!(cards.len(), 3);
        assert_eq!(cards[0].action.id, first);
        assert_eq!(cards[0].locked, None);
        assert_eq!(
            cards[0].attack_bonus,
            Some(bonus),
            "{id}: precision not added"
        );
        assert_eq!(
            cards[1].locked,
            Some(Refusal::LevelTooLow {
                required: 3,
                current: 1
            })
        );
        assert_eq!(
            cards[2].locked,
            Some(Refusal::LevelTooLow {
                required: 7,
                current: 1
            })
        );
    }
}

#[test]
fn deciding_that_precision_counts_changes_the_cards_and_the_roll() {
    let s = edited(
        "corsaires",
        "precision: not_applied",
        "precision: added_to_attack_roll",
    );
    let sheet = pc(&s, "a", "bretteur");
    assert_eq!(action_cards(&s, &sheet)[0].attack_bonus, Some(3));
    let scene = begin(
        &s,
        &Scene::new("sol", [sheet, foe(&s, "m", "marin_de_gueule_rouge")]),
        "a",
    );
    // 7 + FOR 1 + precision 2 = 10: a hit that the draft calls a miss.
    let r = play(
        &s,
        &scene,
        &ActionRequest::new("a", "estocade", &["m"]),
        &[7],
    );
    let roll = attack_rolls(&r)[0];
    assert!(roll.modifiers.contains(&Modifier {
        source: ModifierSource::Precision("estocade".into()),
        value: 2
    }));
    assert_eq!(roll.band, Some(OutcomeBand::Success));
}

#[test]
fn a_free_slot_holds_one_learned_action() {
    let s = system("corsaires");
    let mut sheet = pc(&s, "a", "bretteur");
    let mut learned = s.class("vigie").unwrap().actions[0].clone();
    learned.id = "tir_appris".into();
    sheet.learn_action(&s, learned.clone()).unwrap();
    assert!(
        action_cards(&s, &sheet)
            .iter()
            .any(|c| c.action.id == "tir_appris")
    );
    learned.id = "autre".into();
    assert!(sheet.learn_action(&s, learned).is_err(), "one free slot");
}

// ------------------------------------------------------------- conditions

#[test]
fn a_condition_applies_then_expires_on_both_worlds() {
    // Corsaires: the trap immobilises for 2 of the target's turns.
    let s = system("corsaires");
    let b = with_xp(&s, pc(&s, "b", "boucanier"), 10);
    let scene = begin(
        &s,
        &Scene::new("sol", [b, foe(&s, "gr", "gueule_rouge")]),
        "b",
    );
    let r = play(
        &s,
        &scene,
        &ActionRequest::new("b", "piege_a_machoires", &["gr"]),
        &[15],
    );
    let mut scene = finish(&s, &r.scene, "b");
    assert!(!scene.get("gr").unwrap().can_move());
    scene = finish(&s, &begin(&s, &scene, "gr"), "gr");
    assert!(!scene.get("gr").unwrap().can_move(), "one turn left");
    let (after, events) = end_turn(
        &s,
        &begin(&s, &scene, "gr"),
        "gr",
        &mut ScriptedDice::new([]),
    )
    .unwrap();
    assert!(after.get("gr").unwrap().can_move());
    assert!(events.contains(&Event::ConditionEnded {
        target: "gr".into(),
        name: "Immobilisé".into()
    }));

    // Brasier: the plasma grenade stuns unless a CON save succeeds.
    let s = system("brasier");
    let c = with_xp(&s, pc(&s, "c", "canonnier"), 10);
    let scene = begin(&s, &Scene::new("sol", [c, rival(&s, "t", "pilote")]), "c");
    let mut req = ActionRequest::new("c", "grenade_a_plasma", &["t"]);
    assert_eq!(
        refused(&s, &scene, &req),
        Refusal::DifficultyRequired,
        "the source gives no difficulty"
    );
    req.save_difficulty = Some(10);
    let resisted = play(&s, &scene, &req, &[15, 15]);
    assert!(!has(&resisted.scene, "t", "etourdi"));
    let stunned = play(&s, &scene, &req, &[15, 3]);
    assert!(has(&stunned.scene, "t", "etourdi"));
    let (turn, events) = start_turn(&s, &finish(&s, &stunned.scene, "c"), "t").unwrap();
    assert!(events.contains(&Event::TurnLost {
        who: "t".into(),
        because: "Étourdi".into()
    }));
    assert_eq!(
        refused(&s, &turn, &ActionRequest::new("t", "tir_reflexe", &["c"])),
        Refusal::Incapacitated {
            because: "Étourdi".into()
        }
    );
    assert!(!has(&finish(&s, &turn, "t"), "t", "etourdi"));
}

#[test]
fn a_self_buff_covers_the_enemy_turn_and_ends_after_the_next_own_turn() {
    let s = system("corsaires");
    let br = with_xp(&s, pc(&s, "br", "bretteur"), 10);
    let scene = begin(
        &s,
        &Scene::new("sol", [br, foe(&s, "gr", "gueule_rouge")]),
        "br",
    );
    let r = play(
        &s,
        &scene,
        &ActionRequest::new("br", "riposte_en_quarte", &["gr"]),
        &[2],
    );
    let sabre = ActionRequest::new("gr", "gueule_rouge_sabre", &["br"]);
    // 15 + FOR 2 hits AC 12; 3 + 2 = 5 damage, -2 from the riposte.
    let hit = play(
        &s,
        &begin(&s, &finish(&s, &r.scene, "br"), "gr"),
        &sabre,
        &[15, 3],
    );
    assert_eq!(hp(&hit.scene, "br"), 7);
    let next = finish(&s, &begin(&s, &finish(&s, &hit.scene, "gr"), "br"), "br");
    let again = play(&s, &begin(&s, &next, "gr"), &sabre, &[15, 3]);
    assert_eq!(hp(&again.scene, "br"), 2, "the buff is gone");
}

#[test]
fn a_contest_lets_the_actor_win_ties_on_both_worlds() {
    for (id, action, target_class) in [
        ("corsaires", "intimidation", "vigie"),
        ("brasier", "sommation", "pilote"),
    ] {
        let s = system(id);
        let qm = with_xp(&s, pc(&s, "qm", "quartier_maitre"), 10); // CHA 14 → +2
        let scene = begin(
            &s,
            &Scene::new("sol", [qm, rival(&s, "t", target_class)]),
            "qm",
        );
        let req = ActionRequest::new("qm", action, &["t"]);
        let sag = scene.get("t").unwrap().modifier(&s, "SAG").unwrap();
        // Target rolls first: 10 + SAG; the actor ties it exactly.
        let tie = (10 + sag - 2) as u32;
        let won = play(&s, &scene, &req, &[10, tie]);
        assert!(has(&won.scene, "t", "apeure"), "{id}");
        assert_eq!(xp(&won.scene, "qm"), 11);
        let lost = play(&s, &scene, &req, &[10, tie - 1]);
        assert!(!has(&lost.scene, "t", "apeure"));
    }
}

#[test]
fn a_marked_weak_point_is_spent_by_the_next_attack() {
    let s = system("corsaires");
    let vigie = with_xp(&s, pc(&s, "v", "vigie"), 10);
    let scene = Scene::new(
        "sol",
        [
            vigie,
            pc(&s, "br", "bretteur"),
            foe(&s, "gr", "gueule_rouge"),
        ],
    );
    let marked = play(
        &s,
        &begin(&s, &scene, "v"),
        &ActionRequest::new("v", "point_faible_repere", &["gr"]),
        &[],
    );
    let scene = begin(&s, &finish(&s, &marked.scene, "v"), "br");
    let estocade = ActionRequest::new("br", "estocade", &["gr"]);
    let first = play(&s, &scene, &estocade, &[15]);
    assert_eq!(
        hp(&first.scene, "gr"),
        10 - 5,
        "3 + 2 against the weak point"
    );
    assert!(!has(&first.scene, "gr", "point_faible"));
    assert_eq!(hp(&play(&s, &first.scene, &estocade, &[15]).scene, "gr"), 2);
}

#[test]
fn a_poisoned_blade_poisons_whoever_the_ally_hits() {
    let s = system("corsaires");
    let chir = with_xp(&s, pc(&s, "ch", "chirurgien"), 10);
    let scene = Scene::new(
        "sol",
        [
            chir,
            pc(&s, "br", "bretteur"),
            foe(&s, "gr", "gueule_rouge"),
        ],
    );
    let coated = play(
        &s,
        &begin(&s, &scene, "ch"),
        &ActionRequest::new("ch", "lame_empoisonnee", &["br"]),
        &[],
    );
    let scene = begin(&s, &finish(&s, &coated.scene, "ch"), "br");
    let hit = play(
        &s,
        &scene,
        &ActionRequest::new("br", "estocade", &["gr"]),
        &[15],
    );
    assert_eq!(hp(&hit.scene, "gr"), 5);
    assert!(has(&hit.scene, "gr", "empoisonne"));
    let gr = hit.scene.get("gr").unwrap();
    let r = ability_check(
        &s,
        gr,
        "DEX",
        RollScope::Checks,
        None,
        Advantage::Normal,
        &mut ScriptedDice::new([10]),
    )
    .unwrap();
    assert_eq!(r.total, 10 + 1 - 1, "DEX +1, poison -1 to every roll");
}

#[test]
fn an_option_must_be_picked_when_the_card_offers_a_choice() {
    let s = system("brasier");
    let mec = with_xp(&s, pc(&s, "m", "mecano"), 10);
    let scene = begin(&s, &Scene::new("sol", [mec, rival(&s, "t", "pilote")]), "m");
    let mut req = ActionRequest::new("m", "piratage_de_combat", &["t"]);
    assert_eq!(
        refused(&s, &scene, &req),
        Refusal::ChoiceRequired { options: 2 }
    );
    req.choice = Some(1);
    let r = play(&s, &scene, &req, &[]);
    let t = r.scene.get("t").unwrap();
    assert!(
        t.effects()
            .any(|(_, e)| *e == ConditionEffect::DamageDealt(-2))
    );
    assert!(
        !t.effects()
            .any(|(_, e)| matches!(e, ConditionEffect::RollModifier { .. }))
    );
}

#[test]
fn knocked_out_then_out_of_the_scene_unless_healed_on_both_worlds() {
    for (id, healer, heal) in [
        ("corsaires", "chirurgien", "premiers_soins"),
        ("brasier", "toubib", "stim_de_soin"),
    ] {
        let s = system(id);
        let mut down = pc(&s, "d", if id == "corsaires" { "vigie" } else { "pilote" });
        down.hit_points = 1;
        let attacker = rival(
            &s,
            "e",
            if id == "corsaires" {
                "flibustier"
            } else {
                "canonnier"
            },
        );
        let scene = Scene::new("sol", [down, pc(&s, "h", healer), attacker]);
        let e_turn = begin(&s, &scene, "e");
        let attack = if id == "corsaires" {
            "coup_de_hache"
        } else {
            "rafale_lourde"
        };
        let knocked = play(&s, &e_turn, &ActionRequest::new("e", attack, &["d"]), &[19]);
        assert!(
            knocked
                .events
                .contains(&Event::KnockedOut { target: "d".into() }),
            "{id}"
        );
        let mut scene = finish(&s, &knocked.scene, "e");

        // Healed after one lost turn: back up.
        let lost = finish(&s, &begin(&s, &scene, "d"), "d");
        let healed = play(
            &s,
            &begin(&s, &lost, "h"),
            &ActionRequest::new("h", heal, &["d"]),
            &[],
        );
        assert_eq!(hp(&healed.scene, "d"), 3);
        assert!(!has(&healed.scene, "d", "inconscient"));
        assert!(
            healed
                .events
                .contains(&Event::Revived { target: "d".into() })
        );

        // Never healed: out of the scene after 3 of their turns.
        for turn in 1..=3 {
            assert!(has(&scene, "d", "inconscient"), "{id} turn {turn}");
            scene = finish(&s, &begin(&s, &scene, "d"), "d");
        }
        assert!(!has(&scene, "d", "inconscient"));
        assert!(has(&scene, "d", "hors_combat"));
        let late = play(
            &s,
            &begin(&s, &scene, "h"),
            &ActionRequest::new("h", heal, &["d"]),
            &[],
        );
        assert!(
            has(&late.scene, "d", "hors_combat"),
            "a heal does not bring back someone out of the scene"
        );
    }
}

#[test]
fn damage_over_time_ticks_at_the_end_of_the_bearers_turns() {
    let s = system("corsaires");
    let v = with_xp(&s, pc(&s, "v", "vigie"), 30);
    let scene = begin(
        &s,
        &Scene::new("sol", [v, foe(&s, "gr", "gueule_rouge")]),
        "v",
    );
    let r = play(
        &s,
        &scene,
        &ActionRequest::new("v", "oeil_du_faucon", &["gr"]),
        &[],
    );
    assert_eq!(hp(&r.scene, "gr"), 4, "6, auto-hit");
    let mut scene = finish(&s, &r.scene, "v");
    for expected in [3, 2, 1, 1] {
        scene = finish(&s, &begin(&s, &scene, "gr"), "gr");
        assert_eq!(hp(&scene, "gr"), expected, "bleeding lasts 3 turns");
    }
}

// ------------------------------------------------------------ progression

#[test]
fn xp_upgrade_points_and_levels_follow_the_system_on_both_worlds() {
    for (id, class) in [("corsaires", "bretteur"), ("brasier", "pilote")] {
        let s = system(id);
        let mut sheet = pc(&s, "a", class);
        let p = sheet.progress.as_mut().unwrap();
        for _ in 0..4 {
            assert_eq!(
                gain_xp(&s, p, 1),
                vec![ProgressEvent::XpGained {
                    amount: 1,
                    total: p.total_xp
                }]
            );
        }
        assert_eq!(
            gain_xp(&s, p, 1),
            vec![
                ProgressEvent::XpGained {
                    amount: 1,
                    total: 5
                },
                ProgressEvent::UpgradePoint { available: 1 },
                ProgressEvent::LevelUp { level: 2 },
            ],
            "{id}"
        );
        assert_eq!(p.bar, 0);
        gain_xp(&s, p, 5);
        assert_eq!((p.upgrade_points, p.bar), (2, 0));
        assert_eq!(sheet.level(&s), Some(3));

        assert_eq!(sheet.armor_class(&s).unwrap(), 12); // DEX 14
        spend_upgrade_point(&s, &mut sheet, "DEX").unwrap();
        assert_eq!(sheet.armor_class(&s).unwrap(), 12, "15 still gives +2");
        spend_upgrade_point(&s, &mut sheet, "DEX").unwrap();
        assert_eq!(sheet.armor_class(&s).unwrap(), 13);
        assert!(spend_upgrade_point(&s, &mut sheet, "DEX").is_err());
    }
}

#[test]
fn level_seven_needs_thirty_xp() {
    let s = system("corsaires");
    assert_eq!(with_xp(&s, pc(&s, "a", "vigie"), 29).level(&s), Some(6));
    assert_eq!(with_xp(&s, pc(&s, "a", "vigie"), 30).level(&s), Some(7));
}

// ------------------------------------------------------------ items, turns

#[test]
fn a_consumable_item_heals_then_runs_out() {
    let s = system("corsaires");
    let mut ch = pc(&s, "ch", "chirurgien");
    ch.hit_points = 2;
    let scene = begin(&s, &Scene::new("sol", [ch]), "ch");
    let req = ActionRequest::item("ch", "fiole_de_rhum_fortifiant", &["ch"]);
    let one = play(&s, &scene, &req, &[]);
    assert_eq!(hp(&one.scene, "ch"), 5);
    let two = play(&s, &one.scene, &req, &[]);
    assert_eq!(hp(&two.scene, "ch"), 8);
    let next = begin(&s, &finish(&s, &two.scene, "ch"), "ch");
    assert_eq!(
        refused(&s, &next, &req),
        Refusal::ItemMissing {
            item: "fiole_de_rhum_fortifiant".into()
        }
    );
}

#[test]
fn healing_stops_at_maximum_hit_points() {
    let s = system("brasier");
    let scene = begin(
        &s,
        &Scene::new("sol", [pc(&s, "t", "toubib"), pc(&s, "p", "pilote")]),
        "t",
    );
    let r = play(
        &s,
        &scene,
        &ActionRequest::new("t", "stim_de_soin", &["p"]),
        &[],
    );
    assert_eq!(hp(&r.scene, "p"), 10);
}

#[test]
fn only_one_attack_per_turn_aboard_the_brasier_ship() {
    let s = system("brasier");
    let req = ActionRequest::new("p", "tir_reflexe", &["t"]);
    for (context, second_allowed) in [("sol", true), ("vaisseau", false)] {
        let scene = begin(
            &s,
            &Scene::new(
                context,
                [pc(&s, "p", "pilote"), rival(&s, "t", "canonnier")],
            ),
            "p",
        );
        let once = play(&s, &scene, &req, &[2]);
        let second = resolve_action(&s, &once.scene, &req, &mut ScriptedDice::new([2]));
        if second_allowed {
            assert!(second.is_ok());
        } else {
            assert_eq!(
                second.unwrap_err(),
                Refusal::KindLimitReached {
                    kind: "attaque".into(),
                    max: 1
                }
            );
        }
    }
}

#[test]
fn nobody_acts_outside_their_turn_nor_hits_the_wrong_side() {
    let s = system("corsaires");
    let scene = Scene::new(
        "sol",
        [
            pc(&s, "a", "bretteur"),
            pc(&s, "b", "vigie"),
            foe(&s, "gr", "gueule_rouge"),
        ],
    );
    let req = ActionRequest::new("a", "estocade", &["gr"]);
    assert_eq!(refused(&s, &scene, &req), Refusal::NotTheirTurn);
    let mine = begin(&s, &scene, "a");
    assert_eq!(
        refused(&s, &mine, &ActionRequest::new("a", "estocade", &["b"])),
        Refusal::WrongSide { target: "b".into() }
    );
    assert_eq!(
        refused(
            &s,
            &mine,
            &ActionRequest::new("a", "estocade", &["gr", "b"])
        ),
        Refusal::TargetCount {
            expected: "one enemy",
            got: 2
        }
    );
    assert!(matches!(
        refused(&s, &mine, &ActionRequest::new("a", "tir_embusque", &["gr"])),
        Refusal::UnknownAction { .. }
    ));
}

#[test]
fn the_same_seed_resolves_the_same_way() {
    let s = system("corsaires");
    let scene = begin(
        &s,
        &Scene::new(
            "sol",
            [pc(&s, "a", "flibustier"), foe(&s, "gr", "gueule_rouge")],
        ),
        "a",
    );
    let req = ActionRequest::new("a", "coup_de_hache", &["gr"]);
    let a = resolve_action(&s, &scene, &req, &mut SeededDice::new(2026)).unwrap();
    let b = resolve_action(&s, &scene, &req, &mut SeededDice::new(2026)).unwrap();
    assert_eq!(a, b);
}

// ------------------------------------------- rules are data: edit, re-run

#[test]
fn editing_the_file_changes_the_game_without_code() {
    for id in ["corsaires", "brasier"] {
        let base = system(id);
        let (attacker, light, heavy) = if id == "corsaires" {
            ("bretteur", "estocade", ("canonnier", "tir_de_silex"))
        } else {
            ("pilote", "tir_reflexe", ("canonnier", "rafale_lourde"))
        };
        // A target with DEX 10 (AC 10) and 10 HP in each world.
        let target = if id == "corsaires" {
            "navigateur"
        } else {
            "toubib"
        };
        let scene_with = |s: &RuleSystem, class: &str| {
            begin(
                s,
                &Scene::new("sol", [pc(s, "a", class), rival(s, "t", target)]),
                "a",
            )
        };

        // Damage: 3 → 5.
        let light_req = ActionRequest::new("a", light, &["t"]);
        let s = edited(
            id,
            "tags: [{ damage: { amount: 3 } }, { precision: 2 }, { cooldown: 0 }]",
            "tags: [{ damage: { amount: 5 } }, { precision: 2 }, { cooldown: 0 }]",
        );
        assert_eq!(
            hp(
                &play(&base, &scene_with(&base, attacker), &light_req, &[19]).scene,
                "t"
            ),
            7
        );
        assert_eq!(
            hp(
                &play(&s, &scene_with(&s, attacker), &light_req, &[19]).scene,
                "t"
            ),
            5,
            "{id}"
        );

        // Cooldown: 1 → 0, the second shot of the turn is allowed.
        let heavy_req = ActionRequest::new("a", heavy.1, &["t"]);
        let s = edited(
            id,
            "tags: [{ damage: { amount: 4 } }, { precision: 0 }, { cooldown: 1 }]",
            "tags: [{ damage: { amount: 4 } }, { precision: 0 }, { cooldown: 0 }]",
        );
        let once = play(&base, &scene_with(&base, heavy.0), &heavy_req, &[2]);
        assert!(
            resolve_action(&base, &once.scene, &heavy_req, &mut ScriptedDice::new([2])).is_err()
        );
        let once = play(&s, &scene_with(&s, heavy.0), &heavy_req, &[2]);
        play(&s, &once.scene, &heavy_req, &[2]);

        // Actions per turn: 2 → 3.
        let s = edited(id, "actions_per_turn: 2", "actions_per_turn: 3");
        for (sys, allowed) in [(&base, false), (&s, true)] {
            let mut scene = scene_with(sys, attacker);
            for _ in 0..2 {
                scene = play(sys, &scene, &light_req, &[2]).scene;
            }
            assert_eq!(
                resolve_action(sys, &scene, &light_req, &mut ScriptedDice::new([2])).is_ok(),
                allowed,
                "{id}"
            );
        }
    }
}
