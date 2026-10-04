//! Shared helpers for integration tests.
//!
//! Cargo treats files directly under `tests/` as standalone test binaries,
//! so common helpers live in `tests/common/mod.rs`. Each test file that
//! needs them includes this module with `mod common;`.

#![allow(dead_code)]

use sqlx::PgPool;

/// Cap on a response body read in tests — every response here is a
/// small JSON envelope.
pub const MAX_TEST_BODY: usize = 1024 * 1024;

/// Connect to the **test** database and apply the migrations.
///
/// Integration tests never touch the dev database: they read
/// `TEST_DATABASE_URL` (a separate `promptus_test` database in the dev
/// Postgres container, see `docker-compose.dev.yml`). A missing variable
/// fails the test loudly rather than skipping it — a skipped suite reads
/// as green and hides that nothing ran.
pub async fn test_pool() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL must be set to run integration tests \
         (copy .env.example to .env, then `bun run db:start`)",
    );
    if std::env::var("DATABASE_URL").is_ok_and(|dev| dev == url) {
        panic!("TEST_DATABASE_URL equals DATABASE_URL — tests would write to the dev database");
    }
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect to the test database");
    promptus_back::db::migrate(&pool)
        .await
        .expect("apply migrations to the test database");
    pool
}

/// Consume an axum response body and decode it as JSON.
pub async fn read_json(response: axum::response::Response) -> serde_json::Value {
    let body = axum::body::to_bytes(response.into_body(), MAX_TEST_BODY)
        .await
        .expect("read response body");
    serde_json::from_slice(&body).expect("decode response as JSON")
}
