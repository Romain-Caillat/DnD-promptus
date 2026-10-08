//! session/pair-shared-screen and tv/show-evening over the API: a TV
//! shows a code, the GM types it, the TV follows the evening with what
//! every player may see and nothing more, until the GM forgets it. The
//! leak sweep of the screen routes is in `player_routes_test.rs`, on
//! the marked table.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::live::{next_of, open_screen, ready};
use common::{Reply, call, call_as_player, call_as_screen, imported_campaign, invite_code, join};
use promptus_back::live::LiveConfig;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

/// A TV says hello: its token and its code.
async fn tv(app: &Router) -> (String, String) {
    let r = call_as_screen(app, None, "POST", "/api/tv", None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["paired"], false);
    let code = r.body["data"]["code"].as_str().unwrap().to_string();
    (r.screen_token().expect("a new screen keeps a token"), code)
}

async fn show(app: &Router, token: &str) -> Reply {
    call_as_screen(app, Some(token), "GET", "/api/tv/show", None).await
}

#[tokio::test]
async fn a_tv_pairs_with_its_code_follows_the_table_and_is_forgotten() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let screens = format!("/api/campaigns/{campaign}/screens");

    let (token, code) = tv(&app).await;
    assert_eq!(code.len(), 4);
    // Not paired yet: nothing to show.
    let r = show(&app, &token).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.body["error"]["code"], "NOT_PAIRED");
    // Saying hello again keeps the screen and its code.
    let r = call_as_screen(&app, Some(&token), "POST", "/api/tv", None).await;
    assert_eq!(r.body["data"]["code"], code.as_str());
    assert!(r.screen_token().is_none());

    // Another GM types a wrong code; Romain types his TV's, lower case.
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &screens,
        Some(json!({ "code": "ZZZZ" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "NO_SUCH_CODE");
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &screens,
        Some(json!({ "code": code.to_lowercase() })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let listed = r.body["data"]["screens"].as_array().unwrap().clone();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["kind"], "tv");
    // The code is spent: another GM cannot take the TV with it.
    let (_, other) = common::signed_in_gm(&pool, "Autre").await;
    let theirs = imported_campaign(&app, &other, FIXTURE).await;
    let r = call(
        &app,
        Some(&other),
        "POST",
        &format!("/api/campaigns/{theirs}/screens"),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    // …nor see or forget Romain's screens.
    let r = call(&app, Some(&other), "GET", &screens, None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    let r = call_as_screen(&app, Some(&token), "POST", "/api/tv", None).await;
    assert_eq!(
        r.body["data"],
        json!({ "paired": true, "code": null, "expiresAt": null })
    );
    let r = show(&app, &token).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["title"], "Le Phare de Kerbrume");

    // The screen's token opens no GM route and no player route; the GM's
    // session opens no screen route.
    let r = call_as_screen(&app, Some(&token), "GET", &screens, None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = call_as_screen(
        &app,
        Some(&token),
        "GET",
        &format!("/api/play/{campaign}/evening"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = call(&app, Some(&gm), "GET", "/api/tv/show", None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    // Forgotten: the TV is back to a code, with a new token.
    let id = listed[0]["id"].as_str().unwrap();
    let r = call(&app, Some(&gm), "DELETE", &format!("{screens}/{id}"), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["screens"], json!([]));
    assert_eq!(show(&app, &token).await.status, StatusCode::UNAUTHORIZED);
    let r = call_as_screen(&app, Some(&token), "POST", "/api/tv", None).await;
    assert_eq!(r.body["data"]["paired"], false);
    assert!(r.screen_token().is_some_and(|t| t != token));
}

#[tokio::test]
async fn an_expired_code_pairs_nothing_and_the_tv_gets_a_new_one() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let (token, code) = tv(&app).await;
    sqlx::query(
        "UPDATE shared_screens SET code_expires_at = now() - interval '1 second' WHERE code = $1",
    )
    .bind(&code)
    .execute(&pool)
    .await
    .unwrap();
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("/api/campaigns/{campaign}/screens"),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call_as_screen(&app, Some(&token), "POST", "/api/tv", None).await;
    assert_eq!(r.body["data"]["paired"], false);
    assert!(
        r.body["data"]["code"]
            .as_str()
            .is_some_and(|c| c.len() == 4)
    );
    // The QR names the pairing page for that code, for a plain origin
    // only.
    let r = common::send(
        &app,
        Some(&format!("promptus_screen={token}")),
        "GET",
        "/api/tv/qr.svg?origin=https://evil.example/x",
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

/// Romain's table on the Brasier: Marc plays Kaël (validated, here),
/// Léa watches; the session is live.
struct Evening {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
    marc: String,
}

async fn evening(app: Router, pool: PgPool) -> Evening {
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, BRASIER).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let marc_id = Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    sqlx::query(
        "UPDATE characters SET status = 'validated',
                sheet = '{\"name\":\"Kaël\",\"classId\":\"pilote\",\"abilities\":{\"DEX\":14}}'
         WHERE player_id = $1",
    )
    .bind(marc_id)
    .execute(&pool)
    .await
    .unwrap();
    join(&app, &code, "Léa", "spectator").await;
    let e = Evening {
        app,
        pool,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
    };
    for path in ["/session", "/session/start"] {
        let r = e.gm("POST", path, None).await;
        assert!(r.status.is_success(), "{path}: {}", r.body);
    }
    let r = e
        .marc(
            "POST",
            "/lobby",
            Some(json!({ "soundOk": true, "remote": true })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    e
}

impl Evening {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn marc(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/play/{}{path}", self.campaign);
        call_as_player(&self.app, Some(&self.marc), method, &uri, body).await
    }

    async fn paired_tv(&self) -> String {
        let (token, code) = tv(&self.app).await;
        let r = self
            .gm("POST", "/screens", Some(json!({ "code": code })))
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        token
    }
}

#[tokio::test]
async fn the_tv_shows_the_party_the_dice_and_what_the_gm_lets_it() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let e = evening(app, pool).await;
    let tv = e.paired_tv().await;

    // Marc asks to dodge; the GM asks for a check; Marc rolls it. His
    // other request, refused with the GM's reason, stays between them.
    let r = e
        .marc(
            "POST",
            "/requests",
            Some(json!({ "card": { "kind": "ability", "ability": "DEX" }, "text": "J'esquive." })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let request = r.body["data"]["requests"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = e
        .gm(
            "POST",
            &format!("/session/requests/{request}"),
            Some(json!({ "kind": "check", "ability": "DEX", "value": 10 })),
        )
        .await;
    assert!(r.status.is_success(), "{}", r.body);
    let r = e
        .marc("POST", &format!("/requests/{request}/roll"), None)
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let rolled = r.body["data"]["requests"][0]["roll"].clone();
    let secret = "Pas maintenant : Marc garde ce plan pour plus tard.";
    let r = e
        .marc(
            "POST",
            "/requests",
            Some(json!({ "card": { "kind": "other" }, "text": "Je fais semblant de dormir." })),
        )
        .await;
    let other = r.body["data"]["requests"][1]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = e
        .gm(
            "POST",
            &format!("/session/requests/{other}"),
            Some(json!({ "kind": "refuse", "reason": secret })),
        )
        .await;
    assert!(r.status.is_success(), "{}", r.body);

    let view = show(&e.app, &tv).await.body["data"].clone();
    assert_eq!(view["session"]["status"], "live");
    // The party: Marc's character, here; never Léa, a spectator.
    let party: Vec<(&str, &str, bool)> = view["party"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["name"].as_str().unwrap(),
                s["nickname"].as_str().unwrap(),
                s["here"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(party, vec![("Kaël", "Marc", true)]);
    assert!(view["party"][0]["hitPoints"].as_i64().is_some(), "{view}");
    // The die everyone saw roll, the same faces; not the GM's answer to
    // another request, nor that request.
    assert_eq!(view["rolls"].as_array().unwrap().len(), 1);
    assert_eq!(view["rolls"][0]["character"], "Kaël");
    assert_eq!(view["rolls"][0]["roll"], rolled);
    assert!(!view.to_string().contains(secret), "{view}");
    assert!(!view.to_string().contains("semblant"), "{view}");
    // The roll's shared journal line is a moment.
    assert!(
        view["moments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["kind"] == "roll"),
        "{view}"
    );

    // The GM narrows the screen: no party, no moments, no map, no scene.
    let r = e
        .gm(
            "PUT",
            "/screens/shows",
            Some(json!({ "scene": false, "map": false, "party": false, "moments": false })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let view = show(&e.app, &tv).await.body["data"].clone();
    assert_eq!(view["party"], json!([]));
    assert_eq!(view["rolls"], json!([]));
    assert_eq!(view["moments"], json!([]));
    assert_eq!(view["board"], Value::Null);
    assert_eq!(view["scene"], Value::Null);
    let _ = &e.pool;
}

#[tokio::test]
async fn the_screen_is_present_as_a_screen_and_hears_the_gm_change_what_it_shows() {
    let t = common::live::table(LiveConfig::default()).await;
    let r = call_as_screen(&t.router, None, "POST", "/api/tv", None).await;
    let token = r.screen_token().unwrap();
    let code = r.body["data"]["code"].as_str().unwrap().to_string();
    let r = call(
        &t.router,
        Some(&t.token),
        "POST",
        &format!("/api/campaigns/{}/screens", t.campaign),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);

    let mut ws = open_screen(t.addr, &token)
        .await
        .expect("a paired screen opens its socket");
    ready(&mut ws).await;
    let presence = next_of(&mut ws, "presence").await;
    assert_eq!(presence["players"], json!([]), "a screen is not a player");
    assert_eq!(presence["screens"].as_array().unwrap().len(), 1);
    let r = call(
        &t.router,
        Some(&t.token),
        "GET",
        &format!("/api/campaigns/{}/screens", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["screens"][0]["online"], true);

    let r = call(
        &t.router,
        Some(&t.token),
        "PUT",
        &format!("/api/campaigns/{}/screens/shows", t.campaign),
        Some(json!({ "scene": true, "map": false, "party": true, "moments": true })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let changed = next_of(&mut ws, "changed").await;
    assert_eq!(changed["topic"], "screens");

    // An unpaired token opens no socket.
    let r = call_as_screen(&t.router, None, "POST", "/api/tv", None).await;
    let waiting = r.screen_token().unwrap();
    assert!(open_screen(t.addr, &waiting).await.is_err());
}

#[tokio::test]
async fn the_gm_opens_a_window_in_their_own_browser_and_keeps_one() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let path = format!("/api/campaigns/{campaign}/screens/window");
    let r = call(&app, Some(&gm), "POST", &path, None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let first = r.screen_token().expect("the window's token");
    assert_eq!(show(&app, &first).await.status, StatusCode::OK);
    // Opening it again replaces it: the campaign keeps one window.
    let r = call(&app, Some(&gm), "POST", &path, None).await;
    let second = r.screen_token().unwrap();
    assert_eq!(r.body["data"]["screens"].as_array().unwrap().len(), 1);
    assert_eq!(r.body["data"]["screens"][0]["kind"], "window");
    assert_eq!(show(&app, &first).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(show(&app, &second).await.status, StatusCode::OK);
}
