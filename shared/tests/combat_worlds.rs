//! Whole fights on the two witness maps, played to the end with seeded
//! dice: the dock of Port-Louis (six Corsaires against Gueule-Rouge and
//! five sailors, act 1 scene 4) and the Cure-Dent's corridor (six crew
//! against a Vorr boarding party). A ticket is done only when it works on
//! both worlds.

use std::collections::BTreeMap;
use std::path::PathBuf;

use promptus_shared::combat::fight::Standing;
use promptus_shared::combat::{
    Brawler, Decision, EndReason, Fight, FightEvent, FightLog, Limits, Policy, run_fight,
};
use promptus_shared::maps::{Cell, DoorState, Map, Side as MapSide, standable};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::check::{ModifierSource, OutcomeBand};
use promptus_shared::rules::dice::SeededDice;
use promptus_shared::rules::events::Event;
use promptus_shared::rules::sheet::{Combatant, Side};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn map(rel: &str) -> Map {
    Map::from_yaml(&read(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The map's start cells for one side, in file order.
fn starts(map: &Map, side: MapSide) -> Vec<Cell> {
    map.starts
        .iter()
        .filter(|s| s.side == Some(side))
        .map(|s| s.at)
        .collect()
}

/// Seeds every fight below is played with.
const SEEDS: std::ops::RangeInclusive<u64> = 1..=20;

/// What every finished fight must satisfy, whatever the dice.
fn check_invariants(system: &RuleSystem, log: &FightLog) {
    let fight = &log.fight;
    let end = fight.end.as_ref().expect("the fight ended");
    assert_eq!(end.reason, EndReason::SideDown, "decided, not stopped");
    assert!(end.winner.is_some());
    // The policies only ask for what the engine accepts: what `check`
    // allows, `act` and `move_along` play.
    let refusals: Vec<_> = log.refusals().collect();
    assert!(refusals.is_empty(), "refused: {refusals:?}");
    // Nobody shares a cell or stands in a wall.
    let rules = promptus_shared::combat::fight::movement_rules(system, &fight.map);
    let mut seen = BTreeMap::new();
    for (id, cell) in &fight.positions {
        assert!(
            standable(&fight.map, &rules, *cell).is_ok(),
            "{id} on {cell:?}"
        );
        assert!(seen.insert(*cell, id).is_none(), "two on {cell:?}");
        assert!(fight.in_fight(id));
    }
    // XP is exactly what the bands of each character's rolls grant.
    let mut granted: BTreeMap<String, u32> = BTreeMap::new();
    for e in log.events() {
        let (who, band) = match e {
            FightEvent::Rules {
                event: Event::Roll {
                    roller, breakdown, ..
                },
            } => (roller, breakdown.band),
            FightEvent::FleeRoll { who, roll } => (who, roll.band),
            _ => continue,
        };
        if let Some(b) = band {
            *granted.entry(who.clone()).or_default() += b.xp(system);
        }
    }
    for (pc, xp) in &end.xp {
        assert_eq!(*xp, granted.get(pc).copied().unwrap_or(0), "{pc}'s XP");
    }
    // The winners' foes are all out of the fight or on the ground.
    let loser = match end.winner {
        Some(Side::Party) => Side::Opposition,
        _ => Side::Party,
    };
    for id in fight.side(loser) {
        assert_eq!(fight.combatant(id).unwrap().hit_points, 0, "{id} still up");
    }
    // Initiative was rolled once for everyone and the order never changed.
    assert_eq!(fight.order.len(), fight.scene.combatants.len());
}

// ---------------------------------------------------------- Corsaires

fn corsaires() -> RuleSystem {
    RuleSystem::from_yaml(&read("content/rules/corsaires/v1.yaml")).unwrap()
}

const CREW: [&str; 5] = ["marin_1", "marin_2", "marin_3", "marin_4", "marin_5"];

/// Act 1, scene 4: six characters on the quay, Gueule-Rouge and his five
/// sailors where the map's ambush layer puts them.
fn dock_fight(system: &RuleSystem, seed: u64) -> (promptus_shared::combat::Step, SeededDice) {
    let dock = map("content/maps/corsaires/quai-port-louis.yaml");
    let classes = [
        "bretteur",
        "canonnier",
        "navigateur",
        "chirurgien",
        "vigie",
        "flibustier",
    ];
    let mut placements: Vec<(Combatant, Cell)> = classes
        .iter()
        .zip(starts(&dock, MapSide::Party))
        .map(|(class, at)| {
            (
                Combatant::from_class(system, class, class, class).unwrap(),
                at,
            )
        })
        .collect();
    let foes = starts(&dock, MapSide::Foes);
    placements.push((
        Combatant::from_adversary(system, "gueule_rouge", "Gueule-Rouge", "gueule_rouge").unwrap(),
        foes[0],
    ));
    for (id, at) in CREW.iter().zip(&foes[1..]) {
        placements.push((
            Combatant::from_adversary(system, id, id, "marin_de_gueule_rouge").unwrap(),
            *at,
        ));
    }
    let mut dice = SeededDice::new(seed);
    let step = Fight::start(system, "sol", dock, placements, &mut dice).unwrap();
    (step, dice)
}

/// Gueule-Rouge's crew as acte_1.md writes its tactics: "if three
/// sailors fall, the other two try to flee; if Gueule-Rouge falls, every
/// sailor left flees at once". Otherwise they brawl.
struct GueuleRougeTactics(Brawler);

impl Policy for GueuleRougeTactics {
    fn decide(&mut self, system: &RuleSystem, fight: &Fight, who: &str) -> Decision {
        let down = |id: &str| fight.standing.get(id) != Some(&Standing::InFight);
        let sailors_down = CREW.iter().filter(|id| down(id)).count();
        let fresh = fight
            .combatant(who)
            .is_some_and(|c| c.turn.actions_left == 2);
        if who != "gueule_rouge" && fresh && (down("gueule_rouge") || sailors_down >= 3) {
            return Decision::Flee { difficulty: None };
        }
        self.0.decide(system, fight, who)
    }
}

fn play_dock(system: &RuleSystem, seed: u64) -> FightLog {
    let (step, mut dice) = dock_fight(system, seed);
    run_fight(
        system,
        step,
        &mut Brawler::default(),
        &mut GueuleRougeTactics(Brawler::default()),
        &mut dice,
        Limits::default(),
    )
}

#[test]
fn the_dock_fight_is_played_to_the_end_with_the_crew_s_tactics() {
    let s = corsaires();
    let mut fled = 0;
    for seed in SEEDS {
        let log = play_dock(&s, seed);
        check_invariants(&s, &log);
        let end = log.fight.end.as_ref().unwrap();
        // "Si les joueurs perdent (très improbable)": they win.
        assert_eq!(end.winner, Some(Side::Party), "seed {seed}");
        // Every sailor is defeated or ran; nobody of the crew is left.
        for id in CREW.iter().chain(&["gueule_rouge"]) {
            assert_ne!(
                log.fight.standing[*id],
                Standing::InFight,
                "seed {seed}: {id}"
            );
        }
        fled += end.fled.len();
        // Twelve fighters rolled initiative, the party first on ties.
        assert_eq!(log.fight.initiative.len(), 12);
        for w in log.fight.initiative.windows(2) {
            assert!(w[0].total >= w[1].total);
        }
        // Someone landed a hit and earned XP for it.
        assert!(end.xp.values().sum::<u32>() > 0, "seed {seed}");
    }
    assert!(fled > 0, "the tactics make sailors run in some fights");
}

#[test]
fn the_same_seed_plays_the_same_fight() {
    let s = corsaires();
    let a = play_dock(&s, 7);
    let b = play_dock(&s, 7);
    assert_eq!(a.log, b.log);
    assert_eq!(a.fight, b.fight);
    let c = play_dock(&s, 8);
    assert_ne!(a.log, c.log, "another seed, another fight");
}

#[test]
fn the_dock_fight_shows_moves_shots_cover_and_falls() {
    let s = corsaires();
    let log = play_dock(&s, 7);
    let events: Vec<&FightEvent> = log.events().collect();
    let moved = events
        .iter()
        .filter(|e| matches!(e, FightEvent::Moved { .. }))
        .count();
    assert!(moved > 0, "the two lines start out of reach");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, FightEvent::Acted { action, .. } if action == "tir_embusque")),
        "the vigie shoots from afar"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, FightEvent::Defeated { .. })),
        "someone falls"
    );
    let rolls_with_bands = events.iter().any(|e| {
        matches!(
            e,
            FightEvent::Rules {
                event: Event::Roll { breakdown, .. }
            } if breakdown.band == Some(OutcomeBand::Success)
        )
    });
    assert!(rolls_with_bands);
    // Crates, barrels and bodies on the quay cover someone in some fight.
    let covered = SEEDS.into_iter().any(|seed| {
        play_dock(&s, seed).events().any(|e| {
            matches!(
                e,
                FightEvent::Rules {
                    event: Event::Roll { breakdown, .. }
                } if breakdown
                    .modifiers
                    .iter()
                    .any(|m| matches!(m.source, ModifierSource::Cover(_)))
            )
        })
    });
    assert!(covered);
}

// ---------------------------------------------------------- Brasier

/// The Brasier's draft has no ground adversary yet (the source has
/// none). Until `campaign/rewrite-two-worlds` writes Act 1's boarders,
/// this test writes stand-ins into its copy of the file, the way Romain
/// would add a stat block.
const VORR: &str = r#"adversaries:
  - id: abordeur_vorr
    name: Abordeur vorr
    abilities: { FOR: 12, DEX: 12, CON: 11, INT: 9, SAG: 10, CHA: 8 }
    armor_class: 11
    hit_points: 5
    actions:
      - { id: vorr_lame, name: Lame d'abordage, kind: attaque, target: enemy, roll: attack, ability: FOR, tags: [{ damage: { amount: 1d4+1 } }] }
      - { id: vorr_decharge, name: Pistolet à décharge, kind: attaque, target: enemy, range: 8, roll: attack, ability: DEX, tags: [{ damage: { amount: 1d4 } }] }
  - id: chef_d_escouade_vorr
    name: Chef d'escouade vorr
    abilities: { FOR: 14, DEX: 12, CON: 13, INT: 10, SAG: 11, CHA: 10 }
    armor_class: 13
    hit_points: 9
    actions:
      - { id: chef_vorr_hache, name: Hache à plasma, kind: attaque, target: enemy, roll: attack, ability: FOR, tags: [{ damage: { amount: 1d6+2 } }] }
      - { id: chef_vorr_decharge, name: Fusil à décharge, kind: attaque, target: enemy, range: 10, roll: attack, ability: DEX, tags: [{ damage: { amount: 1d6 } }] }
"#;

fn brasier() -> RuleSystem {
    let text = read("content/rules/brasier/v1.yaml");
    let system = RuleSystem::from_yaml(&text).unwrap();
    if system.adversary("abordeur_vorr").is_some() {
        return system;
    }
    assert!(text.contains("adversaries: []"));
    RuleSystem::from_yaml(&text.replacen("adversaries: []\n", VORR, 1)).unwrap()
}

/// The Vorr have docked at the port airlock and forced its inner door;
/// the crew comes down the corridor from the bridge.
fn corridor_fight(system: &RuleSystem, seed: u64) -> FightLog {
    let mut corridor = map("content/maps/brasier/cure-dent-coursive.yaml");
    assert!(corridor.set_door_state("sas-babord-interieur", DoorState::Open));
    let classes = [
        "pilote",
        "canonnier",
        "mecano",
        "xenologue",
        "toubib",
        "quartier_maitre",
    ];
    let mut placements: Vec<(Combatant, Cell)> = classes
        .iter()
        .zip(starts(&corridor, MapSide::Party))
        .map(|(class, at)| {
            (
                Combatant::from_class(system, class, class, class).unwrap(),
                at,
            )
        })
        .collect();
    for (i, at) in starts(&corridor, MapSide::Foes).into_iter().enumerate() {
        let (id, block) = if i == 5 {
            ("chef".to_string(), "chef_d_escouade_vorr")
        } else {
            (format!("vorr_{}", i + 1), "abordeur_vorr")
        };
        placements.push((
            Combatant::from_adversary(system, &id, &id, block).unwrap(),
            at,
        ));
    }
    let mut dice = SeededDice::new(seed);
    let step = Fight::start(system, "sol", corridor, placements, &mut dice).unwrap();
    run_fight(
        system,
        step,
        &mut Brawler::default(),
        &mut Brawler::default(),
        &mut dice,
        Limits::default(),
    )
}

#[test]
fn the_corridor_fight_is_played_to_the_end() {
    let s = brasier();
    let mut wins = BTreeMap::new();
    for seed in SEEDS {
        let log = corridor_fight(&s, seed);
        check_invariants(&s, &log);
        let end = log.fight.end.as_ref().unwrap();
        *wins
            .entry(format!("{:?}", end.winner.unwrap()))
            .or_insert(0) += 1;
        // The crew comes through the airlock door to fight.
        assert!(log.events().any(
            |e| matches!(e, FightEvent::Moved { path, .. } if path.contains(&Cell::new(5, 10)))
        ));
    }
    // Rushing one by one through the airlock door against six stand-ins
    // goes either way: neither side wins every time.
    assert_eq!(wins.len(), 2, "{wins:?}");
}

#[test]
fn a_closed_airlock_keeps_the_boarders_out_of_reach() {
    let s = brasier();
    let corridor = map("content/maps/brasier/cure-dent-coursive.yaml");
    let pilote = Combatant::from_class(&s, "pilote", "pilote", "pilote").unwrap();
    let vorr = Combatant::from_adversary(&s, "vorr", "vorr", "abordeur_vorr").unwrap();
    let step = Fight::start(
        &s,
        "sol",
        corridor,
        vec![(pilote, Cell::new(7, 10)), (vorr, Cell::new(3, 10))],
        &mut SeededDice::new(1),
    )
    .unwrap();
    let f = step.fight;
    let tir = s.class("pilote").unwrap().actions[0].clone();
    // Four cells away, in range — but the inner door is shut.
    assert!(f.reach(&s, "pilote", &tir, "vorr").is_err());
    let mut open = f.clone();
    open.map
        .set_door_state("sas-babord-interieur", DoorState::Open);
    let r = open.reach(&s, "pilote", &tir, "vorr").unwrap();
    assert_eq!(r.distance, 4);
}
