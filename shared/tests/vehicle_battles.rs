//! Vehicle combat (engine/support-vehicle-combat) on the two witness
//! worlds: one engine, two skins. The Cure-Dent's shields take hits
//! before the hull, the brig's sails do not; the Cure-Dent's crew has
//! one attack a turn, the brig's two; arcs come from each ship's data;
//! damage aboard, morale, boarding; and both battles of the ticket
//! simulated end to end, reproducibly, under 45 minutes.

use std::path::PathBuf;

use promptus_shared::maps::{Cell, Map};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::dice::{ScriptedDice, SeededDice};
use promptus_shared::rules::sheet::{Combatant, Side};
use promptus_shared::vehicle::battle::Unit;
use promptus_shared::vehicle::policy::{Order, propose_enemy_turn, run_battle};
use promptus_shared::vehicle::scenario::{BattleSpec, BattleTime, VehicleScenario};
use promptus_shared::vehicle::{
    Aim, Battle, BattleEndReason, BattleEvent, BattleRefusal, Crew, Facing, Placed, Setup,
    ShipStanding,
};

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn rules(world: &str) -> RuleSystem {
    RuleSystem::from_yaml(&read(&format!("content/rules/{world}/v1.yaml")))
        .unwrap_or_else(|e| panic!("{world}: {e}"))
}

fn map(world: &str, id: &str) -> Map {
    Map::from_yaml(&read(&format!("content/maps/{world}/{id}.yaml"))).unwrap()
}

fn scenario(world: &str, id: &str) -> VehicleScenario {
    VehicleScenario::from_yaml(&read(&format!(
        "content/scenarios/{world}/vaisseau/{id}.yaml"
    )))
    .unwrap()
}

fn c(x: i32, y: i32) -> Cell {
    Cell::new(x, y)
}

fn crew(system: &RuleSystem, id: &str, class: &str, station: &str) -> Crew {
    let sheet = Combatant::from_class(system, id, id, class).unwrap();
    let mut m = Crew::from_combatant(system, &sheet, Some(class.into()));
    m.station = Some(station.into());
    m
}

/// A battle on `map` with the party ship at `at` and one enemy, crew as
/// given, initiative scripted so the crew acts first.
fn duel(
    system: &RuleSystem,
    map: Map,
    party: (&str, Cell, Facing),
    enemy: (&str, Cell, Facing),
    members: Vec<Crew>,
) -> Battle {
    let setup = Setup {
        party_ship: Placed {
            id: "nous".into(),
            kind: party.0.into(),
            name: None,
            at: party.1,
            facing: party.2,
        },
        crew: members,
        enemies: vec![Placed {
            id: "eux".into(),
            kind: enemy.0.into(),
            name: None,
            at: enemy.1,
            facing: enemy.2,
        }],
    };
    // Party 20, enemy 1.
    let mut dice = ScriptedDice::new([20, 1]);
    let step = Battle::start(system, map, setup, &mut dice).unwrap();
    assert!(step.battle.crew_turn(), "the crew acts first");
    step.battle
}

fn aim_at(target: &str) -> Aim {
    Aim {
        target: Some(target.into()),
        ..Aim::default()
    }
}

#[test]
fn both_worlds_carry_one_vehicle_system_with_two_skins() {
    let (b, k) = (rules("brasier"), rules("corsaires"));
    let (vb, vk) = (b.vehicles.as_ref().unwrap(), k.vehicles.as_ref().unwrap());
    assert_eq!(vb.hull.name, "Coque");
    assert_eq!(vb.screen.as_ref().unwrap().name, "Boucliers");
    assert_eq!(vk.screen.as_ref().unwrap().name, "Voilure");
    assert_eq!(vb.power.as_ref().unwrap().name, "Énergie");
    assert_eq!(vk.power.as_ref().unwrap().name, "Équipage");
    assert_eq!(vb.current.as_ref().unwrap().name, "Gravité");
    assert_eq!(vk.current.as_ref().unwrap().name, "Vent");
    // Ground fights still play the first context.
    assert_eq!(k.turn_contexts[0].id, "sol");
}

#[test]
fn shields_take_the_hit_before_the_hull_and_sails_do_not() {
    // The Cure-Dent's heavy gun on a pirate corvette ahead: 8 damage,
    // 6 into its shields, 2 into the hull.
    let b = rules("brasier");
    let battle = duel(
        &b,
        map("brasier", "abords-du-toboggan"),
        ("cure_dent", c(3, 8), Facing::E),
        ("corvette_pirate", c(8, 8), Facing::W),
        vec![crew(&b, "canonnier", "canonnier", "piece_lourde")],
    );
    let mut dice = ScriptedDice::new([15]);
    let s = battle
        .crew_act(
            &b,
            "canonnier",
            "tir_canon_lourd",
            &aim_at("eux"),
            &mut dice,
        )
        .unwrap();
    let eux = s.battle.ship("eux").unwrap();
    assert_eq!((eux.screen, eux.hull), (0, 13));

    // The brig's broadside on the sloop abeam: 6 damage, all to the hull;
    // the sails stay whole.
    let k = rules("corsaires");
    let battle = duel(
        &k,
        map("corsaires", "large-de-belle-ile"),
        ("la_machoire", c(10, 9), Facing::N),
        ("sloop_escorte", c(6, 9), Facing::N),
        vec![crew(&k, "canonnier", "canonnier", "batterie_babord")],
    );
    let mut dice = ScriptedDice::new([15]);
    let s = battle
        .crew_act(&k, "canonnier", "bordee_babord", &aim_at("eux"), &mut dice)
        .unwrap();
    let eux = s.battle.ship("eux").unwrap();
    assert_eq!((eux.screen, eux.hull), (6, 8));
}

#[test]
fn arcs_come_from_each_ships_data() {
    // The Cure-Dent's heavy gun fires ahead only: the same corvette
    // astern is out of its arc. Its stern is blind for every weapon.
    let b = rules("brasier");
    let battle = duel(
        &b,
        map("brasier", "abords-du-toboggan"),
        ("cure_dent", c(8, 8), Facing::E),
        ("corvette_pirate", c(5, 8), Facing::E),
        vec![
            crew(&b, "canonnier", "canonnier", "piece_lourde"),
            crew(&b, "pilote", "pilote", "tourelle_dorsale"),
        ],
    );
    let mut dice = ScriptedDice::new([15]);
    for (who, action) in [
        ("canonnier", "tir_canon_lourd"),
        ("pilote", "tir_tourelle_dorsale"),
    ] {
        let r = battle.crew_act(&b, who, action, &aim_at("eux"), &mut dice);
        assert_eq!(r.err(), Some(BattleRefusal::OutOfArc), "{action}");
    }
    // The brig is the other way round: its broadsides miss what is dead
    // ahead.
    let k = rules("corsaires");
    let battle = duel(
        &k,
        map("corsaires", "large-de-belle-ile"),
        ("la_machoire", c(10, 9), Facing::N),
        ("sloop_escorte", c(10, 5), Facing::S),
        vec![crew(&k, "canonnier", "canonnier", "batterie_babord")],
    );
    let r = battle.crew_act(&k, "canonnier", "bordee_babord", &aim_at("eux"), &mut dice);
    assert_eq!(r.err(), Some(BattleRefusal::OutOfArc));
    // Turned a quarter, the same target is abeam to port.
    let battle = duel(
        &k,
        map("corsaires", "large-de-belle-ile"),
        ("la_machoire", c(10, 9), Facing::E),
        ("sloop_escorte", c(10, 5), Facing::S),
        vec![crew(&k, "canonnier", "canonnier", "batterie_babord")],
    );
    assert!(
        battle
            .crew_act(&k, "canonnier", "bordee_babord", &aim_at("eux"), &mut dice)
            .is_ok()
    );
}

#[test]
fn a_crew_member_has_the_turn_contexts_budget() {
    // Brasier: « 1 seule attaque par tour ».
    let b = rules("brasier");
    let battle = duel(
        &b,
        map("brasier", "abords-du-toboggan"),
        ("cure_dent", c(3, 8), Facing::E),
        ("carapace_sereth", c(10, 8), Facing::W),
        vec![
            crew(&b, "canonnier", "canonnier", "piece_lourde"),
            crew(&b, "mecano", "mecano", "ingenieur"),
        ],
    );
    let mut dice = ScriptedDice::new([2, 2, 2, 2]);
    let s = battle
        .crew_act(
            &b,
            "canonnier",
            "tir_canon_lourd",
            &aim_at("eux"),
            &mut dice,
        )
        .unwrap();
    let r = s.battle.crew_act(
        &b,
        "canonnier",
        "tir_canon_lourd",
        &aim_at("eux"),
        &mut dice,
    );
    assert_eq!(r.err(), Some(BattleRefusal::NoAttackLeft));
    // A charged shot needs both actions.
    let r = s
        .battle
        .crew_act(&b, "canonnier", "tir_charge", &aim_at("eux"), &mut dice);
    assert_eq!(r.err(), Some(BattleRefusal::NoActionsLeft));

    // Corsaires: two attacks, as on the ground.
    let k = rules("corsaires");
    let battle = duel(
        &k,
        map("corsaires", "large-de-belle-ile"),
        ("la_machoire", c(10, 9), Facing::N),
        ("hms_greyhound", c(6, 9), Facing::N),
        vec![
            crew(&k, "canonnier", "canonnier", "batterie_babord"),
            crew(&k, "vigie", "vigie", "hune"),
        ],
    );
    let s = battle
        .crew_act(&k, "canonnier", "bordee_babord", &aim_at("eux"), &mut dice)
        .unwrap();
    assert!(
        s.battle
            .crew_act(&k, "canonnier", "bordee_babord", &aim_at("eux"), &mut dice)
            .is_ok()
    );
}

#[test]
fn the_crew_turn_ends_when_everyone_is_done_and_the_enemy_plays() {
    let b = rules("brasier");
    let battle = duel(
        &b,
        map("brasier", "abords-du-toboggan"),
        ("cure_dent", c(3, 8), Facing::E),
        ("corvette_pirate", c(8, 8), Facing::W),
        vec![crew(&b, "toubib", "toubib", "integrite")],
    );
    let mut dice = SeededDice::new(4);
    // LUMEN holds the sensors: the GM plays it.
    let lumen = battle
        .crew
        .iter()
        .find(|m| m.npc)
        .expect("LUMEN at the sensors");
    assert_eq!(lumen.station.as_deref(), Some("capteurs"));
    let s = battle
        .crew_act(&b, "toubib", "se_braquer", &Aim::default(), &mut dice)
        .unwrap();
    let s = s.battle.pass(&b, "toubib", &mut dice).unwrap();
    let s = s.battle.pass(&b, &lumen.id, &mut dice).unwrap();
    assert!(!s.battle.crew_turn(), "the corvette's turn");
    // Braced until the ship's next turn: -4 on the corvette's 5.
    let s = s
        .battle
        .enemy_fire(&b, "eux", "canon", "nous", &mut ScriptedDice::new([19]))
        .unwrap();
    let hit = s.events.iter().find_map(|e| match e {
        BattleEvent::Fired { damage, .. } => Some(*damage),
        _ => None,
    });
    assert_eq!(hit, Some(1));
    // Once a turn.
    let r = s.battle.enemy_fire(&b, "eux", "canon", "nous", &mut dice);
    assert_eq!(r.err(), Some(BattleRefusal::AlreadyFired));
}

#[test]
fn a_hull_threshold_breaks_something_aboard_and_it_can_be_repaired() {
    let b = rules("brasier");
    let mut battle = duel(
        &b,
        map("brasier", "abords-du-toboggan"),
        ("cure_dent", c(3, 8), Facing::E),
        ("corvette_pirate", c(8, 8), Facing::W),
        vec![crew(&b, "toubib", "toubib", "integrite")],
    );
    // Shields down, hull 22: the corvette's 5 takes it under 20.
    battle = battle
        .adjust("nous", Some(22), Some(0), None)
        .unwrap()
        .battle;
    let mut dice = SeededDice::new(1);
    for m in battle.crew.clone() {
        battle = battle.pass(&b, &m.id, &mut dice).unwrap().battle;
    }
    // Hit (19), then the damage die: 1 = fire.
    let s = battle
        .enemy_fire(&b, "eux", "canon", "nous", &mut ScriptedDice::new([19, 1]))
        .unwrap();
    let nous = s.battle.ship("nous").unwrap();
    assert_eq!(nous.hull, 17);
    assert_eq!(nous.damages[0].id, "incendie");
    // At the start of the crew's next turn the fire burns 2.
    let s = s.battle.end_turn(&b, &mut dice).unwrap();
    assert!(s.battle.crew_turn());
    assert_eq!(s.battle.ship("nous").unwrap().hull, 15);
    // Damage control puts it out (CON 10: 12 + 2).
    let s = s
        .battle
        .crew_act(
            &b,
            "toubib",
            "eteindre_incendie",
            &Aim {
                damage: Some(0),
                ..Aim::default()
            },
            &mut ScriptedDice::new([12]),
        )
        .unwrap();
    assert!(s.battle.ship("nous").unwrap().damages.is_empty());
    // The wrong station cannot.
    let r = s
        .battle
        .crew_act(&b, "toubib", "reparer_systeme", &Aim::default(), &mut dice);
    assert!(matches!(r.err(), Some(BattleRefusal::NotAtStation { .. })));
}

#[test]
fn morale_is_a_second_way_to_win() {
    let k = rules("corsaires");
    let mut battle = duel(
        &k,
        map("corsaires", "large-de-belle-ile"),
        ("la_machoire", c(10, 9), Facing::N),
        ("sloop_escorte", c(6, 9), Facing::N),
        vec![crew(&k, "qm", "quartier_maitre", "gaillard")],
    );
    let mut dice = ScriptedDice::new([18, 18]);
    // Morale 6, two summons of 3.
    battle = battle
        .crew_act(&k, "qm", "sommer_d_amener", &aim_at("eux"), &mut dice)
        .unwrap()
        .battle;
    let s = battle
        .crew_act(&k, "qm", "sommer_d_amener", &aim_at("eux"), &mut dice)
        .unwrap();
    assert_eq!(s.battle.ship("eux").unwrap().standing, ShipStanding::Struck);
    let end = s.battle.end.as_ref().unwrap();
    assert_eq!(
        (end.winner, end.reason),
        (Some(Side::Party), BattleEndReason::Victory)
    );
    // Successes earn XP, as on the ground (+1 per success).
    assert_eq!(end.xp.get("qm"), Some(&2));

    // A Vorr swarm has no morale to break.
    let b = rules("brasier");
    let battle = duel(
        &b,
        map("brasier", "abords-du-toboggan"),
        ("cure_dent", c(3, 8), Facing::E),
        ("chasseur_vorr", c(8, 8), Facing::W),
        vec![crew(&b, "qm", "quartier_maitre", "liaison")],
    );
    let r = battle.crew_act(&b, "qm", "briser_le_moral", &aim_at("eux"), &mut dice);
    assert_eq!(r.err(), Some(BattleRefusal::Immune));
}

#[test]
fn changing_station_costs_an_action_and_lumen_steps_aside() {
    let b = rules("brasier");
    let battle = duel(
        &b,
        map("brasier", "abords-du-toboggan"),
        ("cure_dent", c(3, 8), Facing::E),
        ("corvette_pirate", c(8, 8), Facing::W),
        vec![crew(&b, "xeno", "xenologue", "tourelle_dorsale")],
    );
    let s = battle.take_station(&b, "xeno", Some("capteurs")).unwrap();
    let xeno = s.battle.crew_member("xeno").unwrap();
    assert_eq!(
        (xeno.station.as_deref(), xeno.actions),
        (Some("capteurs"), 1)
    );
    assert!(s.battle.crew.iter().any(|m| m.npc && m.station.is_none()));
    // With its last action, scan: the players then see the corvette.
    let s = s
        .battle
        .crew_act(
            &b,
            "xeno",
            "scanner",
            &aim_at("eux"),
            &mut ScriptedDice::new([]),
        )
        .unwrap();
    assert!(s.battle.ship("eux").unwrap().scanned);
}

#[test]
fn a_manoeuvre_moves_the_bow_a_quarter_and_the_wind_counts() {
    let k = rules("corsaires");
    let v = k.vehicles.as_ref().unwrap();
    let battle = duel(
        &k,
        map("corsaires", "large-de-belle-ile"),
        ("la_machoire", c(10, 9), Facing::E),
        ("sloop_escorte", c(2, 14), Facing::N),
        vec![crew(&k, "nav", "navigateur", "barre")],
    );
    // Westerly: bow east sails 3 + 2.
    let me = battle.party_ship().clone();
    assert_eq!(battle.maneuver_budget(v, &me, 1), Some(5));
    let mut dice = ScriptedDice::new([]);
    let turned = Aim {
        to: Some(c(14, 9)),
        facing: Some(Facing::W),
        ..Aim::default()
    };
    let r = battle.crew_act(&k, "nav", "manoeuvrer", &turned, &mut dice);
    assert_eq!(r.err(), Some(BattleRefusal::TooSharpATurn));
    let ok = Aim {
        to: Some(c(15, 9)),
        facing: Some(Facing::N),
        ..Aim::default()
    };
    let s = battle
        .crew_act(&k, "nav", "manoeuvrer", &ok, &mut dice)
        .unwrap();
    let me = s.battle.party_ship();
    assert_eq!((me.at, me.facing), (c(15, 9), Facing::N));
    // Into the wind, the brig makes 1.
    let mut into = battle.clone();
    into.ships[0].facing = Facing::W;
    let me = into.party_ship().clone();
    assert_eq!(into.maneuver_budget(v, &me, 1), Some(1));
}

#[test]
fn boarding_hands_the_battle_to_the_deck_and_back() {
    let k = rules("corsaires");
    let battle = duel(
        &k,
        map("corsaires", "large-de-belle-ile"),
        ("la_machoire", c(10, 9), Facing::N),
        ("hms_greyhound", c(14, 9), Facing::N),
        vec![crew(&k, "nav", "navigateur", "barre")],
    );
    assert_eq!(
        battle.board(&k, "nous", "eux").err(),
        Some(BattleRefusal::OutOfRange)
    );
    let mut close = battle.clone();
    close.ships[1].at = c(12, 9);
    let s = close.board(&k, "nous", "eux").unwrap();
    // While the deck fight lasts, the battle waits.
    let r = s.battle.crew_act(
        &k,
        "nav",
        "prendre_le_vent",
        &Aim::default(),
        &mut ScriptedDice::new([]),
    );
    assert_eq!(r.err(), Some(BattleRefusal::Boarding));
    let s = s.battle.end_boarding(true).unwrap();
    assert_eq!(
        s.battle.ship("eux").unwrap().standing,
        ShipStanding::Captured
    );
    assert_eq!(
        s.battle.end.as_ref().unwrap().reason,
        BattleEndReason::Victory
    );
}

#[test]
fn a_swarm_shares_one_initiative() {
    let b = rules("brasier");
    let sc = scenario("brasier", "essaim-vorr");
    let battle = sc
        .battle(&b, &map("brasier", "abords-du-toboggan"), 3)
        .unwrap();
    let squads: Vec<&Unit> = battle
        .units
        .iter()
        .filter(|u| u.side == Side::Opposition)
        .collect();
    assert_eq!(squads.len(), 1);
    assert_eq!(squads[0].ships.len(), 5);
    // Everyone sits where their class suits, LUMEN stays out.
    let seat = |id: &str| battle.crew_member(id).unwrap().station.clone();
    assert_eq!(seat("pilote").as_deref(), Some("barre"));
    assert_eq!(seat("xenologue").as_deref(), Some("capteurs"));
    assert!(battle.crew.iter().all(|m| !m.npc));
}

#[test]
fn the_co_gm_proposes_a_whole_enemy_turn_on_a_copy() {
    let b = rules("brasier");
    let sc = scenario("brasier", "essaim-vorr");
    let mut battle = sc
        .battle(&b, &map("brasier", "abords-du-toboggan"), 3)
        .unwrap();
    let mut dice = SeededDice::new(9);
    while battle.crew_turn() {
        let id = battle.crew.iter().find(|m| !m.done).unwrap().id.clone();
        battle = battle.pass(&b, &id, &mut dice).unwrap().battle;
    }
    let before = battle.clone();
    let (orders, events, after) = propose_enemy_turn(&b, &battle, &mut dice);
    assert_eq!(battle, before, "the battle itself is untouched");
    assert!(orders.iter().any(|o| matches!(o, Order::Maneuver { .. })));
    assert_eq!(orders.last(), Some(&Order::EndTurn));
    assert!(!events.is_empty());
    assert!(after.crew_turn() || after.is_over());
}

/// Both battles of the ticket, simulated: they end, the engine never
/// refuses the tactics, the runs are reproducible, and a battle fits in
/// 45 minutes at the table.
#[test]
fn both_battles_simulate_under_45_minutes() {
    for (world, id, map_id) in [
        ("corsaires", "interception-greyhound", "large-de-belle-ile"),
        ("brasier", "essaim-vorr", "abords-du-toboggan"),
    ] {
        let system = rules(world);
        let sc = scenario(world, id);
        let m = map(world, map_id);
        let spec = Some(BattleSpec {
            battles: 30,
            seed: 1,
        });
        let r = sc
            .simulate(&system, &m, spec, BattleTime::default())
            .unwrap();
        eprintln!(
            "{id}: {:.0}% won, {} lost, {} stalemates, rounds {:.1} (max {}), {:.0} min (max {:.0}), hull left {:.1}, damages {:.1}, destroyed {}, struck {}",
            r.party_win_rate * 100.0,
            r.defeats,
            r.stalemates,
            r.rounds.median,
            r.rounds.max,
            r.minutes.median,
            r.minutes.max,
            r.hull_left.mean,
            r.damages_aboard,
            r.destroyed,
            r.struck
        );
        assert_eq!(r.refusals, 0, "{id}");
        assert_eq!(r.stalemates, 0, "{id}");
        assert!(r.minutes.median < 45.0, "{id}: {} min", r.minutes.median);
        let again = sc
            .simulate(&system, &m, spec, BattleTime::default())
            .unwrap();
        assert_eq!(r, again, "{id}: reproducible");
    }
}

#[test]
fn a_whole_battle_runs_and_ends() {
    let b = rules("brasier");
    let sc = scenario("brasier", "essaim-vorr");
    let battle = sc
        .battle(&b, &map("brasier", "abords-du-toboggan"), 5)
        .unwrap();
    let log = run_battle(&b, battle, &mut SeededDice::new(5));
    assert!(log.battle.is_over());
    assert!(log.refusals.is_empty(), "{:?}", log.refusals);
}
