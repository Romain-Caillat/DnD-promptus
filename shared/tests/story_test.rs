//! campaign/model-story-graph — the model, its YAML, the validator and
//! the world operations. Ports the intent of V1's
//! `story-validator.test.ts` and `world.test.ts`.

use promptus_shared::story::{
    Campaign, Clue, ClueReveal, Exit, Importance, Issue, NodeStatus, Revelation, Severity,
    WorldError, WorldState, from_yaml, to_yaml, validate,
};

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

fn fixture() -> Campaign {
    from_yaml(FIXTURE).expect("the fixture parses")
}

fn codes(issues: &[Issue]) -> Vec<(&'static str, String)> {
    issues.iter().map(|i| (i.code, i.path.clone())).collect()
}

fn clue(id: &str, revelation: &str, node: &str) -> Clue {
    Clue {
        id: id.into(),
        revelation: revelation.into(),
        node: node.into(),
        text: "Un indice.".into(),
        discovery: String::new(),
        source: None,
        check: None,
    }
}

// --- YAML ------------------------------------------------------------------

#[test]
fn the_fixture_parses_into_the_full_model() {
    let c = fixture();
    assert_eq!(c.title, "Le Phare de Kerbrume");
    assert_eq!(c.rules.id, "corsaires");
    assert_eq!(c.nodes.len(), 4);
    let crique = c.node("sc_crique").unwrap();
    let enc = crique.encounter.as_ref().unwrap();
    assert_eq!(enc.opponents[1].count, 4);
    assert_eq!(enc.opponents[0].count, 1, "count defaults to one");
    assert_eq!(enc.morale.len(), 2);
    assert_eq!(crique.ambience.music.len(), 2);
    assert_eq!(crique.player_hooks.len(), 3);
    assert_eq!(crique.requires, vec!["rev_crique"]);
    let check = &c.node("sc_taverne").unwrap().checks[0];
    assert_eq!((check.stat.as_str(), check.difficulty), ("CHA", 10));
    assert!(!check.natural_1.is_empty() && !check.natural_20.is_empty());
    let corentin = c.npc("pnj_corentin").unwrap();
    assert_eq!(corentin.stats.as_ref().unwrap().hit_points, Some(10));
    assert!(!corentin.hides.is_empty());
}

#[test]
fn export_then_import_is_an_identity() {
    let c = fixture();
    let exported = to_yaml(&c).unwrap();
    let back = from_yaml(&exported).unwrap();
    assert_eq!(back, c);
    // And exporting again gives the same text: nothing drifts.
    assert_eq!(to_yaml(&back).unwrap(), exported);
    // Empty optional fields stay out of the export.
    assert!(!exported.contains("gm_notes: ''"), "{exported}");
    assert!(!exported.contains("hidden: false"), "{exported}");
}

#[test]
fn a_typo_in_a_key_is_refused_with_its_line() {
    let typo = FIXTURE.replace("    hides: Il paie le douanier.", "    hidez: Il paie le douanier.");
    assert_ne!(typo, FIXTURE);
    let err = from_yaml(&typo).unwrap_err().to_string();
    assert!(err.contains("hidez"), "{err}");
    assert!(err.contains("line"), "{err}");
}

#[test]
fn a_wrong_enum_value_is_refused() {
    let bad = FIXTURE.replacen("disposition: friendly", "disposition: amical", 1);
    assert!(from_yaml(&bad).is_err());
}

// --- Validator ---------------------------------------------------------------

#[test]
fn the_fixture_validates_without_any_issue() {
    let issues = validate(&fixture());
    assert!(issues.is_empty(), "{issues:#?}");
}

#[test]
fn a_critical_revelation_with_two_clues_breaks_the_three_clue_rule() {
    let mut c = fixture();
    c.clues.retain(|x| x.id != "cl_carte");
    let issues = validate(&c);
    assert_eq!(
        codes(&issues),
        vec![("THREE_CLUE_RULE", "revelations[1]".to_string())],
        "{issues:#?}"
    );
    assert!(issues[0].detail.contains("rev_crique"));
    assert!(issues[0].detail.contains("2 clue(s) in 2 node(s)"));
}

#[test]
fn three_clues_in_the_same_two_nodes_still_break_the_rule() {
    let mut c = fixture();
    // Move the phare clue to the tavern: three clues, two nodes.
    c.clues.iter_mut().find(|x| x.id == "cl_carte").unwrap().node = "sc_taverne".into();
    let issues = validate(&c);
    assert_eq!(codes(&issues), vec![("THREE_CLUE_RULE", "revelations[1]".into())]);
}

#[test]
fn an_optional_revelation_needs_only_one_clue() {
    let mut c = fixture();
    assert_eq!(c.clues_for("rev_douane").count(), 1);
    c.revelations[2].importance = Importance::Optional;
    assert!(validate(&c).is_empty());
    // But one with no clue at all is reported.
    c.revelations.push(Revelation {
        id: "rev_orpheline".into(),
        statement: "Personne ne le saura.".into(),
        importance: Importance::Optional,
    });
    assert_eq!(codes(&validate(&c)), vec![("REVELATION_NO_CLUE", "revelations[3]".into())]);
}

#[test]
fn a_dangling_clue_is_an_error_on_each_broken_reference() {
    let mut c = fixture();
    c.clues.push(clue("cl_fantome", "rev_inexistante", "sc_nulle_part"));
    let issues = validate(&c);
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| (i.code, i.path.as_str()))
        .collect();
    assert_eq!(
        errors,
        vec![
            ("REF_DANGLING", "clues[7].revelation"),
            ("REF_DANGLING", "clues[7].node"),
        ]
    );
}

#[test]
fn a_reference_to_the_wrong_kind_is_caught() {
    let mut c = fixture();
    // An NPC presence pointing at a location, an exit pointing at an NPC.
    c.nodes[0].npcs[0].npc = "lieu_port".into();
    c.nodes[0].exits.push(Exit {
        to: "pnj_gwen".into(),
        label: "Suivre Gwen".into(),
    });
    let got: Vec<_> = validate(&c)
        .into_iter()
        .map(|i| (i.code, i.path))
        .collect();
    assert_eq!(
        got,
        vec![
            ("REF_WRONG_KIND", "nodes[0].npcs[0].npc".into()),
            ("REF_WRONG_KIND", "nodes[0].exits[2].to".into()),
        ]
    );
}

#[test]
fn every_kind_of_reference_is_checked() {
    // One broken reference per referencing field: each must be reported.
    let mut c = fixture();
    c.bible.start_node = Some("x_start".into());
    c.nodes[0].act = "x_act".into();
    c.nodes[0].location = Some("x_loc".into());
    c.nodes[3].encounter.as_mut().unwrap().opponents[1].who = "x_who".into();
    c.nodes[3].loot[1].item = Some("x_loot".into());
    c.nodes[3].requires[0] = "x_req".into();
    c.nodes[3].player_hooks[0].character = "x_pc".into();
    c.clues[0].source = Some("x_src".into());
    c.npcs[0].sells[0].item = "x_sell".into();
    c.npcs[1].inventory[0].item = "x_inv".into();
    c.npcs[2].faction = Some("x_fac".into());
    c.npcs[2].location = Some("x_npcloc".into());
    c.locations[1].parent = Some("x_parent".into());
    c.factions[0].rivals[0] = "x_rival".into();
    c.goals[0].item = Some("x_goal_item".into());
    c.goals[0].held_by = Some("x_holder".into());
    let dangling: Vec<String> = validate(&c)
        .into_iter()
        .filter(|i| i.code == "REF_DANGLING")
        .map(|i| i.path)
        .collect();
    for path in [
        "bible.start_node",
        "nodes[0].act",
        "nodes[0].location",
        "nodes[3].encounter.opponents[1].who",
        "nodes[3].loot[1].item",
        "nodes[3].requires[0]",
        "nodes[3].player_hooks[0].character",
        "clues[0].source",
        "npcs[0].sells[0].item",
        "npcs[1].inventory[0].item",
        "npcs[2].faction",
        "npcs[2].location",
        "locations[1].parent",
        "factions[0].rivals[0]",
        "goals[0].item",
        "goals[0].held_by",
    ] {
        assert!(dangling.iter().any(|p| p == path), "{path} not reported: {dangling:?}");
    }
}

#[test]
fn ids_are_unique_across_the_whole_campaign_and_well_formed() {
    let mut c = fixture();
    // An item named like an NPC: two kinds, one id.
    c.items[0].id = "pnj_gwen".into();
    c.locations[0].id = "Lieu Port".into();
    let got = codes(&validate(&c));
    assert!(got.contains(&("ID_DUPLICATE", "items[0].id".into())), "{got:?}");
    assert!(got.contains(&("ID_INVALID", "locations[0].id".into())), "{got:?}");
}

#[test]
fn required_knowledge_given_nowhere_else_is_reported() {
    let mut c = fixture();
    // The crique requires rev_crique; leave only a clue placed in the
    // crique itself (useless: the players need it before entering).
    c.clues.retain(|x| x.revelation != "rev_crique");
    c.clues.push(clue("cl_sur_place", "rev_crique", "sc_crique"));
    let got = codes(&validate(&c));
    assert!(
        got.contains(&("KNOWLEDGE_NEVER_GIVEN", "nodes[3].requires[0]".into())),
        "{got:?}"
    );
}

#[test]
fn required_knowledge_only_in_optional_scenes_is_reported() {
    // The Corsaires defect: the Greyhound route only in an optional scene.
    let mut c = fixture();
    c.clues.retain(|x| x.revelation != "rev_crique" || x.node == "sc_port");
    assert!(c.node("sc_port").unwrap().optional);
    let got = codes(&validate(&c));
    assert!(
        got.contains(&("KNOWLEDGE_ONLY_OPTIONAL", "nodes[3].requires[0]".into())),
        "{got:?}"
    );
    // Making the port mandatory clears that warning (the three-clue one stays).
    c.nodes[1].optional = false;
    let got = codes(&validate(&c));
    assert!(!got.iter().any(|(code, _)| code.starts_with("KNOWLEDGE")), "{got:?}");
}

#[test]
fn structural_checks() {
    let mut c = fixture();
    c.fronts[0].steps.truncate(3);
    c.nodes[2].exits.clear(); // the crique is now unreachable
    c.nodes[0].exits.push(Exit {
        to: "sc_taverne".into(),
        label: "Rester".into(),
    });
    c.nodes[3].loot[0].coins = Some(0);
    c.nodes[0].ambience.music[0].url = "https://example.com/air.mp3".into();
    c.factions[1].affinity.start = 9;
    let got = codes(&validate(&c));
    for expected in [
        ("FRONT_CLOCK_LENGTH", "fronts[0].steps"),
        ("NODE_UNREACHABLE", "nodes[3]"),
        ("EXIT_TO_SELF", "nodes[0].exits[2]"),
        ("LOOT_EMPTY", "nodes[3].loot[0]"),
        ("MUSIC_NOT_YOUTUBE", "nodes[0].ambience.music[0].url"),
        ("AFFINITY_RANGE", "factions[1].affinity"),
    ] {
        assert!(
            got.contains(&(expected.0, expected.1.to_string())),
            "{expected:?} missing from {got:?}"
        );
    }
    // Only the unreachable node is unreachable.
    assert_eq!(got.iter().filter(|(c, _)| *c == "NODE_UNREACHABLE").count(), 1);
}

#[test]
fn a_campaign_with_nodes_needs_an_opening_node() {
    let mut c = fixture();
    c.bible.start_node = None;
    assert_eq!(codes(&validate(&c)), vec![("START_NODE_MISSING", "bible.start_node".into())]);
}

// --- World -------------------------------------------------------------------

#[test]
fn a_revelation_is_known_once_any_of_its_clues_is_found() {
    let c = fixture();
    let mut w = WorldState::default();
    assert!(!w.is_revelation_known(&c, "rev_crique"));
    assert_eq!(
        w.reveal_clue(&c, "cl_traces").unwrap(),
        ClueReveal::NewRevelation("rev_crique".into())
    );
    assert!(w.is_revelation_known(&c, "rev_crique"));
    assert!(!w.is_revelation_known(&c, "rev_naufrageurs"));
    // A second path to the same revelation reinforces it…
    assert_eq!(
        w.reveal_clue(&c, "cl_carte").unwrap(),
        ClueReveal::Reinforced("rev_crique".into())
    );
    // …and finding a clue twice changes nothing.
    assert_eq!(w.reveal_clue(&c, "cl_carte").unwrap(), ClueReveal::AlreadyFound);
    assert_eq!(w.found_clues.len(), 2);
    assert_eq!(
        w.reveal_clue(&c, "cl_invente"),
        Err(WorldError::UnknownClue("cl_invente".into()))
    );
}

#[test]
fn entering_a_node_makes_it_current_and_visited_and_resolved_stays_resolved() {
    let c = fixture();
    let mut w = WorldState::default();
    w.enter_node(&c, "sc_taverne").unwrap();
    assert_eq!(w.current_node.as_deref(), Some("sc_taverne"));
    assert_eq!(w.node_status["sc_taverne"], NodeStatus::Visited);
    w.resolve_node(&c, "sc_taverne").unwrap();
    w.enter_node(&c, "sc_port").unwrap();
    w.enter_node(&c, "sc_taverne").unwrap();
    assert_eq!(w.node_status["sc_taverne"], NodeStatus::Resolved);
    assert!(w.is_visited("sc_taverne"));
    assert!(!w.is_visited("sc_crique"));
    assert!(w.enter_node(&c, "sc_inconnue").is_err());
    assert_eq!(w.current_node.as_deref(), Some("sc_taverne"));
}

#[test]
fn a_front_clock_is_bounded_and_reports_the_steps_reached() {
    let c = fixture();
    let mut w = WorldState::default();
    let a = w.advance_front(&c, "front_naufrageurs", 2).unwrap();
    assert_eq!((a.from, a.to, a.reached), (0, 2, vec![0, 1]));
    let b = w.advance_front(&c, "front_naufrageurs", 99).unwrap();
    assert_eq!((b.to, b.reached), (4, vec![2, 3]));
    let back = w.advance_front(&c, "front_naufrageurs", -10).unwrap();
    assert_eq!((back.to, back.reached), (0, vec![]));
    assert!(w.advance_front(&c, "front_x", 1).is_err());
}

#[test]
fn only_npcs_and_adversaries_can_be_revealed() {
    let c = fixture();
    let mut w = WorldState::default();
    w.reveal_entity(&c, "pnj_corentin").unwrap();
    w.reveal_entity(&c, "adv_contrebandier").unwrap();
    assert!(w.reveal_entity(&c, "lieu_port").is_err());
    assert_eq!(w.revealed.len(), 2);
}
