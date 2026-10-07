use std::path::Path;

use axum::Router;
use axum::extract::DefaultBodyLimit;
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
        .route("/api/auth/sign-in", post(api::auth::sign_in))
        // Sprites: a description is drawn the same for GM, players and TV.
        .route("/api/sprites/render.png", get(api::sprites::render_png))
        .route("/api/sprites/sheet.png", get(api::sprites::sheet_png))
        .route("/api/sprites/looks", get(api::sprites::looks))
        .route("/api/sprites/packs/{pack}", get(api::sprites::pack));
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
            "/api/campaigns/{id}/settings",
            put(api::campaigns::update_settings),
        )
        .route("/api/campaigns/{id}/archive", put(api::campaigns::archive))
        .route("/api/rule-systems", get(api::campaigns::rule_systems))
        // campaign/edit-rule-system: the campaign's own rule versions.
        .route("/api/campaigns/{id}/rules", get(api::rule_versions::editor))
        .route(
            "/api/campaigns/{id}/rules/draft",
            post(api::rule_versions::start_draft)
                .put(api::rule_versions::save_draft)
                .delete(api::rule_versions::discard_draft),
        )
        .route(
            "/api/campaigns/{id}/rules/draft/lock",
            post(api::rule_versions::lock_draft),
        )
        .route(
            "/api/campaigns/{id}/rules/compare",
            get(api::rule_versions::compare),
        )
        .route("/api/campaigns/{id}/story/edits", post(api::prep::edits))
        .route("/api/campaigns/{id}/readiness", get(api::prep::readiness))
        .route(
            "/api/campaigns/{id}/generation",
            get(api::prep::generations).post(api::prep::generate),
        )
        .route(
            "/api/campaigns/{id}/generation/{job}/apply",
            post(api::prep::apply_generation),
        )
        .route(
            "/api/campaigns/{id}/story/validate",
            post(api::prep::validate),
        )
        .route(
            "/api/campaigns/{id}/workshop",
            get(api::prep::proposals).post(api::prep::ask),
        )
        .route(
            "/api/campaigns/{id}/workshop/{proposal}/accept",
            post(api::prep::accept),
        )
        .route(
            "/api/campaigns/{id}/workshop/{proposal}/reject",
            post(api::prep::reject),
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
        .route(
            "/api/campaigns/{id}/characters/{character}",
            get(api::table::character),
        )
        .route(
            "/api/campaigns/{id}/characters/{character}/validate",
            post(api::table::validate_character),
        )
        .route(
            "/api/campaigns/{id}/characters/{character}/return",
            post(api::table::return_character),
        )
        .route(
            "/api/campaigns/{id}/characters/{character}/note-draft",
            post(api::table::draft_note),
        )
        .route(
            "/api/campaigns/{id}/characters/{character}/hooks/propose",
            post(api::table::propose_hooks),
        )
        .route(
            "/api/campaigns/{id}/hooks",
            get(api::table::hooks).post(api::table::add_hook),
        )
        .route(
            "/api/campaigns/{id}/hooks/{hook}",
            put(api::table::edit_hook).delete(api::table::delete_hook),
        )
        // gm/adjust-sheets-fast: the characters in play, one gesture each.
        .route("/api/campaigns/{id}/sheets", get(api::sheets::board))
        .route(
            "/api/campaigns/{id}/characters/{character}/adjust",
            post(api::sheets::adjust),
        )
        // The evening (phase 3): lobby, scenes, requests, journal, end.
        .route(
            "/api/campaigns/{id}/session",
            get(api::evening::live_screen).post(api::evening::open),
        )
        .route(
            "/api/campaigns/{id}/session/start",
            post(api::evening::start),
        )
        .route("/api/campaigns/{id}/session/end", post(api::evening::end))
        .route(
            "/api/campaigns/{id}/session/reveal",
            post(api::evening::reveal),
        )
        .route(
            "/api/campaigns/{id}/session/music",
            put(api::evening::music),
        )
        .route(
            "/api/campaigns/{id}/session/journal",
            post(api::evening::note),
        )
        .route(
            "/api/campaigns/{id}/session/requests/{request}",
            post(api::evening::decide),
        )
        .route(
            "/api/campaigns/{id}/session/spotlight/{player}",
            post(api::evening::give_spotlight),
        )
        .route(
            "/api/campaigns/{id}/hooks/{hook}/played",
            put(api::evening::hook_played),
        )
        .route(
            "/api/campaigns/{id}/knowledge",
            get(api::evening::knowledge),
        )
        .route("/api/campaigns/{id}/sessions", get(api::evening::chronicle))
        .route(
            "/api/campaigns/{id}/sessions/{session}/recap",
            put(api::evening::edit_recap),
        )
        .route(
            "/api/campaigns/{id}/sessions/{session}/recap-draft",
            post(api::evening::recap_draft),
        )
        .route(
            "/api/campaigns/{id}/sessions/{session}/feedback",
            get(api::evening::feedback_report),
        )
        .route(
            "/api/campaigns/{id}/sessions/{session}/changes",
            put(api::evening::note_changes),
        )
        .route("/api/campaigns/{id}/ai", get(api::evening::ai_usage))
        // The grid and the fights.
        .route(
            "/api/campaigns/{id}/board",
            get(api::board::gm_board).post(api::board::show),
        )
        .route("/api/campaigns/{id}/board/edit", post(api::board::edit))
        .route("/api/campaigns/{id}/fight", post(api::board::start_fight))
        .route(
            "/api/campaigns/{id}/fight/command",
            post(api::board::gm_command),
        )
        .route(
            "/api/campaigns/{id}/fight/loot",
            post(api::board::give_loot),
        )
        // The campaign's own maps (maps/edit-map-gm, maps/import-image-map,
        // maps/generate-map-llm).
        .route(
            "/api/campaigns/{id}/maps",
            get(api::maps::list).post(api::maps::create),
        )
        .route(
            "/api/campaigns/{id}/maps/import",
            // An imported image travels as base64 in the JSON body.
            post(api::maps::import).layer(DefaultBodyLimit::max(
                crate::campaign_maps::MAX_BACKDROP_BYTES * 4 / 3 + 1024 * 1024,
            )),
        )
        .route(
            "/api/campaigns/{id}/maps/generate",
            post(api::maps::generate),
        )
        .route(
            "/api/campaigns/{id}/maps/{map}",
            get(api::maps::get)
                .put(api::maps::save)
                .delete(api::maps::delete)
                // The editor sends the whole map.
                .layer(DefaultBodyLimit::max(4 * 1024 * 1024)),
        )
        .route(
            "/api/campaigns/{id}/maps/{map}/validate",
            post(api::maps::validate),
        )
        .route(
            "/api/campaigns/{id}/maps/{map}/backdrop",
            get(api::maps::gm_backdrop),
        )
        // Pixel-art images, reviewed by the GM.
        .route(
            "/api/campaigns/{id}/media",
            get(api::media::gm_list).post(api::media::ask),
        )
        .route("/api/campaigns/{id}/media/batch", post(api::media::batch))
        .route(
            "/api/campaigns/{id}/media/{asset}/image",
            get(api::media::gm_image),
        )
        .route(
            "/api/campaigns/{id}/media/{asset}/decision",
            post(api::media::decide),
        )
        .route(
            "/api/campaigns/{id}/session/copilot",
            post(api::evening::copilot_ask),
        )
        .route(
            "/api/campaigns/{id}/session/copilot/voice",
            // The dictation travels as base64 in the JSON body.
            post(api::evening::copilot_voice).layer(DefaultBodyLimit::max(
                crate::copilot::voice::MAX_WAV_BYTES * 4 / 3 + 64 * 1024,
            )),
        )
        .route(
            "/api/campaigns/{id}/session/copilot/{draft}/show",
            post(api::evening::copilot_show),
        )
        .route(
            "/api/campaigns/{id}/session/copilot/{draft}/dismiss",
            post(api::evening::copilot_dismiss),
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
        // The character creator: what it offers, then the player's own
        // sheet — saved as a draft, then sent to the GM.
        (
            "GET",
            "/api/play/{campaign}/creation",
            get(api::play::creation),
        ),
        (
            "PUT",
            "/api/play/{campaign}/character",
            put(api::play::save_character),
        ),
        // copilot/co-write-backstory: the three answers made a paragraph.
        (
            "POST",
            "/api/play/{campaign}/character/backstory",
            post(api::play::write_backstory),
        ),
        (
            "POST",
            "/api/play/{campaign}/character/submit",
            post(api::play::submit_character),
        ),
        // The campaign's rules, and « I read what changed ».
        ("GET", "/api/play/{campaign}/rules", get(api::rules::rules)),
        (
            "POST",
            "/api/play/{campaign}/rules/seen",
            post(api::rules::mark_seen),
        ),
        // In play: what the character carries is the player's choice.
        (
            "POST",
            "/api/play/{campaign}/character/equip",
            post(api::play::equip),
        ),
        // The evening: the scene, the music, my requests and their rolls,
        // the journal; the lobby; the feedback at the end.
        (
            "GET",
            "/api/play/{campaign}/evening",
            get(api::play_evening::evening),
        ),
        (
            "POST",
            "/api/play/{campaign}/lobby",
            post(api::play_evening::arrive),
        ),
        (
            "POST",
            "/api/play/{campaign}/requests",
            post(api::play_evening::ask),
        ),
        (
            "POST",
            "/api/play/{campaign}/requests/{request}/roll",
            post(api::play_evening::roll),
        ),
        (
            "POST",
            "/api/play/{campaign}/requests/{request}/withdraw",
            post(api::play_evening::withdraw),
        ),
        (
            "POST",
            "/api/play/{campaign}/requests/{request}/contest",
            post(api::play_evening::contest),
        ),
        (
            "POST",
            "/api/play/{campaign}/feedback",
            post(api::play_evening::answer_feedback),
        ),
        // The grid: the map as I may see it, my walk, my fight turn.
        (
            "GET",
            "/api/play/{campaign}/board",
            get(api::board::player_board),
        ),
        (
            "GET",
            "/api/play/{campaign}/board/backdrop",
            get(api::maps::player_backdrop),
        ),
        (
            "POST",
            "/api/play/{campaign}/board/walk",
            post(api::board::walk),
        ),
        (
            "POST",
            "/api/play/{campaign}/fight",
            post(api::board::command),
        ),
        // The images the table may see, and the world's theme.
        (
            "GET",
            "/api/play/{campaign}/media",
            get(api::media::player_list),
        ),
        (
            "GET",
            "/api/play/{campaign}/media/{asset}/image",
            get(api::media::player_image),
        ),
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
