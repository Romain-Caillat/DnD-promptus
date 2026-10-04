//! campaign/rewrite-two-worlds — import and export. Ports the intent of
//! V1's `import-export.test.ts` (entity YAML accepted and refused, the
//! multi-document stream, the round trip) onto the new format, through
//! the V1 importer; and checks that both witness worlds survive export →
//! import, so Romain can edit them as text.

use std::path::Path;

use promptus_shared::maps::load_v1_story_maps;
use promptus_shared::story::v1::{V1EntityType, import_v1, parse_v1_entities, slug};
use promptus_shared::story::{
    Library, MusicMood, RuleSystemRef, Severity, from_yaml, to_yaml, validate_with,
};

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../content")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn dnd5e() -> RuleSystemRef {
    RuleSystemRef {
        id: "dnd5e".into(),
        version: 1,
    }
}

// --- V1 entities ----------------------------------------------------------

#[test]
fn a_single_v1_entity_document_parses() {
    let yaml = r#"
type: spell
name: Magic Missile
description: Three darts of force.
tags: [evocation, force]
attributes:
  level: 1
  school: evocation
effects:
  - type: damage
    amount: 3d4+3
    damageType: force
    target:
      type: single
      entityId: "ent_x"
visibility: public
"#;
    let e = parse_v1_entities(yaml).unwrap();
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].name, "Magic Missile");
    assert_eq!(e[0].kind, V1EntityType::Spell);
    assert_eq!(e[0].effects.len(), 1);
}

#[test]
fn an_unknown_effect_type_is_refused_even_nested() {
    let top = "type: spell\nname: Broken\neffects:\n  - type: not_a_real_effect\n    amount: 1d6\n";
    let err = parse_v1_entities(top).unwrap_err().to_string();
    assert!(err.contains("not_a_real_effect"), "{err}");

    let nested = r#"
type: spell
name: Boule de feu
effects:
  - type: roll_check
    stat: DEX
    dc: 15
    outcomeFail:
      - type: dommages
"#;
    let err = parse_v1_entities(nested).unwrap_err().to_string();
    assert!(
        err.contains("outcomeFail[0]") && err.contains("dommages"),
        "{err}"
    );

    let bad_type = "type: dragon\nname: Smaug\n";
    assert!(parse_v1_entities(bad_type).is_err());
}

#[test]
fn a_multi_document_stream_and_the_export_envelope_both_parse() {
    let stream = r"
type: item
name: Sword
attributes: { rarity: common }
---
type: monster
name: Goblin
attributes: { hp: 7, hpMax: 7, ac: 15 }
visibility: mj_only
";
    let e = parse_v1_entities(stream).unwrap();
    let names: Vec<&str> = e.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["Sword", "Goblin"]);

    let export = read("fixtures/v1-demo/entities.yaml");
    let all = parse_v1_entities(&export).unwrap();
    assert_eq!(all.len(), 48);
    assert!(parse_v1_entities("---\n").is_err(), "no entity at all");
}

// --- V1 campaign → new format ---------------------------------------------

#[test]
fn the_v1_demo_imports_into_one_campaign_that_round_trips() {
    let imported = import_v1(
        &read("fixtures/v1-demo/entities.yaml"),
        &read("fixtures/v1-demo/story.json"),
        dnd5e(),
    )
    .unwrap();
    let c = &imported.campaign;
    assert_eq!(c.title, "Le Donjon des gobelins (démo)");
    assert_eq!(c.id, "le-donjon-des-gobelins-demo");
    assert_eq!(
        (c.party.len(), c.npcs.len(), c.adversaries.len()),
        (4, 4, 10)
    );
    assert_eq!((c.items.len(), c.locations.len()), (10, 5));
    assert_eq!(c.nodes.len(), 7);
    assert_eq!(c.bible.start_node.as_deref(), Some("sc_auberge"));

    // Faction names became factions the NPCs point at.
    let mira = c.npc("ent_soeur_mira").unwrap();
    assert_eq!(mira.faction.as_deref(), Some("fac_ordre-du-soleil-levant"));
    assert!(mira.hides.contains("revenant"));
    // A monster keeps its stat block; an attack its damage formula.
    let roi = c.adversary("ent_revenant_mort_roi").unwrap();
    assert_eq!(
        (roi.stats.hit_points, roi.stats.armor_class),
        (Some(65), Some(17))
    );
    assert_eq!(roi.stats.attacks[0].damage, "1d8+4");
    // The camp: its monsters an encounter, its battle map the node's
    // map, a music query a track still to choose, the objective a key point.
    let camp = c.node("sc_camp").unwrap();
    assert_eq!(camp.encounter.as_ref().unwrap().opponents.len(), 3);
    assert_eq!(camp.map.as_deref(), Some("map_camp"));
    let track = &camp.ambience.music[0];
    assert_eq!(
        (track.mood, track.url.as_str(), track.search.as_str()),
        (MusicMood::Combat, "", "goblin battle music")
    );
    assert!(camp.key_points[0].starts_with("Objectif : "));
    // Clue checks keep their difficulty.
    assert_eq!(
        c.clue("cl_tomas")
            .unwrap()
            .check
            .as_ref()
            .unwrap()
            .difficulty,
        13
    );

    // Nothing silently lost: spells and triggers are listed.
    assert!(
        imported
            .dropped
            .iter()
            .any(|d| d.contains("Projectile magique"))
    );
    assert!(
        imported
            .dropped
            .iter()
            .any(|d| d.starts_with("sc_village: 1 trigger"))
    );
    assert!(
        imported
            .dropped
            .iter()
            .any(|d| d.starts_with("fr_gobelins: 1 step effect"))
    );

    // Export → import gives the same campaign, and the same text again.
    let text = to_yaml(c).unwrap();
    let back = from_yaml(&text).unwrap();
    assert_eq!(&back, c);
    assert_eq!(to_yaml(&back).unwrap(), text);

    // It points at real things: no error, battle maps and their tokens
    // included (the V1 demo maps are converted separately).
    let maps = load_v1_story_maps(&read("maps/v1-demo/demo-story-maps.json")).unwrap();
    let issues = validate_with(
        c,
        &Library {
            rules: None,
            maps: Some(&maps),
        },
    );
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn a_story_that_is_not_v1_json_is_refused() {
    let entities = read("fixtures/v1-demo/entities.yaml");
    assert!(import_v1(&entities, "{\"story\": 3}", dnd5e()).is_err());
    assert!(import_v1(&entities, "not json", dnd5e()).is_err());
}

#[test]
fn slugs_fold_french_text() {
    assert_eq!(
        slug("Le Donjon des gobelins (démo)"),
        "le-donjon-des-gobelins-demo"
    );
    assert_eq!(slug("Sœur Mira"), "soeur-mira");
    assert_eq!(slug("Conseil de Creux-d’Étain"), "conseil-de-creux-d-etain");
}

// --- The two witness worlds as text ---------------------------------------

#[test]
fn both_worlds_survive_export_then_import() {
    for rel in [
        "campaigns/corsaires/campagne.yaml",
        "campaigns/brasier/campagne.yaml",
    ] {
        let c = from_yaml(&read(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let text = to_yaml(&c).unwrap();
        let back = from_yaml(&text).unwrap();
        assert_eq!(back, c, "{rel}");
        assert_eq!(to_yaml(&back).unwrap(), text, "{rel}: the text drifts");
    }
}
