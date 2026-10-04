//! Loading the map fixtures and the V1 demo maps, the file format's
//! round trip and checks, and the players' projection.

use std::collections::HashSet;
use std::path::Path;

use promptus_shared::maps::{
    Cell, Diagonal, DoorState, Geometry, Issue, Map, MapError, MovementRules, Occupancy, Scale,
    Side, Viewer, Visibility, Water, load_v1_story_maps, reachable,
};

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../content/maps")
        .join(rel);
    std::fs::read_to_string(path).expect("fixture readable")
}

fn load(rel: &str) -> Map {
    Map::from_yaml(&read(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn dock() -> Map {
    load("corsaires/quai-port-louis.yaml")
}

fn corridor() -> Map {
    load("brasier/cure-dent-coursive.yaml")
}

fn v1_maps() -> Vec<Map> {
    load_v1_story_maps(&read("v1-demo/demo-story-maps.json")).expect("V1 maps load")
}

fn c(x: i32, y: i32) -> Cell {
    Cell::new(x, y)
}

fn issues(map: &Map) -> Vec<Issue> {
    match map.validate() {
        Err(MapError::Invalid(issues)) => issues,
        other => panic!("expected issues, got {other:?}"),
    }
}

#[test]
fn both_witness_maps_load_with_six_against_six() {
    for map in [dock(), corridor()] {
        assert_eq!(map.scale, Scale::Encounter);
        assert_eq!(map.geometry(), Geometry::Square);
        assert!((map.cell_meters() - 1.5).abs() < 1e-9);
        let side = |s| map.starts.iter().filter(|st| st.side == Some(s)).count();
        assert_eq!(side(Side::Party), 6, "{}", map.id);
        assert_eq!(side(Side::Foes), 6, "{}", map.id);
    }
    let dock = dock();
    assert_eq!((dock.grid.width(), dock.grid.height()), (24, 16));
    assert_eq!(dock.grid.kind(c(5, 12)).unwrap().water, Water::Deep);
    assert_eq!(dock.grid.kind(c(15, 12)).unwrap().elevation, 1);
    assert_eq!(dock.door("porte-kerjean").unwrap().state, DoorState::Locked);
    let corridor = corridor();
    assert_eq!((corridor.grid.width(), corridor.grid.height()), (15, 20));
}

#[test]
fn a_map_survives_yaml_and_json_round_trips() {
    for map in [dock(), corridor()] {
        let yaml = map.to_yaml().unwrap();
        assert_eq!(Map::from_yaml(&yaml).unwrap(), map);
        let json = map.to_json().unwrap();
        assert_eq!(Map::from_json(&json).unwrap(), map);
        assert_eq!(Map::from_json(&json).unwrap().grid.rows(), map.grid.rows());
    }
}

#[test]
fn the_grid_rejects_ragged_rows_and_unknown_glyphs() {
    let base = read("corsaires/quai-port-louis.yaml");
    let ragged = base.replacen(
        "    - \"........................\"\n",
        "    - \".......................\"\n",
        1,
    );
    assert!(matches!(Map::from_yaml(&ragged), Err(MapError::Yaml(_))));
    let unknown = base.replacen(
        "    - \"........................\"\n",
        "    - \".......X................\"\n",
        1,
    );
    let err = Map::from_yaml(&unknown).unwrap_err().to_string();
    assert!(err.contains("'X'"), "{err}");
}

#[test]
fn validation_catches_what_types_cannot() {
    let mut map = dock();
    map.version = 2;
    map.doors[0].at = c(3, 4); // on the quay, not a wall
    map.props[0].layer = "inconnu".into();
    map.props[1].at = c(30, 2);
    map.starts[0].at = c(4, 12); // in the water
    map.lights[0].dim = 0;
    map.objects[0].id = "caisse-1".into(); // already a prop
    let found = issues(&map);
    assert!(found.contains(&Issue::UnsupportedVersion(2)));
    assert!(found.contains(&Issue::DoorNotOnWall {
        door: "porte-kerjean".into(),
        cell: c(3, 4)
    }));
    assert!(found.contains(&Issue::UnknownLayer {
        item: "caisses-pile-1".into(),
        layer: "inconnu".into()
    }));
    assert!(found.contains(&Issue::OutOfBounds {
        item: "caisse-1".into(),
        cell: c(30, 2)
    }));
    assert!(found.contains(&Issue::StartNotWalkable {
        start: "pj-1".into(),
        cell: c(4, 12)
    }));
    assert!(found.contains(&Issue::LightRadius("lanterne-kerjean".into())));
    assert!(found.contains(&Issue::DuplicateId("caisse-1".into())));
    assert_eq!(found.len(), 7, "{found:?}");
}

#[test]
fn players_never_receive_the_dock_ambush_or_its_hidden_trapdoor() {
    let map = dock();
    let gm = map.project(Viewer::Gm);
    assert!(gm.object("trappe-contrebande").is_some());
    assert_eq!(gm.starts.len(), 12);

    let players = map.project(Viewer::Player);
    assert!(players.object("trappe-contrebande").is_none());
    assert!(
        players
            .layers
            .iter()
            .all(|l| l.visibility != Visibility::Gm)
    );
    assert_eq!(players.starts.len(), 6);
    assert!(players.starts.iter().all(|s| s.side == Some(Side::Party)));
    assert!(players.gm_notes.is_none());
    players.validate().expect("the projection is a valid map");
    let json = players.to_json().unwrap();
    for secret in [
        "trappe-contrebande",
        "Cachette",
        "Gueule-Rouge",
        "gueule-rouge",
        "SAG",
    ] {
        assert!(!json.contains(secret), "{secret} leaked to players");
    }
}

#[test]
fn players_see_the_secret_hatch_as_a_plain_bulkhead() {
    let map = corridor();
    let players = map.project(Viewer::Player);
    assert!(players.door("trappe-maintenance").is_none());
    assert!(
        players.grid.kind(c(5, 13)).unwrap().wall,
        "no gap where the hatch was"
    );
    assert!(players.object("balise-vorr").is_none());
    assert!(players.labels.iter().all(|l| l.text != "Gaine technique"));
    let json = players.to_json().unwrap();
    for secret in ["balise", "Vorr", "vorr", "trappe-maintenance", "Gaine"] {
        assert!(!json.contains(secret), "{secret} leaked to players");
    }
    // Revealing the secrets layer hands them over.
    let mut revealed = map.clone();
    revealed
        .layers
        .iter_mut()
        .find(|l| l.id == "secrets")
        .unwrap()
        .visibility = Visibility::All;
    let players = revealed.project(Viewer::Player);
    assert!(players.door("trappe-maintenance").is_some());
    assert!(players.object("balise-vorr").is_some());
    assert!(
        players.object("balise-vorr").unwrap().check.is_none(),
        "the DC stays with the GM"
    );
}

#[test]
fn a_players_only_illusion_is_ignored_by_the_rules() {
    let mut map = corridor();
    map.layers.push(promptus_shared::maps::Layer {
        id: "illusion".into(),
        name: "Mur illusoire".into(),
        visibility: Visibility::Players,
    });
    let mut wall = map.prop("console-integrite").unwrap().clone();
    wall.id = "faux-mur".into();
    wall.at = c(7, 7);
    wall.layer = "illusion".into();
    wall.cover = promptus_shared::maps::Cover::Total;
    map.props.push(wall);
    let r = reachable(
        &map,
        &MovementRules::default(),
        &Occupancy::default(),
        c(7, 6),
        1,
    );
    assert!(r.contains_key(&c(7, 7)), "the illusory wall does not block");
    assert!(map.project(Viewer::Player).prop("faux-mur").is_some());
    let none = HashSet::new();
    assert!(promptus_shared::maps::line_of_sight(&map, c(7, 4), c(7, 9), &none).visible);
}

#[test]
fn the_v1_demo_maps_load_into_the_new_model() {
    let maps = v1_maps();
    let ids: Vec<&str> = maps.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["map_monde", "map_vallee", "map_camp", "map_crypte"]);
    for map in &maps {
        let again = Map::from_yaml(&map.to_yaml().unwrap()).unwrap();
        assert_eq!(&again, map, "{} round trip", map.id);
    }

    let monde = &maps[0];
    assert_eq!(
        (monde.scale, monde.geometry()),
        (Scale::World, Geometry::Hex)
    );
    assert!(
        monde.grid.kind(c(5, 0)).unwrap().wall,
        "mountains along the north"
    );
    assert_eq!(monde.grid.kind(c(5, 5)).unwrap().terrain, "forêt");
    assert!(
        monde
            .exits
            .iter()
            .any(|e| e.to == "map_vallee" && e.cells == [c(3, 4)])
    );
    assert_eq!(monde.distance(c(3, 4), c(10, 6), Diagonal::Chebyshev), 8);

    let vallee = &maps[1];
    assert!((vallee.cell_meters() - 500.0).abs() < 1e-9);
    assert_eq!(vallee.grid.kind(c(4, 4)).unwrap().water, Water::Deep);
    let r = reachable(
        vallee,
        &MovementRules::default(),
        &Occupancy::default(),
        c(2, 4),
        20,
    );
    assert!(r.contains_key(&c(4, 5)), "the ford crosses the river");
    assert!(!r.contains_key(&c(4, 4)));
    assert!(r.contains_key(&c(7, 1)), "the camp lies beyond the river");

    let crypte = &maps[3];
    assert_eq!(
        (crypte.scale, crypte.grid.width(), crypte.grid.height()),
        (Scale::Encounter, 20, 14)
    );
    assert!(crypte.grid.kind(c(0, 0)).unwrap().wall);
    assert!(
        !crypte.grid.kind(c(11, 6)).unwrap().wall,
        "the vault's doorway"
    );
    let throne = crypte
        .props
        .iter()
        .find(|p| p.at == c(17, 6))
        .expect("throne prop");
    assert!(throne.blocks_movement);
    assert_eq!(throne.label.as_deref(), Some("Trône du Mort-Roi"));
    assert!(
        crypte
            .labels
            .iter()
            .any(|l| l.text == "Escalier" && l.at == c(5, 12))
    );
    assert_eq!(crypte.starts.len(), 3);
    // V1's move check: from the stairs, three steps reach (6,9).
    let r = reachable(
        crypte,
        &MovementRules::default(),
        &Occupancy::default(),
        c(5, 12),
        6,
    );
    assert_eq!(r.get(&c(6, 9)), Some(&3));
    assert!(!r.contains_key(&c(17, 6)));

    let camp = &maps[2];
    let fire = camp
        .props
        .iter()
        .find(|p| p.at == c(7, 5))
        .expect("campfire prop");
    assert_eq!(fire.kind, "feu");
    assert!(
        camp.backdrop
            .as_ref()
            .and_then(|b| b.prompt.as_ref())
            .is_some()
    );
}
