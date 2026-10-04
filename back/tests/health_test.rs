mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn get_health() -> Request<Body> {
    Request::builder()
        .uri("/api/health")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn health_is_ok_when_the_database_answers() {
    let pool = common::test_pool().await;
    let app = common::app(pool);

    let response = app.oneshot(get_health()).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = common::read_json(response).await;
    assert_eq!(json["data"]["status"], "ok");
}

#[tokio::test]
async fn health_is_unavailable_when_the_database_is_down() {
    // Nothing listens on port 1: every connection attempt is refused,
    // which is what the server sees when Postgres goes away.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy("postgres://promptus:promptus@127.0.0.1:1/promptus")
        .unwrap();
    let app = common::app(pool);

    let response = app.oneshot(get_health()).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json = common::read_json(response).await;
    assert_eq!(json["error"]["code"], "DATABASE_UNAVAILABLE");
}
