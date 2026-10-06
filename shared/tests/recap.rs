//! session/write-recaps — what a session changed, read from the data
//! (V1 `tests/unit/recap.test.ts`), and the two recaps written from it:
//! the GM's may hold secrets, « Précédemment… » holds none.

use promptus_shared::story::recap::{
    PartyFact, SessionRecord, default_title, facts_recap, session_facts,
};
use promptus_shared::story::{Campaign, NodeStatus, WorldState, from_yaml};

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

fn fixture() -> Campaign {
    from_yaml(FIXTURE).unwrap()
}

/// The table came in at the tavern and ends on the wreck: one clue
/// found (its revelation now known), the wreckers' clock one step on,
/// Loïc's name learned, a fight where a smuggler fell.
fn session(campaign: &Campaign) -> (WorldState, WorldState, SessionRecord) {
    let mut before = WorldState::default();
    before.enter_node(campaign, "sc_taverne").unwrap();
    let mut after = before.clone();
    after.resolve_node(campaign, "sc_taverne").unwrap();
    after.enter_node(campaign, "sc_port").unwrap();
    after.reveal_clue(campaign, "cl_cordages").unwrap();
    after
        .advance_front(campaign, "front_naufrageurs", 1)
        .unwrap();
    after.reveal_entity(campaign, "pnj_loic").unwrap();
    let record = SessionRecord {
        duration_minutes: 210,
        fights: 1,
        fallen: vec!["Contrebandier".into()],
        party: vec![PartyFact {
            name: "Bobby".into(),
            hit_points: 4,
            max_hit_points: 12,
        }],
        open_threads: vec!["Promis à Gwen de retrouver son frère.".into()],
    };
    (before, after, record)
}

#[test]
fn the_facts_measure_what_the_session_changed() {
    let c = fixture();
    let (before, after, record) = session(&c);
    let f = session_facts(&c, &before, &after, record);
    assert_eq!(f.duration_minutes, 210);
    let scenes: Vec<(&str, bool)> = f
        .scenes
        .iter()
        .map(|s| (s.id.as_str(), s.resolved))
        .collect();
    assert_eq!(scenes, [("sc_taverne", true), ("sc_port", false)]);
    assert_eq!(
        f.clues.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["cl_cordages"]
    );
    assert_eq!(
        f.revelations
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["rev_naufrageurs"]
    );
    assert_eq!(f.fronts.len(), 1);
    assert_eq!((f.fronts[0].from, f.fronts[0].to), (0, 1));
    assert_eq!(f.fronts[0].step, "Un feu éteint");
    assert_eq!(f.revealed, ["Loïc Kervella"]);
    assert_eq!(f.fights, 1);
    assert_eq!(default_title(&f), "L'épave du port");
}

#[test]
fn only_what_is_new_counts() {
    let c = fixture();
    let (_, after, record) = session(&c);
    let mut later = after.clone();
    later
        .node_status
        .insert("sc_port".into(), NodeStatus::Visited);
    let f = session_facts(&c, &after, &later, record);
    assert!(f.clues.is_empty());
    assert!(f.revelations.is_empty());
    assert!(f.fronts.is_empty());
    assert!(f.revealed.is_empty());
    // A second clue of a known revelation teaches nothing new.
    let mut more = after.clone();
    more.reveal_clue(&c, "cl_carnet").unwrap();
    let f = session_facts(&c, &after, &more, SessionRecord::default());
    assert_eq!(f.clues.len(), 1);
    assert!(f.revelations.is_empty());
}

#[test]
fn the_players_recap_holds_no_threat_and_no_secret() {
    let c = fixture();
    let (before, after, record) = session(&c);
    let f = session_facts(&c, &before, &after, record);
    let (gm, players) = facts_recap(&f);
    assert!(gm.contains("Les naufrageurs de Corentin"), "{gm}");
    assert!(gm.contains("Un feu éteint"));
    assert!(gm.contains("Bobby 4/12 PV"));
    assert!(gm.contains("Promis à Gwen"));
    assert!(players.contains("L'épave du port"), "{players}");
    assert!(players.contains("Les cordages de l'épave ont été tranchés au couteau."));
    assert!(players.contains("Corentin éteint le phare"));
    assert!(players.contains("Contrebandier"));
    assert!(!players.contains("naufrageurs de Corentin"), "{players}");
    assert!(!players.contains("feu éteint"), "{players}");
    // A party member down is said; one merely hurt is the GM's business.
    assert!(!players.contains("Bobby"));
}

#[test]
fn an_empty_session_still_reads() {
    let c = fixture();
    let w = WorldState::default();
    let f = session_facts(&c, &w, &w, SessionRecord::default());
    let (_, players) = facts_recap(&f);
    assert_eq!(players, "Précédemment : l’aventure a commencé.");
    assert_eq!(default_title(&f), "");
}
