//! campaign/review-story-graph — done when the GM corrects a campaign,
//! accepts a diff of the co-GM, resolves an alert and validates the
//! campaign, which only then opens a session.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call};
use serde_json::{Value, json};

const PLAYED: &str = include_str!("../../content/fixtures/corsaires-acte-1-joue.yaml");

async fn gm(app: &Router, token: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
    call(app, Some(token), method, path, body).await
}

fn issues<'a>(detail: &'a Value, code: &str) -> Vec<&'a Value> {
    detail["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["code"] == code)
        .collect()
}

/// A campaign to prepare (not imported whole): created from a pitch,
/// then filled with the played act 1 of the Corsaires.
async fn prepared(app: &Router, token: &str) -> String {
    let r = gm(
        app,
        token,
        "POST",
        "/api/campaigns",
        Some(json!({
            "title": "Corsaires",
            "rules": { "id": "corsaires", "version": 1 },
            "aiBudgetCents": 500
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert!(r.body["data"]["validatedAt"].is_null());
    let id = r.body["data"]["id"].as_str().unwrap().to_string();
    let r = gm(
        app,
        token,
        "PUT",
        &format!("/api/campaigns/{id}/import"),
        Some(json!({ "yaml": PLAYED })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    id
}

#[tokio::test]
async fn the_gm_corrects_works_with_the_co_gm_and_validates() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let id = prepared(&app, &token).await;
    let base = format!("/api/campaigns/{id}");

    // Not validated: no session yet.
    let r = gm(&app, &token, "POST", &format!("{base}/session"), None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "CAMPAIGN_NOT_VALIDATED");

    // The GM corrects a scene and the bible by id.
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/story/edits"),
        Some(json!({ "edits": [
            { "op": "set", "target": "sc_quai", "field": "summary", "value": "La bagarre éclate sur le quai." },
            { "op": "set", "target": "bible", "field": "tone", "value": "Salé, nerveux." },
        ]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let c = &r.body["data"]["campaign"];
    let quay = c["story"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "sc_quai")
        .unwrap();
    assert_eq!(quay["summary"], "La bagarre éclate sur le quai.");
    assert_eq!(c["story"]["bible"]["tone"], "Salé, nerveux.");
    assert_eq!(r.body["data"]["changes"][0]["op"], "set");
    assert_eq!(r.body["data"]["changes"][0]["kind"], "node");

    // A field the format does not have is refused, and nothing is saved.
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/story/edits"),
        Some(json!({ "edits": [
            { "op": "set", "target": "bible", "field": "tone", "value": "Autre." },
            { "op": "set", "target": "sc_quai", "field": "humeur", "value": "x" },
        ]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "EDIT_INVALID");
    let r = gm(&app, &token, "GET", &base, None).await;
    assert_eq!(r.body["data"]["story"]["bible"]["tone"], "Salé, nerveux.");

    // The coherence alert: the Greyhound's route has one clue, in an
    // optional scene. The co-GM proposes where to place the missing ones.
    let route = issues(&r.body["data"], "THREE_CLUE_RULE")
        .into_iter()
        .find(|i| i["detail"].as_str().unwrap().contains("rev_route"))
        .expect("the route breaks the three-clue rule")
        .clone();
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/workshop"),
        Some(json!({ "issue": { "code": route["code"], "path": route["path"] } })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let proposal = r.body["data"].clone();
    assert_eq!(proposal["status"], "pending");
    // The co-GM's invented id was dropped before the GM saw it.
    assert_eq!(proposal["dropped"], 1);
    let changes = proposal["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 2, "{changes:?}");
    assert!(
        changes
            .iter()
            .all(|c| c["op"] == "add" && c["kind"] == "clue")
    );
    // Nothing changed before the GM accepts.
    let r = gm(&app, &token, "GET", &base, None).await;
    assert!(!issues(&r.body["data"], "THREE_CLUE_RULE").is_empty());

    let pid = proposal["id"].as_str().unwrap();
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/workshop/{pid}/accept"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["proposal"]["status"], "accepted");
    let detail = &r.body["data"]["campaign"];
    assert!(
        issues(detail, "THREE_CLUE_RULE")
            .iter()
            .all(|i| !i["detail"].as_str().unwrap().contains("rev_route")),
        "{}",
        detail["issues"]
    );
    assert!(
        issues(detail, "KNOWLEDGE_ONLY_OPTIONAL")
            .iter()
            .all(|i| !i["detail"].as_str().unwrap().contains("rev_route"))
    );
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/workshop/{pid}/accept"),
        None,
    )
    .await;
    assert_eq!(r.body["error"]["code"], "PROPOSAL_ALREADY_DECIDED");

    // A free request on a scene, rejected: the campaign keeps its text.
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/workshop"),
        Some(json!({ "prompt": "Ajoute un indice dans la taverne.", "node": "sc_quai" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let pid = r.body["data"]["id"].as_str().unwrap().to_string();
    let clue = r.body["data"]["changes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/workshop/{pid}/reject"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(
        r.body["data"]["campaign"]["story"]["clues"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["id"] != clue.as_str())
    );

    // A proposal the campaign has outgrown is not applied.
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/workshop"),
        Some(json!({ "prompt": "Encore un indice.", "node": "sc_quai" })),
    )
    .await;
    let pid = r.body["data"]["id"].as_str().unwrap().to_string();
    let rev = r.body["data"]["edits"][0]["value"]["revelation"]
        .as_str()
        .unwrap()
        .to_string();
    gm(
        &app,
        &token,
        "POST",
        &format!("{base}/story/edits"),
        Some(json!({ "edits": [{ "op": "add", "kind": "clue", "value": {
            "id": r.body["data"]["edits"][0]["value"]["id"], "revelation": rev,
            "node": "sc_quai", "text": "Écrit par le MJ."
        }}]})),
    )
    .await;
    let list = gm(&app, &token, "GET", &format!("{base}/workshop"), None).await;
    let stale = list.body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == pid.as_str())
        .unwrap();
    assert_eq!(stale["stale"], true);
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/workshop/{pid}/accept"),
        None,
    )
    .await;
    assert_eq!(r.body["error"]["code"], "PROPOSAL_STALE");

    // An error stops the validation; fixed, the campaign is playable.
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/story/edits"),
        Some(json!({ "edits": [
            { "op": "set", "target": "sc_quai", "field": "location", "value": "lieu_disparu" }
        ]})),
    )
    .await;
    assert!(!issues(&r.body["data"]["campaign"], "REF_DANGLING").is_empty());
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/story/validate"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "CAMPAIGN_HAS_ERRORS");
    gm(
        &app,
        &token,
        "POST",
        &format!("{base}/story/edits"),
        Some(json!({ "edits": [
            { "op": "set", "target": "sc_quai", "field": "location", "value": null }
        ]})),
    )
    .await;
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("{base}/story/validate"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(r.body["data"]["validatedAt"].is_string());
    let r = gm(&app, &token, "POST", &format!("{base}/session"), None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);

    // The gauge: the act still lacks scene fields and hooks, but the
    // route now has its paths.
    let r = gm(&app, &token, "GET", &format!("{base}/readiness"), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let act = &r.body["data"][0];
    assert_eq!(act["act"], "acte_1");
    assert_eq!(act["ready"], false);
    let knowledge = act["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "knowledge_paths")
        .unwrap();
    assert!(
        knowledge["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .all(|g| g["id"] != "rev_route"),
        "{knowledge}"
    );

    // Every workshop call was counted.
    let calls: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_calls WHERE campaign_id = $1 AND purpose = 'workshop'",
    )
    .bind(uuid::Uuid::parse_str(&id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(calls, 3);
}

#[tokio::test]
async fn another_gm_cannot_edit_or_decide() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, owner) = common::signed_in_gm(&pool, "Romain").await;
    let (_, other) = common::signed_in_gm(&pool, "Autre").await;
    let id = prepared(&app, &owner).await;
    let r = gm(
        &app,
        &owner,
        "POST",
        &format!("/api/campaigns/{id}/workshop"),
        Some(json!({ "prompt": "Un indice de plus." })),
    )
    .await;
    let pid = r.body["data"]["id"].as_str().unwrap().to_string();
    for (method, path, body) in [
        (
            "POST",
            format!("/api/campaigns/{id}/story/edits"),
            Some(json!({ "edits": [] })),
        ),
        ("POST", format!("/api/campaigns/{id}/story/validate"), None),
        ("GET", format!("/api/campaigns/{id}/workshop"), None),
        (
            "POST",
            format!("/api/campaigns/{id}/workshop/{pid}/accept"),
            None,
        ),
    ] {
        let r = gm(&app, &other, method, &path, body).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{method} {path}");
    }
}
