mod common;

use std::path::PathBuf;

use axum::Router;
use axum::body::Body;
use axum::http::header::CACHE_CONTROL;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

/// A built front in miniature: the shell and one hashed asset, in a
/// directory of its own per test.
fn front_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("promptus-front-{name}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(
        dir.join("index.html"),
        "<!doctype html><title>shell</title>",
    )
    .unwrap();
    std::fs::write(dir.join("assets/app-abc123.js"), "console.log(1)").unwrap();
    dir
}

/// The front is served without touching the database: a lazy pool that
/// never connects is enough.
fn app(name: &str) -> Router {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .unwrap();
    promptus_back::app::with_front(promptus_back::app::router(pool, &[]), &front_dir(name))
}

async fn get(app: Router, uri: &str) -> axum::response::Response {
    app.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), common::MAX_TEST_BODY)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn a_client_route_opens_the_app_shell_uncached() {
    // A pasted invite link must open the app, not a 404.
    let response = get(app("route"), "/join/some-invite-code").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CACHE_CONTROL], "no-cache");
    assert!(body_text(response).await.contains("<title>shell</title>"));
}

#[tokio::test]
async fn hashed_assets_are_cached_and_a_missing_one_is_a_404() {
    let response = get(app("assets"), "/assets/app-abc123.js").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("immutable")
    );

    // Not the shell: a browser holding a stale page must not cache HTML
    // under a script's name forever.
    let missing = get(app("missing"), "/assets/gone-000000.js").await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_unknown_api_path_is_a_json_404_not_the_shell() {
    let response = get(app("api"), "/api/no-such-route").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = common::read_json(response).await;
    assert_eq!(json["error"]["code"], "ROUTE_NOT_FOUND");
}
