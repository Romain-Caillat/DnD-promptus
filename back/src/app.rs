use std::path::Path;

use axum::Router;
use axum::http::header::CACHE_CONTROL;
use axum::http::{HeaderValue, Method};
use axum::routing::{any, get};
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeader;
use tower_http::trace::TraceLayer;

use crate::api;
use crate::error::AppError;

/// Build the full HTTP router. Kept out of `main` so integration tests
/// exercise exactly the routes the server serves.
pub fn router(pool: PgPool, allowed_origins: &[String]) -> Router {
    let origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(api::health::health))
        // An unknown API path is a JSON 404, never the app shell that
        // `with_front` serves for every other path.
        .route("/api/{*rest}", any(api_not_found))
        .with_state(pool)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

async fn api_not_found() -> AppError {
    AppError::NotFound("ROUTE_NOT_FOUND")
}

/// Serve the built front (`front/dist`) next to the API, so production
/// is one origin and one process (docs/install.md).
///
/// - `/assets/*` are Vite's hashed files: cached forever, and a missing
///   one is a plain 404 rather than the app shell.
/// - Any other path is a file of `dir` when one exists, otherwise
///   `index.html`, so a reload or a pasted link on a client-side route
///   (an invite link, say) opens the app. The shell is never cached, so
///   a deploy reaches browsers at their next load.
pub fn with_front(router: Router, dir: &Path) -> Router {
    let assets = SetResponseHeader::overriding(
        ServeDir::new(dir.join("assets")),
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    let shell = SetResponseHeader::overriding(
        ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html"))),
        CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    router
        .nest_service("/assets", assets)
        .fallback_service(shell)
}
