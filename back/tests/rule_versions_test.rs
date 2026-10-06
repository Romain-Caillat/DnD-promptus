//! campaign/edit-rule-system — done when the GM changes the rules of a
//! campaign, sees what the change touches, locks it for the next
//! session, and the players read the change before they play — never
//! mid-game.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const RULES_V1: &str = include_str!("../../content/rules/corsaires/v1.yaml");

async fn gm(app: &Router, token: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
    call(app, Some(token), method, path, body).await
}

fn v2(text: &str) -> String {
    text.replacen("\nversion: 1\n", "\nversion: 2\n", 1)
}

#[tokio::test]
async fn a_draft_shows_what_it_touches_and_ships_at_the_next_session() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &token, CORSAIRES).await;
    let base = format!("/api/campaigns/{campaign}/rules");
    let code = invite_code(&app, &token, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let device = marc.player_token().unwrap();

    // The campaign plays the preset; nothing is drafted yet.
    let r = gm(&app, &token, "GET", &base, None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["current"], 1);
    assert!(r.body["data"]["draft"].is_null());
    assert_eq!(r.body["data"]["versions"][0]["preset"], true);

    // A draft copies it, one version up, comments kept; unchanged, it
    // touches nothing, and the quay is fought the same on both.
    let r = gm(&app, &token, "POST", &format!("{base}/draft"), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let draft = &r.body["data"]["draft"];
    assert_eq!(draft["version"], 2);
    let yaml = draft["yaml"].as_str().unwrap().to_string();
    assert!(yaml.starts_with("# Corsaires de la Couronne"), "{yaml:.80}");
    assert!(yaml.contains("\nversion: 2\n"));
    assert_eq!(draft["document"]["version"], 2);
    let report = &draft["report"];
    assert_eq!(report["changes"], json!([]));
    assert_eq!(report["story"], json!([]));
    let quay = report["fights"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "bagarre-du-quai")
        .expect("the world's scenario is played")
        .clone();
    assert_eq!(quay["current"], quay["draft"], "{quay}");
    // The campaign's own quay fight is staged from its scene.
    assert!(
        report["fights"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["source"] == "encounter" && f["id"] == "sc_quai"),
        "{}",
        report["fights"]
    );
    // Asking again answers the same draft.
    let again = gm(&app, &token, "POST", &format!("{base}/draft"), None).await;
    assert_eq!(again.body["data"]["draft"]["version"], 2);

    // « Moyen » goes from 10 to 12, and Gueule-Rouge leaves the rules:
    // the players will read the first, the campaign breaks on the second.
    let edited = yaml
        .replace(
            "{ id: moyen, name: Moyen, value: 10,",
            "{ id: moyen, name: Moyen, value: 12,",
        )
        .replace("id: gueule_rouge\n", "id: gueule_rouge_parti\n");
    assert_ne!(edited, yaml);
    let r = gm(
        &app,
        &token,
        "PUT",
        &format!("{base}/draft"),
        Some(json!({ "yaml": edited, "note": "Moyen à 12." })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let report = &r.body["data"]["draft"]["report"];
    let changes = report["changes"].as_array().unwrap();
    assert!(
        changes.iter().any(|c| c["section"] == "difficulties"
            && c["subject"] == "Moyen"
            && c["to"]["value"] == 12),
        "{changes:?}"
    );
    assert!(
        report["story"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "RULES_UNKNOWN_ADVERSARY"),
        "{}",
        report["story"]
    );
    let quay = report["fights"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "bagarre-du-quai")
        .unwrap()
        .clone();
    assert!(quay["draft"].is_null(), "{quay}");
    assert!(quay["error"].as_str().unwrap().contains("gueule_rouge"));

    // A text that does not load, or that renames the system, is refused.
    let r = gm(
        &app,
        &token,
        "PUT",
        &format!("{base}/draft"),
        Some(json!({ "yaml": "id: corsaires\nversion: 2\n" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "RULES_INVALID");
    let r = gm(
        &app,
        &token,
        "PUT",
        &format!("{base}/draft"),
        Some(json!({ "yaml": RULES_V1 })),
    )
    .await;
    assert_eq!(r.body["error"]["code"], "RULES_ID_CHANGED");

    // The structured editor saves a document: Gueule-Rouge back, a house
    // rule added.
    let r = gm(&app, &token, "GET", &base, None).await;
    let mut doc = r.body["data"]["draft"]["document"].clone();
    for a in doc["adversaries"].as_array_mut().unwrap() {
        if a["id"] == "gueule_rouge_parti" {
            a["id"] = json!("gueule_rouge");
        }
    }
    doc["house_rules"] = json!([{
        "id": "feu",
        "name": "Le feu effraie",
        "text": "Un marin touché par le feu fuit un tour."
    }]);
    let r = gm(
        &app,
        &token,
        "PUT",
        &format!("{base}/draft"),
        Some(json!({ "document": doc, "note": "Moyen à 12, le feu effraie." })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let report = &r.body["data"]["draft"]["report"];
    // Gueule-Rouge is back; every scene check still planned at 10 is now
    // off the scale, and the GM is told where.
    let story = report["story"].as_array().unwrap();
    assert!(!story.is_empty());
    assert!(
        story.iter().all(|i| i["code"] == "DIFFICULTY_OFF_SCALE"),
        "{story:?}"
    );
    assert!(
        story
            .iter()
            .any(|i| i["path"] == "nodes[0].checks[2].difficulty")
    );
    assert!(
        report["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["section"] == "house_rules" && c["field"] == "added")
    );

    // Compared with the version played: what players read, and lines.
    let r = gm(
        &app,
        &token,
        "GET",
        &format!("{base}/compare?from=1&to=2"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(!r.body["data"]["changes"].as_array().unwrap().is_empty());
    assert!(
        r.body["data"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["kind"] == "added" && l["text"].as_str().unwrap().contains("Moyen"))
    );

    // Locked: it waits for the next session. Players still read v1.
    let r = gm(&app, &token, "POST", &format!("{base}/draft/lock"), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["current"], 1);
    assert_eq!(r.body["data"]["next"], 2);
    assert!(r.body["data"]["draft"].is_null());
    let r = gm(
        &app,
        &token,
        "PUT",
        &format!("{base}/draft"),
        Some(json!({ "yaml": v2(RULES_V1) })),
    )
    .await;
    assert_eq!(
        r.body["error"]["code"], "NO_DRAFT",
        "a locked version never changes"
    );
    let page = call_as_player(
        &app,
        Some(&device),
        "GET",
        &format!("/api/play/{campaign}/rules"),
        None,
    )
    .await;
    assert_eq!(page.body["data"]["version"], 1);
    assert!(page.body["data"]["changes"].is_null());

    // The next session opens on v2, and Marc reads what changed.
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("/api/campaigns/{campaign}/session"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let r = gm(&app, &token, "GET", &base, None).await;
    assert_eq!(r.body["data"]["current"], 2);
    assert!(r.body["data"]["next"].is_null());
    let page = call_as_player(
        &app,
        Some(&device),
        "GET",
        &format!("/api/play/{campaign}/rules"),
        None,
    )
    .await;
    assert_eq!(page.body["data"]["version"], 2);
    let changes = &page.body["data"]["changes"];
    assert_eq!(
        (changes["fromVersion"].clone(), changes["toVersion"].clone()),
        (json!(1), json!(2))
    );
    assert_eq!(changes["replaced"], false);
    let items = changes["items"].as_array().unwrap();
    assert!(items.iter().any(|c| c["subject"] == "Moyen"), "{items:?}");
    assert!(items.iter().any(|c| c["section"] == "house_rules"));
    assert_eq!(page.body["data"]["houseRules"][0]["name"], "Le feu effraie");
    // The stored version is what the table now plays.
    let difficulty: i64 = page.body["data"]["difficulties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "moyen")
        .unwrap()["value"]
        .as_i64()
        .unwrap();
    assert_eq!(difficulty, 12);
}

#[tokio::test]
async fn another_gm_cannot_read_or_draft_the_rules() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, owner) = common::signed_in_gm(&pool, "Romain").await;
    let (_, other) = common::signed_in_gm(&pool, "Autre").await;
    let campaign = imported_campaign(&app, &owner, CORSAIRES).await;
    for (method, path) in [
        ("GET", format!("/api/campaigns/{campaign}/rules")),
        ("POST", format!("/api/campaigns/{campaign}/rules/draft")),
        (
            "POST",
            format!("/api/campaigns/{campaign}/rules/draft/lock"),
        ),
        ("DELETE", format!("/api/campaigns/{campaign}/rules/draft")),
    ] {
        let r = gm(&app, &other, method, &path, None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{method} {path}");
    }
    let drafts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rule_versions WHERE campaign_id = $1")
            .bind(uuid::Uuid::parse_str(&campaign).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(drafts, 0);
}
