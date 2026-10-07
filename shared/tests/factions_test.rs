//! campaign/track-factions-and-goals — the affinity gauges and the
//! campaign goals, on both witness worlds as written.

use std::path::Path;

use promptus_shared::story::{
    AffinityShift, Campaign, GoalStatus, WorldError, WorldState, from_yaml,
};

fn campaign(world: &str) -> Campaign {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../content/campaigns")
        .join(world)
        .join("campagne.yaml");
    from_yaml(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn shift(faction: &str, from: i32, to: i32) -> AffinityShift {
    AffinityShift {
        faction: faction.to_string(),
        from,
        to,
    }
}

#[test]
fn favour_with_the_sereth_costs_their_rivals_within_their_bounds() {
    let c = campaign("brasier");
    let mut w = WorldState::default();
    // The Vorr start hostile (-3), the others at 0.
    assert_eq!(w.affinity_of(&c, "fac_vorr"), Some(-3));
    assert_eq!(w.affinity_of(&c, "fac_sereth"), Some(0));

    let moved = w.shift_affinity(&c, "fac_sereth", 2).unwrap();
    assert_eq!(
        moved,
        vec![
            shift("fac_sereth", 0, 2),
            shift("fac_vorr", -3, -5),
            shift("fac_cephalopodes", 0, -2),
        ]
    );
    // The Mireth are not the Sereth's rivals: they do not move.
    assert_eq!(w.affinity_of(&c, "fac_mireth"), Some(0));
    // The Vorr sit at their floor: another point of Sereth favour moves
    // the Céphalopodes only.
    let moved = w.shift_affinity(&c, "fac_sereth", 1).unwrap();
    assert_eq!(
        moved,
        vec![shift("fac_sereth", 2, 3), shift("fac_cephalopodes", -2, -3)]
    );
    // Only the faction dealt with becomes known; its rivals' gauges moved
    // out of the players' sight.
    assert!(w.known_factions.contains("fac_sereth"));
    assert!(!w.known_factions.contains("fac_vorr"));
}

#[test]
fn losing_favour_moves_no_one_else_and_stops_at_the_floor() {
    let c = campaign("brasier");
    let mut w = WorldState::default();
    let moved = w.shift_affinity(&c, "fac_cephalopodes", -7).unwrap();
    assert_eq!(moved, vec![shift("fac_cephalopodes", 0, -5)]);
    assert_eq!(w.affinity_of(&c, "fac_mireth"), Some(0));
    assert_eq!(w.affinity_of(&c, "fac_sereth"), Some(0));
    // Already at the floor: nothing moves, nothing is reported.
    assert!(
        w.shift_affinity(&c, "fac_cephalopodes", -1)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        w.shift_affinity(&c, "fac_inconnue", 1),
        Err(WorldError::UnknownFaction("fac_inconnue".into()))
    );
}

#[test]
fn the_crown_and_gueule_rouge_pull_against_each_other() {
    let c = campaign("corsaires");
    let mut w = WorldState::default();
    let moved = w.shift_affinity(&c, "fac_couronne", 1).unwrap();
    assert_eq!(
        moved,
        vec![
            shift("fac_couronne", 0, 1),
            shift("fac_bande_gueule_rouge", 0, -1)
        ]
    );
    let moved = w.shift_affinity(&c, "fac_bande_gueule_rouge", 3).unwrap();
    assert_eq!(
        moved,
        vec![
            shift("fac_bande_gueule_rouge", -1, 2),
            shift("fac_couronne", 1, -2)
        ]
    );
}

#[test]
fn a_component_ticked_off_makes_its_holder_known() {
    let c = campaign("brasier");
    let mut w = WorldState::default();
    w.set_goal(&c, "but_lentille_echo", Some(GoalStatus::Known))
        .unwrap();
    assert!(w.known_factions.is_empty());
    w.set_goal(&c, "but_lentille_echo", Some(GoalStatus::Done))
        .unwrap();
    assert_eq!(w.goals.get("but_lentille_echo"), Some(&GoalStatus::Done));
    assert!(w.known_factions.contains("fac_sereth"));
    // Undone: hidden again, the faction stays met.
    w.set_goal(&c, "but_lentille_echo", None).unwrap();
    assert!(w.goals.is_empty());
    assert!(w.known_factions.contains("fac_sereth"));
    // The Corsaires' one goal has no holder.
    let k = campaign("corsaires");
    let mut w = WorldState::default();
    w.set_goal(&k, "but_plans_greyhound", Some(GoalStatus::Done))
        .unwrap();
    assert!(w.known_factions.is_empty());
}

#[test]
fn lumen_is_the_brasier_companion_and_the_corsaires_have_none() {
    let ids: Vec<String> = campaign("brasier")
        .companions()
        .map(|n| n.id.clone())
        .collect();
    assert_eq!(ids, ["pnj_lumen"]);
    assert_eq!(campaign("corsaires").companions().count(), 0);
}

#[test]
fn a_world_stored_before_factions_still_loads() {
    let w: WorldState =
        serde_json::from_str(r#"{"current_node":"sc_quai","found_clues":["cl_a"]}"#).unwrap();
    assert!(w.affinity.is_empty() && w.goals.is_empty() && w.known_factions.is_empty());
}
