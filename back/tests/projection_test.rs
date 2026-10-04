//! The player projection drops every GM-only field (`MEMORY.md` §3).
//!
//! Each GM-only field of the fixture is overwritten with a marker naming
//! it; the world puts the table in the scene that shows the most (NPCs,
//! a fight, loot, clues found). Not one marker may reach the view, and
//! the fields players are entitled to must.

use promptus_back::campaigns::projection::{UNKNOWN_OPPONENT, project_for_players};
use promptus_shared::story::{Campaign, WorldState, from_yaml};

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");
/// Hit points no text of the fixture contains.
const SECRET_HP: i32 = 4_271;

fn m(field: &str) -> String {
    format!("GMONLY<{field}>")
}

/// The fixture with every GM-only field marked.
fn marked() -> Campaign {
    let mut c = from_yaml(FIXTURE).unwrap();
    let b = &mut c.bible;
    b.pitch = m("bible.pitch");
    b.tone = m("bible.tone");
    b.themes = vec![m("bible.themes")];
    b.art_direction = m("bible.art_direction");
    b.truths = vec![m("bible.truths")];
    b.secrets = vec![m("bible.secrets")];
    for a in &mut c.acts {
        a.summary = m("acts.summary");
        a.gm_notes = m("acts.gm_notes");
    }
    for f in &mut c.fronts {
        f.name = m("fronts.name");
        f.goal = m("fronts.goal");
        f.description = m("fronts.description");
        for s in &mut f.steps {
            s.label = m("fronts.steps.label");
            s.description = m("fronts.steps.description");
        }
    }
    for r in &mut c.revelations {
        r.statement = m("revelations.statement");
    }
    for cl in &mut c.clues {
        cl.discovery = m("clues.discovery");
        if !["cl_gwen", "cl_carte"].contains(&cl.id.as_str()) {
            cl.text = m("clues.text (not found)");
        }
    }
    for n in &mut c.nodes {
        n.summary = m("nodes.summary");
        n.flow = m("nodes.flow");
        n.hook = m("nodes.hook");
        n.ambience.mood = m("nodes.ambience.mood");
        n.ambience.sounds = m("nodes.ambience.sounds");
        for k in &mut n.checks {
            k.action = m("nodes.checks.action");
            k.success = m("nodes.checks.success");
            k.failure = m("nodes.checks.failure");
            k.natural_1 = m("nodes.checks.natural_1");
            k.natural_20 = m("nodes.checks.natural_20");
        }
        for p in &mut n.npcs {
            p.role = m("nodes.npcs.role");
        }
        n.key_points = vec![m("nodes.key_points")];
        if let Some(e) = &mut n.encounter {
            e.tactics = vec![m("nodes.encounter.tactics")];
            for r in &mut e.morale {
                r.when = m("nodes.encounter.morale.when");
                r.then = m("nodes.encounter.morale.then");
            }
            e.on_victory = m("nodes.encounter.on_victory");
            e.on_defeat = m("nodes.encounter.on_defeat");
        }
        for l in &mut n.loot {
            l.found = m("nodes.loot.found");
        }
        for x in &mut n.xp {
            x.reason = m("nodes.xp.reason");
        }
        n.transition = m("nodes.transition");
        for x in &mut n.exits {
            x.label = m("nodes.exits.label");
        }
        for h in &mut n.player_hooks {
            h.reason = m("nodes.player_hooks.reason");
        }
        n.if_skipped = m("nodes.if_skipped");
        n.art = m("nodes.art");
        n.gm_notes = m("nodes.gm_notes");
        if n.id != "sc_crique" {
            n.title = m("nodes.title (other scene)");
            n.read_aloud = m("nodes.read_aloud (other scene)");
        }
    }
    for n in &mut c.npcs {
        n.portrait = m("npcs.portrait");
        n.roleplay = m("npcs.roleplay");
        n.traits = vec![m("npcs.traits")];
        n.flaw = m("npcs.flaw");
        n.motivation = m("npcs.motivation");
        n.wants = m("npcs.wants");
        n.hides = m("npcs.hides");
        n.age = m("npcs.age");
        n.gm_notes = m("npcs.gm_notes");
        if let Some(s) = &mut n.stats {
            s.hit_points = Some(SECRET_HP);
            for a in &mut s.attacks {
                a.name = m("npcs.stats.attacks");
            }
        }
        if n.id == "pnj_corentin" {
            // Not met: even his name is secret.
            n.name = m("npcs.name (not met)");
            n.title = m("npcs.title (not met)");
            n.appearance = m("npcs.appearance (not met)");
        }
    }
    for a in &mut c.adversaries {
        a.name = m("adversaries.name (not revealed)");
        a.description = m("adversaries.description");
        a.art = m("adversaries.art");
        a.gm_notes = m("adversaries.gm_notes");
        a.stats.hit_points = Some(SECRET_HP);
        for at in &mut a.stats.attacks {
            at.name = m("adversaries.stats.attacks");
        }
    }
    for l in &mut c.locations {
        l.art = m("locations.art");
        l.gm_notes = m("locations.gm_notes");
    }
    for i in &mut c.items {
        i.name = m("items.name");
        i.description = m("items.description");
        i.effect = m("items.effect");
        i.art = m("items.art");
        i.gm_notes = m("items.gm_notes");
    }
    for f in &mut c.factions {
        f.description = m("factions.description");
        f.diplomacy = m("factions.diplomacy");
        f.gm_notes = m("factions.gm_notes");
    }
    for g in &mut c.goals {
        g.description = m("goals.description");
        g.gm_notes = m("goals.gm_notes");
    }
    c
}

/// In the crique, Gwen's and the map's clues found, Loïc met.
fn world(c: &Campaign) -> WorldState {
    let mut w = WorldState::default();
    w.enter_node(c, "sc_taverne").unwrap();
    w.enter_node(c, "sc_crique").unwrap();
    w.reveal_clue(c, "cl_gwen").unwrap();
    w.reveal_clue(c, "cl_carte").unwrap();
    w.reveal_entity(c, "pnj_loic").unwrap();
    w
}

#[test]
fn no_gm_only_field_reaches_players() {
    let c = marked();
    let view = project_for_players(&c, &world(&c));
    let json = serde_json::to_string(&view).unwrap();
    let leaks: Vec<&str> = json
        .match_indices("GMONLY<")
        .map(|(i, _)| &json[i..json.len().min(i + 60)])
        .collect();
    assert!(leaks.is_empty(), "GM-only fields leaked: {leaks:#?}");
    assert!(!json.contains(&SECRET_HP.to_string()), "hit points leaked");
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
