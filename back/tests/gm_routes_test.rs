//! platform/sign-in-gm — done when every GM route refuses without the
//! GM's session, and refuses another GM where ownership applies.
//!
//! `GM_ROUTES` must list every route of the GM router in `app.rs`. Each
//! is swept with no cookie, a made-up cookie, an expired session and a
//! signed-out session (all 401), then — as a control that the 401s come
//! from the guard and not from a broken route — with the right session.
//! Routes on a resource another GM owns answer 404, exactly like a
//! resource that does not exist.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::call;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Every GM route, `{invite}` standing for an invitation of the caller
/// and `{campaign}` for one of their campaigns.
const GM_ROUTES: &[(&str, &str)] = &[
    ("GET", "/api/me"),
    ("GET", "/api/gm-invites"),
    ("POST", "/api/gm-invites"),
    ("DELETE", "/api/gm-invites/{invite}"),
    ("GET", "/api/campaigns"),
    ("POST", "/api/campaigns"),
    ("POST", "/api/campaigns/import"),
    ("GET", "/api/campaigns/{campaign}"),
    ("GET", "/api/campaigns/{campaign}/export"),
    ("PUT", "/api/campaigns/{campaign}/import"),
    ("GET", "/api/campaigns/{campaign}/player-view"),
    ("GET", "/api/campaigns/{campaign}/live"),
    // Last: it ends the session the control sweep uses.
    ("POST", "/api/auth/sign-out"),
];

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

/// The body a route needs to succeed, so the control sweep proves the
/// route works and the refusals come from the guard.
fn body_for(method: &str, path: &str) -> Option<Value> {
    match (method, path) {
        ("POST", "/api/campaigns") => Some(serde_json::json!({
            "title": "Sweep",
            "rules": { "id": "corsaires", "version": 1 }
        })),
        (_, p) if p.ends_with("/import") => Some(serde_json::json!({ "yaml": FIXTURE })),
        _ => None,
    }
}

fn route_uri(path: &str, invite: &str, campaign: &str) -> String {
    path.replace("{invite}", invite)
        .replace("{campaign}", campaign)
}

async fn campaign_of(app: &Router, token: &str) -> String {
    let r = call(
        app,
        Some(token),
        "POST",
        "/api/campaigns/import",
        body_for("POST", "/api/campaigns/import"),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["data"]["id"].as_str().unwrap().to_string()
}

async fn campaign_count(pool: &PgPool, gm: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM campaigns WHERE gm_id = $1")
        .bind(gm)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn invite_of(app: &Router, token: &str) -> String {
    let r = call(app, Some(token), "POST", "/api/gm-invites", None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["data"]["id"].as_str().unwrap().to_string()
}

async fn invite_exists(pool: &PgPool, id: &str) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM gm_invites WHERE id = $1)")
        .bind(Uuid::parse_str(id).unwrap())
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Every GM route answers 401 `UNAUTHENTICATED` with `cookie`, and
/// touches nothing.
async fn assert_all_refuse(
    app: &Router,
    pool: &PgPool,
    cookie: Option<&str>,
    invite: &str,
    campaign: &str,
) {
    for (method, path) in GM_ROUTES {
        let uri = route_uri(path, invite, campaign);
        let r = call(app, cookie, method, &uri, body_for(method, path)).await;
        assert_eq!(
            r.status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} with {cookie:?}: {}",
            r.body
        );
        assert_eq!(r.body["error"]["code"], "UNAUTHENTICATED", "{method} {uri}");
    }
    assert!(
        invite_exists(pool, invite).await,
        "a refused DELETE deleted"
    );
}

#[tokio::test]
async fn every_gm_route_refuses_without_a_valid_session() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, token) = common::signed_in_gm(&pool, "Romain").await;
    let invite = invite_of(&app, &token).await;
    let campaign = campaign_of(&app, &token).await;
    let campaigns_before = campaign_count(&pool, gm).await;
    let invites_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM gm_invites WHERE created_by = $1")
            .bind(gm)
            .fetch_one(&pool)
            .await
            .unwrap();

    // No cookie, and a cookie no session has.
    assert_all_refuse(&app, &pool, None, &invite, &campaign).await;
    assert_all_refuse(&app, &pool, Some("made-up-token"), &invite, &campaign).await;
    // The stored hash itself is not a session token either.
    let hash = promptus_back::auth::tokens::hash_token(&token);
    assert_all_refuse(&app, &pool, Some(&hash), &invite, &campaign).await;

    // A refused POST minted nothing.
    let invites_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM gm_invites WHERE created_by = $1")
            .bind(gm)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(invites_after, invites_before);
    assert_eq!(campaign_count(&pool, gm).await, campaigns_before);

    // An expired session.
    let (_, expired) = common::signed_in_gm(&pool, "Expired").await;
    sqlx::query(
        "UPDATE gm_sessions SET expires_at = now() - interval '1 second' WHERE token_hash = $1",
    )
    .bind(promptus_back::auth::tokens::hash_token(&expired))
    .execute(&pool)
    .await
    .unwrap();
    assert_all_refuse(&app, &pool, Some(&expired), &invite, &campaign).await;

    // Control: the same routes work with the right session.
    for (method, path) in GM_ROUTES {
        let uri = route_uri(path, &invite, &campaign);
        let r = call(&app, Some(&token), method, &uri, body_for(method, path)).await;
        if path.ends_with("/live") {
            // Past the guard and the ownership check, a plain request
            // (not a WebSocket upgrade) is refused by the route itself.
            assert_eq!(r.body["error"]["code"], "WEBSOCKET_REQUIRED", "{uri}");
            continue;
        }
        assert!(
            r.status.is_success(),
            "{method} {uri} with the owner's session: {} {}",
            r.status,
            r.body
        );
    }

    // Signed out (the last control call): the session no longer opens
    // anything, even with the cookie kept.
    let other_invite = {
        let (_, t) = common::signed_in_gm(&pool, "Keeper").await;
        invite_of(&app, &t).await
    };
    assert_all_refuse(&app, &pool, Some(&token), &other_invite, &campaign).await;
}

#[tokio::test]
async fn sign_out_clears_the_cookie_and_ends_only_that_session() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, phone) = common::signed_in_gm(&pool, "Romain").await;
    // The same GM on a second device.
    let mut tx = pool.begin().await.unwrap();
    let laptop = promptus_back::auth::session::create_in_tx(&mut tx, gm)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let r = call(&app, Some(&phone), "POST", "/api/auth/sign-out", None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let cookie = r.set_cookie.unwrap();
    assert!(cookie.starts_with("promptus_gm=;"), "{cookie}");
    assert!(cookie.contains("Max-Age=0"), "{cookie}");

    assert_eq!(
        call(&app, Some(&phone), "GET", "/api/me", None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, Some(&laptop), "GET", "/api/me", None)
            .await
            .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn another_gm_cannot_reach_what_a_gm_owns() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, romain) = common::signed_in_gm(&pool, "Romain").await;
    let (marc_id, marc) = common::signed_in_gm(&pool, "Marc").await;
    let invite = invite_of(&app, &romain).await;

    // Each session is its own GM.
    let r = call(&app, Some(&marc), "GET", "/api/me", None).await;
    assert_eq!(r.body["data"]["id"], marc_id.to_string());
    assert_eq!(r.body["data"]["displayName"], "Marc");

    // Romain's invitation is invisible to Marc…
    let r = call(&app, Some(&marc), "GET", "/api/gm-invites", None).await;
    assert_eq!(r.status, StatusCode::OK);
    let listed: Vec<&Value> = r.body["data"].as_array().unwrap().iter().collect();
    assert!(
        listed.iter().all(|i| i["id"] != invite.as_str()),
        "{listed:?}"
    );

    // …and cannot be revoked by him: 404, like one that does not exist.
    let r = call(
        &app,
        Some(&marc),
        "DELETE",
        &format!("/api/gm-invites/{invite}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "NOT_FOUND");
    assert!(invite_exists(&pool, &invite).await);
    let missing = Uuid::new_v4();
    let r = call(
        &app,
        Some(&marc),
        "DELETE",
        &format!("/api/gm-invites/{missing}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call(
        &app,
        Some(&marc),
        "DELETE",
        "/api/gm-invites/not-a-uuid",
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // Its owner can.
    let r = call(&app, Some(&romain), "GET", "/api/gm-invites", None).await;
    let listed = r.body["data"].as_array().unwrap();
    assert!(listed.iter().any(|i| i["id"] == invite.as_str()));
    assert!(
        listed.iter().all(|i| i.get("code").is_none()),
        "the code is shown once"
    );
    let r = call(
        &app,
        Some(&romain),
        "DELETE",
        &format!("/api/gm-invites/{invite}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(!invite_exists(&pool, &invite).await);
}

/// The `/api/{*rest}` catch-all (a JSON 404 for unknown API paths) must
/// neither swallow a GM route nor answer an unknown path with 401, with
/// or without a session; and it must not open GM routes to a visitor.
#[tokio::test]
async fn the_api_catch_all_neither_shadows_nor_opens_gm_routes() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;

    for session in [None, Some(token.as_str())] {
        for uri in ["/api/nope", "/api/me/extra", "/api/gm-invites/a/b"] {
            let r = call(&app, session, "GET", uri, None).await;
            assert_eq!(r.status, StatusCode::NOT_FOUND, "{uri} with {session:?}");
            assert_eq!(r.body["error"]["code"], "ROUTE_NOT_FOUND", "{uri}");
        }
    }
    let r = call(&app, None, "GET", "/api/me", None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = call(&app, Some(&token), "GET", "/api/me", None).await;
    assert_eq!(r.status, StatusCode::OK);
}
