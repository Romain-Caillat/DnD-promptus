use std::path::Path;

use axum::Router;
use axum::http::header::CACHE_CONTROL;
use axum::http::{HeaderValue, Method};
use axum::middleware;
use axum::routing::{MethodRouter, any, delete, get, post, put};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeader;
use tower_http::trace::TraceLayer;

use crate::api;
use crate::auth::guard::require_gm;
use crate::auth::player::require_player;
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
        .route("/api/auth/sign-in", post(api::auth::sign_in));
    let public = invitation_routes()
        .into_iter()
        .fold(public, |r, (_, path, handler)| r.route(path, handler))
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
        .route(
            "/api/campaigns/{id}/invite",
            get(api::table::invite)
                .post(api::table::mint_invite)
                .delete(api::table::revoke_invite),
        )
        .route("/api/campaigns/{id}/players", get(api::table::seats))
        .route(
            "/api/campaigns/{id}/players/{player}",
            delete(api::table::remove_player),
        )
        .route("/api/campaigns/{id}/live", get(api::live::gm_socket))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_gm));

    // Every player route: `require_player` refuses a request without the
    // device token of a player of the campaign in the path.
    let player = player_routes()
        .into_iter()
        .fold(Router::new(), |r, (_, path, handler)| {
            r.route(path, handler)
        })
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_player,
        ));

    public
        .merge(gm)
        .merge(player)
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

/// A route as the router mounts it: method (for the sweep), path,
/// handler.
type RouteSpec = (&'static str, &'static str, MethodRouter<AppState>);

/// Every route a player or the shared screen calls once seated, mounted
/// behind `require_player`. Each must build its answer with
/// `campaigns::projection` (`MEMORY.md` §3): `tests/player_routes_test.rs`
/// calls every route of this list and fails on any GM-only marker, so a
/// route cannot be added here without being swept. Shared-screen (TV)
/// routes belong here too.
fn player_routes() -> Vec<RouteSpec> {
    vec![
        ("GET", "/api/play/{campaign}/me", get(api::play::me)),
        ("GET", "/api/play/{campaign}/view", get(api::play::view)),
        // A socket: it carries versions and presence, never data.
        (
            "GET",
            "/api/play/{campaign}/live",
            get(api::live::player_socket),
        ),
    ]
}

/// The routes an invitation code opens, before any seat is taken:
/// public, and swept like the player routes.
fn invitation_routes() -> Vec<RouteSpec> {
    vec![
        ("GET", "/api/join/{code}", get(api::play::invitation)),
        ("POST", "/api/join/{code}", post(api::play::join)),
    ]
}

/// Method and path of every player and invitation route, for the sweep.
pub fn player_facing_routes() -> Vec<(&'static str, &'static str)> {
    invitation_routes()
        .into_iter()
        .chain(player_routes())
        .map(|(method, path, _)| (method, path))
        .collect()
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
