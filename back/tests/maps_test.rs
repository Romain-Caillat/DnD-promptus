//! The campaign's own maps, over the API: maps/generate-map-llm,
//! maps/edit-map-gm, maps/import-image-map — drafted, edited, validated
//! by the GM, then shown at the table without their secrets.

mod common;

use axum::Router;
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use promptus_back::ai::Ai;
use promptus_back::live::{LiveConfig, LiveHub};
use serde_json::{Value, json};
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");

struct Table {
    app: Router,
    gm: String,
    campaign: String,
    marc: String,
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn player(&self, path: &str) -> Reply {
        let uri = format!("/api/play/{}{path}", self.campaign);
        call_as_player(&self.app, Some(&self.marc), "GET", &uri, None).await
    }
}

async fn table() -> Table {
    let pool = common::test_pool().await;
    let app = common::app_with_ai(
        pool.clone(),
        LiveHub::new(LiveConfig::default()),
        Ai::fake(),
    );
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, CORSAIRES).await;
    sqlx::query("UPDATE campaigns SET ai_budget_cents = 100 WHERE id = $1")
        .bind(Uuid::parse_str(&campaign).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player")
        .await
        .player_token()
        .unwrap();
    Table {
        app,
        gm,
        campaign,
        marc,
    }
}

fn png() -> String {
    STANDARD.encode(b"\x89PNG\r\n\x1a\nnot really an image")
}

#[tokio::test]
async fn a_generated_map_is_a_draft_until_the_gm_validates_it_and_keeps_its_secrets() {
    let t = table().await;

    let r = t.gm("GET", "/maps", None).await;
    assert_eq!(r.body["data"]["maps"], json!([]));
    assert!(
        r.body["data"]["world"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "quai-port-louis")
    );

    let r = t
        .gm(
            "POST",
            "/maps/generate",
            Some(json!({ "node": "sc_inconnue" })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "UNKNOWN_NODE");
    let r = t
        .gm("POST", "/maps/generate", Some(json!({ "node": "sc_quai" })))
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let drafted = r.body["data"].clone();
    let id = drafted["map"]["id"].as_str().unwrap().to_string();
    assert_eq!(drafted["source"], "generated");
    assert_eq!(drafted["node"], "sc_quai");
    assert_eq!(drafted["validatedAt"], Value::Null);
    assert_eq!(drafted["map"]["theme"], "port-1718");
    // The adversary the model invented lost its link; the real ones kept it.
    assert_eq!(drafted["dropped"], 1);
    let entities: Vec<&str> = drafted["map"]["starts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["entity"].as_str())
        .collect();
    assert_eq!(
        entities,
        ["gueule-rouge", "marin-de-gueule-rouge", "pnj_fanch"]
    );

    // The editor saves the whole map; a door off the wall is refused in
    // the GM's words.
    let mut map = drafted["map"].clone();
    map["name"] = json!("La réserve du quai");
    map["doors"][0]["state"] = json!("locked");
    let r = t.gm("PUT", &format!("/maps/{id}"), Some(map.clone())).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["map"]["name"], "La réserve du quai");
    let mut off = map.clone();
    off["doors"][0]["at"] = json!([3, 2]);
    let r = t.gm("PUT", &format!("/maps/{id}"), Some(off)).await;
    assert_eq!(r.body["error"]["code"], "MAP_INVALID");
    assert!(
        r.body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("n’est pas sur un mur"),
        "{}",
        r.body
    );

    // Not validated, it cannot reach the table.
    t.gm("POST", "/session", None).await;
    t.gm("POST", "/session/start", None).await;
    let r = t.gm("POST", "/board", Some(json!({ "map": id }))).await;
    assert_eq!(r.body["error"]["code"], "UNKNOWN_MAP", "{}", r.body);
    let r = t.gm("POST", &format!("/maps/{id}/validate"), None).await;
    assert_ne!(r.body["data"]["validatedAt"], Value::Null);
    let choices = t.gm("GET", "/board", None).await.body["data"]["maps"].clone();
    assert_eq!(choices[0]["id"], id.as_str(), "{choices}");
    let r = t.gm("POST", "/board", Some(json!({ "map": id }))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    // The table gets the map without the trapdoor, its check or the notes.
    let view = t.player("/board").await.body.to_string();
    assert!(view.contains("La réserve du quai"), "{view}");
    for secret in ["trappe", "Un passage vers la cave", "attendent derrière"] {
        assert!(!view.contains(secret), "leaked {secret}: {view}");
    }
    let r = t.gm("DELETE", &format!("/maps/{id}"), None).await;
    assert_eq!(r.body["error"]["code"], "MAP_ON_THE_TABLE");

    // Saving again withdraws the validation.
    let r = t.gm("PUT", &format!("/maps/{id}"), Some(map)).await;
    assert_eq!(r.body["data"]["validatedAt"], Value::Null);
}

#[tokio::test]
async fn a_dungeondraft_file_and_a_plain_image_become_maps_behind_their_grid() {
    let t = table().await;
    let file = json!({
        "format": 0.3,
        "resolution": {
            "map_origin": { "x": 0, "y": 0 },
            "map_size": { "x": 6, "y": 4 },
            "pixels_per_grid": 128
        },
        "line_of_sight": [[{ "x": 3, "y": 0 }, { "x": 3, "y": 4 }]],
        "portals": [{ "position": { "x": 3, "y": 2.5 }, "closed": true }],
        "lights": [],
        "image": png()
    })
    .to_string();
    let r = t
        .gm(
            "POST",
            "/maps/import",
            Some(json!({ "kind": "uvtt", "name": "Les caves", "file": file })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let caves = r.body["data"].clone();
    assert_eq!(caves["map"]["id"], "les-caves");
    assert_eq!(caves["backdrop"], true);
    assert_eq!(
        caves["map"]["grid"]["rows"],
        json!(["..#...", "..#...", "..#...", "..#..."])
    );
    assert_eq!(caves["map"]["doors"][0]["at"], json!([2, 2]));
    let r = t.gm("GET", "/maps/les-caves/backdrop", None).await;
    assert_eq!(r.status, StatusCode::OK);

    let r = t
        .gm(
            "POST",
            "/maps/import",
            Some(json!({
                "kind": "image", "name": "Les caves", "image": format!("data:image/png;base64,{}", png()),
                "cellPx": 70, "offsetX": 12, "offsetY": 5, "columns": 9, "rows": 7
            })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let plain = &r.body["data"]["map"];
    assert_eq!(plain["id"], "les-caves-2");
    assert_eq!(plain["grid"]["rows"][0], ".........");
    assert_eq!(plain["grid"]["rows"].as_array().unwrap().len(), 7);
    assert_eq!(plain["backdrop"]["cell_px"], 70.0);
    assert_eq!(plain["backdrop"]["offset"], json!([12.0, 5.0]));
    let r = t
        .gm(
            "POST",
            "/maps/import",
            Some(json!({
                "kind": "image", "name": "Piège", "image": STANDARD.encode(b"<svg/>"),
                "cellPx": 70, "offsetX": 0, "offsetY": 0, "columns": 9, "rows": 7
            })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "BAD_IMAGE");

    // Shown, its image reaches the table; nothing before.
    t.gm("POST", "/session", None).await;
    t.gm("POST", "/session/start", None).await;
    assert_eq!(
        t.player("/board/backdrop").await.status,
        StatusCode::NOT_FOUND
    );
    t.gm("POST", "/maps/les-caves/validate", None).await;
    let r = t
        .gm("POST", "/board", Some(json!({ "map": "les-caves" })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(t.player("/board/backdrop").await.status, StatusCode::OK);

    // A copy of the world's quay, to touch up.
    let r = t
        .gm(
            "POST",
            "/maps",
            Some(json!({ "name": "Quai de nuit", "copy": "quai-port-louis" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["data"]["map"]["id"], "quai-de-nuit");
    assert_eq!(r.body["data"]["map"]["doors"].as_array().unwrap().len(), 3);
    let r = t
        .gm(
            "POST",
            "/maps",
            Some(json!({ "name": "Vide", "width": 2, "height": 9 })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "BAD_SIZE");
}
