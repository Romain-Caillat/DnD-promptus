//! The live socket of a campaign (`session/stream-live-changes`):
//! - `GET /api/campaigns/{id}/live` → the GM's socket, behind
//!   `require_gm` like every GM route; another GM's campaign answers 404
//!   before any upgrade.
//!
//! The protocol is in `live::socket`. The player socket will be its own
//! route with the player extractor of `session/invite-and-join`, ending
//! in the same `live::socket::upgrade` with a `Viewer::Player`.

use axum::extract::ws::rejection::WebSocketUpgradeRejection;
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::Response;
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;
use crate::live::{Viewer, socket};
use crate::state::AppState;

/// `GET /api/campaigns/{id}/live`
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's (checked first, so
/// a stranger learns nothing); 400 `WEBSOCKET_REQUIRED` for a plain
/// HTTP request; a database error.
pub async fn gm_socket(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Result<Response, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError::NotFound("NOT_FOUND"))?;
    owned_by(campaigns::find(&state.pool, id).await?, &gm)?;
    let ws = ws.map_err(|_| AppError::BadRequest("WEBSOCKET_REQUIRED"))?;
    Ok(socket::upgrade(
        ws,
        state.pool,
        state.live,
        id,
        Viewer::Gm(gm.id),
    ))
}
