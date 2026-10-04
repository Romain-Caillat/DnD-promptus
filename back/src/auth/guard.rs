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
//! Status codes: 401 when there is no GM session at all (the front then
//! shows the sign-in page); 404 when the GM is signed in but the
//! resource is not theirs; 403 is kept for a refusal that does not depend
//! on which resource is asked (a wrong registration code).

use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

use super::session;
pub use super::session::CurrentGm;
use crate::error::AppError;
use crate::state::AppState;

fn unauthenticated() -> AppError {
    AppError::Unauthorized("UNAUTHENTICATED")
}

/// Resolve the session cookie to a GM, or answer 401.
///
/// # Errors
///
/// 401 `UNAUTHENTICATED` without a cookie, or with one whose session is
/// unknown, expired or signed out.
pub async fn require_gm(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = session::token_from_headers(req.headers()).ok_or_else(unauthenticated)?;
    let gm = session::find_gm(&state.pool, &token)
        .await?
        .ok_or_else(unauthenticated)?;
    req.extensions_mut().insert(gm);
    Ok(next.run(req).await)
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
