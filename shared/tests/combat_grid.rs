//! The combat layer on small maps written inline, with the Corsaires
//! rules: range, sight and cover read on the grid (V1 `combat.test.ts`),
//! movement budgets (V1 `grid.test.ts`), initiative, turns, areas, who
//! leaves the fight and when it ends. The grid's own geometry — paths,
//! corners, hexes, line of sight, cover from props — is pinned in
//! `maps_rules.rs` and not repeated here.

use std::collections::HashSet;
use std::path::PathBuf;

use promptus_shared::combat::fight::{CombatRefusal, movement_rules};
use promptus_shared::combat::reach::{self, ReachRefusal};
use promptus_shared::combat::{EndReason, Fight, FightEvent, Play, SetupError, Standing, Step};
use promptus_shared::maps::{Cell, Cover, Diagonal, Map, Obstacle};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::action::Refusal;
use promptus_shared::rules::check::{ModifierSource, RollBreakdown};
use promptus_shared::rules::dice::ScriptedDice;
use promptus_shared::rules::events::{Event, RollPurpose};
use promptus_shared::rules::model::ActionDef;
use promptus_shared::rules::sheet::{Combatant, Side};

fn text(id: &str) -> String {
    let p =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../content/rules/{id}/v1.yaml"));
    std::fs::read_to_string(p).unwrap()
}

fn corsaires() -> RuleSystem {
    RuleSystem::from_yaml(&text("corsaires")).unwrap()
}

/// The Corsaires with one edit to the file — what Romain does.
fn edited(from: &str, to: &str) -> RuleSystem {
    let t = text("corsaires");
    assert!(t.contains(from), "`{from}` is not in the file");
    RuleSystem::from_yaml(&t.replacen(from, to, 1)).unwrap()
}

fn c(x: i32, y: i32) -> Cell {
    Cell::new(x, y)
}

/// A small encounter map: `.` floor, `#` wall, `,` difficult.
/// `extra` is appended YAML (props, ambience…).
fn tiny(rows: &[&str], extra: &str) -> Map {
    let rows: String = rows.iter().map(|r| format!("    - \"{r}\"\n")).collect();
    let yaml = format!(
        "version: 1\nid: t\nname: Test\nscale: encounter\ntheme: test\ngrid:\n  legend:\n    \".\": {{ terrain: sol }}\n    \"#\": {{ terrain: mur, wall: true }}\n    \",\": {{ terrain: boue, difficult: true }}\n  rows:\n{rows}{extra}"
    );
    Map::from_yaml(&yaml).unwrap_or_else(|e| panic!("{e}\n{yaml}"))
}

fn room(w: usize, h: usize) -> Map {
    let row = ".".repeat(w);
    let rows: Vec<&str> = (0..h).map(|_| row.as_str()).collect();
    tiny(&rows, "")
}

fn pc(s: &RuleSystem, id: &str, class: &str) -> Combatant {
    Combatant::from_class(s, id, id, class).unwrap()
}

fn sailor(s: &RuleSystem, id: &str) -> Combatant {
    Combatant::from_adversary(s, id, id, "marin_de_gueule_rouge").unwrap()
}

fn action(s: &RuleSystem, id: &str) -> ActionDef {
    s.classes
        .iter()
        .flat_map(|c| &c.actions)
        .chain(s.adversaries.iter().flat_map(|a| &a.actions))
        .find(|a| a.id == id)
        .unwrap_or_else(|| panic!("no action {id}"))
        .clone()
}

/// Starts a fight with exactly these initiative faces.
fn start(s: &RuleSystem, map: Map, who: Vec<(Combatant, Cell)>, faces: &[u32]) -> Fight {
    let mut dice = ScriptedDice::new(faces.iter().copied());
    let step = Fight::start(s, "sol", map, who, &mut dice).unwrap();
    assert_eq!(dice.remaining(), 0, "initiative faces left unrolled");
    step.fight
}

fn faces(f: &[u32]) -> ScriptedDice {
    ScriptedDice::new(f.iter().copied())
}

/// Plays with exactly these faces; panics on a refusal.
fn ok(r: Result<Step, CombatRefusal>) -> Step {
    r.unwrap_or_else(|e| panic!("refused: {e:?}"))
}

fn attack_rolls(events: &[FightEvent]) -> Vec<&RollBreakdown> {
    events
        .iter()
        .filter_map(|e| match e {
            FightEvent::Rules {
                event:
                    Event::Roll {
                        purpose: RollPurpose::Attack,
                        breakdown,
                        ..
                    },
            } => Some(breakdown),
            _ => None,
        })
        .collect()
}

fn hp(f: &Fight, id: &str) -> i32 {
    f.combatant(id).unwrap().hit_points
}

// ------------------------------------------------- range and sight (V1 combat.test.ts)

#[test]
fn melee_reaches_only_touching_cells_diagonals_included() {
    let s = corsaires();
    let map = room(12, 12);
    let estocade = action(&s, "estocade");
    assert_eq!(estocade.reach(), 1, "no range written: adjacent only");
    let none = HashSet::new();
    let r = reach::reach(
        &map,
        Diagonal::Chebyshev,
        &estocade,
        c(2, 2),
        c(3, 3),
        &none,
    )
    .unwrap();
    assert_eq!((r.distance, r.long_range), (1, false));
    assert_eq!(
        reach::reach(
            &map,
            Diagonal::Chebyshev,
            &estocade,
            c(2, 2),
            c(4, 2),
            &none
        ),
        Err(ReachRefusal::OutOfRange {
            distance: 2,
            range: 1
        })
    );
}

#[test]
fn long_range_is_allowed_with_its_penalty_then_out_of_range() {
    let s = corsaires();
    let map = room(24, 24);
    let mut bow = action(&s, "tir_de_silex");
    bow.range = Some(6);
    bow.long_range = Some(20);
    let none = HashSet::new();
    let at = |to| reach::reach(&map, Diagonal::Chebyshev, &bow, c(0, 0), to, &none);
    assert!(at(c(10, 0)).unwrap().long_range);
    assert!(!at(c(5, 0)).unwrap().long_range);
    bow.long_range = Some(10);
    let at = |to| reach::reach(&map, Diagonal::Chebyshev, &bow, c(0, 0), to, &none);
    assert!(matches!(
        at(c(11, 11)),
        Err(ReachRefusal::OutOfRange { range: 10, .. })
    ));
    // What long range does is the system's: −2 by default.
    let (mods, dis) = reach::roll_effects(&s, &at(c(9, 0)).unwrap());
    assert_eq!(mods.len(), 1);
    assert_eq!(
        (mods[0].source.clone(), mods[0].value),
        (ModifierSource::LongRange, -2)
    );
    assert!(!dis);
}

#[test]
fn a_wall_between_blocks_the_shot_but_contact_always_sees() {
    let s = corsaires();
    let map = tiny(&["........", "........", "....#...", "........"], "");
    let shot = action(&s, "tir_de_silex");
    let none = HashSet::new();
    assert_eq!(
        reach::reach(&map, Diagonal::Chebyshev, &shot, c(2, 2), c(6, 2), &none),
        Err(ReachRefusal::NoLineOfSight)
    );
    assert!(reach::reach(&map, Diagonal::Chebyshev, &shot, c(2, 3), c(6, 3), &none).is_ok());
    // Squeezed diagonally between two walls, two fighters still touch.
    let map = tiny(&["....", "...#", "..#.", "...."], "");
    let estocade = action(&s, "estocade");
    let r = reach::reach(
        &map,
        Diagonal::Chebyshev,
        &estocade,
        c(2, 1),
        c(3, 2),
        &none,
    );
    assert_eq!(r.map(|r| r.cover), Ok(Cover::None));
}

#[test]
fn fog_hides_targets_beyond_the_sight_limit() {
    let s = corsaires();
    let map = tiny(&["..............."], "ambience: { sight_limit: 5 }\n");
    let shot = action(&s, "tir_embusque");
    let none = HashSet::new();
    assert!(reach::reach(&map, Diagonal::Chebyshev, &shot, c(0, 0), c(5, 0), &none).is_ok());
    assert_eq!(
        reach::reach(&map, Diagonal::Chebyshev, &shot, c(0, 0), c(6, 0), &none),
        Err(ReachRefusal::NoLineOfSight)
    );
}

#[test]
fn cover_on_the_grid_lowers_the_attack_roll_by_the_systems_value() {
    let crate_ = "props:\n  - { id: caisse, kind: caisse, at: [4, 0], cover: half, blocks_movement: true }\n";
    let map = || tiny(&["..........", ".........."], crate_);
    let fight = |s: &RuleSystem| {
        start(
            s,
            map(),
            vec![
                (pc(s, "canonnier", "canonnier"), c(0, 0)),
                (sailor(s, "marin"), c(7, 0)),
            ],
            &[15, 3],
        )
    };
    let s = corsaires();
    let f = fight(&s);
    // INT 14 (+2), natural 10, half cover −2: 10 against AC 10, a hit.
    let step = ok(f.act(
        &s,
        &Play::on("canonnier", "tir_de_silex", &["marin"]),
        &mut faces(&[10]),
    ));
    let roll = attack_rolls(&step.events)[0];
    let cover: Vec<_> = roll
        .modifiers
        .iter()
        .filter(|m| m.source == ModifierSource::Cover(Cover::Half))
        .collect();
    assert_eq!(cover.len(), 1);
    assert_eq!((cover[0].value, roll.total), (-2, 10));
    assert_eq!(hp(&step.fight, "marin"), 0);

    // Romain makes half cover −4: the same roll now misses.
    let s = edited(
        "  flee: { kind: fuite, ability: DEX }\n",
        "  flee: { kind: fuite, ability: DEX }\n  cover: { half: -4, three_quarters: -6 }\n",
    );
    let f = fight(&s);
    let step = ok(f.act(
        &s,
        &Play::on("canonnier", "tir_de_silex", &["marin"]),
        &mut faces(&[10]),
    ));
    assert_eq!(attack_rolls(&step.events)[0].total, 8);
    assert_eq!(hp(&step.fight, "marin"), 4);
}

#[test]
fn an_attack_out_of_reach_is_refused_and_changes_nothing() {
    let s = corsaires();
    let f = start(
        &s,
        room(10, 3),
        vec![
            (pc(&s, "bretteur", "bretteur"), c(0, 1)),
            (sailor(&s, "marin"), c(3, 1)),
        ],
        &[15, 3],
    );
    let before = f.clone();
    let e = f
        .act(
            &s,
            &Play::on("bretteur", "estocade", &["marin"]),
            &mut faces(&[]),
        )
        .unwrap_err();
    assert_eq!(
        e,
        CombatRefusal::OutOfReach {
            target: "marin".into(),
            why: ReachRefusal::OutOfRange {
                distance: 3,
                range: 1
            }
        }
    );
    assert_eq!(f, before);
}

// ------------------------------------------------- movement (V1 grid.test.ts)

#[test]
fn a_move_spends_one_action_and_covers_the_systems_cells_per_move() {
    let s = corsaires();
    assert!(
        s.movement.iter().all(|m| m.cells_per_move.is_none()),
        "the Corsaires give no distance"
    );
    let f = start(
        &s,
        room(12, 3),
        vec![
            (pc(&s, "bretteur", "bretteur"), c(0, 1)),
            (sailor(&s, "marin"), c(11, 1)),
        ],
        &[15, 3],
    );
    assert_eq!(f.move_budget(&s, "bretteur"), 6, "the engine's default");
    let path: Vec<Cell> = (1..=6).map(|x| c(x, 1)).collect();
    let step = ok(f.move_along(&s, "bretteur", &path));
    assert_eq!(step.fight.position("bretteur"), Some(c(6, 1)));
    let me = step.fight.combatant("bretteur").unwrap();
    assert_eq!(me.turn.actions_left, 1);
    assert_eq!(me.turn.spent_by_kind.get("deplacement"), Some(&1));
    // Seven cells is one too many.
    let long: Vec<Cell> = (1..=7).map(|x| c(x, 1)).collect();
    assert!(matches!(
        f.move_along(&s, "bretteur", &long),
        Err(CombatRefusal::Path { .. })
    ));

    // Written in the file, the distance is the file's.
    let s2 = edited("    cells_per_move: null", "    cells_per_move: 3");
    let f2 = start(
        &s2,
        room(12, 3),
        vec![
            (pc(&s2, "bretteur", "bretteur"), c(0, 1)),
            (sailor(&s2, "marin"), c(11, 1)),
        ],
        &[15, 3],
    );
    assert_eq!(f2.move_budget(&s2, "bretteur"), 3);
    assert!(f2.move_along(&s2, "bretteur", &path[..4]).is_err());
}

#[test]
fn movement_conditions_shrink_or_forbid_a_move() {
    let s = corsaires();
    let mut b = pc(&s, "bretteur", "bretteur");
    let maelstrom =
        promptus_shared::rules::conditions::named(&s, "maelstrom", Some(3), "x").unwrap();
    b.conditions.push(maelstrom);
    let f = start(
        &s,
        room(12, 3),
        vec![(b, c(0, 1)), (sailor(&s, "marin"), c(11, 1))],
        &[15, 3],
    );
    assert_eq!(f.move_budget(&s, "bretteur"), 3, "half of 6");

    let mut b = pc(&s, "bretteur", "bretteur");
    b.conditions
        .push(promptus_shared::rules::conditions::named(&s, "immobilise", Some(2), "x").unwrap());
    let f = start(
        &s,
        room(12, 3),
        vec![(b, c(0, 1)), (sailor(&s, "marin"), c(11, 1))],
        &[15, 3],
    );
    assert_eq!(
        f.move_along(&s, "bretteur", &[c(1, 1)]).unwrap_err(),
        CombatRefusal::CannotMove
    );
}

#[test]
fn enemies_block_the_way_and_allies_are_crossed_but_not_stood_on() {
    let s = corsaires();
    let f = start(
        &s,
        room(6, 1),
        vec![
            (pc(&s, "bretteur", "bretteur"), c(0, 0)),
            (pc(&s, "vigie", "vigie"), c(1, 0)),
            (sailor(&s, "marin"), c(3, 0)),
        ],
        &[20, 1, 1],
    );
    assert_eq!(f.active(), Some("bretteur"));
    assert!(
        f.move_along(&s, "bretteur", &[c(1, 0)]).is_err(),
        "stops on an ally"
    );
    assert!(f.move_along(&s, "bretteur", &[c(1, 0), c(2, 0)]).is_ok());
    assert!(
        f.move_along(&s, "bretteur", &[c(1, 0), c(2, 0), c(3, 0)])
            .is_err()
    );
}

#[test]
fn a_free_move_system_allows_one_move_per_turn() {
    let s = edited("  move_kind: deplacement\n", "");
    let f = start(
        &s,
        room(12, 3),
        vec![
            (pc(&s, "bretteur", "bretteur"), c(0, 1)),
            (sailor(&s, "marin"), c(11, 1)),
        ],
        &[15, 3],
    );
    let step = ok(f.move_along(&s, "bretteur", &[c(1, 1)]));
    assert_eq!(
        step.fight.combatant("bretteur").unwrap().turn.actions_left,
        2
    );
    assert_eq!(
        step.fight
            .move_along(&s, "bretteur", &[c(2, 1)])
            .unwrap_err(),
        CombatRefusal::AlreadyMoved
    );
}

// ------------------------------------------------- initiative and turns

#[test]
fn initiative_orders_by_total_and_ties_go_to_the_party() {
    let s = corsaires();
    let who = |s: &RuleSystem| {
        vec![
            (sailor(s, "marin"), c(5, 0)),                // DEX 10: +0
            (pc(s, "bretteur", "bretteur"), c(0, 0)),     // DEX 14: +2
            (pc(s, "flibustier", "flibustier"), c(0, 1)), // DEX 9: −1
        ]
    };
    let f = start(&s, room(6, 2), who(&s), &[12, 10, 16]);
    assert_eq!(f.order, ["flibustier", "bretteur", "marin"]);
    assert_eq!(f.initiative[1].total, 12);
    assert_eq!(f.active(), Some("flibustier"));

    // The same faces with ties rerolled: the sailor rerolls higher.
    let s = edited("ties: party_first", "ties: reroll");
    let f = start(&s, room(6, 2), who(&s), &[12, 10, 16, 18, 4]);
    assert_eq!(f.order, ["flibustier", "marin", "bretteur"]);
    assert_eq!(f.initiative[1].rerolls, [18]);
}

#[test]
fn turns_follow_the_order_and_wrap_into_a_new_round() {
    let s = corsaires();
    let f = start(
        &s,
        room(6, 2),
        vec![
            (pc(&s, "bretteur", "bretteur"), c(0, 0)),
            (sailor(&s, "marin"), c(5, 0)),
        ],
        &[15, 3],
    );
    assert_eq!((f.active(), f.round), (Some("bretteur"), 1));
    assert_eq!(
        f.end_turn(&s, "marin", &mut faces(&[])).unwrap_err(),
        CombatRefusal::NotTheirTurn
    );
    let f = ok(f.end_turn(&s, "bretteur", &mut faces(&[]))).fight;
    assert_eq!((f.active(), f.round), (Some("marin"), 1));
    let step = ok(f.end_turn(&s, "marin", &mut faces(&[])));
    assert_eq!(
        (step.fight.active(), step.fight.round),
        (Some("bretteur"), 2)
    );
    assert!(step.events.contains(&FightEvent::RoundStarted { round: 2 }));
    assert_eq!(
        step.fight.combatant("bretteur").unwrap().turn.actions_left,
        2
    );
}

#[test]
fn a_stunned_combatant_s_turn_passes_on_its_own() {
    let s = corsaires();
    let mut m = sailor(&s, "marin");
    m.conditions
        .push(promptus_shared::rules::conditions::named(&s, "etourdi", Some(1), "x").unwrap());
    let f = start(
        &s,
        room(6, 2),
        vec![(pc(&s, "bretteur", "bretteur"), c(0, 0)), (m, c(5, 0))],
        &[15, 3],
    );
    let step = ok(f.end_turn(&s, "bretteur", &mut faces(&[])));
    assert_eq!(
        (step.fight.active(), step.fight.round),
        (Some("bretteur"), 2)
    );
    assert!(step.events.iter().any(|e| matches!(
        e,
        FightEvent::Rules { event: Event::TurnLost { who, .. } } if who == "marin"
    )));
    assert!(
        step.fight.combatant("marin").unwrap().conditions.is_empty(),
        "stun spent"
    );
}

#[test]
fn a_fight_needs_both_sides_on_free_distinct_cells() {
    let s = corsaires();
    let mut d = faces(&[]);
    let map = tiny(&["..#"], "");
    let err = |who| Fight::start(&s, "sol", map.clone(), who, &mut faces(&[])).unwrap_err();
    assert_eq!(
        err(vec![(pc(&s, "b", "bretteur"), c(0, 0))]),
        SetupError::MissingSide(Side::Opposition)
    );
    assert_eq!(
        err(vec![
            (pc(&s, "b", "bretteur"), c(2, 0)),
            (sailor(&s, "m"), c(0, 0))
        ]),
        SetupError::CannotStand {
            who: "b".into(),
            at: c(2, 0),
            obstacle: Obstacle::Wall
        }
    );
    assert!(matches!(
        err(vec![
            (pc(&s, "b", "bretteur"), c(0, 0)),
            (sailor(&s, "m"), c(0, 0))
        ]),
        SetupError::SameCell { .. }
    ));
    assert!(Fight::start(&s, "abordage", map, vec![], &mut d).is_err());
}

// ------------------------------------------------- areas from the grid

fn canonnier_level_3(s: &RuleSystem) -> Combatant {
    let mut c = pc(s, "canonnier", "canonnier");
    promptus_shared::rules::progression::gain_xp(s, c.progress.as_mut().unwrap(), 10);
    c
}

#[test]
fn a_zone_catches_the_enemies_around_its_centre_and_not_behind_a_wall() {
    // A wider zone than the default, so a wall can stand inside it.
    let s = edited(
        "  flee: { kind: fuite, ability: DEX }\n",
        "  flee: { kind: fuite, ability: DEX }\n  zone_radius: 2\n",
    );
    let map = tiny(
        &["..........", "..........", "......#...", ".........."],
        "",
    );
    let f = start(
        &s,
        map,
        vec![
            (canonnier_level_3(&s), c(0, 1)),
            (sailor(&s, "m1"), c(5, 1)),
            (sailor(&s, "m2"), c(5, 2)),
            (sailor(&s, "m3"), c(7, 2)), // within the radius, behind the wall
            (sailor(&s, "m4"), c(8, 1)), // too far from the centre (3)
        ],
        &[20, 1, 1, 1, 1],
    );
    let mut bomb = Play::at("canonnier", "bombe_a_meche", c(5, 1));
    bomb.save_difficulty = Some(10);
    assert_eq!(f.check(&s, &bomb).unwrap(), ["m1", "m2"]);
    // The centre must be in range (5 cells).
    let far = Play::at("canonnier", "bombe_a_meche", c(6, 1));
    assert!(matches!(
        f.check(&s, &far),
        Err(CombatRefusal::AimOutOfReach { .. })
    ));
    // A centre with nobody around it is refused, not wasted.
    let empty = Play::at("canonnier", "bombe_a_meche", c(2, 3));
    assert_eq!(f.check(&s, &empty).unwrap_err(), CombatRefusal::AreaEmpty);
}

#[test]
fn a_burst_catches_adjacent_enemies_only_and_a_line_those_on_it() {
    let s = corsaires();
    let mut flib = pc(&s, "flibustier", "flibustier");
    promptus_shared::rules::progression::gain_xp(&s, flib.progress.as_mut().unwrap(), 10);
    let f = start(
        &s,
        room(8, 5),
        vec![
            (flib, c(3, 2)),
            (sailor(&s, "m1"), c(4, 3)),
            (sailor(&s, "m2"), c(2, 2)),
            (sailor(&s, "m3"), c(5, 2)),
        ],
        &[20, 1, 1, 1],
    );
    assert_eq!(
        f.check(&s, &Play::alone("flibustier", "cri_de_guerre"))
            .unwrap(),
        ["m1", "m2"]
    );

    // Lines (Salve de bordée, level 7): everyone ahead on the ray.
    let mut can = pc(&s, "canonnier", "canonnier");
    promptus_shared::rules::progression::gain_xp(&s, can.progress.as_mut().unwrap(), 30);
    let f = start(
        &s,
        tiny(&["..............", "..............", "........#....."], ""),
        vec![
            (can, c(0, 1)),
            (sailor(&s, "m1"), c(3, 1)),
            (sailor(&s, "m2"), c(9, 1)),
            (sailor(&s, "m3"), c(4, 2)),
            (sailor(&s, "m4"), c(12, 2)),
        ],
        &[20, 1, 1, 1, 1],
    );
    assert_eq!(
        f.check(&s, &Play::at("canonnier", "salve_de_bordee", c(13, 1)))
            .unwrap(),
        ["m1", "m2"]
    );
}

// ------------------------------------------------- leaving the fight, the end

#[test]
fn a_felled_adversary_leaves_the_order_and_frees_its_cell() {
    let s = corsaires();
    let f = start(
        &s,
        room(6, 1),
        vec![
            (pc(&s, "flibustier", "flibustier"), c(0, 0)),
            (sailor(&s, "m1"), c(1, 0)),
            (sailor(&s, "m2"), c(5, 0)),
        ],
        &[20, 1, 1],
    );
    // FOR 14 (+2), 12 against AC 10: 4 damage, the sailor's 4 HP.
    let step = ok(f.act(
        &s,
        &Play::on("flibustier", "coup_de_hache", &["m1"]),
        &mut faces(&[12]),
    ));
    let f = step.fight;
    assert!(
        step.events
            .contains(&FightEvent::Defeated { who: "m1".into() })
    );
    assert_eq!(f.standing["m1"], Standing::Defeated);
    assert_eq!(f.position("m1"), None);
    assert!(!f.is_over(), "one sailor left");
    // The way through is free now.
    assert!(f.move_along(&s, "flibustier", &[c(1, 0), c(2, 0)]).is_ok());
    // m1's turn is skipped: after the flibustier comes m2.
    let f = ok(f.end_turn(&s, "flibustier", &mut faces(&[]))).fight;
    assert_eq!(f.active(), Some("m2"));
}

#[test]
fn the_last_enemy_down_ends_the_fight_with_the_xp_gained() {
    let s = corsaires();
    let f = start(
        &s,
        room(4, 1),
        vec![
            (pc(&s, "flibustier", "flibustier"), c(0, 0)),
            (pc(&s, "vigie", "vigie"), c(3, 0)),
            (sailor(&s, "m1"), c(1, 0)),
        ],
        &[20, 1, 1],
    );
    // A miss (no XP) then a hit (+1 XP, 4 damage: down).
    let f = ok(f.act(
        &s,
        &Play::on("flibustier", "coup_de_hache", &["m1"]),
        &mut faces(&[2]),
    ))
    .fight;
    let step = ok(f.act(
        &s,
        &Play::on("flibustier", "coup_de_hache", &["m1"]),
        &mut faces(&[15]),
    ));
    let f = step.fight;
    let end = f.end.clone().expect("over");
    assert_eq!(end.winner, Some(Side::Party));
    assert_eq!(end.reason, EndReason::SideDown);
    assert_eq!(end.defeated, ["m1"]);
    assert_eq!(end.xp.get("flibustier"), Some(&1));
    assert_eq!(end.xp.get("vigie"), Some(&0));
    assert_eq!(f.active(), None);
    assert_eq!(
        f.end_turn(&s, "flibustier", &mut faces(&[])).unwrap_err(),
        CombatRefusal::FightOver
    );
}

#[test]
fn a_knocked_out_character_stays_down_then_leaves_the_scene() {
    let s = corsaires();
    let mut b = pc(&s, "bretteur", "bretteur");
    b.hit_points = 1;
    let f = start(
        &s,
        room(5, 1),
        vec![
            (pc(&s, "vigie", "vigie"), c(4, 0)),
            (sailor(&s, "m1"), c(1, 0)),
            (b, c(0, 0)),
        ],
        &[20, 10, 1],
    );
    let f = ok(f.end_turn(&s, "vigie", &mut faces(&[]))).fight;
    // Coutelas: 15 hits, 1d4+1 → 3.
    let f = ok(f.act(
        &s,
        &Play::on("m1", "marin_coutelas", &["bretteur"]),
        &mut faces(&[15, 2]),
    ))
    .fight;
    assert_eq!(hp(&f, "bretteur"), 0);
    assert!(f.in_fight("bretteur"), "knocked out, still on the ground");
    assert!(!f.is_over(), "the vigie still stands");
    // Three of the bretteur's turns pass on their own; then out.
    let mut f = f;
    let mut out = false;
    for _ in 0..3 {
        let step = ok(f.end_turn(&s, f.active().unwrap(), &mut faces(&[])));
        out |= step.events.contains(&FightEvent::LeftTheScene {
            who: "bretteur".into(),
        });
        f = step.fight;
        if f.active() == Some("vigie") {
            f = ok(f.end_turn(&s, "vigie", &mut faces(&[]))).fight;
        }
    }
    assert!(out);
    assert_eq!(f.standing["bretteur"], Standing::OutOfScene);
    assert_eq!(f.position("bretteur"), None);
}

#[test]
fn a_whole_party_down_loses_the_fight() {
    let s = corsaires();
    let mut b = pc(&s, "bretteur", "bretteur");
    b.hit_points = 1;
    let f = start(
        &s,
        room(3, 1),
        vec![(sailor(&s, "m1"), c(1, 0)), (b, c(0, 0))],
        &[20, 1],
    );
    let step = ok(f.act(
        &s,
        &Play::on("m1", "marin_coutelas", &["bretteur"]),
        &mut faces(&[15, 1]),
    ));
    let end = step.fight.end.expect("over");
    assert_eq!(end.winner, Some(Side::Opposition));
}

#[test]
fn fleeing_spends_the_turn_and_a_hard_flight_needs_the_roll() {
    let s = corsaires();
    let who = |s: &RuleSystem| {
        vec![
            (sailor(s, "m1"), c(1, 0)),
            (sailor(s, "m2"), c(4, 0)),
            (pc(s, "bretteur", "bretteur"), c(0, 0)),
        ]
    };
    let f = start(&s, room(6, 1), who(&s), &[20, 19, 1]);
    // The GM says it is hard (12): DEX 10 (+0) rolls 5, a failure.
    let step = ok(f.flee(&s, "m1", Some(12), &mut faces(&[5])));
    assert!(
        step.events
            .contains(&FightEvent::FleeFailed { who: "m1".into() })
    );
    let f = step.fight;
    assert!(f.in_fight("m1"));
    assert_eq!(
        f.combatant("m1").unwrap().turn.actions_left,
        0,
        "Fuir costs both actions"
    );
    let f = ok(f.end_turn(&s, "m1", &mut faces(&[]))).fight;
    // m2 just runs: no roll asked.
    let step = ok(f.flee(&s, "m2", None, &mut faces(&[])));
    assert_eq!(step.fight.standing["m2"], Standing::Fled);
    assert_eq!(step.fight.position("m2"), None);
    assert_eq!(
        step.fight.active(),
        Some("bretteur"),
        "the next turn begins"
    );

    // A flight that succeeds against the last foe standing ends the fight.
    let f = start(&s, room(6, 1), who(&s), &[20, 19, 1]);
    let f = ok(f.flee(&s, "m1", Some(12), &mut faces(&[15]))).fight;
    let step = ok(f.flee(&s, "m2", None, &mut faces(&[])));
    let end = step.fight.end.expect("over");
    assert_eq!((end.winner, end.fled.len()), (Some(Side::Party), 2));
}

#[test]
fn the_gm_stops_a_fight_whenever() {
    let s = corsaires();
    let f = start(
        &s,
        room(6, 1),
        vec![
            (pc(&s, "b", "bretteur"), c(0, 0)),
            (sailor(&s, "m"), c(5, 0)),
        ],
        &[20, 1],
    );
    let step = f.stop();
    let end = step.fight.end.clone().expect("stopped");
    assert_eq!((end.winner, end.reason), (None, EndReason::StoppedByGm));
    assert_eq!(step.fight.active(), None);
}

#[test]
fn an_action_refused_by_the_rules_changes_nothing() {
    let s = corsaires();
    let f = start(
        &s,
        room(6, 1),
        vec![
            (pc(&s, "c", "canonnier"), c(0, 0)),
            (sailor(&s, "m"), c(3, 0)),
        ],
        &[20, 1],
    );
    // Tir de silex has a cooldown: the second shot of the turn is refused.
    let f = ok(f.act(&s, &Play::on("c", "tir_de_silex", &["m"]), &mut faces(&[2]))).fight;
    let before = f.clone();
    let e = f
        .act(&s, &Play::on("c", "tir_de_silex", &["m"]), &mut faces(&[]))
        .unwrap_err();
    assert!(matches!(
        e,
        CombatRefusal::Rules {
            refusal: Refusal::OnCooldown { .. }
        }
    ));
    assert_eq!(f, before);
    let rules = movement_rules(&s, &f.map);
    assert_eq!(rules.diagonal, Diagonal::Chebyshev, "the grid's default");
}
