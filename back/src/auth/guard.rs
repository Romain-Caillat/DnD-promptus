//! What stands in front of every GM route.
//!
//! Two layers, so forgetting one is not enough to open a route:
//! - [`require_gm`] is a middleware on the whole GM router
//!   (`app::router`): no valid session cookie, no handler runs (401);
//! - [`CurrentGm`] is the extractor handlers take; it reads what the
//!   middleware resolved and refuses (401) when nothing is there, so a
//!   handler mounted outside the GM router by mistake fails closed.
//!
//! Then, per resource, [`owned_by`]: a row another GM owns answers
//! exactly like a row that does not exist (404), so ids cannot be probed
//! across accounts. Campaigns will use it the same way:
//!
//! ```ignore
//! let campaign = owned_by(campaigns::find(&mut tx, id).await?, &gm)?;
//! ```
//!
//! For a world write, run it on the row read with `SELECT … FOR UPDATE`
//! inside the transaction (`MEMORY.md` §3, one lock per campaign).
//!
//! A personal access token (`Authorization: Bearer`, [`api_tokens`])
//! passes the same middleware, but only on the routes of
//! `api_tokens::TOKEN_ROUTES`: anywhere else it is refused (403
//! `TOKEN_NOT_ALLOWED`) before any lookup. When the header is there the
//! cookie is not read. Handlers that must never run for a token take
//! [`SessionOnly`] as well, so a route added to the list by mistake
//! still fails closed.
//!
//! Status codes: 401 when there is no GM session at all (the front then
//! shows the sign-in page); 404 when the GM is signed in but the
//! resource is not theirs; 403 is kept for a refusal that does not depend
//! on which resource is asked (a wrong registration code).

use axum::extract::{FromRequestParts, MatchedPath, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, header};
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

pub use super::session::CurrentGm;
use super::{api_tokens, session};
use crate::error::AppError;
use crate::state::AppState;

fn unauthenticated() -> AppError {
    AppError::Unauthorized("UNAUTHENTICATED")
}

/// How the GM of a request proved who they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Credential {
    /// The session cookie of the GM's browser.
    Session,
    /// A personal access token (`Authorization: Bearer`).
    Token,
}

/// The bearer token of the `Authorization` header, if any.
pub fn bearer_from_headers(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.trim().split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then(|| token.to_string())
}

/// Resolve the session cookie — or, on the routes a token may reach,
/// the bearer token — to a GM, or answer 401.
///
/// # Errors
///
/// 401 `UNAUTHENTICATED` without a cookie, or with one whose session is
/// unknown, expired or signed out; 401 `INVALID_TOKEN` for an unknown or
/// revoked token; 403 `TOKEN_NOT_ALLOWED` for a token on a route outside
/// `api_tokens::TOKEN_ROUTES`.
pub async fn require_gm(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    if let Some(secret) = bearer_from_headers(req.headers()) {
        let path = req
            .extensions()
            .get::<MatchedPath>()
            .map(MatchedPath::as_str);
        if !api_tokens::allows(req.method(), path) {
            return Err(AppError::Forbidden("TOKEN_NOT_ALLOWED"));
        }
        let gm = api_tokens::find_gm(&state.pool, &secret)
            .await?
            .ok_or(AppError::Unauthorized("INVALID_TOKEN"))?;
        req.extensions_mut().insert(gm);
        req.extensions_mut().insert(Credential::Token);
        return Ok(next.run(req).await);
    }
    let token = session::token_from_headers(req.headers()).ok_or_else(unauthenticated)?;
    let gm = session::find_gm(&state.pool, &token)
        .await?
        .ok_or_else(unauthenticated)?;
    req.extensions_mut().insert(gm);
    req.extensions_mut().insert(Credential::Session);
    Ok(next.run(req).await)
}

/// Taken by the handlers a token must never reach (minting and revoking
/// tokens): refuses anything but the GM's own session cookie.
#[derive(Debug, Clone, Copy)]
pub struct SessionOnly;

impl<S: Send + Sync> FromRequestParts<S> for SessionOnly {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match parts.extensions.get::<Credential>() {
            Some(Credential::Session) => Ok(Self),
            Some(Credential::Token) => Err(AppError::Forbidden("TOKEN_NOT_ALLOWED")),
            None => Err(unauthenticated()),
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for CurrentGm {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or_else(unauthenticated)
    }
}

/// A row that belongs to one GM.
pub trait OwnedByGm {
    fn owner_id(&self) -> Uuid;
}

/// The row, if it exists **and** belongs to `gm`; 404 `NOT_FOUND`
/// otherwise — the same answer in both cases.
///
/// # Errors
///
/// 404 `NOT_FOUND` when `row` is `None` or owned by another GM.
pub fn owned_by<T: OwnedByGm>(row: Option<T>, gm: &CurrentGm) -> Result<T, AppError> {
    match row {
        Some(r) if r.owner_id() == gm.id => Ok(r),
        _ => Err(AppError::NotFound("NOT_FOUND")),
    }
}
