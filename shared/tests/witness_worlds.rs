//! campaign/rewrite-two-worlds — the two witness worlds against their
//! rule systems and maps, and the played Corsaires act whose defects the
//! checks must keep finding (`docs/lecons-des-parties.md` §4).

use std::path::Path;

use promptus_shared::maps::Map;
use promptus_shared::rules::RuleSystem;
use promptus_shared::story::{Campaign, Issue, Library, Severity, from_yaml, validate_with};

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../content")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn campaign(rel: &str) -> Campaign {
    from_yaml(&read(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn rules(world: &str) -> RuleSystem {
    RuleSystem::from_yaml(&read(&format!("rules/{world}/v1.yaml"))).unwrap()
}

fn maps() -> Vec<Map> {
    [
        "maps/corsaires/quai-port-louis.yaml",
        "maps/brasier/cure-dent-coursive.yaml",
    ]
    .iter()
    .map(|rel| Map::from_yaml(&read(rel)).unwrap())
    .collect()
}

fn check(c: &Campaign, world: &str) -> Vec<Issue> {
    let system = rules(world);
    let maps = maps();
    validate_with(
        c,
        &Library {
            rules: Some(&system),
            maps: Some(&maps),
        },
    )
}

fn count(issues: &[Issue], code: &str) -> usize {
    issues.iter().filter(|i| i.code == code).count()
}

const PLAYED_DEFECTS: [&str; 4] = [
    "THREE_CLUE_RULE",
    "KNOWLEDGE_ONLY_OPTIONAL",
    "PLAYER_WITHOUT_HOOK",
    "DIFFICULTY_OFF_SCALE",
];

#[test]
fn the_played_act_shows_what_went_wrong() {
    let played = campaign("fixtures/corsaires-acte-1-joue.yaml");
    let issues = check(&played, "corsaires");
    // Route, escort and repair: one clue each, in an optional place.
    assert_eq!(count(&issues, "THREE_CLUE_RULE"), 3, "{issues:#?}");
    assert_eq!(count(&issues, "KNOWLEDGE_ONLY_OPTIONAL"), 3, "{issues:#?}");
    assert!(
        issues
            .iter()
            .any(|i| i.code == "KNOWLEDGE_ONLY_OPTIONAL" && i.detail.contains("rev_route"))
    );
    // Nobody has a reason to act anywhere: six players, one head.
    assert_eq!(count(&issues, "PLAYER_WITHOUT_HOOK"), 6);
    // Jacquot at 8 and 6, off the 5/10/15/20 scale.
    assert_eq!(count(&issues, "DIFFICULTY_OFF_SCALE"), 2);
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "{issues:#?}"
    );
}

#[test]
fn the_rewritten_corsaires_act_is_clean() {
    let c = campaign("campaigns/corsaires/campagne.yaml");
    let issues = check(&c, "corsaires");
    assert!(issues.is_empty(), "{issues:#?}");
    // Every party slot has a reason to act in at least two scenes.
    for p in &c.party {
        let scenes = c
            .nodes
            .iter()
            .filter(|n| n.player_hooks.iter().any(|h| h.character == p.id))
            .count();
        assert!(scenes >= 2, "{} has hooks in {scenes} scene(s)", p.id);
    }
    // The quay is played on its map, with stats from the rule system.
    let quai = c.node("sc_quai").unwrap();
    assert_eq!(quai.map.as_deref(), Some("quai-port-louis"));
    assert_eq!(
        c.adversary("marin-de-gueule-rouge")
            .unwrap()
            .stats
            .from_rules
            .as_deref(),
        Some("marin_de_gueule_rouge")
    );
}

#[test]
fn the_brasier_is_playable_for_one_scene_and_says_it_is_not_ready() {
    let c = campaign("campaigns/brasier/campagne.yaml");
    let issues = check(&c, "brasier");
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "{issues:#?}"
    );
    for code in PLAYED_DEFECTS.iter().skip(1) {
        assert_eq!(count(&issues, code), 0, "{code}: {issues:#?}");
    }
    // Its first scene: six hooks, an encounter on the corridor map.
    let first = c.node("sc_toboggan").unwrap();
    assert_eq!(first.player_hooks.len(), 6);
    assert_eq!(first.map.as_deref(), Some("cure-dent-coursive"));
    assert!(first.encounter.is_some());
    // Beyond it the act is a sketch: what the act needs is not yet
    // reachable three ways, and its music is still to choose.
    assert_eq!(count(&issues, "THREE_CLUE_RULE"), 2);
    assert!(count(&issues, "MUSIC_NO_URL") > 0);
    assert_eq!((c.factions.len(), c.goals.len()), (4, 4));
}

#[test]
fn rule_and_map_references_are_checked_against_the_library() {
    let text = read("campaigns/corsaires/campagne.yaml")
        .replace("from_rules: capitaine_morel", "from_rules: capitaine_moral")
        .replace("class: vigie", "class: gabier")
        .replace("map: quai-port-louis", "map: quai-de-brest");
    let c = from_yaml(&text).unwrap();
    let issues = check(&c, "corsaires");
    let codes: Vec<&str> = issues.iter().map(|i| i.code).collect();
    assert!(codes.contains(&"RULES_UNKNOWN_ADVERSARY"), "{codes:?}");
    assert!(codes.contains(&"RULES_UNKNOWN_CLASS"), "{codes:?}");
    assert!(codes.contains(&"MAP_UNKNOWN"), "{codes:?}");

    // The wrong rule system is named as such, not as a pile of misses.
    let issues = check(&campaign("campaigns/corsaires/campagne.yaml"), "brasier");
    assert_eq!(
        issues.iter().map(|i| i.code).collect::<Vec<_>>(),
        ["RULES_MISMATCH"]
    );

    // A map token naming no one of the campaign is reported.
    let mut c = campaign("campaigns/brasier/campagne.yaml");
    c.adversaries.retain(|a| a.id != "chef-d-escouade-vorr");
    if let Some(e) = c.nodes[0].encounter.as_mut() {
        e.opponents.retain(|o| o.who != "chef-d-escouade-vorr");
    }
    let issues = check(&c, "brasier");
    assert_eq!(count(&issues, "MAP_ENTITY_UNKNOWN"), 1, "{issues:#?}");
}
