use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// Open the connection pool used by the server.
///
/// Queries are bounded server-side so a client that disconnects in the
/// middle of a transaction cannot keep its row locks alive until the
/// connection is reclaimed. Acquiring a connection is bounded too, so a
/// database that went away turns into a quick 503 instead of a request
/// hanging for the default 30 s.
///
/// The live listener (`live::listener`) holds one connection for good:
/// the eleventh, so requests keep ten.
///
/// # Errors
///
/// Fails when the database cannot be reached at startup.
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(11)
        .acquire_timeout(Duration::from_secs(3))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                use sqlx::Executor;
                conn.execute("SET statement_timeout = '30s'").await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
}

/// Apply every pending migration from `back/migrations`.
///
/// # Errors
///
/// Fails when a migration does not apply, or when an already-applied
/// migration was edited after the fact (checksum mismatch).
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
