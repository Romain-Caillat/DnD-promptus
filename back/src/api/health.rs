use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::error::AppError;

/// `GET /api/health` — 200 when the server is up **and** can round-trip
/// a query to the database, 503 `DATABASE_UNAVAILABLE` otherwise. The
/// server is useless without its database, so a health check that
/// ignored it would report a broken instance as healthy.
///
/// # Errors
///
/// Returns `AppError::ServiceUnavailable` when the database query fails.
pub async fn health(State(pool): State<PgPool>) -> Result<Json<Value>, AppError> {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => Ok(Json(json!({ "data": { "status": "ok" } }))),
        Err(e) => {
            tracing::error!(error = %e, "health check failed: database round-trip errored");
            Err(AppError::ServiceUnavailable("DATABASE_UNAVAILABLE"))
        }
    }
}
