//! Shared helpers for integration tests.
//!
//! Cargo treats files directly under `tests/` as standalone test binaries,
//! so common helpers live in `tests/common/mod.rs`. Each test file that
//! needs them includes this module with `mod common;`.

#![allow(dead_code)]

use std::str::FromStr;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use promptus_back::auth::setup::SetupState;
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
    let pool = PgPoolOptions::new()
        .max_connections(2)
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

pub fn app_with(pool: PgPool, setup: SetupState) -> Router {
    promptus_back::app::router(
        AppState {
            pool,
            auth: auth(setup),
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
}

/// Send one request; `session` is the GM session cookie to carry.
pub async fn call(
    app: &Router,
    session: Option<&str>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> Reply {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(token) = session {
        req = req.header(header::COOKIE, format!("promptus_gm={token}"));
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
