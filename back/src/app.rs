use std::path::Path;

use axum::Router;
use axum::http::header::CACHE_CONTROL;
use axum::http::{HeaderValue, Method};
use axum::middleware;
use axum::routing::{any, delete, get, post, put};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeader;
use tower_http::trace::TraceLayer;

use crate::api;
use crate::auth::guard::require_gm;
use crate::error::AppError;
use crate::state::AppState;

/// Build the full HTTP router. Kept out of `main` so integration tests
/// exercise exactly the routes the server serves.
pub fn router(state: AppState, allowed_origins: &[String]) -> Router {
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

    // Reachable without an account. Keep this list short: anything
    // added here is open to the whole Internet.
    let public = Router::new()
        .route("/api/health", get(api::health::health))
        .route("/api/auth/status", get(api::auth::status))
        .route(
            "/api/auth/register/options",
            post(api::auth::register_options),
        )
        .route("/api/auth/register", post(api::auth::register))
        .route(
            "/api/auth/sign-in/options",
            post(api::auth::sign_in_options),
        )
        .route("/api/auth/sign-in", post(api::auth::sign_in))
        // Sprites: a description is drawn the same for GM, players and TV.
        .route("/api/sprites/render.png", get(api::sprites::render_png))
        .route("/api/sprites/looks", get(api::sprites::looks))
        // An unknown API path is a JSON 404, never the app shell that
        // `with_front` serves for every other path. A wildcard loses to
        // every exact route, so it never shadows a GM route, and it sits
        // outside `require_gm`: an unknown path is 404, not 401.
        .route("/api/{*rest}", any(api_not_found));

    // Every GM route goes here: `require_gm` refuses a request without a
    // valid GM session before any handler runs. `tests/gm_routes_test.rs`
    // lists them all.
    let gm = Router::new()
        .route("/api/me", get(api::gm::me))
        .route("/api/auth/sign-out", post(api::auth::sign_out))
        .route(
            "/api/gm-invites",
            get(api::gm::list_invites).post(api::gm::create_invite),
        )
        .route("/api/gm-invites/{id}", delete(api::gm::revoke_invite))
        .route(
            "/api/campaigns",
            get(api::campaigns::list).post(api::campaigns::create),
        )
        .route("/api/campaigns/import", post(api::campaigns::import_new))
        .route("/api/campaigns/{id}", get(api::campaigns::get))
        .route("/api/campaigns/{id}/export", get(api::campaigns::export))
        .route(
            "/api/campaigns/{id}/import",
            put(api::campaigns::import_replace),
        )
        .route(
            "/api/campaigns/{id}/player-view",
            get(api::campaigns::player_view),
        )
        .route("/api/campaigns/{id}/live", get(api::live::gm_socket))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_gm));

    public
        .merge(gm)
        .with_state(state)
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
