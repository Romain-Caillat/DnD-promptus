//! engine/support-vehicle-combat — ship fights from the Brasier's
//! `ship_combat` rules: arcs around the bow, orders checked before
//! anything moves, shields before hull, the damage table, the reactor,
//! morale as a second way to win, and the two rehearsed scenarios.

use std::path::PathBuf;

use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::dice::ScriptedDice;
use promptus_shared::ships::fight::{Out, arc_of, distance};
use promptus_shared::ships::model::{Allocation, Arc, DamageKind};
use promptus_shared::ships::{
    CrewOrders, Facing, Order, ShipEnd, ShipFight, ShipScenario, run_once, simulate,
};

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn brasier() -> RuleSystem {
    RuleSystem::from_yaml(&read("content/rules/brasier/v1.yaml")).expect("the Brasier's rules load")
}

fn scenario(id: &str) -> ShipScenario {
    ShipScenario::from_yaml(&read(&format!("content/ship-scenarios/brasier/{id}.yaml"))).unwrap()
}

/// The corvette scenario with `dice`: the crew wins the initiative
/// (20 against 1) and acts first.
fn corvette(faces: Vec<u32>) -> (RuleSystem, ShipFight, ScriptedDice) {
    let rules = brasier();
    let ships = scenario("corvette-pirate").ships(&rules).unwrap();
    let mut dice = ScriptedDice::new([20, 1].into_iter().chain(faces));
    let sc = rules.ship_combat.clone().unwrap();
    let fight = ShipFight::start(&sc, ships, &mut dice, &mut Vec::new());
    (rules, fight, dice)
}

fn orders(crew: &str, list: Vec<Order>) -> CrewOrders {
    CrewOrders {
        crew: crew.into(),
        orders: list,
    }
}

fn act(action: &str) -> Order {
    Order {
        action: action.into(),
        ..Order::default()
    }
}

fn at(action: &str, target: &str) -> Order {
    Order {
        target: Some(target.into()),
        ..act(action)
    }
}

#[test]
fn the_brasier_rules_carry_the_cure_dent() {
    let rules = brasier();
    let sc = rules.ship_combat.as_ref().unwrap();
    let ship = sc.ship("cure_dent").unwrap();
    assert_eq!((ship.hull, ship.shields, ship.armor), (30, 12, 14));
    assert_eq!(ship.weapons.len(), 4);
    assert_eq!(sc.reactor(30), 6);
    assert_eq!(sc.reactor(20), 5);
    assert_eq!(sc.reactor(10), 4);
    // A weapon fired from a station that does not exist is refused.
    let broken = read("content/rules/brasier/v1.yaml").replace(
        "station: tourelle_dorsale, arcs:",
        "station: nulle_part, arcs:",
    );
    let err = RuleSystem::from_yaml(&broken).unwrap_err();
    assert!(
        err.0
            .iter()
            .any(|e| e.path == "ship_combat.ships[cure_dent].weapons[dorsale].station"),
        "{err}"
    );
}

#[test]
fn arcs_are_read_around_the_bow() {
    let o = (0, 0);
    assert_eq!(arc_of(o, Facing::North, (0, -3)), Arc::Front);
    assert_eq!(arc_of(o, Facing::North, (1, -3)), Arc::Front);
    assert_eq!(arc_of(o, Facing::North, (3, 0)), Arc::Starboard);
    assert_eq!(arc_of(o, Facing::North, (-3, 0)), Arc::Port);
    assert_eq!(arc_of(o, Facing::North, (0, 4)), Arc::Rear);
    // A diagonal is a flank: the blind spot is the pure stern.
    assert_eq!(arc_of(o, Facing::North, (2, -2)), Arc::Starboard);
    assert_eq!(arc_of(o, Facing::North, (-2, 2)), Arc::Port);
    assert_eq!(arc_of(o, Facing::East, (3, 0)), Arc::Front);
    assert_eq!(arc_of(o, Facing::East, (0, 3)), Arc::Starboard);
    assert_eq!(arc_of(o, Facing::South, (0, -3)), Arc::Rear);
    assert_eq!(distance((0, 0), (3, -5)), 5);
    assert_eq!(Facing::North.turns_to(Facing::South), 2);
    assert_eq!(Facing::West.turns_to(Facing::North), 1);
}

#[test]
fn a_refused_order_changes_nothing() {
    // A refused turn may have rolled (the dice are spent, not the turn).
    let (rules, mut fight, mut dice) = corvette(vec![15; 6]);
    let sc = rules.ship_combat.as_ref().unwrap();
    let ship = fight.ship("cure_dent").unwrap();
    assert_eq!(fight.acting(), &[ship]);
    let before = fight.clone();
    let refused = |fight: &mut ShipFight, dice: &mut ScriptedDice, o: Vec<CrewOrders>| {
        fight.crew_turn(sc, ship, &o, dice).unwrap_err().code
    };
    // The corvette is 9 cells away: out of the heavy gun's long range.
    assert_eq!(
        refused(
            &mut fight,
            &mut dice,
            vec![orders("canonnier", vec![at("tir_canon_lourd", "corvette")])]
        ),
        "OUT_OF_RANGE"
    );
    // The helm moves first, then a second order fails: all of it is undone.
    let advance = Order {
        to: Some((0, -3)),
        facing: Some(Facing::North),
        ..act("manoeuvre")
    };
    assert_eq!(
        refused(
            &mut fight,
            &mut dice,
            vec![
                orders("pilote", vec![advance.clone()]),
                orders("mecano", vec![at("tir_canon_lourd", "corvette")]),
            ]
        ),
        "NOT_AT_STATION"
    );
    assert_eq!(fight, before);
    for (o, code) in [
        (
            vec![
                at("tir_canon_lourd", "corvette"),
                at("tir_canon_lourd", "corvette"),
            ],
            "TOO_MANY_ATTACKS",
        ),
        (
            vec![at("tir_charge", "corvette"), act("changer_de_poste")],
            "TOO_MANY_ACTIONS",
        ),
    ] {
        let o = vec![
            orders("pilote", vec![advance.clone()]),
            orders("canonnier", o),
        ];
        assert_eq!(refused(&mut fight, &mut dice, o), code);
    }
    let too_far = Order {
        to: Some((0, -4)),
        ..advance.clone()
    };
    assert_eq!(
        refused(&mut fight, &mut dice, vec![orders("pilote", vec![too_far])]),
        "TOO_FAR"
    );
    let about_face = Order {
        facing: Some(Facing::South),
        ..advance.clone()
    };
    assert_eq!(
        refused(
            &mut fight,
            &mut dice,
            vec![orders("pilote", vec![about_face])]
        ),
        "TOO_SHARP"
    );
    assert_eq!(fight, before);

    // Moved in, the heavy gun fires: 15 + FOR 2 = 17 against 13; 8
    // damage, the shields take 6, the hull 2.
    let ev = fight
        .crew_turn(
            sc,
            ship,
            &[
                orders("pilote", vec![advance]),
                orders("canonnier", vec![at("tir_canon_lourd", "corvette")]),
            ],
            &mut dice,
        )
        .unwrap();
    assert!(ev.iter().any(|e| matches!(
        e,
        promptus_shared::ships::ShipEvent::Shot {
            total: 17,
            armor: 13,
            hit: true,
            damage: 8,
            ..
        }
    )));
    let c = &fight.ships[fight.ship("corvette").unwrap()];
    assert_eq!((c.shields, c.hull), (0, 13));
    assert_eq!(fight.ships[ship].at, (0, -3));
}

#[test]
fn shields_first_then_the_hull_crosses_a_threshold_and_catches_fire() {
    // The corvette wins the initiative this time.
    let rules = brasier();
    let sc = rules.ship_combat.as_ref().unwrap();
    let mut ships = scenario("corvette-pirate").ships(&rules).unwrap();
    ships[0].shields = 0;
    ships[0].hull = 22;
    ships[1].at = (1, -6);
    // Initiative 1 and 20; the corvette hits (15 + 4 against 14); the
    // hull goes 22 → 17, past 20: the damage die says 1, a fire.
    let mut dice = ScriptedDice::new([1, 20, 15, 1]);
    let mut ev = Vec::new();
    let mut fight = ShipFight::start(sc, ships, &mut dice, &mut ev);
    let corvette = fight.ship("corvette").unwrap();
    let cure = fight.ship("cure_dent").unwrap();
    assert_eq!(fight.acting(), &[corvette]);
    let ev = fight.npc_turn(sc, corvette, &mut dice);
    assert!(
        ev.iter().any(|e| matches!(
            e,
            promptus_shared::ships::ShipEvent::Shot {
                hit: true,
                damage: 5,
                ..
            }
        )),
        "{ev:?}"
    );
    assert_eq!(fight.ships[cure].hull, 17);
    assert_eq!(fight.ships[cure].damages[0].kind, DamageKind::Fire);
    // Its turn opens: the fire burns 2.
    fight.end_turn(sc, &mut dice, &mut Vec::new());
    assert_eq!(fight.ships[cure].hull, 15);
    // Integrity puts it out (CON 14: +2, 9 + 2 against 10).
    let mut dice = ScriptedDice::new([9]);
    fight
        .crew_turn(
            sc,
            cure,
            &[orders("toubib", vec![act("eteindre")])],
            &mut dice,
        )
        .unwrap();
    assert!(fight.ships[cure].damages.is_empty());
    let err = fight
        .crew_turn(
            sc,
            cure,
            &[orders("toubib", vec![act("eteindre")])],
            &mut dice,
        )
        .unwrap_err();
    assert_eq!(err.code, "NOTHING_TO_FIX");
}

#[test]
fn a_hurt_reactor_gives_fewer_points() {
    let rules = brasier();
    let sc = rules.ship_combat.as_ref().unwrap();
    let mut ships = scenario("corvette-pirate").ships(&rules).unwrap();
    ships[0].hull = 9;
    let mut dice = ScriptedDice::new([20, 1]);
    let fight = ShipFight::start(sc, ships, &mut dice, &mut Vec::new());
    let cure = fight.ship("cure_dent").unwrap();
    // 4 points: navigation went first.
    assert_eq!(fight.reactor(sc, cure), 4);
    assert_eq!(
        fight.ships[cure].energy,
        Allocation {
            navigation: 0,
            weapons: 2,
            shields: 2
        }
    );
    // Engines dead: no manoeuvre until the engineer reroutes.
    let mut f = fight.clone();
    let go = Order {
        to: Some((0, -1)),
        ..act("manoeuvre")
    };
    let err = f
        .crew_turn(sc, cure, &[orders("pilote", vec![go.clone()])], &mut dice)
        .unwrap_err();
    assert_eq!(err.code, "ENGINES_DEAD");
    let reroute = |n, w, s| Order {
        energy: Some(Allocation {
            navigation: n,
            weapons: w,
            shields: s,
        }),
        ..act("rerouter")
    };
    let err = f
        .crew_turn(
            sc,
            cure,
            &[orders("mecano", vec![reroute(2, 1, 2)])],
            &mut dice,
        )
        .unwrap_err();
    assert_eq!(err.code, "INVALID_ENERGY");
    f.crew_turn(
        sc,
        cure,
        &[
            orders("mecano", vec![reroute(1, 1, 2)]),
            orders("pilote", vec![go]),
        ],
        &mut dice,
    )
    .unwrap();
    assert_eq!(f.ships[cure].at, (0, -1));
}

#[test]
fn breaking_morale_wins_without_sinking() {
    let (rules, mut fight, mut dice) = corvette(vec![15]);
    let sc = rules.ship_combat.as_ref().unwrap();
    let cure = fight.ship("cure_dent").unwrap();
    let corvette = fight.ship("corvette").unwrap();
    fight.ships[corvette].morale = Some(2);
    // 15 + CHA 2 against resolve 12: −2 morale, it breaks off.
    fight
        .crew_turn(
            sc,
            cure,
            &[orders(
                "quartier_maitre",
                vec![at("briser_moral", "corvette")],
            )],
            &mut dice,
        )
        .unwrap();
    assert_eq!(fight.ships[corvette].out, Some(Out::Broken));
    assert_eq!(fight.ships[corvette].hull, 15);
    assert_eq!(fight.ended, Some(ShipEnd::Victory));
}

#[test]
fn the_rehearsals_end_in_time_and_the_same_way_twice() {
    let rules = brasier();
    // Five Vorr fighters: the Cure-Dent, ahead by generations, wins.
    let swarm = simulate(&rules, &scenario("essaim-vorr"), 100).unwrap();
    assert!(swarm.victories >= 95, "{swarm:#?}");
    assert_eq!(swarm.defeats, 0, "{swarm:#?}");
    assert!(swarm.average_minutes < 45.0, "{swarm:#?}");
    assert_eq!(
        swarm,
        simulate(&rules, &scenario("essaim-vorr"), 100).unwrap()
    );
    // The corvette is sunk or breaks off: both ways happen.
    let corvette = simulate(&rules, &scenario("corvette-pirate"), 100).unwrap();
    assert_eq!(corvette.victories, 100, "{corvette:#?}");
    assert!(
        corvette.broken > 0 && corvette.destroyed > 0,
        "{corvette:#?}"
    );
    assert!(corvette.average_minutes < 45.0, "{corvette:#?}");
    // A run's log says what the report counts.
    let run = run_once(&rules, &scenario("corvette-pirate"), 7).unwrap();
    assert!(run.fight.ended.is_some());
    assert!(!run.events.is_empty());
}
