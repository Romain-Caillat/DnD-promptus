//! Movement, range and sight on the two witness maps and on small maps
//! written inline. Each test pins a rule a plausible bug would break.

use std::collections::HashSet;
use std::path::Path;

use promptus_shared::maps::{
    Cell, Cover, Diagonal, DoorState, LightLevel, Map, MovementRules, Obstacle, Occupancy,
    PathError, check_path, illumination, line_of_sight, reachable, shortest_path, visible_cells,
};

fn load(rel: &str) -> Map {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../content/maps")
        .join(rel);
    let text = std::fs::read_to_string(&path).expect("fixture readable");
    Map::from_yaml(&text).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn dock() -> Map {
    load("corsaires/quai-port-louis.yaml")
}

fn corridor() -> Map {
    load("brasier/cure-dent-coursive.yaml")
}

fn c(x: i32, y: i32) -> Cell {
    Cell::new(x, y)
}

fn cells(list: &[(i32, i32)]) -> Vec<Cell> {
    list.iter().map(|&(x, y)| c(x, y)).collect()
}

/// A small square map: `.` floor, `#` wall, `~` deep water, `^` floor
/// two levels up, `,` difficult. `extra` is appended YAML (props…).
fn tiny(rows: &[&str], extra: &str) -> Map {
    let rows: String = rows.iter().map(|r| format!("    - \"{r}\"\n")).collect();
    let yaml = format!(
        "version: 1\nid: t\nname: Test\nscale: encounter\ntheme: test\ngrid:\n  legend:\n    \".\": {{ terrain: sol }}\n    \"#\": {{ terrain: mur, wall: true }}\n    \"~\": {{ terrain: eau, water: deep }}\n    \"^\": {{ terrain: rocher, elevation: 2 }}\n    \",\": {{ terrain: boue, difficult: true }}\n  rows:\n{rows}{extra}"
    );
    Map::from_yaml(&yaml).unwrap_or_else(|e| panic!("{e}\n{yaml}"))
}

fn tiny_hex(cols: usize, rows: usize) -> Map {
    let row = ".".repeat(cols);
    let rows: String = (0..rows).map(|_| format!("    - \"{row}\"\n")).collect();
    let yaml = format!(
        "version: 1\nid: h\nname: Hex\nscale: world\ntheme: test\ngrid:\n  legend:\n    \".\": {{ terrain: plaine }}\n  rows:\n{rows}"
    );
    Map::from_yaml(&yaml).unwrap()
}

fn rules() -> MovementRules {
    MovementRules::default()
}

fn nobody() -> Occupancy {
    Occupancy::default()
}

// ---------------------------------------------------------------------------
// Movement — ported V1 intents
// ---------------------------------------------------------------------------

#[test]
fn walks_around_a_wall_without_cutting_its_corner() {
    // Wall at x = 2 from y = 0 to 3; the way round is through (2,4).
    let map = tiny(
        &["..#...", "..#...", "..#...", "..#...", "......", "......"],
        "",
    );
    let r = reachable(&map, &rules(), &nobody(), c(1, 0), 6);
    assert!(!r.contains_key(&c(2, 1)));
    // (1,3) → (2,4) would graze the wall at (2,3): one more step.
    assert_eq!(r.get(&c(2, 4)), Some(&5));
    assert_eq!(r.get(&c(3, 4)), Some(&6));
    assert!(!r.contains_key(&c(3, 3)));
    assert!(!r.contains_key(&c(3, 0)));
}

#[test]
fn alternate_rule_makes_every_second_diagonal_cost_two() {
    let map = tiny(&[".........."; 10], "");
    let alternate = MovementRules {
        diagonal: Diagonal::Alternate,
        ..rules()
    };
    let r = reachable(&map, &alternate, &nobody(), c(0, 0), 3);
    assert_eq!(r.get(&c(1, 1)), Some(&1));
    assert_eq!(r.get(&c(2, 2)), Some(&3));
    assert!(!r.contains_key(&c(3, 3)));
    let chebyshev = reachable(&map, &rules(), &nobody(), c(0, 0), 3);
    assert_eq!(chebyshev.get(&c(3, 3)), Some(&3));
}

#[test]
fn enemies_block_and_allies_can_be_crossed_but_not_stopped_on() {
    let map = tiny(&["....."], "");
    let enemy = Occupancy {
        enemies: HashSet::from([c(1, 0)]),
        ..nobody()
    };
    let r = reachable(&map, &rules(), &enemy, c(0, 0), 5);
    assert!(!r.contains_key(&c(2, 0)));

    let ally = Occupancy {
        allies: HashSet::from([c(1, 0)]),
        ..nobody()
    };
    let r = reachable(&map, &rules(), &ally, c(0, 0), 5);
    assert!(!r.contains_key(&c(1, 0)), "cannot stop on an ally");
    assert_eq!(r.get(&c(2, 0)), Some(&2), "but can cross one");
}

#[test]
fn the_shortest_path_is_one_check_path_accepts_at_its_reach_cost() {
    let map = tiny(
        &["..#...", "..#...", "..#...", "..#...", "......", "......"],
        "",
    );
    for rules in [
        rules(),
        MovementRules {
            diagonal: Diagonal::Alternate,
            ..rules()
        },
    ] {
        let reach = reachable(&map, &rules, &nobody(), c(1, 0), 9);
        for (&goal, &cost) in &reach {
            let (path, found) =
                shortest_path(&map, &rules, &nobody(), c(1, 0), goal, 9).expect("reachable");
            assert_eq!(found, cost, "{goal:?}");
            assert_eq!(
                check_path(&map, &rules, &nobody(), c(1, 0), &path, cost),
                Ok(cost)
            );
        }
        assert_eq!(
            shortest_path(&map, &rules, &nobody(), c(1, 0), c(3, 0), 9),
            None
        );
    }
}

#[test]
fn hex_maps_count_one_per_step() {
    let map = tiny_hex(10, 8);
    let r = reachable(&map, &rules(), &nobody(), c(4, 4), 2);
    assert_eq!(r.get(&c(4, 4)), Some(&0));
    assert_eq!(r.get(&c(6, 4)), Some(&2));
    assert!(!r.contains_key(&c(7, 4)));
    // From an even row, the row above is reached at x and x - 1 only.
    assert_eq!(r.get(&c(4, 3)), Some(&1));
    assert_eq!(r.get(&c(3, 3)), Some(&1));
    assert_eq!(r.get(&c(5, 3)), Some(&2));
}

#[test]
fn height_needs_climbing_gear_beyond_one_step_and_costs_per_level() {
    let map = tiny(&[".^."], "");
    let err = check_path(&map, &rules(), &nobody(), c(0, 0), &cells(&[(1, 0)]), 10);
    assert_eq!(
        err,
        Err(PathError::TooSteep {
            step: 0,
            cell: c(1, 0)
        })
    );

    let ladder = tiny(
        &[".^."],
        "props:\n  - { id: echelle, kind: echelle, at: [1, 0], climbable: true }\n",
    );
    let cost = check_path(
        &ladder,
        &rules(),
        &nobody(),
        c(0, 0),
        &cells(&[(1, 0), (2, 0)]),
        10,
    );
    // Up two levels: 1 + 2 climb; down again: 1.
    assert_eq!(cost, Ok(4));
}

#[test]
fn deep_water_is_impassable_unless_the_rules_allow_swimming() {
    let map = tiny(&[".~."], "");
    let r = reachable(&map, &rules(), &nobody(), c(0, 0), 10);
    assert!(!r.contains_key(&c(1, 0)) && !r.contains_key(&c(2, 0)));
    let swim = MovementRules {
        swim_factor: Some(3),
        ..rules()
    };
    let r = reachable(&map, &swim, &nobody(), c(0, 0), 10);
    assert_eq!(r.get(&c(1, 0)), Some(&3));
    assert_eq!(r.get(&c(2, 0)), Some(&4));
}

// ---------------------------------------------------------------------------
// The dock of Port-Louis
// ---------------------------------------------------------------------------

#[test]
fn dock_reach_goes_round_the_crates_and_pays_for_puddles() {
    let map = dock();
    let r = reachable(&map, &rules(), &nobody(), c(9, 6), 2);
    for crate_cell in [c(7, 5), c(8, 5), c(7, 6), c(8, 6), c(11, 7)] {
        assert!(!r.contains_key(&crate_cell), "{crate_cell:?} is a crate");
    }
    // Diagonal past the crate stack's corner is refused: two straight steps.
    assert_eq!(r.get(&c(8, 7)), Some(&2));
    assert_eq!(r.get(&c(10, 6)), Some(&1));
    assert_eq!(r.get(&c(11, 5)), Some(&2));
    // (11,6) is a puddle: entering it costs 2, so 3 in all.
    assert!(!r.contains_key(&c(11, 6)));
    let r3 = reachable(&map, &rules(), &nobody(), c(9, 6), 3);
    assert_eq!(r3.get(&c(11, 6)), Some(&3));
}

#[test]
fn dock_reach_stops_at_the_water_and_climbs_the_gangway() {
    let map = dock();
    let r = reachable(&map, &rules(), &nobody(), c(10, 9), 3);
    assert_eq!(r.get(&c(10, 10)), Some(&1), "the pier is walkable");
    assert_eq!(r.get(&c(9, 10)), Some(&1));
    for cell in r.keys() {
        assert_ne!(
            map.grid.kind(*cell).unwrap().terrain,
            "eau",
            "{cell:?} is water"
        );
    }
    // The gangway is one level up: 2 steps + 1 step and 1 climb.
    assert!(!r.contains_key(&c(12, 12)));
    let r4 = reachable(&map, &rules(), &nobody(), c(10, 9), 4);
    assert_eq!(r4.get(&c(12, 12)), Some(&4));
    let free_climb = MovementRules {
        climb_cost: 0,
        ..rules()
    };
    let r = reachable(&map, &free_climb, &nobody(), c(10, 9), 4);
    assert_eq!(r.get(&c(12, 12)), Some(&3));

    // Swimming: the water by the pier, reached from the pier itself
    // (the mooring post forbids the diagonal from the quay).
    let swim = MovementRules {
        swim_factor: Some(2),
        ..rules()
    };
    let r = reachable(&map, &swim, &nobody(), c(10, 9), 3);
    assert_eq!(r.get(&c(8, 10)), Some(&3));
}

#[test]
fn dock_cover_from_a_crate_depends_on_who_hides_behind_it() {
    let map = dock();
    let none = HashSet::new();
    // A single crate between shooter and target: half cover.
    let s = line_of_sight(&map, c(2, 7), c(14, 7), &none);
    assert!(s.visible);
    assert_eq!(s.cover, Cover::Half);
    // The shooter stands right behind it and leans over: no cover.
    let s = line_of_sight(&map, c(10, 7), c(14, 7), &none);
    assert_eq!(s.cover, Cover::None);
    // Barrels give three-quarters.
    let s = line_of_sight(&map, c(15, 7), c(23, 7), &none);
    assert_eq!(s.cover, Cover::ThreeQuarters);
    // A creature in the way gives half cover.
    let s = line_of_sight(&map, c(2, 4), c(14, 4), &HashSet::from([c(9, 4)]));
    assert_eq!(s.cover, Cover::Half);
    // A stack of crates blocks sight entirely.
    let s = line_of_sight(&map, c(2, 5), c(12, 5), &none);
    assert!(!s.visible);
    assert_eq!(s.cover, Cover::Total);
}

#[test]
fn dock_walls_and_doors_block_sight_until_opened() {
    let mut map = dock();
    let none = HashSet::new();
    assert!(
        !line_of_sight(&map, c(12, 4), c(11, 1), &none).visible,
        "warehouse wall"
    );
    assert!(
        line_of_sight(&map, c(18, 6), c(18, 1), &none).visible,
        "open door"
    );
    assert!(
        !line_of_sight(&map, c(10, 6), c(10, 1), &none).visible,
        "closed door"
    );
    assert!(map.set_door_state("porte-douanes", DoorState::Open));
    assert!(line_of_sight(&map, c(10, 6), c(10, 1), &none).visible);
    assert!(map.set_door_state("porte-chantier", DoorState::Closed));
    assert!(!line_of_sight(&map, c(18, 6), c(18, 1), &none).visible);
}

#[test]
fn dock_hull_of_the_brig_hides_the_water_beyond() {
    let map = dock();
    let s = line_of_sight(&map, c(11, 12), c(23, 12), &HashSet::new());
    assert!(!s.visible, "the deck is higher than both ends");
    // From the pier, the gangway's foot is in plain view.
    assert!(line_of_sight(&map, c(10, 12), c(12, 12), &HashSet::new()).visible);
}

#[test]
fn dock_view_in_the_fog_stops_at_crates_walls_and_distance() {
    let map = dock();
    let limit = map.ambience.sight_limit;
    assert_eq!(limit, Some(10));
    let seen = visible_cells(&map, c(0, 6), limit);
    assert!(seen.contains(&c(0, 6)) && seen.contains(&c(5, 6)));
    assert!(seen.contains(&c(3, 3)), "the warehouse wall face is seen");
    assert!(!seen.contains(&c(9, 6)), "behind the crate stack");
    assert!(!seen.contains(&c(3, 1)), "inside the locked warehouse");
    assert!(!seen.contains(&c(11, 4)), "beyond the fog");
    assert!(seen.iter().all(|s| s.x <= 10));
}

#[test]
fn dock_lanterns_light_the_quay_but_not_through_walls() {
    let map = dock();
    assert_eq!(illumination(&map, c(3, 5)), LightLevel::Bright);
    assert_eq!(illumination(&map, c(3, 6)), LightLevel::Dim);
    assert_eq!(
        illumination(&map, c(3, 8)),
        LightLevel::Dark,
        "night beyond the lantern"
    );
    assert_eq!(
        illumination(&map, c(3, 2)),
        LightLevel::Dark,
        "behind the locked door"
    );
}

#[test]
fn dock_range_follows_the_scale() {
    let map = dock();
    assert_eq!(map.distance(c(0, 6), c(13, 6), Diagonal::Chebyshev), 13);
    assert!((map.range_meters(c(0, 6), c(13, 6), Diagonal::Chebyshev) - 19.5).abs() < 1e-9);
    assert_eq!(map.distance(c(0, 4), c(4, 8), Diagonal::Alternate), 6);
    assert!((map.range_meters(c(0, 4), c(4, 8), Diagonal::Alternate) - 9.0).abs() < 1e-9);
    // Speeds in feet, as V1 converted them: 30 ft is 6 cells, 25 ft is 5.
    assert_eq!(map.meters_to_cells(30.0 * 0.3048), 6);
    assert_eq!(map.meters_to_cells(25.0 * 0.3048), 5);
}

// ---------------------------------------------------------------------------
// The Cure-Dent corridor
// ---------------------------------------------------------------------------

#[test]
fn corridor_paths_through_closed_doors_are_refused_and_open_ones_accepted() {
    let mut map = corridor();
    let west = cells(&[(6, 10), (5, 10), (4, 10), (3, 10)]);
    let east = cells(&[(8, 10), (9, 10), (10, 10), (11, 10)]);
    assert_eq!(
        check_path(&map, &rules(), &nobody(), c(7, 10), &west, 6),
        Err(PathError::Blocked {
            step: 1,
            cell: c(5, 10),
            by: Obstacle::Door {
                id: "sas-babord-interieur".into(),
                state: DoorState::Closed
            },
        })
    );
    assert_eq!(
        check_path(&map, &rules(), &nobody(), c(7, 10), &east, 6),
        Ok(4)
    );
    map.set_door_state("sas-babord-interieur", DoorState::Open);
    assert_eq!(
        check_path(&map, &rules(), &nobody(), c(7, 10), &west, 6),
        Ok(4)
    );
    // The outer hatch is locked.
    let out = check_path(&map, &rules(), &nobody(), c(1, 10), &cells(&[(0, 10)]), 6);
    assert!(matches!(
        out,
        Err(PathError::Blocked {
            by: Obstacle::Door {
                state: DoorState::Locked,
                ..
            },
            ..
        })
    ));
}

#[test]
fn corridor_paths_obey_every_step_rule() {
    let map = corridor();
    let east = cells(&[(8, 10), (9, 10), (10, 10), (11, 10)]);
    let run = |occ: &Occupancy, path: &[Cell], budget| {
        check_path(&map, &rules(), occ, c(7, 10), path, budget)
    };
    assert_eq!(
        run(&nobody(), &east, 3),
        Err(PathError::OverBudget { cost: 4, budget: 3 })
    );
    assert_eq!(
        run(&nobody(), &cells(&[(9, 10)]), 6),
        Err(PathError::NotAdjacent {
            step: 0,
            cell: c(9, 10)
        })
    );
    // Into a doorway diagonally: squeezes past the wall.
    assert_eq!(
        check_path(&map, &rules(), &nobody(), c(8, 9), &cells(&[(9, 10)]), 6),
        Err(PathError::CornerCut {
            step: 0,
            cell: c(9, 10)
        })
    );
    let enemy = Occupancy {
        enemies: HashSet::from([c(10, 10)]),
        ..nobody()
    };
    assert_eq!(
        run(&enemy, &east, 6),
        Err(PathError::Occupied {
            step: 2,
            cell: c(10, 10)
        })
    );
    let ally = Occupancy {
        allies: HashSet::from([c(10, 10)]),
        ..nobody()
    };
    assert_eq!(run(&ally, &east, 6), Ok(4));
    assert_eq!(
        run(&ally, &east[..3], 6),
        Err(PathError::EndsOnOccupied(c(10, 10)))
    );
    // Blocked by the console.
    let into_console = check_path(&map, &rules(), &nobody(), c(11, 6), &cells(&[(12, 6)]), 6);
    assert_eq!(
        into_console,
        Err(PathError::Blocked {
            step: 0,
            cell: c(12, 6),
            by: Obstacle::Prop("console-integrite".into())
        })
    );
}

#[test]
fn corridor_reach_enters_the_open_airlock_only() {
    let mut map = corridor();
    let r = reachable(&map, &rules(), &nobody(), c(7, 4), 8);
    assert_eq!(r.get(&c(9, 10)), Some(&7));
    assert_eq!(r.get(&c(10, 10)), Some(&8));
    assert!(!r.contains_key(&c(4, 10)), "west airlock door is closed");
    map.set_door_state("sas-babord-interieur", DoorState::Open);
    let r = reachable(&map, &rules(), &nobody(), c(7, 4), 8);
    assert_eq!(r.get(&c(4, 10)), Some(&8));
}

#[test]
fn corridor_secret_hatch_is_real_for_the_rules() {
    let mut map = corridor();
    let into_duct = cells(&[(6, 13), (5, 13), (4, 13)]);
    assert!(matches!(
        check_path(&map, &rules(), &nobody(), c(7, 13), &into_duct, 10),
        Err(PathError::Blocked { step: 1, .. })
    ));
    map.set_door_state("trappe-maintenance", DoorState::Open);
    // The duct's grating is difficult terrain: 1 + 1 + 2.
    assert_eq!(
        check_path(&map, &rules(), &nobody(), c(7, 13), &into_duct, 10),
        Ok(4)
    );
}

#[test]
fn corridor_sight_stops_at_bulkheads_and_passes_open_doors() {
    let map = corridor();
    let none = HashSet::new();
    assert!(
        !line_of_sight(&map, c(7, 7), c(12, 7), &none).visible,
        "bulkhead"
    );
    let s = line_of_sight(&map, c(7, 6), c(11, 6), &none);
    assert!(s.visible);
    assert_eq!(s.cover, Cover::None);
    let s = line_of_sight(&map, c(7, 6), c(13, 6), &none);
    assert_eq!(s.cover, Cover::Half, "behind the integrity console");
    assert!(
        !line_of_sight(&map, c(4, 17), c(10, 17), &none).visible,
        "reactor"
    );
    // Seen past the corner where the corridor opens on the engine room:
    // the line grazes the bulkhead's corner, half cover.
    let s = line_of_sight(&map, c(8, 13), c(4, 17), &none);
    assert!(s.visible);
    assert_eq!(s.cover, Cover::Half);
}

#[test]
fn hex_distance_on_the_world_scale() {
    let map = tiny_hex(12, 8);
    assert_eq!(map.distance(c(3, 4), c(10, 6), Diagonal::Chebyshev), 8);
    assert!((map.range_meters(c(3, 4), c(10, 6), Diagonal::Chebyshev) - 80_000.0).abs() < 1e-6);
}
