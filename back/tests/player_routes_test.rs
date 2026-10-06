//! session/project-player-view — done when a test walks every player
//! and shared-screen route.
//!
//! The routes come from the router itself (`app::player_facing_routes`,
//! the list `app::router` mounts), so a route cannot be added without
//! being swept. The campaign is the fixture with every GM-only field
//! marked (`common::marked`), in the world that shows the most; each
//! route is called by a player and by a spectator, must succeed (so the
//! sweep is not vacuous), and must carry no marker, no hidden hit points
//! and no clue or revelation id. A player never sees another player's
//! sheet. Without the device token of a player of that campaign — or
//! with a GM session — every seated route answers 401.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::marked::{ITEM_NOTE, leaks, mark_review, marked, world};
use common::{call, call_as_player, imported_campaign, invite_code, join, send};
use promptus_back::app::player_facing_routes;
use promptus_shared::story::to_yaml;
use serde_json::{Value, json};
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

/// Text only Marc's own sheet holds.
const MARC_SECRET: &str = "OTHERPLAYER<Marc's sheet>";

struct Table {
    gm: String,
    campaign: String,
    code: String,
    marc: String,
    spectator: String,
}

/// Romain's marked campaign mid-scene, Marc seated as a player with a
/// written sheet the GM reviewed and drew a secret hook from, Léa as a
/// spectator.
async fn marked_table(app: &Router, pool: &PgPool) -> Table {
    let (_, gm) = common::signed_in_gm(pool, "Romain").await;
    let story = marked();
    let campaign = imported_campaign(app, &gm, &to_yaml(&story).unwrap()).await;
    sqlx::query("UPDATE campaigns SET world = $2 WHERE id = $1")
        .bind(Uuid::parse_str(&campaign).unwrap())
        .bind(Json(world(&story)))
        .execute(pool)
        .await
        .unwrap();
    let code = invite_code(app, &gm, &campaign).await;
    let marc = join(app, &code, "Marc", "player").await;
    let marc_id = Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    // Returned by the GM with a word, the sweep's character writes may
    // edit and resend it; what the GM keeps about it (the last review, a
    // hook) must not reach the player.
    let character: Uuid = sqlx::query_scalar(
        "UPDATE characters SET sheet = $2, status = 'returned', gm_note = 'Retouche.'
         WHERE player_id = $1 RETURNING id",
    )
    .bind(marc_id)
    .bind(json!({ "name": "Borin", "appearance": MARC_SECRET }))
    .fetch_one(pool)
    .await
    .unwrap();
    mark_review(pool, Uuid::parse_str(&campaign).unwrap(), character).await;
    let spectator = join(app, &code, "Léa", "spectator").await;
    Table {
        gm,
        campaign,
        code,
        marc: marc.player_token().unwrap(),
        spectator: spectator.player_token().unwrap(),
    }
}

fn uri(path: &str, t: &Table) -> String {
    path.replace("{campaign}", &t.campaign)
        .replace("{code}", &t.code)
}

fn seated(path: &str) -> bool {
    path.contains("{campaign}")
}

/// The routes that write the caller's own character: a spectator has
/// none (404 `NO_CHARACTER`).
fn writes_character(method: &str, path: &str) -> bool {
    method != "GET" && path.contains("/character")
}

/// What the sweep sends to a route that takes a body.
fn sweep_body(n: usize, method: &str, path: &str) -> Option<Value> {
    match (method, path) {
        ("POST", p) if p.starts_with("/api/join/") => {
            Some(json!({ "nickname": format!("Sweep {n}"), "role": "player" }))
        }
        // A complete sheet, so the submit route that follows succeeds.
        // The line of Marc's bag `mark_review` stored.
        ("POST", p) if p.ends_with("/equip") => Some(json!({ "entry": "k1", "equipped": true })),
        ("PUT", p) if p.ends_with("/character") => Some(json!({
            "name": "Borin",
            "classId": "bretteur",
            "appearance": MARC_SECRET,
            "look": { "pack": "marins-1718", "body": "robuste", "skin": "hale",
                      "hair": { "style": "court", "colour": "roux" } },
        })),
        _ => None,
    }
}

fn assert_clean(what: &str, body: &Value) {
    let json = body.to_string();
    let found = leaks(&json);
    assert!(found.is_empty(), "{what} leaked GM-only data: {found:#?}");
    assert!(
        !json.contains("\"cl_") && !json.contains("\"rev_"),
        "{what} leaked a clue or revelation id: {json}"
    );
}

#[tokio::test]
async fn no_player_route_leaks_what_only_the_gm_may_see() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let t = marked_table(&app, &pool).await;
    let routes = player_facing_routes();
    assert!(routes.len() >= 4, "{routes:?}");

    for (n, (method, path)) in routes.iter().enumerate() {
        let uri = uri(path, &t);
        let body = sweep_body(n, method, path);
        for (who, token) in [("Marc", &t.marc), ("Léa", &t.spectator)] {
            // Joining twice under one nickname is refused: one join is
            // enough to sweep.
            if *method == "POST" && !seated(path) && who == "Léa" {
                continue;
            }
            // Carrying an item is for a character in play: the GM
            // validates the sheet the sweep just sent.
            if path.ends_with("/equip") && who == "Marc" {
                sqlx::query(
                    "UPDATE characters SET status = 'validated'
                     WHERE player_id = (SELECT id FROM players WHERE nickname = 'Marc'
                                        AND campaign_id = $1)",
                )
                .bind(Uuid::parse_str(&t.campaign).unwrap())
                .execute(&pool)
                .await
                .unwrap();
            }
            let r = call_as_player(&app, Some(token), method, &uri, body.clone()).await;
            // The live socket carries versions and presence only
            // (`live_test.rs`); over plain HTTP, getting past the guard
            // to the upgrade check is what this sweep can see.
            if path.ends_with("/live") {
                assert_eq!(r.status, StatusCode::BAD_REQUEST, "{uri} as {who}");
                assert_eq!(r.body["error"]["code"], "WEBSOCKET_REQUIRED");
                continue;
            }
            if who == "Léa" && writes_character(method, path) {
                assert_eq!(r.status, StatusCode::NOT_FOUND, "{method} {uri} as Léa");
                assert_eq!(r.body["error"]["code"], "NO_CHARACTER");
                continue;
            }
            assert!(
                r.status.is_success(),
                "{method} {uri} as {who}: {} {}",
                r.status,
                r.body
            );
            assert_clean(&format!("{method} {uri} as {who}"), &r.body);
            if who == "Léa" {
                assert!(
                    !r.body.to_string().contains(MARC_SECRET),
                    "{method} {uri} showed Marc's sheet to Léa"
                );
            }
        }
    }

    // What the routes are for is there: Marc reads his own sheet, the
    // view names the scene.
    let r = call_as_player(
        &app,
        Some(&t.marc),
        "GET",
        &format!("/api/play/{}/me", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["character"]["sheet"]["name"], "Borin");
    // Validated for the equip route: in play, his bag reaches him, the
    // GM's note on an item and the history of adjustments do not.
    assert_eq!(r.body["data"]["character"]["status"], "validated");
    assert_clean("Marc's character in play", &r.body);
    let play = &r.body["data"]["character"]["play"];
    assert_eq!(
        play["inventory"][0]["name"], "Épée de bonne facture",
        "{play}"
    );
    assert_eq!(play["inventory"][0]["equipped"], true, "{play}");
    assert!(!r.body.to_string().contains(ITEM_NOTE), "{play}");
    let r = call_as_player(
        &app,
        Some(&t.marc),
        "GET",
        &format!("/api/play/{}/view", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["scene"]["title"], "La crique aux Morts");
}

#[tokio::test]
async fn seated_routes_refuse_whoever_has_no_seat_at_that_table() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let t = marked_table(&app, &pool).await;
    // A seat at another table of the same GM.
    let other = imported_campaign(&app, &t.gm, common::marked::FIXTURE).await;
    let code = invite_code(&app, &t.gm, &other).await;
    let stranger = join(&app, &code, "Hugo", "player")
        .await
        .player_token()
        .unwrap();

    let cookies = [
        None,
        Some("promptus_player=made-up-token".to_string()),
        Some(format!("promptus_player={stranger}")),
        // The GM's own session is not a seat.
        Some(format!("promptus_gm={}", t.gm)),
        Some(format!("promptus_gm={}", t.marc)),
    ];
    for (method, path) in player_facing_routes() {
        if !seated(path) {
            continue;
        }
        let uri = uri(path, &t);
        for cookie in &cookies {
            let r = send(&app, cookie.as_deref(), method, &uri, None).await;
            assert_eq!(
                r.status,
                StatusCode::UNAUTHORIZED,
                "{method} {uri} with {cookie:?}"
            );
            assert_eq!(r.body["error"]["code"], "NOT_JOINED");
        }
    }
    // The GM previews the same projection on their own route.
    let r = call(
        &app,
        Some(&t.gm),
        "GET",
        &format!("/api/campaigns/{}/player-view", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_clean("the GM's preview", &r.body);
}
