//! What stands in front of every player route — the player's twin of
//! `guard` (GM).
//!
//! A player has no account: joining a campaign leaves a random token on
//! the device, in an HttpOnly cookie whose `Path` is that campaign's
//! player API (`/api/play/<campaign id>`). The browser therefore sends it
//! to that campaign's routes and nowhere else — not to another campaign,
//! not to a GM route — and a phone in two campaigns holds two cookies
//! that never meet. The server stores the token's hash only
//! (`players.token_hash`).
//!
//! Two layers, as for the GM:
//! - [`require_player`], a middleware on the whole player router
//!   (`app::router`): no token of a player of the campaign named in the
//!   path, no handler runs (401 `NOT_JOINED`);
//! - [`CurrentPlayer`], the extractor handlers take; it refuses (401)
//!   when the middleware resolved nothing, so a handler mounted outside
//!   the player router by mistake fails closed.
//!
//! A GM session opens no player route, and a player token opens no GM
//! route: the cookies differ in name and path, and each guard reads only
//! its own.

use std::collections::HashMap;

use axum::extract::{FromRequestParts, Path, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, header};
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

use crate::error::AppError;
use crate::players::{self, Player};
use crate::state::AppState;

pub const COOKIE_NAME: &str = "promptus_player";
/// Browsers cap a cookie's life at 400 days; the token itself never
/// expires (the GM removes a player to end it).
const COOKIE_DAYS: i64 = 400;

/// The player behind a request of the player router.
#[derive(Debug, Clone)]
pub struct CurrentPlayer(pub Player);

fn not_joined() -> AppError {
    AppError::Unauthorized("NOT_JOINED")
}

/// The path every player route of `campaign_id` lives under: the
/// cookie's `Path`.
pub fn play_path(campaign_id: Uuid) -> String {
    format!("/api/play/{campaign_id}")
}

/// Every player token the browser sent (one per cookie path that
/// matched; normally one).
pub fn tokens_from_headers(headers: &HeaderMap) -> Vec<String> {
    let prefix = format!("{COOKIE_NAME}=");
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|part| part.trim().strip_prefix(prefix.as_str()))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

/// `Set-Cookie` value that keeps `token` on the device for
/// `campaign_id`. `SameSite=Strict` as for the GM: no other site's page
/// can make the browser send it.
pub fn set_cookie(campaign_id: Uuid, token: &str, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    let max_age = COOKIE_DAYS * 24 * 60 * 60;
    format!(
        "{COOKIE_NAME}={token}; Path={}; HttpOnly; SameSite=Strict; Max-Age={max_age}{secure}",
        play_path(campaign_id)
    )
}

/// Resolve the player cookie to a player of the campaign in the path,
/// or answer 401.
///
/// # Errors
///
/// 401 `NOT_JOINED` without a token of a player of this campaign (a
/// malformed campaign id included); a database error.
pub async fn require_player(
    State(state): State<AppState>,
    Path(params): Path<HashMap<String, String>>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let campaign_id = params
        .get("campaign")
        .and_then(|raw| Uuid::parse_str(raw).ok())
        .ok_or_else(not_joined)?;
    let tokens = tokens_from_headers(req.headers());
    let player = players::find_by_token(&state.pool, campaign_id, &tokens)
        .await?
        .ok_or_else(not_joined)?;
    req.extensions_mut().insert(CurrentPlayer(player));
    Ok(next.run(req).await)
}

impl<S: Send + Sync> FromRequestParts<S> for CurrentPlayer {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or_else(not_joined)
    }
}
