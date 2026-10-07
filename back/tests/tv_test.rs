//! The shared screen, over the API: session/pair-shared-screen (a TV
//! shows a code, the GM types it, the TV collects its seat once; a
//! window paired at once; forgetting a screen), gm/launch-session (the
//! reading of « Précédemment… » the screen follows).

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, send, send_raw};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

struct Table {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
}

async fn table() -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    Table {
        app,
        pool,
        gm,
        campaign,
    }
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn ok(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        let r = self.gm(method, path, body).await;
        assert!(r.status.is_success(), "{method} {path}: {}", r.body);
        r.body["data"].clone()
    }

    async fn screen(&self, token: &str, path: &str) -> Reply {
        let uri = format!("/api/play/{}{path}", self.campaign);
        call_as_player(&self.app, Some(token), "GET", &uri, None).await
    }
}

async fn waiting_tv(app: &Router) -> (String, String) {
    let r = send(app, None, "POST", "/api/tv/pairings", None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    (
        r.body["data"]["code"].as_str().unwrap().to_string(),
        r.body["data"]["secret"].as_str().unwrap().to_string(),
    )
}

#[tokio::test]
async fn a_tv_is_paired_by_its_code_and_watches_as_a_spectator() {
    let t = table().await;
    let (code, secret) = waiting_tv(&t.app).await;
    assert_eq!(code.len(), 4);

    // It waits, and shows a QR leading the GM to the pairing page.
    let poll = format!("/api/tv/pairings/{secret}");
    let r = send(&t.app, None, "GET", &poll, None).await;
    assert_eq!(r.body["data"], json!({ "state": "waiting", "code": code }));
    assert!(r.set_cookie.is_none());
    let (status, svg) = send_raw(&t.app, None, "GET", &format!("{poll}/qr.svg")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(svg.contains("</svg>"), "{svg}");

    // A wrong code pairs nothing; another GM cannot pair onto Romain's
    // table.
    let r = t.gm("POST", "/tv", Some(json!({ "code": "ZZ00" }))).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "TV_CODE_UNKNOWN");
    let (_, marc) = common::signed_in_gm(&t.pool, "Marc").await;
    let uri = format!("/api/campaigns/{}/tv", t.campaign);
    let r = call(
        &t.app,
        Some(&marc),
        "POST",
        &uri,
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "NOT_FOUND");
    let r = call(&t.app, Some(&marc), "GET", &uri, None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // Romain types it, lower case and spaced as read from the sofa.
    let listed = t
        .ok(
            "POST",
            "/tv",
            Some(json!({ "code": format!(" {} ", code.to_lowercase()) })),
        )
        .await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["name"], "TV");

    // The TV collects its seat once.
    let r = send(&t.app, None, "GET", &poll, None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["state"], "paired");
    assert_eq!(r.body["data"]["campaign"], t.campaign.as_str());
    let token = r.player_token().expect("the seat's cookie");
    let r = send(&t.app, None, "GET", &poll, None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "TV_PAIRING_UNKNOWN");
    // The code is spent.
    let r = t.gm("POST", "/tv", Some(json!({ "code": code }))).await;
    assert_eq!(r.body["error"]["code"], "TV_CODE_UNKNOWN");

    // Seated as a spectator: no character, no request, no fight turn.
    let r = t.screen(&token, "/me").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["me"]["role"], "spectator");
    assert_eq!(r.body["data"]["me"]["nickname"], "TV");
    let r = t.screen(&token, "/evening").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["data"]["cards"], json!([]));

    // A window to share on Discord is paired at once, under another name.
    let secret = t.ok("POST", "/tv/window", None).await["secret"]
        .as_str()
        .unwrap()
        .to_string();
    let r = send(
        &t.app,
        None,
        "GET",
        &format!("/api/tv/pairings/{secret}"),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["state"], "paired");
    let window = r.player_token().unwrap();
    let screens = t.ok("GET", "/tv", None).await;
    let names: Vec<&str> = screens
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["TV", "TV 2"]);

    // Forgotten, the TV's token stops working; the window's still does.
    let id = screens[0]["id"].as_str().unwrap();
    let r = call(&t.app, Some(&marc), "DELETE", &format!("{uri}/{id}"), None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = t.gm("DELETE", &format!("/tv/{id}"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(
        t.screen(&token, "/me").await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(t.screen(&window, "/me").await.status, StatusCode::OK);
    // Only screens can be forgotten this way: not a player's seat.
    let player: Uuid = sqlx::query_scalar(
        "INSERT INTO players (campaign_id, nickname, role, token_hash)
         VALUES ($1, 'Marc', 'player', gen_random_uuid()::text) RETURNING id",
    )
    .bind(Uuid::parse_str(&t.campaign).unwrap())
    .fetch_one(&t.pool)
    .await
    .unwrap();
    let r = t.gm("DELETE", &format!("/tv/{player}"), None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_unknown_secret_is_refused() {
    let t = table().await;
    let r = send(&t.app, None, "GET", "/api/tv/pairings/made-up", None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "TV_PAIRING_UNKNOWN");
    let (status, _) = send_raw(&t.app, None, "GET", "/api/tv/pairings/made-up/qr.svg").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // A screen only ever watches.
    let refused = sqlx::query(
        "INSERT INTO players (campaign_id, nickname, role, token_hash, screen)
         VALUES ($1, 'TV', 'player', gen_random_uuid()::text, true)",
    )
    .bind(Uuid::parse_str(&t.campaign).unwrap())
    .execute(&t.pool)
    .await;
    let refused = refused.unwrap_err().to_string();
    assert!(refused.contains("players_screen_check"), "{refused}");
}

#[tokio::test]
async fn the_screen_follows_the_reading_of_previously() {
    let t = table().await;
    let secret = t.ok("POST", "/tv/window", None).await["secret"]
        .as_str()
        .unwrap()
        .to_string();
    let r = send(
        &t.app,
        None,
        "GET",
        &format!("/api/tv/pairings/{secret}"),
        None,
    )
    .await;
    let tv = r.player_token().unwrap();

    // No session: nothing to read.
    let r = t
        .gm("POST", "/session/reading", Some(json!({ "line": 1 })))
        .await;
    assert_eq!(r.body["error"]["code"], "NO_SESSION");

    // Session 1 ends; its « Précédemment… » is published.
    t.ok("POST", "/session", None).await;
    t.ok("POST", "/session/start", None).await;
    t.ok(
        "POST",
        "/session/end",
        Some(json!({
            "recap": "Ils ont trouvé la lanterne.",
            "previously": "Les corsaires ont accosté. Dorn a menti !\nLa tempête approche."
        })),
    )
    .await;
    // Unpublished, nothing is read.
    t.ok("POST", "/session", None).await;
    let r = t
        .gm("POST", "/session/reading", Some(json!({ "line": 1 })))
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "NO_PREVIOUSLY");
    sqlx::query(
        "UPDATE game_sessions SET recap_status = 'published', published_at = now()
         WHERE campaign_id = $1 AND status = 'ended'",
    )
    .bind(Uuid::parse_str(&t.campaign).unwrap())
    .execute(&t.pool)
    .await
    .unwrap();

    let reading = |body: &Value| body["data"]["reading"].clone();
    assert_eq!(reading(&t.screen(&tv, "/evening").await.body), Value::Null);

    // Romain starts reading: line by line, as he reads.
    t.ok("POST", "/session/reading", Some(json!({ "line": 0 })))
        .await;
    let r = t.screen(&tv, "/evening").await;
    assert_eq!(
        reading(&r.body),
        json!({
            "lines": ["Les corsaires ont accosté. Dorn a menti !", "La tempête approche."],
            "shown": 0
        })
    );
    let r = t
        .ok("POST", "/session/reading", Some(json!({ "line": 2 })))
        .await;
    assert_eq!(r["line"], 2);
    // The GM's screen has the lines to read and where they are.
    let gm = t.ok("GET", "/session", None).await;
    assert_eq!(gm["readingLines"].as_array().unwrap().len(), 2);
    assert_eq!(gm["session"]["readingLine"], 2);
    assert_eq!(reading(&t.screen(&tv, "/evening").await.body)["shown"], 2);
    let r = t
        .gm("POST", "/session/reading", Some(json!({ "line": 3 })))
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "INVALID_LINE");

    // Done: the screen leaves the reading.
    t.ok("POST", "/session/reading", Some(json!({ "line": null })))
        .await;
    assert_eq!(reading(&t.screen(&tv, "/evening").await.body), Value::Null);
}
