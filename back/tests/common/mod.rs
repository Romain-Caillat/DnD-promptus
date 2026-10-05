//! Shared helpers for integration tests.
//!
//! Cargo treats files directly under `tests/` as standalone test binaries,
//! so common helpers live in `tests/common/mod.rs`. Each test file that
//! needs them includes this module with `mod common;`.

#![allow(dead_code)]

pub mod live;
pub mod marked;

use std::str::FromStr;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use promptus_back::auth::setup::SetupState;
use promptus_back::live::{LiveConfig, LiveHub};
use promptus_back::state::{AppState, Auth};
use serde_json::Value;
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

/// Cap on a response body read in tests — every response here is a
/// small JSON envelope.
pub const MAX_TEST_BODY: usize = 1024 * 1024;

/// The origin the test "browser" opens the app at.
pub const ORIGIN: &str = "http://localhost:4334";

fn test_database_url() -> String {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL must be set to run integration tests \
         (copy .env.example to .env, then `bun run db:start`)",
    );
    if std::env::var("DATABASE_URL").is_ok_and(|dev| dev == url) {
        panic!("TEST_DATABASE_URL equals DATABASE_URL — tests would write to the dev database");
    }
    url
}

/// Connect to the **test** database and apply the migrations.
///
/// Integration tests never touch the dev database: they read
/// `TEST_DATABASE_URL` (a separate `promptus_test` database in the dev
/// Postgres container, see `docker-compose.dev.yml`). A missing variable
/// fails the test loudly rather than skipping it — a skipped suite reads
/// as green and hides that nothing ran.
pub async fn test_pool() -> PgPool {
    test_pool_sized(2).await
}

/// [`test_pool`] with room for `max` concurrent connections, for tests
/// that race transactions against each other.
pub async fn test_pool_sized(max: u32) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(max)
        .connect(&test_database_url())
        .await
        .expect("connect to the test database");
    promptus_back::db::migrate(&pool)
        .await
        .expect("apply migrations to the test database");
    pool
}

/// A pool on a **fresh schema** of the test database, migrated from
/// scratch: an instance with no GM at all, which tests running in
/// parallel on the shared schema cannot disturb. The schema is left
/// behind (each name is unique); `bun run db:reset` clears them.
pub async fn fresh_instance_pool() -> PgPool {
    let url = test_database_url();
    let schema = format!("t_{}", Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("connect to the test database");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .expect("create a test schema");
    admin.close().await;
    let options = PgConnectOptions::from_str(&url)
        .expect("parse TEST_DATABASE_URL")
        .options([("search_path", schema.as_str())]);
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .expect("connect to the test schema");
    promptus_back::db::migrate(&pool)
        .await
        .expect("apply migrations to the test schema");
    pool
}

/// GM authentication as the server builds it, for `ORIGIN`.
pub fn auth(setup: SetupState) -> Auth {
    Auth::new(ORIGIN, None, setup).expect("relying party")
}

/// The real router over `pool`, with setup closed.
pub fn app(pool: PgPool) -> Router {
    app_with(pool, SetupState::closed())
}

/// The real router, without the live listener: it would hold one of the
/// test pool's two connections, and only tests opening live sockets need
/// it ([`app_live`]).
pub fn app_with(pool: PgPool, setup: SetupState) -> Router {
    router_with(pool, setup, LiveHub::new(LiveConfig::default()))
}

/// The real router with its live listener running, and the hub, for
/// tests that open live sockets. Must run inside a Tokio runtime.
pub fn app_live(pool: PgPool, config: LiveConfig) -> (Router, LiveHub) {
    let live = LiveHub::new(config);
    promptus_back::live::listener::spawn(pool.clone(), live.clone());
    (router_with(pool, SetupState::closed(), live.clone()), live)
}

fn router_with(pool: PgPool, setup: SetupState, live: LiveHub) -> Router {
    promptus_back::app::router(
        AppState {
            pool,
            auth: auth(setup),
            live,
        },
        &[],
    )
}

/// Consume an axum response body and decode it as JSON.
pub async fn read_json(response: axum::response::Response) -> serde_json::Value {
    let body = axum::body::to_bytes(response.into_body(), MAX_TEST_BODY)
        .await
        .expect("read response body");
    serde_json::from_slice(&body).expect("decode response as JSON")
}

/// What a test reads back from one request.
pub struct Reply {
    pub status: StatusCode,
    pub body: Value,
    pub set_cookie: Option<String>,
}

impl Reply {
    /// The session token the response stored, if it set one.
    pub fn session_token(&self) -> Option<String> {
        let cookie = self.set_cookie.as_deref()?;
        let value = cookie.strip_prefix("promptus_gm=")?.split(';').next()?;
        (!value.is_empty()).then(|| value.to_string())
    }

    /// The player device token the response stored, if it set one.
    pub fn player_token(&self) -> Option<String> {
        let cookie = self.set_cookie.as_deref()?;
        let value = cookie.strip_prefix("promptus_player=")?.split(';').next()?;
        (!value.is_empty()).then(|| value.to_string())
    }
}

/// Send one request; `session` is the GM session cookie to carry.
pub async fn call(
    app: &Router,
    session: Option<&str>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> Reply {
    let cookie = session.map(|token| format!("promptus_gm={token}"));
    send(app, cookie.as_deref(), method, uri, body).await
}

/// Send one request as the device holding player `token`.
pub async fn call_as_player(
    app: &Router,
    token: Option<&str>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> Reply {
    let cookie = token.map(|token| format!("promptus_player={token}"));
    send(app, cookie.as_deref(), method, uri, body).await
}

/// Send one request with `cookie` as its whole `Cookie` header.
pub async fn send(
    app: &Router,
    cookie: Option<&str>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> Reply {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        req = req.header(header::COOKIE, cookie);
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let set_cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = axum::body::to_bytes(res.into_body(), MAX_TEST_BODY)
        .await
        .unwrap();
    Reply {
        status,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        set_cookie,
    }
}

/// A campaign of the GM behind `session`, imported from `yaml`. Returns
/// its id.
pub async fn imported_campaign(app: &Router, session: &str, yaml: &str) -> String {
    let r = call(
        app,
        Some(session),
        "POST",
        "/api/campaigns/import",
        Some(serde_json::json!({ "yaml": yaml })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["data"]["id"].as_str().unwrap().to_string()
}

/// A fresh invitation code for `campaign`.
pub async fn invite_code(app: &Router, session: &str, campaign: &str) -> String {
    let r = call(
        app,
        Some(session),
        "POST",
        &format!("/api/campaigns/{campaign}/invite"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["data"]["code"].as_str().unwrap().to_string()
}

/// Join with `code` as `nickname` in `role`; returns the reply, whose
/// `player_token()` is the device token.
pub async fn join(app: &Router, code: &str, nickname: &str, role: &str) -> Reply {
    call(
        app,
        None,
        "POST",
        &format!("/api/join/{code}"),
        Some(serde_json::json!({ "nickname": nickname, "role": role })),
    )
    .await
}

/// A GM account with an open session, created straight in the database
/// (the passkey ceremonies have their own tests). Returns the GM id and
/// the session token.
pub async fn signed_in_gm(pool: &PgPool, name: &str) -> (Uuid, String) {
    let id: Uuid = sqlx::query_scalar("INSERT INTO gms (display_name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let token = promptus_back::auth::session::create_in_tx(&mut tx, id)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    (id, token)
}
