//! maps/travel-hex-world on the two witness worlds: the Corsaires sail
//! from Port-Louis to Belle-Île, the Brasier crosses its star system to
//! the Sereth station. Each test pins a rule a plausible bug would break.

use std::collections::BTreeSet;
use std::path::Path;

use promptus_shared::maps::{Cell, Geometry, Map, Scale, neighbours};
use promptus_shared::rules::dice::SeededDice;
use promptus_shared::travel::{
    Advanced, Guide, Party, Route, Way, draw_events, portions_for, routes,
};

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../content")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn map(rel: &str) -> Map {
    Map::from_yaml(&read(&format!("maps/{rel}"))).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn guide(rel: &str) -> Guide {
    Guide::from_yaml(&read(&format!("travel/{rel}"))).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

struct World {
    map: Map,
    guide: Guide,
    /// The place the demo journey goes to.
    to: Cell,
    /// Maps of the same world an exit may open.
    places: Vec<Map>,
}

fn corsaires() -> World {
    World {
        map: map("corsaires/cotes-bretagne-sud.yaml"),
        guide: guide("corsaires/cotes-bretagne-sud.yaml"),
        to: Cell::new(9, 9),
        places: vec![
            map("corsaires/le-palais.yaml"),
            map("corsaires/quai-port-louis.yaml"),
        ],
    }
}

fn brasier() -> World {
    World {
        map: map("brasier/systeme-brasier.yaml"),
        guide: guide("brasier/systeme-brasier.yaml"),
        to: Cell::new(11, 7),
        places: vec![
            map("brasier/reliquaire-sereth.yaml"),
            map("brasier/cure-dent-coursive.yaml"),
        ],
    }
}

fn worlds() -> [World; 2] {
    [corsaires(), brasier()]
}

fn start(w: &World) -> Party {
    Party::start(&w.map, 9).expect("a party start")
}

fn touching(a: Cell, b: Cell) -> bool {
    neighbours(Geometry::Hex, a).contains(&b)
}

#[test]
fn both_world_maps_have_a_sound_guide_and_open_real_maps() {
    for w in worlds() {
        assert_eq!(w.map.scale, Scale::World, "{}", w.map.id);
        assert_eq!(w.guide.issues(&w.map), [], "{}", w.map.id);
        for e in &w.map.exits {
            assert!(
                w.places.iter().any(|p| p.id == e.to),
                "{}: exit {} opens {} which is not a map of the world",
                w.map.id,
                e.id,
                e.to
            );
        }
        // The destination is a place: a label, and an exit to its map.
        assert!(w.map.labels.iter().any(|l| l.at == w.to), "{}", w.map.id);
        assert!(w.map.exits.iter().any(|e| e.cells.contains(&w.to)));
    }
}

#[test]
fn two_ways_lead_to_the_place_and_neither_crosses_what_cannot_be_crossed() {
    for w in worlds() {
        let from = start(&w).at;
        let found: Vec<Route> = routes(&w.map, &w.guide, from, w.to);
        assert_eq!(found.len(), 2, "{}: {found:?}", w.map.id);
        for r in &found {
            assert_eq!(r.hexes.last(), Some(&w.to));
            assert!(touching(from, r.hexes[0]), "{}: first step", w.map.id);
            for pair in r.hexes.windows(2) {
                assert!(touching(pair[0], pair[1]), "{}: {pair:?}", w.map.id);
            }
            let costs: Vec<u32> = r
                .hexes
                .iter()
                .map(|c| w.guide.cost(&w.map, *c).expect("never an impassable hex"))
                .collect();
            assert_eq!(r.steps, costs.iter().sum::<u32>());
            assert_eq!(r.portions, portions_for(&costs, w.guide.steps_per_portion));
            assert!(r.days >= 1);
        }
        // The first is the cheapest; the second is another road.
        assert!(found[0].steps <= found[1].steps, "{}", w.map.id);
        let first: BTreeSet<Cell> = found[0].hexes[..found[0].hexes.len() - 1]
            .iter()
            .copied()
            .collect();
        let second = &found[1].hexes[..found[1].hexes.len() - 1];
        let shared = second.iter().filter(|c| first.contains(c)).count();
        assert!(
            shared * 2 <= second.len(),
            "{}: {shared} of {}",
            w.map.id,
            second.len()
        );
    }
}

#[test]
fn the_brasier_goes_around_its_star_one_way_or_the_other() {
    let w = brasier();
    let found = routes(&w.map, &w.guide, start(&w).at, w.to);
    let star: Vec<Cell> = w
        .map
        .grid
        .cells()
        .filter(|c| w.map.grid.kind(*c).is_some_and(|k| k.terrain == "étoile"))
        .collect();
    let north = |r: &Route| r.hexes.iter().filter(|c| c.y < 5).count() > r.hexes.len() / 2;
    // One road passes north of the star, the other south of it.
    assert_ne!(north(&found[0]), north(&found[1]));
    for r in &found {
        assert!(r.hexes.iter().all(|c| !star.contains(c)));
    }
}

#[test]
fn a_place_out_of_reach_or_underfoot_has_no_route() {
    let w = corsaires();
    let from = start(&w).at;
    // Inland Brittany.
    assert_eq!(routes(&w.map, &w.guide, from, Cell::new(10, 0)), []);
    assert_eq!(routes(&w.map, &w.guide, from, from), []);
    assert_eq!(routes(&w.map, &w.guide, from, Cell::new(40, 40)), []);
}

#[test]
fn a_leftover_carries_so_a_dear_hex_takes_several_portions() {
    assert_eq!(portions_for(&[1, 1, 1, 1, 1], 2), 3);
    assert_eq!(portions_for(&[3], 2), 2);
    assert_eq!(portions_for(&[3, 1], 2), 2);
    assert_eq!(portions_for(&[], 2), 0);
}

/// Follow `route` portion by portion until arrival; the advances made.
fn follow(w: &World, party: &mut Party, route: &Route, size: u32) -> Vec<Advanced> {
    party.way = Some(Way {
        hexes: route.hexes.clone(),
        step: 0,
        bank: 0,
    });
    let mut out = Vec::new();
    for _ in 0..100 {
        let a = party.advance(&w.guide, &w.map, false, size);
        let done = matches!(a, Advanced::Portion { arrived: true, .. });
        out.push(a);
        if done {
            return out;
        }
    }
    panic!("{}: never arrived", w.map.id);
}

#[test]
fn the_party_arrives_after_the_portions_its_route_announced() {
    for w in worlds() {
        let mut party = start(&w);
        let route = routes(&w.map, &w.guide, party.at, w.to).remove(0);
        let advances = follow(&w, &mut party, &route, 4);
        let portions = advances
            .iter()
            .filter(|a| matches!(a, Advanced::Portion { .. }))
            .count();
        assert_eq!(portions as u32, route.portions, "{}", w.map.id);
        assert_eq!(party.at, w.to);
        // Every hex entered and its neighbours are seen.
        for c in &route.hexes {
            assert!(party.revealed.contains(c));
            for n in neighbours(Geometry::Hex, *c) {
                if w.map.grid.contains(n) {
                    assert!(party.revealed.contains(&n), "{}: {n:?}", w.map.id);
                }
            }
        }
        // Hexes far from the road stay unseen.
        assert!(party.revealed.len() < w.map.grid.len(), "{}", w.map.id);
        // The hexes entered are the route, in order, and nothing else.
        let entered: Vec<Cell> = advances
            .iter()
            .filter_map(|a| match a {
                Advanced::Portion { entered, .. } => Some(entered.clone()),
                Advanced::Dawn { .. } => None,
            })
            .flatten()
            .collect();
        assert_eq!(entered, route.hexes);
    }
}

#[test]
fn night_falls_after_the_last_portion_and_dawn_eats_the_supplies() {
    let w = corsaires();
    let mut party = start(&w);
    for (i, name) in ["Matin", "Après-midi", "Soir"].iter().enumerate() {
        let a = party.advance(&w.guide, &w.map, true, 6);
        assert_eq!(
            a,
            Advanced::Portion {
                day: 1,
                name: (*name).into(),
                entered: vec![],
                arrived: false,
                night: i == 2,
            }
        );
    }
    assert!(party.clock.night);
    assert_eq!(party.supplies, 9);
    let dawn = party.advance(&w.guide, &w.map, true, 6);
    assert_eq!(
        dawn,
        Advanced::Dawn {
            day: 2,
            eaten: 6,
            hungry: false
        }
    );
    assert_eq!(party.supplies, 3);
    for _ in 0..3 {
        party.advance(&w.guide, &w.map, true, 6);
    }
    let dawn = party.advance(&w.guide, &w.map, true, 6);
    assert_eq!(
        dawn,
        Advanced::Dawn {
            day: 3,
            eaten: 6,
            hungry: true
        }
    );
}

#[test]
fn holding_a_portion_spends_the_time_without_moving() {
    let w = brasier();
    let mut party = start(&w);
    let route = routes(&w.map, &w.guide, party.at, w.to).remove(0);
    party.way = Some(Way {
        hexes: route.hexes.clone(),
        step: 0,
        bank: 0,
    });
    let before = party.at;
    let a = party.advance(&w.guide, &w.map, true, 3);
    assert!(matches!(a, Advanced::Portion { ref entered, .. } if entered.is_empty()));
    assert_eq!(party.at, before);
    assert_eq!(party.clock.portion, 1);
    assert_eq!(party.way.as_ref().map(|w| (w.step, w.bank)), Some((0, 0)));
}

#[test]
fn events_are_drawn_from_the_terrain_table_without_repeat() {
    let w = corsaires();
    let sea = Cell::new(5, 5);
    let mut dice = SeededDice::new(7);
    let drawn = draw_events(&w.guide, &w.map, sea, &[], &mut dice);
    assert_eq!(drawn.len(), 3);
    let ids: BTreeSet<&str> = drawn.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids.len(), 3);
    let table: Vec<&str> = w.guide.terrains["mer"]
        .events
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    assert!(ids.iter().all(|id| table.contains(id)), "{ids:?}");
    // An event already kept on this journey is not drawn again while
    // others remain.
    let spent = vec!["voile-anglaise".to_string(), "grain".to_string()];
    for seed in 0..20 {
        let drawn = draw_events(&w.guide, &w.map, sea, &spent, &mut SeededDice::new(seed));
        assert!(drawn.iter().all(|e| !spent.contains(&e.id)), "{seed}");
        assert_eq!(drawn.len(), 2);
    }
    // A port has no table: nothing to propose.
    assert_eq!(
        draw_events(&w.guide, &w.map, Cell::new(9, 9), &[], &mut dice).len(),
        0
    );
}

#[test]
fn a_world_map_without_a_guide_travels_by_its_cell_flags() {
    let yaml = r##"
version: 1
id: vallee
name: La vallée
scale: world
theme: x
grid:
  legend:
    ".": { terrain: plaine }
    "f": { terrain: forêt, difficult: true }
    "#": { terrain: montagne, wall: true }
  rows:
    - "..f#"
starts:
  - { id: groupe, side: party, at: [0, 0] }
"##;
    let m = Map::from_yaml(yaml).unwrap();
    let g = Guide::default_for(&m);
    assert_eq!(g.issues(&m), []);
    assert_eq!(g.cost(&m, Cell::new(1, 0)), Some(1));
    assert_eq!(g.cost(&m, Cell::new(2, 0)), Some(2));
    assert_eq!(g.cost(&m, Cell::new(3, 0)), None);
    assert_eq!(g.portions.len(), 3);
}

#[test]
fn a_guide_naming_a_terrain_the_map_lacks_is_flagged() {
    let w = corsaires();
    let mut g = w.guide.clone();
    g.terrains.insert("marais".into(), Default::default());
    g.portions.clear();
    let issues: Vec<String> = g.issues(&w.map).iter().map(ToString::to_string).collect();
    assert!(issues.iter().any(|i| i.contains("marais")), "{issues:?}");
    assert!(issues.iter().any(|i| i.contains("portion")), "{issues:?}");
}
