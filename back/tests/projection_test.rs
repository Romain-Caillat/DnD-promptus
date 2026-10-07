//! The player projection drops every GM-only field (`MEMORY.md` §3).
//!
//! Each GM-only field of the fixture is overwritten with a marker naming
//! it; the world puts the table in the scene that shows the most (NPCs,
//! a fight, loot, clues found). Not one marker may reach the view, and
//! the fields players are entitled to must.
//!
//! Ported from V1's `projection.test.ts` where V1 already had the
//! matter: no summary, GM notes, secrets or clue ids; an adversary's name
//! only once revealed. Its map, turn, action and journal cases come back
//! with the features that serve them (the grid map's projection is
//! tested in `shared/tests/maps_format.rs`); the HTTP routes are swept
//! in `player_routes_test.rs`.

mod common;

use common::marked::{FIXTURE, leaks, marked, world};
use promptus_back::campaigns::projection::{UNKNOWN_OPPONENT, project_for_players};
use promptus_shared::story::{WorldState, from_yaml};

#[test]
fn no_gm_only_field_reaches_players() {
    let c = marked();
    let view = project_for_players(&c, &world(&c));
    let json = serde_json::to_string(&view).unwrap();
    let leaks = leaks(&json);
    assert!(leaks.is_empty(), "GM-only fields leaked: {leaks:#?}");
    // No clue or revelation id either: ids can be telling.
    assert!(!json.contains("cl_") && !json.contains("rev_"), "{json}");
}

#[test]
fn what_players_are_entitled_to_is_there() {
    let c = marked();
    let view = project_for_players(&c, &world(&c));
    let scene = view.scene.as_ref().unwrap();
    assert_eq!(scene.title, "La crique aux Morts");
    assert!(scene.read_aloud.starts_with("Des lanternes dansent"));
    assert_eq!(scene.place.as_ref().unwrap().name, "La crique aux Morts");
    assert_eq!(scene.music.len(), 2);
    assert_eq!(view.clues.len(), 2);
    assert!(view.clues[0].starts_with("Gwen a vu"));
    assert_eq!(view.player_hook, c.bible.player_hook);
    assert_eq!(view.party.len(), 3);
    // Loïc, met, is shown by name in the scene; Corentin, not met, is not.
    let names: Vec<&str> = scene.npcs.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, vec!["Loïc Kervella"]);
    assert_eq!(view.npcs.len(), 1);
    // Opponents: counted, unnamed until revealed.
    let opp: Vec<(&str, u32)> = scene
        .opponents
        .iter()
        .map(|o| (o.label.as_str(), o.count))
        .collect();
    assert_eq!(opp, vec![(UNKNOWN_OPPONENT, 1), (UNKNOWN_OPPONENT, 4)]);
    // campaign/track-factions-and-goals: the customs, met, with their
    // gauge; the wreckers, not met, are absent; the goal, ticked.
    let factions: Vec<(&str, i32, i32, i32)> = view
        .factions
        .iter()
        .map(|f| (f.name.as_str(), f.affinity, f.min, f.max))
        .collect();
    assert_eq!(factions, vec![("La douane royale", 1, -5, 5)]);
    let goals: Vec<(&str, bool)> = view
        .goals
        .iter()
        .map(|g| (g.title.as_str(), g.done))
        .collect();
    assert_eq!(goals, vec![("Rallumer le phare", true)]);
}

#[test]
fn a_revealed_opponent_is_named_but_its_hit_points_stay_hidden() {
    let c = from_yaml(FIXTURE).unwrap();
    let mut w = world(&c);
    w.reveal_entity(&c, "adv_contrebandier").unwrap();
    let view = project_for_players(&c, &w);
    let labels: Vec<&str> = view
        .scene
        .as_ref()
        .unwrap()
        .opponents
        .iter()
        .map(|o| o.label.as_str())
        .collect();
    assert_eq!(labels, vec![UNKNOWN_OPPONENT, "Contrebandier"]);
    let json = serde_json::to_string(&view).unwrap();
    assert!(!json.contains("hit_points") && !json.contains("hitPoints") && !json.contains("armor"));
}

#[test]
fn outside_any_scene_players_see_no_scene() {
    let c = from_yaml(FIXTURE).unwrap();
    let view = project_for_players(&c, &WorldState::default());
    assert!(view.scene.is_none());
    assert!(view.clues.is_empty() && view.npcs.is_empty());
}
