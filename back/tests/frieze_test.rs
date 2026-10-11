//! campaign/order-maps-by-story — the atlas carries the story frieze: per
//! act, the maps its scenes cross in the order of the story, a place with
//! no map of its own as a map to create; creating that map and pinning
//! the place on it puts it in the frieze on the next read.

mod common;

use axum::http::StatusCode;
use common::{call, imported_campaign};
use serde_json::{Value, json};

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");

/// Each slot of `act` as its stops: the map's id, or `!place` to create.
fn slots(atlas: &Value, act: usize) -> Vec<Vec<String>> {
    atlas["frieze"]["acts"][act]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            s["stops"]
                .as_array()
                .unwrap()
                .iter()
                .map(|st| match st["map"].as_str() {
                    Some(m) => m.to_string(),
                    None => format!("!{}", st["place"].as_str().unwrap()),
                })
                .collect()
        })
        .collect()
}

#[tokio::test]
async fn a_place_without_a_map_is_to_create_until_its_map_is_made() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, CORSAIRES).await;
    let edits = |edits: Value| {
        let (app, gm, campaign) = (app.clone(), gm.clone(), campaign.clone());
        async move {
            let r = call(
                &app,
                Some(&gm),
                "POST",
                &format!("/api/campaigns/{campaign}/story/edits"),
                Some(json!({ "edits": edits })),
            )
            .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        }
    };
    let atlas = || {
        let (app, gm, campaign) = (app.clone(), gm.clone(), campaign.clone());
        async move {
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
        }
    };

    // As in production: the tavern is pinned nowhere, Port-Louis only on
    // the region — the tavern's scenes open act 1 on a map to create.
    edits(json!([{ "op": "set", "target": "lieu_perroquet", "field": "anchor", "value": null }]))
        .await;
    let before = atlas().await;
    assert_eq!(before["frieze"]["acts"][0]["evening"], 1);
    assert_eq!(
        before["frieze"]["acts"][0]["title"],
        "L'Offre qu'on ne refuse pas"
    );
    let act1 = slots(&before, 0);
    assert_eq!(act1[0], ["!lieu_perroquet"]);
    assert_eq!(act1[1], ["port-louis-rues"]);
    assert_eq!(
        before["frieze"]["acts"][0]["slots"][0]["stops"][0]["name"],
        "Le Perroquet Noyé"
    );
    // Anaïs's errand leads back to the tavern: inside the branch.
    assert!(
        act1[1..]
            .iter()
            .any(|s| s.contains(&"!lieu_perroquet".to_string()))
    );
    assert_eq!(slots(&before, 1), [["pont-du-greyhound"]]);
    let outside = before["frieze"]["outside"].as_array().unwrap();
    assert!(outside.contains(&json!("le-palais")), "{outside:?}");
    assert!(!outside.contains(&json!("port-louis-rues")));

    // The GM makes the tavern's map from its slot: a blank map, and the
    // place pinned on it.
    let made = call(
        &app,
        Some(&gm),
        "POST",
        &format!("/api/campaigns/{campaign}/maps"),
        Some(json!({ "name": "Le Perroquet Noyé" })),
    )
    .await;
    assert_eq!(made.status, StatusCode::CREATED, "{}", made.body);
    let tavern = made.body["data"]["map"]["id"].as_str().unwrap().to_string();
    // Not crossed yet: outside the story.
    assert!(
        atlas().await["frieze"]["outside"]
            .as_array()
            .unwrap()
            .contains(&json!(tavern))
    );
    edits(json!([{ "op": "set", "target": "lieu_perroquet", "field": "anchor", "value": { "map": tavern, "at": [1, 1] } }])).await;
    let after = atlas().await;
    let act1 = slots(&after, 0);
    assert_eq!(act1[0], std::slice::from_ref(&tavern));
    assert!(
        act1.iter()
            .all(|s| !s.contains(&"!lieu_perroquet".to_string()))
    );
    assert!(
        !after["frieze"]["outside"]
            .as_array()
            .unwrap()
            .contains(&json!(tavern))
    );
}
