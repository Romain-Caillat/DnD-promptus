//! campaign/edit-scenes-in-one-place — the scene sheet sets a scene's
//! fight, sound, checks and map through the story edits; the server
//! refuses an opponent, an exit or a stat that names nothing, and the
//! readiness gauge plays a fight on the campaign's own validated map.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call};
use serde_json::{Value, json};

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

async fn gm(app: &Router, token: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
    call(app, Some(token), method, path, body).await
}

async fn imported(app: &Router, token: &str, yaml: &str) -> String {
    let r = gm(
        app,
        token,
        "POST",
        "/api/campaigns/import",
        Some(json!({ "yaml": yaml })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["data"]["id"].as_str().unwrap().to_string()
}

fn node<'a>(campaign: &'a Value, id: &str) -> &'a Value {
    campaign["story"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()
}

#[tokio::test]
async fn the_gm_prepares_a_fight_and_its_sound_and_the_server_refuses_what_names_nothing() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let id = imported(&app, &token, CORSAIRES).await;
    let edits = format!("/api/campaigns/{id}/story/edits");
    let r = gm(&app, &token, "GET", &format!("/api/campaigns/{id}"), None).await;
    let quay = node(&r.body["data"], "sc_quai").clone();
    let interception = node(&r.body["data"], "sc_interception_greyhound").clone();

    // The fight against Gueule-Rouge, one sailor less, and its music.
    let mut fight = quay["encounter"].clone();
    fight["opponents"][1]["count"] = json!(3);
    fight["tactics"] = json!(["Ils encerclent le plus isolé."]);
    let r = gm(
        &app,
        &token,
        "POST",
        &edits,
        Some(json!({ "edits": [
            { "op": "set", "target": "sc_quai", "field": "encounter", "value": fight },
            { "op": "set", "target": "sc_quai", "field": "ambience.music", "value": [
                { "mood": "combat", "title": "Abordage", "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ" },
                { "mood": "calm", "title": "Port au matin", "search": "harbour ambience 18th century" },
            ]},
        ]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let stored = node(&r.body["data"]["campaign"], "sc_quai");
    assert_eq!(stored["encounter"]["opponents"][1]["count"], 3);
    assert_eq!(stored["encounter"]["opponents"][2]["who"], "pnj_fanch");
    assert_eq!(
        stored["ambience"]["music"][1]["search"],
        "harbour ambience 18th century"
    );

    // An adversary the campaign does not have: refused, nothing saved.
    let mut invented = quay["encounter"].clone();
    invented["opponents"] = json!([{ "who": "adv_kraken", "count": 2 }]);
    let r = gm(
        &app,
        &token,
        "POST",
        &edits,
        Some(json!({ "edits": [
            { "op": "set", "target": "sc_quai", "field": "summary", "value": "Changé." },
            { "op": "set", "target": "sc_quai", "field": "encounter", "value": invented },
        ]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "EDIT_SCENE_INVALID");
    let r = gm(&app, &token, "GET", &format!("/api/campaigns/{id}"), None).await;
    assert_eq!(node(&r.body["data"], "sc_quai")["summary"], quay["summary"]);

    // A check on a stat the Corsaires' rules do not have, an exit to nowhere.
    for (field, value) in [
        (
            "checks",
            json!([{ "action": "Grimper au mât", "stat": "AGI", "difficulty": 10 }]),
        ),
        ("exits", json!([{ "to": "sc_nulle", "label": "Ailleurs" }])),
    ] {
        let r = gm(
            &app,
            &token,
            "POST",
            &edits,
            Some(json!({ "edits": [{ "op": "set", "target": "sc_quai", "field": field, "value": value }] })),
        )
        .await;
        assert_eq!(r.body["error"]["code"], "EDIT_SCENE_INVALID", "{field}");
    }

    // The ship battle keeps its ships when the deck fight changes.
    let mut boarding = interception["encounter"].clone();
    boarding["on_victory"] = json!("Le Greyhound est pris.");
    let r = gm(
        &app,
        &token,
        "POST",
        &edits,
        Some(json!({ "edits": [
            { "op": "set", "target": "sc_interception_greyhound", "field": "encounter", "value": boarding },
        ]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let stored = node(&r.body["data"]["campaign"], "sc_interception_greyhound");
    assert_eq!(
        stored["encounter"]["vehicles"],
        interception["encounter"]["vehicles"]
    );
}

#[tokio::test]
async fn a_scene_played_on_the_campaigns_own_map_is_staged_once_the_map_is_validated() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let id = imported(&app, &token, CORSAIRES).await;
    let fights = |r: &Reply| -> Vec<String> {
        r.body["data"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|a| a["checks"].as_array().unwrap().clone())
            .filter(|c| c["kind"] == "fights")
            .flat_map(|c| c["gaps"].as_array().unwrap().clone())
            .filter(|g| g["id"] == "sc_quai")
            .map(|g| g["detail"].as_str().unwrap().to_string())
            .collect()
    };

    // The quay, copied into a map of the campaign and linked to the scene.
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("/api/campaigns/{id}/maps"),
        Some(json!({ "name": "Quai retouché", "copy": "quai-port-louis", "node": "sc_quai" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let map = r.body["data"]["map"]["id"].as_str().unwrap().to_string();
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("/api/campaigns/{id}/story/edits"),
        Some(json!({ "edits": [{ "op": "set", "target": "sc_quai", "field": "map", "value": map }] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    // A draft map is not played: the fight cannot be staged.
    let r = gm(
        &app,
        &token,
        "GET",
        &format!("/api/campaigns/{id}/readiness"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(
        fights(&r).iter().any(|d| d.contains("map is not known")),
        "{:?}",
        fights(&r)
    );

    // Validated, it is.
    let r = gm(
        &app,
        &token,
        "POST",
        &format!("/api/campaigns/{id}/maps/{map}/validate"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = gm(
        &app,
        &token,
        "GET",
        &format!("/api/campaigns/{id}/readiness"),
        None,
    )
    .await;
    assert!(
        fights(&r).iter().all(|d| !d.contains("map is not known")),
        "{:?}",
        fights(&r)
    );
}

#[tokio::test]
async fn the_brasier_boarding_keeps_its_ships_and_refuses_an_unknown_opponent() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let id = imported(&app, &token, BRASIER).await;
    let r = gm(&app, &token, "GET", &format!("/api/campaigns/{id}"), None).await;
    let boarding = r.body["data"]["story"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| !n["encounter"]["vehicles"].is_null())
        .expect("the Brasier has a ship battle")
        .clone();
    let target = boarding["id"].as_str().unwrap();
    let edits = format!("/api/campaigns/{id}/story/edits");

    let mut fight = boarding["encounter"].clone();
    fight["opponents"][0]["count"] = json!(5);
    let r = gm(
        &app,
        &token,
        "POST",
        &edits,
        Some(json!({ "edits": [{ "op": "set", "target": target, "field": "encounter", "value": fight }] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let stored = node(&r.body["data"]["campaign"], target);
    assert_eq!(stored["encounter"]["opponents"][0]["count"], 5);
    assert_eq!(
        stored["encounter"]["vehicles"],
        boarding["encounter"]["vehicles"]
    );

    fight["opponents"][0]["who"] = json!("adv_inconnu");
    let r = gm(
        &app,
        &token,
        "POST",
        &edits,
        Some(json!({ "edits": [{ "op": "set", "target": target, "field": "encounter", "value": fight }] })),
    )
    .await;
    assert_eq!(r.body["error"]["code"], "EDIT_SCENE_INVALID");
}
