use axum::Router;
use axum::http::{HeaderValue, Method};
use axum::middleware;
use axum::routing::{delete, get, post};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::api;
use crate::auth::guard::require_gm;
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
        .route_layer(middleware::from_fn_with_state(state.clone(), require_gm));

    public
        .merge(gm)
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
