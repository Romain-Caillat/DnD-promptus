//! The shared screen (session/pair-shared-screen, tv/show-evening).
//!
//! The screen's own routes, under `/api/tv` where its cookie goes:
//! - `POST /api/tv` (public) → hello: a new screen gets its token and a
//!   code; a known one learns whether it is paired, its code renewed
//!   when it expired;
//! - `GET  /api/tv/qr.svg?origin=…` (public) → the QR of the pairing
//!   page for this screen's code;
//! - behind `require_screen`, for the campaign it is paired with:
//!   `GET /api/tv/show` (the evening, `projection::screen`),
//!   `GET /api/tv/media`, `GET /api/tv/media/{asset}/image`,
//!   `GET /api/tv/board/backdrop`, `GET /api/tv/live` (the socket).
//!
//! The GM's, behind `require_gm` (another GM's campaign answers 404):
//! - `GET    /api/campaigns/{id}/screens` → the paired screens, online
//!   or not, and what they may show;
//! - `POST   /api/campaigns/{id}/screens` → `{ code }`: pair a TV (201);
//! - `POST   /api/campaigns/{id}/screens/window` → a window for this
//!   browser, paired at birth (201, sets the screen cookie);
//! - `DELETE /api/campaigns/{id}/screens/{screen}` → forget it;
//! - `PUT    /api/campaigns/{id}/screens/shows` → what they may show.

use std::collections::HashMap;

use axum::Json;
use axum::extract::ws::rejection::WebSocketUpgradeRejection;
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use qrcode::QrCode;
use qrcode::render::svg;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use super::media::{image_response, visible_in};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::auth::screen::{self, CurrentScreen};
use crate::campaign_maps;
use crate::campaigns::projection::screen::{ScreenInput, project_screen};
use crate::campaigns::{self, CampaignRow};
use crate::content;
use crate::error::AppError;
use crate::evening::knowledge;
use crate::evening::requests;
use crate::evening::session;
use crate::live::{Viewer, socket};
use crate::media;
use crate::players::play;
use crate::screens::{self, Screen, ScreenKind, Shows};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

/// Where a screen stands, as it is told on hello.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HelloView {
    paired: bool,
    /// The code to type, while waiting.
    code: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

fn hello_view(s: &Screen) -> HelloView {
    HelloView {
        paired: s.campaign_id.is_some(),
        code: s.code.clone(),
        expires_at: s.code_expires_at,
    }
}

/// `POST /api/tv` — public. Sets the screen cookie for a new screen.
///
/// # Errors
///
/// 503 `TOO_MANY_SCREENS`; a database error.
pub async fn hello(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let tokens = screen::tokens_from_headers(&headers);
    let hello = screens::hello(&state.pool, &tokens).await?;
    let body = Json(json!({ "data": hello_view(&hello.screen) }));
    Ok(match hello.token {
        Some(token) => (
            [(
                header::SET_COOKIE,
                screen::set_cookie(&token, state.auth.secure_cookie),
            )],
            body,
        )
            .into_response(),
        None => body.into_response(),
    })
}

/// An origin as the page reads it (`location.origin`): a scheme and a
/// host, nothing after.
fn plain_origin(raw: &str) -> Option<&str> {
    let rest = raw
        .strip_prefix("https://")
        .or_else(|| raw.strip_prefix("http://"))?;
    let host_ok = !rest.is_empty()
        && rest.len() <= 200
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'));
    host_ok.then_some(raw)
}

/// `GET /api/tv/qr.svg?origin=https://…` — public: the QR of the page a
/// GM opens to pair this screen, for its own code only.
///
/// # Errors
///
/// 404 `NO_CODE` for a screen that is paired or unknown; 400
/// `INVALID_ORIGIN`.
pub async fn qr(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let origin = q
        .get("origin")
        .and_then(|o| plain_origin(o))
        .ok_or(AppError::BadRequest("INVALID_ORIGIN"))?;
    let tokens = screen::tokens_from_headers(&headers);
    let code = screens::find_by_token(&state.pool, &tokens)
        .await?
        .and_then(|s| s.code)
        .ok_or(AppError::NotFound("NO_CODE"))?;
    let url = format!("{origin}/tv/jumeler?code={code}");
    let svg = QrCode::new(url.as_bytes())
        .map_err(|e| AppError::internal("qr code", e))?
        .render::<svg::Color<'_>>()
        .min_dimensions(240, 240)
        .dark_color(svg::Color("#0a0a0a"))
        .light_color(svg::Color("#f2f2f2"))
        .build();
    Ok((
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        svg,
    )
        .into_response())
}

async fn campaign_of(state: &AppState, s: &CurrentScreen) -> Result<CampaignRow, AppError> {
    campaigns::find(&state.pool, s.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_PAIRED"))
}

/// `GET /api/tv/show` — the evening as the screen may show it.
///
/// # Errors
///
/// 401 `NOT_PAIRED`; a database error.
pub async fn show(State(state): State<AppState>, s: CurrentScreen) -> Result<Response, AppError> {
    let pool = &state.pool;
    let row = campaign_of(&state, &s).await?;
    let shows = screens::shows(pool, row.id).await?;
    let current = session::current(pool, row.id).await?;
    let last = session::last_ended(pool, row.id).await?;
    let attendance = match &current {
        Some(c) => session::attendance(pool, c.id).await?,
        None => Vec::new(),
    };
    let requests = match &current {
        Some(c) => requests::of_session(pool, c.id).await?,
        None => Vec::new(),
    };
    let in_play = play::in_play(pool, row.id).await?;
    let journal = knowledge::journal(pool, row.id, true).await?;
    let board = super::board::seen_by(&state, row.id, None).await?;
    let view = project_screen(ScreenInput {
        campaign: &row.story,
        world: &row.world,
        rules: row.rules(),
        shows,
        current: current.as_ref(),
        last_ended: last.as_ref(),
        attendance: &attendance,
        in_play: &in_play,
        journal: &journal,
        requests: &requests,
        board,
    });
    Ok(Json(json!({ "data": view })).into_response())
}

/// `GET /api/tv/media` — the images every player may see, and the
/// world's theme.
///
/// # Errors
///
/// 401 `NOT_PAIRED`.
pub async fn media_list(
    State(state): State<AppState>,
    s: CurrentScreen,
) -> Result<Response, AppError> {
    let (row, assets) = visible_in(&state, s.campaign_id).await?;
    let assets: Vec<_> = assets
        .iter()
        .map(|a| json!({ "id": a.id, "kind": a.kind, "subject": a.subject }))
        .collect();
    Ok(Json(json!({ "data": {
        "assets": assets,
        "theme": content::theme(&row.story),
    } }))
    .into_response())
}

/// `GET /api/tv/media/{asset}/image`
///
/// # Errors
///
/// 401 `NOT_PAIRED`; 404 `NO_SUCH_ASSET` (also for one the table may
/// not see).
pub async fn image(
    State(state): State<AppState>,
    s: CurrentScreen,
    headers: HeaderMap,
    Path(asset): Path<String>,
) -> Result<Response, AppError> {
    let id = Uuid::parse_str(&asset).map_err(|_| AppError::NotFound("NO_SUCH_ASSET"))?;
    let (row, assets) = visible_in(&state, s.campaign_id).await?;
    if !assets.iter().any(|a| a.id == id) {
        return Err(AppError::NotFound("NO_SUCH_ASSET"));
    }
    let (_, bytes, mime) = media::image(&state.pool, row.id, id).await?;
    Ok(image_response(&headers, bytes, mime))
}

/// `GET /api/tv/board/backdrop` — the image behind the map shown.
///
/// # Errors
///
/// 401 `NOT_PAIRED`; 404 `NO_SUCH_MAP` when no map with an image is
/// shown.
pub async fn backdrop(
    State(state): State<AppState>,
    s: CurrentScreen,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let shown: Option<String> =
        sqlx::query_scalar("SELECT map_id FROM map_states WHERE campaign_id = $1")
            .bind(s.campaign_id)
            .fetch_optional(&state.pool)
            .await?;
    let shown = shown.ok_or(AppError::NotFound("NO_SUCH_MAP"))?;
    let (bytes, mime) = campaign_maps::backdrop(&state.pool, s.campaign_id, &shown).await?;
    Ok(image_response(&headers, bytes, mime))
}

/// `GET /api/tv/live` — the screen's socket: versions and presence,
/// never data, like every other.
///
/// # Errors
///
/// 401 `NOT_PAIRED`; 400 `WEBSOCKET_REQUIRED` for a plain request.
pub async fn live(
    State(state): State<AppState>,
    s: CurrentScreen,
    ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Result<Response, AppError> {
    let ws = ws.map_err(|_| AppError::BadRequest("WEBSOCKET_REQUIRED"))?;
    Ok(socket::upgrade(
        ws,
        state.pool,
        state.live,
        s.campaign_id,
        Viewer::Screen(s.id),
    ))
}

// ——— The GM's side ———

async fn owned_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GmScreen {
    id: Uuid,
    kind: ScreenKind,
    paired_at: Option<DateTime<Utc>>,
    online: bool,
}

async fn gm_json(state: &AppState, campaign: Uuid) -> Result<serde_json::Value, AppError> {
    let presence = state.live.presence(campaign);
    let list: Vec<GmScreen> = screens::of_campaign(&state.pool, campaign)
        .await?
        .into_iter()
        .map(|s| GmScreen {
            online: presence.screens.contains(&s.id),
            id: s.id,
            kind: s.kind,
            paired_at: s.paired_at,
        })
        .collect();
    Ok(json!({
        "screens": list,
        "shows": screens::shows(&state.pool, campaign).await?,
    }))
}

/// `GET /api/campaigns/{id}/screens`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn list(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    Ok(Json(json!({ "data": gm_json(&state, row.id).await? })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairBody {
    code: String,
}

/// `POST /api/campaigns/{id}/screens` — answers the list (201).
///
/// # Errors
///
/// 404 when missing or another GM's; 404 `NO_SUCH_CODE`.
pub async fn pair(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(body): Body<PairBody>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    screens::pair(&state.pool, row.id, &body.code).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": gm_json(&state, row.id).await? })),
    )
        .into_response())
}

/// `POST /api/campaigns/{id}/screens/window` — answers the list (201)
/// and leaves the screen cookie in this browser: the window it opens
/// next on `/tv` is that screen.
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn window(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let (_, token) = screens::window(&state.pool, row.id).await?;
    Ok((
        StatusCode::CREATED,
        [(
            header::SET_COOKIE,
            screen::set_cookie(&token, state.auth.secure_cookie),
        )],
        Json(json!({ "data": gm_json(&state, row.id).await? })),
    )
        .into_response())
}

/// `DELETE /api/campaigns/{id}/screens/{screen}` — answers the list.
///
/// # Errors
///
/// 404 when missing or another GM's; 404 `NO_SUCH_SCREEN`.
pub async fn forget(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, screen)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let screen = Uuid::parse_str(&screen).map_err(|_| AppError::NotFound("NO_SUCH_SCREEN"))?;
    screens::forget(&state.pool, row.id, screen).await?;
    Ok(Json(json!({ "data": gm_json(&state, row.id).await? })).into_response())
}

/// `PUT /api/campaigns/{id}/screens/shows` — answers the list.
///
/// # Errors
///
/// 404 when missing or another GM's; 400 `INVALID_BODY`.
pub async fn set_shows(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(shows): Body<Shows>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    screens::set_shows(&state.pool, row.id, shows).await?;
    Ok(Json(json!({ "data": gm_json(&state, row.id).await? })).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_plain_origin_goes_into_the_qr() {
        assert_eq!(
            plain_origin("https://promptus.app"),
            Some("https://promptus.app")
        );
        assert_eq!(
            plain_origin("http://localhost:4334"),
            Some("http://localhost:4334")
        );
        assert_eq!(plain_origin("https://evil.example/phish"), None);
        assert_eq!(plain_origin("javascript:alert(1)"), None);
        assert_eq!(plain_origin("https://"), None);
        assert_eq!(plain_origin("https://a.b?c=d"), None);
    }
}
