//! platform/sign-in-gm — a GM signs in with their email and a code sent
//! to it. The emails are recorded instead of sent (`mailer::Recorder`);
//! everything else is the real router on a fresh schema.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call};
use promptus_back::auth::codes::{MAX_ATTEMPTS, MAX_CODES_PER_HOUR};
use promptus_back::auth::mailer::Recorder;
use serde_json::{Value, json};
use sqlx::PgPool;

async fn setup() -> (PgPool, Router, Recorder) {
    let pool = common::fresh_instance_pool().await;
    let mailer = Recorder::default();
    let app = common::app_with_mailer(pool.clone(), &mailer);
    (pool, app, mailer)
}

async fn ask(app: &Router, email: &str) -> Reply {
    call(
        app,
        None,
        "POST",
        "/api/auth/code",
        Some(json!({ "email": email })),
    )
    .await
}

async fn verify(app: &Router, body: Value) -> Reply {
    call(app, None, "POST", "/api/auth/verify", Some(body)).await
}

/// A code is due again: lets a test ask twice without waiting a minute.
async fn age_codes(pool: &PgPool) {
    sqlx::query("UPDATE gm_sign_in_codes SET created_at = created_at - interval '2 minutes'")
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_new_address_creates_its_account_with_a_name_then_signs_in_again() {
    let (pool, app, mailer) = setup().await;

    let r = ask(&app, "  Romain@Example.ORG ").await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let code = mailer
        .last_code("romain@example.org")
        .expect("a code was sent");
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));

    // The right code, but no account and no name: asked for, code kept.
    let r = verify(&app, json!({ "email": "romain@example.org", "code": code })).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "DISPLAY_NAME_REQUIRED");
    assert!(r.session_token().is_none());

    let r = verify(
        &app,
        json!({ "email": "ROMAIN@example.org", "code": code, "displayName": "  " }),
    )
    .await;
    assert_eq!(r.body["error"]["code"], "INVALID_DISPLAY_NAME");

    let r = verify(
        &app,
        json!({ "email": "romain@example.org", "code": code, "displayName": "Romain" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["data"]["displayName"], "Romain");
    let gm = r.body["data"]["id"].as_str().unwrap().to_string();
    let session = r.session_token().expect("a session cookie");
    let me = call(&app, Some(&session), "GET", "/api/me", None).await;
    assert_eq!(me.body["data"]["id"], gm.as_str());

    // The code is spent.
    let r = verify(&app, json!({ "email": "romain@example.org", "code": code })).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.body["error"]["code"], "INVALID_CODE");

    // Next time, the same account, no name needed (and a name given is
    // not a rename).
    age_codes(&pool).await;
    assert_eq!(
        ask(&app, "romain@example.org").await.status,
        StatusCode::NO_CONTENT
    );
    let code = mailer.last_code("romain@example.org").unwrap();
    let r = verify(
        &app,
        json!({ "email": "romain@example.org", "code": code, "displayName": "Autre" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["id"], gm.as_str());
    assert_eq!(r.body["data"]["displayName"], "Romain");
}

#[tokio::test]
async fn wrong_codes_all_look_alike_and_a_code_dies_after_too_many() {
    let (pool, app, mailer) = setup().await;

    // Never asked for.
    let r = verify(
        &app,
        json!({ "email": "marc@example.org", "code": "123456" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.body["error"]["code"], "INVALID_CODE");

    ask(&app, "marc@example.org").await;
    let code = mailer.last_code("marc@example.org").unwrap();
    let wrong = if code == "000000" { "000001" } else { "000000" };
    for _ in 0..MAX_ATTEMPTS {
        let r = verify(&app, json!({ "email": "marc@example.org", "code": wrong })).await;
        assert_eq!(r.body["error"]["code"], "INVALID_CODE");
    }
    // The right code no longer opens anything.
    let r = verify(
        &app,
        json!({ "email": "marc@example.org", "code": code, "displayName": "Marc" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.body["error"]["code"], "INVALID_CODE");

    // An expired code neither.
    age_codes(&pool).await;
    ask(&app, "marc@example.org").await;
    let code = mailer.last_code("marc@example.org").unwrap();
    sqlx::query("UPDATE gm_sign_in_codes SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    let r = verify(
        &app,
        json!({ "email": "marc@example.org", "code": code, "displayName": "Marc" }),
    )
    .await;
    assert_eq!(r.body["error"]["code"], "INVALID_CODE");

    // A code is bound to its address.
    age_codes(&pool).await;
    ask(&app, "marc@example.org").await;
    let code = mailer.last_code("marc@example.org").unwrap();
    let r = verify(
        &app,
        json!({ "email": "eve@example.org", "code": code, "displayName": "Eve" }),
    )
    .await;
    assert_eq!(r.body["error"]["code"], "INVALID_CODE");
}

#[tokio::test]
async fn asking_again_replaces_the_code_but_not_within_a_minute() {
    let (pool, app, mailer) = setup().await;

    ask(&app, "lea@example.org").await;
    let first = mailer.last_code("lea@example.org").unwrap();
    let r = ask(&app, "lea@example.org").await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(r.body["error"]["code"], "CODE_TOO_SOON");
    assert_eq!(mailer.sent().len(), 1);

    age_codes(&pool).await;
    ask(&app, "lea@example.org").await;
    let second = mailer.last_code("lea@example.org").unwrap();
    if first != second {
        let r = verify(
            &app,
            json!({ "email": "lea@example.org", "code": first, "displayName": "Léa" }),
        )
        .await;
        assert_eq!(r.body["error"]["code"], "INVALID_CODE", "the old code died");
    }
    let r = verify(
        &app,
        json!({ "email": "lea@example.org", "code": second, "displayName": "Léa" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
}

#[tokio::test]
async fn the_server_stops_sending_past_its_hourly_cap() {
    let (_, app, mailer) = setup().await;
    for i in 0..MAX_CODES_PER_HOUR {
        let r = ask(&app, &format!("gm{i}@example.org")).await;
        assert_eq!(r.status, StatusCode::NO_CONTENT, "{i}");
    }
    let r = ask(&app, "one-more@example.org").await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(r.body["error"]["code"], "TOO_MANY_CODES");
    assert_eq!(mailer.sent().len(), MAX_CODES_PER_HOUR as usize);
}

#[tokio::test]
async fn an_email_that_cannot_leave_leaves_no_code_behind() {
    let pool = common::fresh_instance_pool().await;
    let app = common::app_with_mailer(pool.clone(), &Recorder::failing());
    let r = ask(&app, "romain@example.org").await;
    assert_eq!(r.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(r.body["error"]["code"], "EMAIL_UNAVAILABLE");
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gm_sign_in_codes")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
    // Not "too soon" either: the GM may try again at once.
    assert_eq!(
        ask(&app, "romain@example.org").await.status,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn implausible_addresses_are_refused_before_anything_is_sent() {
    let (_, app, mailer) = setup().await;
    for email in ["", "romain", "romain@example", "a b@example.org"] {
        let r = ask(&app, email).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{email:?}");
        assert_eq!(r.body["error"]["code"], "INVALID_EMAIL");
    }
    assert!(mailer.sent().is_empty());
}

#[tokio::test]
async fn each_gm_keeps_their_campaigns() {
    let (pool, app, mailer) = setup().await;
    let mut sessions = Vec::new();
    for (email, name) in [
        ("romain@example.org", "Romain"),
        ("marc@example.org", "Marc"),
    ] {
        ask(&app, email).await;
        let code = mailer.last_code(email).unwrap();
        let r = verify(
            &app,
            json!({ "email": email, "code": code, "displayName": name }),
        )
        .await;
        assert_eq!(r.status, StatusCode::CREATED);
        sessions.push(r.session_token().unwrap());
    }
    let r = call(
        &app,
        Some(&sessions[0]),
        "POST",
        "/api/campaigns",
        Some(json!({ "title": "Le Brasier", "rules": { "id": "corsaires", "version": 1 } })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let campaign = r.body["data"]["id"].as_str().unwrap().to_string();

    let r = call(
        &app,
        Some(&sessions[1]),
        "GET",
        &format!("/api/campaigns/{campaign}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call(&app, Some(&sessions[1]), "GET", "/api/campaigns", None).await;
    assert!(
        r.body["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["id"] != campaign.as_str())
    );
    drop(pool);
}
