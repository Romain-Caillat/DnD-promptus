//! What stands in front of every shared-screen route — the screen's twin
//! of `player` and `guard` (session/pair-shared-screen).
//!
//! A screen has no account: saying hello (`POST /api/tv`) leaves a
//! random token on the device, in an HttpOnly cookie whose `Path` is the
//! screen API (`/api/tv`). Before a GM pairs it, the token opens nothing
//! but the hello and the QR code; once paired, it opens the routes of
//! [`require_screen`], and only for the campaign it is paired with —
//! which no path names, so a screen cannot ask for another campaign.
//!
//! A GM session opens no screen route and a screen token opens no GM or
//! player route: the cookies differ in name and path, and each guard
//! reads only its own. That is what keeps the window the GM shares on
//! Discord from ever showing GM data, though it lives in the GM's
//! browser.

use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, header};
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

use crate::error::AppError;
use crate::screens;
use crate::state::AppState;

pub const COOKIE_NAME: &str = "promptus_screen";
/// Every screen route lives under it: the cookie's `Path`.
pub const PATH: &str = "/api/tv";
/// Browsers cap a cookie's life at 400 days; the token itself lasts
/// until the GM forgets the screen.
const COOKIE_DAYS: i64 = 400;

/// The paired screen behind a request of the screen router.
#[derive(Debug, Clone, Copy)]
pub struct CurrentScreen {
    pub id: Uuid,
    pub campaign_id: Uuid,
}

fn not_paired() -> AppError {
    AppError::Unauthorized("NOT_PAIRED")
}

/// Every screen token the browser sent.
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

/// `Set-Cookie` value that keeps `token` on the device.
pub fn set_cookie(token: &str, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    let max_age = COOKIE_DAYS * 24 * 60 * 60;
    format!(
        "{COOKIE_NAME}={token}; Path={PATH}; HttpOnly; SameSite=Strict; Max-Age={max_age}{secure}"
    )
}

/// Resolve the screen cookie to a paired screen, or answer 401.
///
/// # Errors
///
/// 401 `NOT_PAIRED` without the token of a paired screen; a database
/// error.
pub async fn require_screen(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let tokens = tokens_from_headers(req.headers());
    let screen = screens::find_by_token(&state.pool, &tokens)
        .await?
        .ok_or_else(not_paired)?;
    let campaign_id = screen.campaign_id.ok_or_else(not_paired)?;
    req.extensions_mut().insert(CurrentScreen {
        id: screen.id,
        campaign_id,
    });
    Ok(next.run(req).await)
}

impl<S: Send + Sync> FromRequestParts<S> for CurrentScreen {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .copied()
            .ok_or_else(not_paired)
    }
}
