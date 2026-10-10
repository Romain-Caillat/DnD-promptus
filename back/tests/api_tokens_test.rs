//! platform/connect-claude-mcp — a GM's personal access tokens.
//!
//! Done when a token is shown once and stored hashed, reaches the
//! preparation routes of `TOKEN_ROUTES` as its GM (and stamps its last
//! use), is refused at once once revoked, and reaches nothing else
//! (`gm_routes_test.rs` sweeps every other GM route with one).

mod common;

use axum::http::{StatusCode, header};
use common::{call, call_with_token};
use promptus_back::auth::tokens::hash_token;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

async fn last_used(pool: &PgPool, id: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar("SELECT last_used_at FROM gm_api_tokens WHERE id = $1")
        .bind(Uuid::parse_str(id).unwrap())
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Every in-scope route with a body that makes it succeed, on `campaign`.
fn scoped_calls(campaign: &str) -> Vec<(&'static str, String, Option<serde_json::Value>)> {
    vec![
        ("GET", "/api/campaigns".to_string(), None),
        ("GET", format!("/api/campaigns/{campaign}"), None),
        ("GET", format!("/api/campaigns/{campaign}/export"), None),
        (
            "PUT",
            format!("/api/campaigns/{campaign}/import"),
            Some(json!({ "yaml": FIXTURE })),
        ),
        (
            "POST",
            format!("/api/campaigns/{campaign}/story/edits"),
            Some(json!({ "edits": [
                { "op": "set", "target": "bible", "field": "tone", "value": "Sombre." }
            ] })),
        ),
        ("GET", format!("/api/campaigns/{campaign}/readiness"), None),
    ]
}

#[tokio::test]
async fn a_token_is_shown_once_and_stored_hashed() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, session) = common::signed_in_gm(&pool, "Romain").await;

    let r = call(
        &app,
        Some(&session),
        "POST",
        "/api/gm-tokens",
        Some(json!({ "name": "  Mac de Romain " })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let id = r.body["data"]["id"].as_str().unwrap().to_string();
    let secret = r.body["data"]["secret"].as_str().unwrap().to_string();
    assert_eq!(r.body["data"]["name"], "Mac de Romain");
    assert!(secret.starts_with("promptus_"), "{secret}");
    assert!(r.body["data"]["lastUsedAt"].is_null());

    let stored: String = sqlx::query_scalar("SELECT token_hash FROM gm_api_tokens WHERE id = $1")
        .bind(Uuid::parse_str(&id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, hash_token(&secret));
    assert_ne!(stored, secret);

    let r = call(&app, Some(&session), "GET", "/api/gm-tokens", None).await;
    let listed = r.body["data"].as_array().unwrap();
    assert!(listed.iter().any(|t| t["id"] == id.as_str()));
    assert!(
        listed.iter().all(|t| t.get("secret").is_none()),
        "the secret is shown once"
    );

    for name in ["", "   ", &"x".repeat(61)] {
        let r = call(
            &app,
            Some(&session),
            "POST",
            "/api/gm-tokens",
            Some(json!({ "name": name })),
        )
        .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{name:?}");
        assert_eq!(r.body["error"]["code"], "INVALID_TOKEN_NAME");
    }
}

#[tokio::test]
async fn a_token_prepares_its_gms_campaign_then_dies_when_revoked() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, session) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = common::imported_campaign(&app, &session, FIXTURE).await;
    let (id, secret) = common::api_token(&app, &session, "Claude").await;

    for (method, uri, body) in scoped_calls(&campaign) {
        let r = call_with_token(&app, &secret, method, &uri, body).await;
        assert_eq!(r.status, StatusCode::OK, "{method} {uri}: {}", r.body);
    }
    // A campaign written elsewhere lands as a new one of the GM's.
    let r = call_with_token(
        &app,
        &secret,
        "POST",
        "/api/campaigns/import",
        Some(json!({ "yaml": FIXTURE })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let created = r.body["data"]["id"].as_str().unwrap().to_string();
    let r = call(
        &app,
        Some(&session),
        "GET",
        &format!("/api/campaigns/{created}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "the GM owns it");

    // The edit landed, as the GM's own.
    let r = call(
        &app,
        Some(&session),
        "GET",
        &format!("/api/campaigns/{campaign}"),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["story"]["bible"]["tone"], "Sombre.");
    // The list shows the token's last use.
    assert!(last_used(&pool, &id).await.is_some(), "last use stamped");
    let r = call(&app, Some(&session), "GET", "/api/gm-tokens", None).await;
    let listed = r.body["data"].as_array().unwrap();
    let row = listed.iter().find(|t| t["id"] == id.as_str()).unwrap();
    assert!(row["lastUsedAt"].is_string(), "{row}");

    // An edit that does not fit is refused whole, with the server's
    // reason (the MCP server passes it on).
    let r = call_with_token(
        &app,
        &secret,
        "POST",
        &format!("/api/campaigns/{campaign}/story/edits"),
        Some(json!({ "edits": [
            { "op": "set", "target": "bible", "field": "tone", "value": "Clair." },
            { "op": "remove", "target": "nobody_here" }
        ] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    assert!(
        r.body["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("edit 1"),
        "{}",
        r.body
    );

    // Revoked: refused at once on every route it reached.
    let used = last_used(&pool, &id).await;
    let r = call(
        &app,
        Some(&session),
        "DELETE",
        &format!("/api/gm-tokens/{id}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    for (method, uri, body) in scoped_calls(&campaign) {
        let r = call_with_token(&app, &secret, method, &uri, body).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(r.body["error"]["code"], "INVALID_TOKEN", "{method} {uri}");
    }
    assert_eq!(last_used(&pool, &id).await, used, "a refused call");
    let r = call(&app, Some(&session), "GET", "/api/gm-tokens", None).await;
    assert!(
        r.body["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["id"] != id.as_str()),
        "a revoked token leaves the list"
    );
    // Revoking twice is a 404, like a token that never existed.
    let r = call(
        &app,
        Some(&session),
        "DELETE",
        &format!("/api/gm-tokens/{id}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_token_is_its_gms_alone() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, romain) = common::signed_in_gm(&pool, "Romain").await;
    let (_, marc) = common::signed_in_gm(&pool, "Marc").await;
    let romains = common::imported_campaign(&app, &romain, FIXTURE).await;
    let (marc_token_id, marc_secret) = common::api_token(&app, &marc, "Claude de Marc").await;

    // Marc's token does not see Romain's campaign: 404, like a missing one.
    for (method, uri, body) in scoped_calls(&romains).into_iter().skip(1) {
        let r = call_with_token(&app, &marc_secret, method, &uri, body).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{method} {uri}");
    }
    let r = call_with_token(&app, &marc_secret, "GET", "/api/campaigns", None).await;
    assert!(
        r.body["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["id"] != romains.as_str())
    );
    // Romain cannot revoke Marc's token.
    let r = call(
        &app,
        Some(&romain),
        "DELETE",
        &format!("/api/gm-tokens/{marc_token_id}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call_with_token(&app, &marc_secret, "GET", "/api/campaigns", None).await;
    assert_eq!(r.status, StatusCode::OK);
}

#[tokio::test]
async fn only_a_live_bearer_token_passes() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, session) = common::signed_in_gm(&pool, "Romain").await;
    let (_, secret) = common::api_token(&app, &session, "Claude").await;

    // Unknown, the stored hash, or the session cookie's value sent as a
    // bearer: refused.
    for bad in ["promptus_made-up", &hash_token(&secret), &session] {
        let r = call_with_token(&app, bad, "GET", "/api/campaigns", None).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{bad}");
        assert_eq!(r.body["error"]["code"], "INVALID_TOKEN");
    }
    // Another scheme is no token: without a cookie, the GM is unknown.
    let basic = format!("Basic {secret}");
    let r = common::send_with(
        &app,
        &[(header::AUTHORIZATION, &basic)],
        "GET",
        "/api/campaigns",
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.body["error"]["code"], "UNAUTHENTICATED");
    // The scheme's case does not matter.
    let lower = format!("bearer {secret}");
    let r = common::send_with(
        &app,
        &[(header::AUTHORIZATION, &lower)],
        "GET",
        "/api/campaigns",
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    // A token never reaches the player or screen routes either: they
    // read their own cookies only.
    let r = call_with_token(&app, &secret, "GET", "/api/tv/show", None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}
