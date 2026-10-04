//! session/invite-and-join — done when Marc joins from his phone without
//! an account, closes the browser, comes back the next day and finds his
//! character.
//!
//! The device keeps a random token in a cookie scoped to the campaign;
//! the server keeps its hash only. The GM mints one link per campaign,
//! regenerates or closes it, sees who joined, and can remove a seat.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::marked::FIXTURE;
use common::{call, call_as_player, imported_campaign, invite_code, join};
use promptus_back::auth::tokens::hash_token;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

/// Romain, his Phare de Kerbrume campaign and a fresh invitation code.
async fn table(app: &Router, pool: &PgPool) -> (String, String, String) {
    let (_, gm) = common::signed_in_gm(pool, "Romain").await;
    let campaign = imported_campaign(app, &gm, FIXTURE).await;
    let code = invite_code(app, &gm, &campaign).await;
    (gm, campaign, code)
}

async fn me(app: &Router, campaign: &str, token: Option<&str>) -> common::Reply {
    call_as_player(app, token, "GET", &format!("/api/play/{campaign}/me"), None).await
}

async fn seats(app: &Router, gm: &str, campaign: &str) -> Vec<Value> {
    let r = call(
        app,
        Some(gm),
        "GET",
        &format!("/api/campaigns/{campaign}/players"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    r.body["data"].as_array().unwrap().clone()
}

#[tokio::test]
async fn marc_joins_without_an_account_and_finds_his_character_the_next_day() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, campaign, code) = table(&app, &pool).await;

    // The link shows whose table and which campaign, before joining.
    let r = call(&app, None, "GET", &format!("/api/join/{code}"), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["campaignId"], campaign.as_str());
    assert_eq!(r.body["data"]["title"], "Le Phare de Kerbrume");
    assert_eq!(r.body["data"]["gmName"], "Romain");
    assert!(!r.body["data"]["playerHook"].as_str().unwrap().is_empty());

    // A nickname, « create my character »: a seat and a draft character.
    let r = join(&app, &code, "  Marc ", "player").await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let token = r.player_token().expect("a device token");
    let cookie = r.set_cookie.clone().unwrap();
    for attr in [
        "HttpOnly",
        "SameSite=Strict",
        &format!("Path=/api/play/{campaign}"),
    ] {
        assert!(cookie.contains(attr), "{attr} missing from {cookie}");
    }
    assert_eq!(r.body["data"]["me"]["nickname"], "Marc");
    assert_eq!(r.body["data"]["me"]["role"], "player");
    assert_eq!(r.body["data"]["character"]["status"], "draft");
    let character = r.body["data"]["character"]["id"].clone();
    assert!(
        !r.body.to_string().contains(&token),
        "the token travels in the cookie only"
    );

    // The server keeps the hash, never the token.
    let stored: String = sqlx::query_scalar(
        "SELECT token_hash FROM players WHERE nickname = 'Marc' AND campaign_id = $1",
    )
    .bind(Uuid::parse_str(&campaign).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored, hash_token(&token));
    assert_ne!(stored, token);

    // The next day: the link has expired since, the browser kept its
    // cookie, and Marc finds his seat and his character.
    sqlx::query("UPDATE campaign_invites SET expires_at = now() - interval '1 second' WHERE campaign_id = $1")
        .bind(Uuid::parse_str(&campaign).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE players SET last_seen_at = now() - interval '1 day' WHERE token_hash = $1")
        .bind(&stored)
        .execute(&pool)
        .await
        .unwrap();
    let r = me(&app, &campaign, Some(&token)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["me"]["nickname"], "Marc");
    assert_eq!(r.body["data"]["campaign"]["title"], "Le Phare de Kerbrume");
    assert_eq!(r.body["data"]["character"]["id"], character);

    // The GM sees him at the table, seen just now.
    let list = seats(&app, &gm, &campaign).await;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["nickname"], "Marc");
    assert_eq!(list[0]["character"]["status"], "draft");
    let seen: chrono::DateTime<chrono::Utc> =
        serde_json::from_value(list[0]["lastSeenAt"].clone()).unwrap();
    assert!(chrono::Utc::now() - seen < chrono::Duration::minutes(1));
}

#[tokio::test]
async fn a_wrong_or_foreign_token_opens_nothing() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, campaign, code) = table(&app, &pool).await;
    let token = join(&app, &code, "Marc", "player")
        .await
        .player_token()
        .unwrap();

    for wrong in [
        None,
        Some("made-up-token"),
        Some(hash_token(&token).as_str()),
    ] {
        let r = me(&app, &campaign, wrong).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{wrong:?}");
        assert_eq!(r.body["error"]["code"], "NOT_JOINED");
    }
    // A token is scoped to its campaign: Marc's opens no other table.
    let other = imported_campaign(&app, &gm, FIXTURE).await;
    let r = me(&app, &other, Some(&token)).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = me(&app, "not-a-uuid", Some(&token)).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    // A browser holding several player cookies is recognised by the one
    // that belongs here.
    let r = common::send(
        &app,
        Some(&format!("promptus_player=stale; promptus_player={token}")),
        "GET",
        &format!("/api/play/{campaign}/me"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
}

#[tokio::test]
async fn a_regenerated_closed_or_expired_link_lets_nobody_new_in() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, campaign, old) = table(&app, &pool).await;
    let marc = join(&app, &old, "Marc", "player")
        .await
        .player_token()
        .unwrap();

    // A new link revokes the old one; who joined keeps their seat.
    let new = invite_code(&app, &gm, &campaign).await;
    assert_ne!(new, old);
    for method in ["GET", "POST"] {
        let r = call(
            &app,
            None,
            method,
            &format!("/api/join/{old}"),
            Some(json!({ "nickname": "Intrus", "role": "player" })),
        )
        .await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{method}");
        assert_eq!(r.body["error"]["code"], "INVITE_NOT_FOUND");
    }
    assert_eq!(
        me(&app, &campaign, Some(&marc)).await.status,
        StatusCode::OK
    );
    assert_eq!(
        join(&app, &new, "Camille", "player").await.status,
        StatusCode::CREATED
    );

    // The GM sees the link's dates, never its code.
    let uri = format!("/api/campaigns/{campaign}/invite");
    let r = call(&app, Some(&gm), "GET", &uri, None).await;
    assert!(r.body["data"]["expiresAt"].is_string(), "{}", r.body);
    assert!(r.body["data"].get("code").is_none());

    // Closed: nobody gets in, and the GM sees no link.
    let r = call(&app, Some(&gm), "DELETE", &uri, None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(
        join(&app, &new, "Hugo", "player").await.status,
        StatusCode::NOT_FOUND
    );
    let r = call(&app, Some(&gm), "GET", &uri, None).await;
    assert_eq!(r.body["data"], Value::Null);

    // Expired: the same.
    let latest = invite_code(&app, &gm, &campaign).await;
    sqlx::query("UPDATE campaign_invites SET expires_at = now() - interval '1 second' WHERE campaign_id = $1")
        .bind(Uuid::parse_str(&campaign).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        join(&app, &latest, "Hugo", "player").await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(seats(&app, &gm, &campaign).await.len(), 2);
}

#[tokio::test]
async fn a_spectator_watches_without_a_character() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, campaign, code) = table(&app, &pool).await;

    let r = join(&app, &code, "Léa", "spectator").await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["data"]["me"]["role"], "spectator");
    assert_eq!(r.body["data"]["character"], Value::Null);
    let token = r.player_token().unwrap();

    let r = call_as_player(
        &app,
        Some(&token),
        "GET",
        &format!("/api/play/{campaign}/view"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["title"], "Le Phare de Kerbrume");

    let list = seats(&app, &gm, &campaign).await;
    assert_eq!(list[0]["role"], "spectator");
    assert_eq!(list[0]["character"], Value::Null);
}

#[tokio::test]
async fn a_nickname_is_required_and_unique_at_the_table() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, _, code) = table(&app, &pool).await;

    for (nick, role, expected) in [
        ("   ", "player", "INVALID_NICKNAME"),
        (&"x".repeat(41), "player", "INVALID_NICKNAME"),
        ("Marc", "game-master", "INVALID_BODY"),
    ] {
        let r = join(&app, &code, nick, role).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{nick:?} {role}");
        assert_eq!(r.body["error"]["code"], expected);
    }
    assert_eq!(
        join(&app, &code, "Marc", "player").await.status,
        StatusCode::CREATED
    );
    let r = join(&app, &code, "marc", "spectator").await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "NICKNAME_TAKEN");
    assert!(r.player_token().is_none());
}

#[tokio::test]
async fn the_gm_removes_a_seat_and_only_their_own() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, campaign, code) = table(&app, &pool).await;
    let (_, other_gm) = common::signed_in_gm(&pool, "Marc le MJ").await;
    let r = join(&app, &code, "Intrus", "player").await;
    let token = r.player_token().unwrap();
    let player = r.body["data"]["me"]["id"].as_str().unwrap().to_string();
    let seat = format!("/api/campaigns/{campaign}/players/{player}");

    // Another GM learns nothing and changes nothing: 404 everywhere.
    for (method, uri) in [
        ("GET", format!("/api/campaigns/{campaign}/players")),
        ("GET", format!("/api/campaigns/{campaign}/invite")),
        ("POST", format!("/api/campaigns/{campaign}/invite")),
        ("DELETE", format!("/api/campaigns/{campaign}/invite")),
        ("DELETE", seat.clone()),
    ] {
        let r = call(&app, Some(&other_gm), method, &uri, None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{method} {uri}");
    }
    assert_eq!(
        me(&app, &campaign, Some(&token)).await.status,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, None, "GET", &format!("/api/join/{code}"), None)
            .await
            .status,
        StatusCode::OK,
        "the link survived"
    );

    // The owner removes the seat: the device token stops working, and
    // the character goes with it.
    let r = call(&app, Some(&gm), "DELETE", &seat, None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(
        me(&app, &campaign, Some(&token)).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert!(seats(&app, &gm, &campaign).await.is_empty());
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters WHERE player_id = $1")
        .bind(Uuid::parse_str(&player).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
    let r = call(&app, Some(&gm), "DELETE", &seat, None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}
