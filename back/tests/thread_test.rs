//! campaign/draw-story-thread — the atlas carries, for each map, the
//! thread of the scenes' exits between its pins: an exit added in the
//! scene editor (a story edit) is a new segment on the next read, and an
//! exit between two places no path joins is warned.

mod common;

use axum::http::StatusCode;
use common::{call, imported_campaign};
use serde_json::{Value, json};

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");

/// (from place, to place, steps) of each segment of `map`'s thread.
fn segments(atlas: &Value, map: &str) -> Vec<(String, String, usize)> {
    atlas["maps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["map"]["id"] == map)
        .unwrap_or_else(|| panic!("no map {map}"))["thread"]["segments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["from"]["place"].as_str().unwrap().to_string(),
                s["to"]["place"].as_str().unwrap().to_string(),
                s["steps"].as_array().unwrap().len(),
            )
        })
        .collect()
}

fn has(segments: &[(String, String, usize)], from: &str, to: &str) -> bool {
    segments.iter().any(|(f, t, _)| f == from && t == to)
}

#[tokio::test]
async fn an_exit_added_in_the_scene_editor_draws_a_new_segment() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, CORSAIRES).await;
    let atlas = |app: axum::Router, gm: String, campaign: String| async move {
        let r = call(
            &app,
            Some(&gm),
            "GET",
            &format!("/api/campaigns/{campaign}/atlas"),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    };

    let before = atlas(app.clone(), gm.clone(), campaign.clone()).await;
    // In Port-Louis, the night in the port branches to the black market,
    // the shipyard, the harbour office and the quay: four segments.
    let town = segments(&before, "port-louis-rues");
    for to in [
        "lieu_marche_noir",
        "lieu_chantier",
        "lieu_bureau_port",
        "lieu_quai",
    ] {
        assert!(has(&town, "lieu_port", to), "lieu_port > {to}: {town:?}");
    }
    // On the region, act 1 leaves Port-Louis for Belle-Île.
    assert!(has(
        &segments(&before, "cotes-bretagne-sud"),
        "lieu_port_louis",
        "lieu_belle_ile"
    ));
    assert!(!has(&town, "lieu_perroquet", "lieu_chantier"));
    assert_eq!(before["acts"][0]["id"], "acte_1");
    let stops = before["maps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["map"]["id"] == "port-louis-rues")
        .unwrap()["thread"]["stops"]
        .as_array()
        .unwrap()
        .clone();
    let first = stops.iter().find(|s| s["scene"] == "sc_perroquet").unwrap();
    assert_eq!(
        (first["start"].clone(), first["place"].clone()),
        (json!(true), json!("lieu_perroquet"))
    );

    // The scene editor saves Anaïs's exits with one more, to the shipyard.
    let edit = json!({ "edits": [{ "op": "set", "target": "sc_anais", "field": "exits", "value": [
        { "to": "sc_quai", "label": "Au quai" },
        { "to": "sc_chantier", "label": "Au chantier" }
    ] }] });
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("/api/campaigns/{campaign}/story/edits"),
        Some(edit),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let after = atlas(app.clone(), gm.clone(), campaign.clone()).await;
    let now = segments(&after, "port-louis-rues");
    assert_eq!(now.len(), town.len() + 1);
    assert!(has(&now, "lieu_perroquet", "lieu_chantier"));
    assert!(after["revision"].as_i64() > before["revision"].as_i64());
    assert!(
        after["issues"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["code"] != "EXIT_WITHOUT_PATH"),
        "{}",
        after["issues"]
    );

    // The shipyard moved to a map no exit joins to Port-Louis, out of the
    // town: the exits towards it, and from it, are warned.
    let blank = call(
        &app,
        Some(&gm),
        "POST",
        &format!("/api/campaigns/{campaign}/maps"),
        Some(json!({ "name": "Île perdue" })),
    )
    .await;
    assert_eq!(blank.status, StatusCode::CREATED, "{}", blank.body);
    let island = blank.body["data"]["map"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let edit = json!({ "edits": [
        { "op": "set", "target": "lieu_chantier", "field": "parent", "value": null },
        { "op": "set", "target": "lieu_chantier", "field": "anchor", "value": { "map": island, "at": [1, 1] } }
    ] });
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("/api/campaigns/{campaign}/story/edits"),
        Some(edit),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let cut = atlas(app, gm, campaign).await;
    let warned: Vec<&str> = cut["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["code"] == "EXIT_WITHOUT_PATH")
        .map(|i| i["path"].as_str().unwrap())
        .collect();
    assert!(!warned.is_empty(), "{}", cut["issues"]);
    assert!(warned.iter().all(|p| p.contains(".exits[")), "{warned:?}");
}
