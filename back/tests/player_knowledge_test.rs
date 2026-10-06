//! campaign/check-player-knowledge — done when, on act 1 of the
//! Corsaires as Romain played it, the tool says what the table was
//! missing for act 2: at review time, at the end of the evening, and in
//! the feedback after it.

mod common;

use axum::http::StatusCode;
use common::{call, imported_campaign};
use serde_json::{Value, json};

const PLAYED: &str = include_str!("../../content/fixtures/corsaires-acte-1-joue.yaml");

/// The revelations missing to enter the act 2 transition, sorted.
fn missing(gaps: &Value) -> Vec<String> {
    let mut out: Vec<String> = gaps
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["node"] == "sc_depart")
        .map(|g| g["revelation"].as_str().unwrap().to_string())
        .collect();
    out.sort();
    out
}

#[tokio::test]
async fn the_played_corsaires_act_says_what_act_two_needed() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &token, PLAYED).await;
    let gm = |method: &'static str, path: String, body: Option<Value>| {
        let app = app.clone();
        let token = token.clone();
        async move { call(&app, Some(&token), method, &path, body).await }
    };
    let base = format!("/api/campaigns/{campaign}");

    // At review: what the transition needs is only in optional scenes.
    let r = gm("GET", base.clone(), None).await;
    let only_optional: Vec<&Value> = r.body["data"]["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["code"] == "KNOWLEDGE_ONLY_OPTIONAL")
        .collect();
    assert_eq!(only_optional.len(), 3, "{}", r.body["data"]["issues"]);

    // The evening as played: the tavern, the offer, the black market,
    // the quay — never the harbour master's office nor the shipyard.
    let r = gm("POST", format!("{base}/session"), None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let r = gm("POST", format!("{base}/session/start"), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    for node in [
        "sc_perroquet",
        "sc_proposition",
        "sc_marche_noir",
        "sc_quai",
    ] {
        let r = gm(
            "POST",
            format!("{base}/session/reveal"),
            Some(json!({ "kind": "scene", "node": node })),
        )
        .await;
        assert!(r.status.is_success(), "{node}: {}", r.body);
    }

    // Before ending, the GM sees what act 2 needs and where it was.
    let screen = gm("GET", format!("{base}/session"), None).await;
    let gaps = &screen.body["data"]["gaps"];
    assert_eq!(
        missing(gaps),
        ["rev_escorte", "rev_reparation", "rev_route"]
    );
    let route = gaps
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["revelation"] == "rev_route")
        .unwrap();
    assert_eq!(route["nodeTitle"], "Transition vers l'acte 2");
    assert_eq!(route["clues"][0]["node"], "sc_bureau_morel");

    // The GM slips it into « Précédemment… » and ends; the feedback keeps
    // what was missing at the end.
    let r = gm(
        "POST",
        format!("{base}/session/end"),
        Some(json!({
            "recap": "Le quai, puis l'appareillage.",
            "previously": format!("À retenir : {}", route["statement"].as_str().unwrap()),
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let session = r.body["data"]["id"].as_str().unwrap().to_string();
    let r = gm("GET", format!("{base}/sessions/{session}/feedback"), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(
        missing(&r.body["data"]["gaps"]),
        ["rev_escorte", "rev_reparation", "rev_route"]
    );
}
