//! Between two sessions, over the API, on both witness worlds:
//! session/write-recaps (V1 `continuity.test.ts`: the session is
//! measured, a factual draft waits, the co-GM rewrites, the GM publishes,
//! the next session finds it) and gm/launch-session (« Précédemment… »
//! read sentence by sentence, then the first scene).

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use promptus_shared::story::{Campaign, from_yaml};
use serde_json::{Value, json};

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

struct Table {
    app: Router,
    gm: String,
    campaign: String,
    marc: String,
    story: Campaign,
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn marc(&self, path: &str) -> Value {
        let uri = format!("/api/play/{}{path}", self.campaign);
        let r = call_as_player(&self.app, Some(&self.marc), "GET", &uri, None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }

    async fn ok(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        let r = self.gm(method, path, body).await;
        assert!(
            r.status.is_success(),
            "{method} {path}: {} {}",
            r.status,
            r.body
        );
        r.body["data"].clone()
    }
}

async fn table(yaml: &str) -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, yaml).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    // Room for the co-GM's drafts.
    sqlx::query("UPDATE campaigns SET ai_budget_cents = 100 WHERE id = $1::uuid")
        .bind(&campaign)
        .execute(&pool)
        .await
        .unwrap();
    Table {
        app,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
        story: from_yaml(yaml).unwrap(),
    }
}

/// Session 1 played: the first scene shown, its first clue found, its
/// first NPC met, the first front two steps on, a promise in the journal.
/// Returns (scene title, clue text).
async fn first_session(t: &Table) -> (String, String) {
    let s = &t.story;
    let scene = s
        .nodes
        .iter()
        .find(|n| s.clues.iter().any(|c| c.node == n.id))
        .unwrap();
    let clue = s.clues.iter().find(|c| c.node == scene.id).unwrap();
    t.ok("POST", "/session", None).await;
    t.ok("POST", "/session/start", None).await;
    for reveal in [
        json!({ "kind": "scene", "node": scene.id }),
        json!({ "kind": "clue", "clue": clue.id }),
        json!({ "kind": "npc", "npc": s.npcs[0].id }),
        json!({ "kind": "front", "front": s.fronts[0].id, "delta": 2 }),
    ] {
        let r = t.gm("POST", "/session/reveal", Some(reveal.clone())).await;
        assert_eq!(r.status, StatusCode::NO_CONTENT, "{reveal}: {}", r.body);
    }
    t.ok(
        "POST",
        "/session/journal",
        Some(json!({ "kind": "promise", "text": "Rendre la boussole avant l'aube.", "shared": true })),
    )
    .await;
    (scene.title.clone(), clue.text.clone())
}

async fn recaps_are_drafted_reviewed_and_published(yaml: &str) {
    let t = table(yaml).await;
    let (scene, clue) = first_session(&t).await;
    let front = t.story.fronts[0].name.clone();

    // Ended with nothing written: the server drafts from the facts.
    let ended = t.ok("POST", "/session/end", Some(json!({}))).await;
    let session = ended["id"].as_str().unwrap().to_string();
    assert!(ended["publishedAt"].is_null(), "{ended}");
    let players = ended["previously"].as_str().unwrap();
    assert!(players.starts_with("Précédemment…"), "{players}");
    assert!(
        players.contains(&scene) && players.contains(&clue),
        "{players}"
    );
    assert!(players.contains(&t.story.npcs[0].name), "{players}");
    assert!(players.contains("Rendre la boussole"), "{players}");
    assert!(!players.contains(&front), "{players}");
    assert!(ended["recap"].as_str().unwrap().contains(&front), "{ended}");
    assert_eq!(ended["chronicleTitle"], scene.as_str());

    // A draft reaches no phone.
    let evening = t.marc("/evening").await;
    assert!(evening["previously"].is_null(), "{evening}");
    assert_eq!(t.marc("/chronicle").await, json!([]));

    // The co-GM rewrites (counted, nothing saved): its chronicle entry
    // is titled after the scene played.
    let draft = t
        .ok("POST", &format!("/sessions/{session}/recap-draft"), None)
        .await;
    assert!(
        draft["players"].as_str().unwrap().contains(&clue),
        "{draft}"
    );
    assert_eq!(draft["chronicleTitle"], scene.as_str(), "{draft}");
    assert_eq!(draft["warnings"], json!([]), "{draft}");
    let chronicle = t.ok("GET", "/sessions", None).await;
    assert_eq!(chronicle[0]["previously"], ended["previously"]);

    // The GM slips the front's name in: flagged, not blocked.
    let leaky = json!({
        "recap": "Pour moi.", "previously": format!("{front} approche."),
        "chronicleTitle": scene, "chronicle": "Une nuit agitée.",
    });
    t.ok("PUT", &format!("/sessions/{session}/recap"), Some(leaky))
        .await;
    let chronicle = t.ok("GET", "/sessions", None).await;
    assert_eq!(
        chronicle[0]["warnings"][0]["name"],
        front.as_str(),
        "{chronicle}"
    );
    assert_eq!(chronicle[0]["warnings"][0]["kind"], "front");

    // Nothing to publish without « Précédemment… ».
    let r = t
        .gm(
            "PUT",
            &format!("/sessions/{session}/recap"),
            Some(json!({ "previously": " ", "publish": true })),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "NOTHING_TO_PUBLISH");

    // Reread and published: Marc reads it the next day, and the chronicle
    // has its entry — never the GM's recap.
    let published = json!({
        "recap": "Le capitaine ment, ils ne le savent pas.",
        "previously": "Vous avez accosté. Une boussole a changé de main.\nEt la nuit tombait déjà.",
        "chronicleTitle": scene, "chronicle": "Une boussole a changé de main.",
        "publish": true,
    });
    let s = t
        .ok(
            "PUT",
            &format!("/sessions/{session}/recap"),
            Some(published),
        )
        .await;
    assert!(s["publishedAt"].is_string(), "{s}");
    let evening = t.marc("/evening").await;
    assert_eq!(
        evening["previously"],
        "Vous avez accosté. Une boussole a changé de main.\nEt la nuit tombait déjà."
    );
    let entries = t.marc("/chronicle").await;
    assert_eq!(entries.as_array().unwrap().len(), 1, "{entries}");
    assert_eq!(entries[0]["number"], 1);
    assert_eq!(entries[0]["title"], scene.as_str());
    assert_eq!(entries[0]["text"], "Une boussole a changé de main.");
    assert!(!entries.to_string().contains("ment"), "{entries}");
}

#[tokio::test]
async fn corsaires_recaps_are_drafted_reviewed_and_published() {
    recaps_are_drafted_reviewed_and_published(CORSAIRES).await;
}

#[tokio::test]
async fn brasier_recaps_are_drafted_reviewed_and_published() {
    recaps_are_drafted_reviewed_and_published(BRASIER).await;
}

async fn the_next_session_opens_on_previously_read_line_by_line(yaml: &str) {
    let t = table(yaml).await;
    let (scene, clue) = first_session(&t).await;
    let ended = t.ok("POST", "/session/end", Some(json!({}))).await;
    let session = ended["id"].as_str().unwrap().to_string();
    let previously = "Vous avez accosté. Une boussole a changé de main.\nEt la nuit tombait déjà.";
    t.ok(
        "PUT",
        &format!("/sessions/{session}/recap"),
        Some(json!({ "previously": previously, "publish": true })),
    )
    .await;

    // The lobby: « Précédemment… » to read while waiting.
    t.ok("POST", "/session", None).await;
    assert_eq!(t.marc("/evening").await["previously"], previously);

    // Launched: one sentence on the phones, the GM sees them all and the
    // scene where the table stopped.
    t.ok("POST", "/session/start", None).await;
    let evening = t.marc("/evening").await;
    assert!(evening["previously"].is_null(), "{evening}");
    assert_eq!(evening["launch"]["lines"], json!(["Vous avez accosté."]));
    assert_eq!(evening["launch"]["total"], 3);
    assert_eq!(evening["launch"]["number"], 1);
    let screen = t.ok("GET", "/session", None).await;
    assert_eq!(screen["launch"]["lines"].as_array().unwrap().len(), 3);
    assert_eq!(screen["launch"]["shown"], 1);
    let first = screen["launch"]["firstScene"]["node"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(screen["launch"]["firstScene"]["title"], scene.as_str());

    // The GM's pace, never past the last sentence.
    for _ in 0..4 {
        t.ok("POST", "/session/previously/next", None).await;
    }
    let evening = t.marc("/evening").await;
    assert_eq!(
        evening["launch"]["lines"],
        json!([
            "Vous avez accosté.",
            "Une boussole a changé de main.",
            "Et la nuit tombait déjà."
        ])
    );

    // The first scene ends the reading everywhere.
    let r = t
        .gm(
            "POST",
            "/session/reveal",
            Some(json!({ "kind": "scene", "node": first })),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let evening = t.marc("/evening").await;
    assert!(evening["launch"].is_null(), "{evening}");
    assert_eq!(evening["campaign"]["scene"]["title"], scene.as_str());
    let r = t.gm("POST", "/session/previously/next", None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "NOT_READING");

    // Session 2 measures only what is new: session 1's clue is not in it.
    let ended = t.ok("POST", "/session/end", Some(json!({}))).await;
    let players = ended["previously"].as_str().unwrap();
    assert!(!players.contains(&clue), "{players}");
    assert!(players.contains(&scene), "{players}");
}

#[tokio::test]
async fn corsaires_next_session_opens_on_previously_read_line_by_line() {
    the_next_session_opens_on_previously_read_line_by_line(CORSAIRES).await;
}

#[tokio::test]
async fn brasier_next_session_opens_on_previously_read_line_by_line() {
    the_next_session_opens_on_previously_read_line_by_line(BRASIER).await;
}

/// A session without a published recap starts straight on its scene:
/// nothing to read.
#[tokio::test]
async fn without_a_published_recap_there_is_nothing_to_read() {
    let t = table(CORSAIRES).await;
    first_session(&t).await;
    t.ok(
        "POST",
        "/session/end",
        Some(json!({ "previously": "Brouillon." })),
    )
    .await;
    t.ok("POST", "/session", None).await;
    t.ok("POST", "/session/start", None).await;
    assert!(t.marc("/evening").await["launch"].is_null());
    assert!(t.ok("GET", "/session", None).await["launch"].is_null());
    let r = t
        .gm("POST", "/session/end", Some(json!({ "publish": true })))
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "PUBLISH_AFTER_THE_END");
}
