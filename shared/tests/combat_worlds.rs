//! Whole fights on the two witness maps, played to the end with seeded
//! dice, from the scenarios in `content/scenarios/`: the dock of
//! Port-Louis (six Corsaires against Gueule-Rouge and five sailors, act 1
//! scene 4) and the Cure-Dent's corridor (six crew against the Vorr
//! boarding party of the Brasier campaign's first scene). A ticket is
//! done only when it works on both worlds.

use std::collections::BTreeMap;
use std::path::PathBuf;

use promptus_shared::combat::fight::Standing;
use promptus_shared::combat::{
    EndReason, Fight, FightEvent, FightLog, Limits, PolicyKind, Scenario, simulate::play,
};
use promptus_shared::maps::{Cell, DoorState, Map, standable};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::check::{ModifierSource, OutcomeBand};
use promptus_shared::rules::dice::SeededDice;
use promptus_shared::rules::events::Event;
use promptus_shared::rules::model::Tag;
use promptus_shared::rules::sheet::{Combatant, Side};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// A scenario, the rule system it fights under and its map.
struct World {
    system: RuleSystem,
    scenario: Scenario,
    map: Map,
}

fn world(world: &str, id: &str) -> World {
    let scenario = Scenario::from_yaml(&read(&format!("content/scenarios/{world}/{id}.yaml")))
        .unwrap_or_else(|e| panic!("{id}: {e}"));
    let rules = read(&format!(
        "content/rules/{}/v{}.yaml",
        scenario.rules, scenario.version
    ));
    let system = scenario
        .system(&rules, None)
        .unwrap_or_else(|e| panic!("{id}: {e}"));
    let map = Map::from_yaml(&read(&format!(
        "content/maps/{world}/{}.yaml",
        scenario.map
    )))
    .unwrap_or_else(|e| panic!("{id}: {e}"));
    World {
        system,
        scenario,
        map,
    }
}

impl World {
    fn play(&self, party: PolicyKind, seed: u64) -> FightLog {
        play(
            &self.system,
            &self.scenario,
            &self.map,
            party,
            PolicyKind::Brawler,
            seed,
            Limits::default(),
        )
        .unwrap()
    }
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

fn dock() -> World {
    world("corsaires", "bagarre-du-quai")
}

const CREW: [&str; 6] = [
    "gueule_rouge",
    "marin_1",
    "marin_2",
    "marin_3",
    "marin_4",
    "marin_5",
];

#[test]
fn the_dock_fight_is_played_to_the_end_with_the_crew_s_tactics() {
    let w = dock();
    for party in [PolicyKind::Brawler, PolicyKind::Focus] {
        let mut fled = 0;
        for seed in SEEDS {
            let log = w.play(party, seed);
            check_invariants(&w.system, &log);
            let end = log.fight.end.as_ref().unwrap();
            // "Si les joueurs perdent (très improbable)": brawling, they
            // win every time (focusing the weakest can lose, rarely: the
            // report shows how often).
            if party == PolicyKind::Brawler {
                assert_eq!(end.winner, Some(Side::Party), "seed {seed}");
                // Every sailor is defeated or ran; nobody of the crew is left.
                for id in CREW {
                    assert_ne!(
                        log.fight.standing[id],
                        Standing::InFight,
                        "seed {seed}: {id}"
                    );
                }
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
}

#[test]
fn gueule_rouge_stands_while_his_sailors_run() {
    let w = dock();
    let mut sailors_fled = 0;
    for seed in SEEDS {
        let log = w.play(PolicyKind::Brawler, seed);
        let end = log.fight.end.as_ref().unwrap();
        // The leader never runs (no `retreat_all` on the dock).
        assert!(
            !end.fled.contains(&"gueule_rouge".to_string()),
            "seed {seed}"
        );
        sailors_fled += end.fled.len();
    }
    assert!(sailors_fled > 0);
}

#[test]
fn the_same_seed_plays_the_same_fight() {
    let w = dock();
    let a = w.play(PolicyKind::Brawler, 7);
    let b = w.play(PolicyKind::Brawler, 7);
    assert_eq!(a.log, b.log);
    assert_eq!(a.fight, b.fight);
    let c = w.play(PolicyKind::Brawler, 8);
    assert_ne!(a.log, c.log, "another seed, another fight");
}

#[test]
fn the_dock_fight_shows_moves_shots_cover_and_falls() {
    let w = dock();
    let log = w.play(PolicyKind::Brawler, 7);
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
        w.play(PolicyKind::Brawler, seed).events().any(|e| {
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

fn corridor() -> World {
    world("brasier", "abordage-coursive")
}

#[test]
fn the_corridor_fight_is_played_to_the_end() {
    let w = corridor();
    for party in [PolicyKind::Brawler, PolicyKind::Focus] {
        for seed in SEEDS {
            let log = w.play(party, seed);
            check_invariants(&w.system, &log);
            // The crew comes through the airlock door, or the boarders
            // come out of it.
            assert!(
                log.events().any(|e| matches!(
                    e,
                    FightEvent::Moved { path, .. } if path.contains(&Cell::new(5, 10))
                )),
                "{party:?}, seed {seed}"
            );
        }
    }
}

#[test]
fn the_focus_party_does_not_stop_in_the_airlock_door() {
    let w = corridor();
    for seed in SEEDS {
        let log = w.play(PolicyKind::Focus, seed);
        for e in log.events() {
            if let FightEvent::Moved { who, path, .. } = e
                && log.fight.combatant(who).unwrap().side == Side::Party
            {
                assert_ne!(path.last(), Some(&Cell::new(5, 10)), "{who}, seed {seed}");
            }
        }
    }
}

/// The scenario's Vorr are the campaign's: same numbers, only what the
/// rules format needs on top.
#[test]
fn the_corridor_boarders_are_the_campaign_s_stat_blocks() {
    let w = corridor();
    let campaign =
        promptus_shared::story::from_yaml(&read("content/campaigns/brasier/campagne.yaml"))
            .unwrap();
    let placed: Vec<&str> = w
        .scenario
        .opposition
        .fighters
        .iter()
        .filter_map(|f| f.adversary.as_deref())
        .collect();
    for block in ["abordeur-vorr", "chef-d-escouade-vorr"] {
        assert!(placed.contains(&block));
        let theirs = campaign
            .adversaries
            .iter()
            .find(|a| a.id == block)
            .unwrap_or_else(|| panic!("{block} not in the campaign"));
        let ours = w.system.adversary(block).unwrap();
        assert_eq!(ours.name, theirs.name);
        assert_eq!(ours.abilities, theirs.stats.abilities, "{block}");
        assert_eq!(Some(ours.armor_class), theirs.stats.armor_class, "{block}");
        assert_eq!(Some(ours.hit_points), theirs.stats.hit_points, "{block}");
        let damage = |a: &promptus_shared::rules::model::ActionDef| {
            a.tags.iter().find_map(|t| match t {
                Tag::Damage(d) => Some(d.amount.to_string()),
                _ => None,
            })
        };
        let ours: Vec<(String, Option<String>)> = ours
            .actions
            .iter()
            .map(|a| (a.name.clone(), damage(a)))
            .collect();
        let theirs: Vec<(String, Option<String>)> = theirs
            .stats
            .attacks
            .iter()
            .map(|a| (a.name.clone(), Some(a.damage.clone())))
            .collect();
        assert_eq!(ours, theirs, "{block}");
    }
}

#[test]
fn a_closed_airlock_keeps_the_boarders_out_of_reach() {
    let w = corridor();
    let s = &w.system;
    let pilote = Combatant::from_class(s, "pilote", "pilote", "pilote").unwrap();
    let vorr = Combatant::from_adversary(s, "vorr", "vorr", "chef-d-escouade-vorr").unwrap();
    let step = Fight::start(
        s,
        "sol",
        w.map.clone(),
        vec![(pilote, Cell::new(7, 10)), (vorr, Cell::new(3, 10))],
        &mut SeededDice::new(1),
    )
    .unwrap();
    let f = step.fight;
    let tir = s.class("pilote").unwrap().actions[0].clone();
    // Four cells away, in range — but the inner door is shut.
    assert!(f.reach(s, "pilote", &tir, "vorr").is_err());
    let mut open = f.clone();
    open.map
        .set_door_state("sas-babord-interieur", DoorState::Open);
    let r = open.reach(s, "pilote", &tir, "vorr").unwrap();
    assert_eq!(r.distance, 4);
}
