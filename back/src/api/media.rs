//! Pixel-art images and the world's theme over HTTP.
//!
//! GM (behind `require_gm`):
//! - `GET  /api/campaigns/{id}/media` → every image and video (drawing,
//!   pending, approved, rejected), the world's theme, and what a batch
//!   would draw (`media::Plan`);
//! - `POST /api/campaigns/{id}/media` → `media::Ask`: draw one (201);
//! - `POST /api/campaigns/{id}/media/batch` → `media::BatchAsk`: draw
//!   every missing one in the background (202);
//! - `GET  /api/campaigns/{id}/media/{asset}/image` → its bytes;
//! - `POST /api/campaigns/{id}/media/{asset}/decision` → `{ approve }`.
//!
//! Player (behind `require_player`):
//! - `GET /api/play/{campaign}/media` → the approved images whose subject
//!   the table may see, and the world's theme;
//! - `GET /api/play/{campaign}/media/{asset}/image` → the bytes of one of
//!   those (404 otherwise: the GM's other images never leave).
//!
//! The bytes are served with `Range` support: a phone's video player
//! asks for the file in pieces and plays nothing without it.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::auth::player::CurrentPlayer;
use crate::campaigns::{self, CampaignRow};
use crate::content;
use crate::error::AppError;
use crate::media::{self, Ask, BatchAsk};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

async fn owned_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)
}

/// The byte range a `Range` header asks of a file of `len` bytes: `None`
/// for the whole file, `Err` when it cannot be served. One range only.
fn range(headers: &HeaderMap, len: usize) -> Result<Option<(usize, usize)>, ()> {
    let Some(raw) = headers.get(header::RANGE) else {
        return Ok(None);
    };
    let spec = raw
        .to_str()
        .ok()
        .and_then(|v| v.trim().strip_prefix("bytes="))
        .filter(|v| !v.contains(','))
        .ok_or(())?;
    let (start, end) = spec.split_once('-').ok_or(())?;
    let (start, end) = match (start.trim(), end.trim()) {
        ("", suffix) => {
            let n: usize = suffix.parse().map_err(|_| ())?;
            if n == 0 {
                return Err(());
            }
            (len.saturating_sub(n), len.checked_sub(1).ok_or(())?)
        }
        (start, "") => (
            start.parse().map_err(|_| ())?,
            len.checked_sub(1).ok_or(())?,
        ),
        (start, end) => {
            let end: usize = end.parse().map_err(|_| ())?;
            (
                start.parse().map_err(|_| ())?,
                end.min(len.saturating_sub(1)),
            )
        }
    };
    if start > end || start >= len {
        return Err(());
    }
    Ok(Some((start, end)))
}

fn image_response(headers: &HeaderMap, bytes: Vec<u8>, mime: String) -> Response {
    let len = bytes.len();
    let mut response = match range(headers, len) {
        Ok(None) => (StatusCode::OK, bytes).into_response(),
        Ok(Some((start, end))) => {
            let mut r = (StatusCode::PARTIAL_CONTENT, bytes[start..=end].to_vec()).into_response();
            if let Ok(v) = HeaderValue::from_str(&format!("bytes {start}-{end}/{len}")) {
                r.headers_mut().insert(header::CONTENT_RANGE, v);
            }
            r
        }
        Err(()) => {
            let mut r = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
            if let Ok(v) = HeaderValue::from_str(&format!("bytes */{len}")) {
                r.headers_mut().insert(header::CONTENT_RANGE, v);
            }
            return r;
        }
    };
    let h = response.headers_mut();
    if let Ok(v) = HeaderValue::from_str(&mime) {
        h.insert(header::CONTENT_TYPE, v);
    }
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    response
}

/// `GET /api/campaigns/{id}/media`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn gm_list(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    media::settle_interrupted(&state.pool, row.id).await?;
    let assets = media::all(&state.pool, row.id).await?;
    let plan = media::plan(&state.pool, &state.ai, &row).await?;
    Ok(Json(json!({ "data": {
        "assets": assets,
        "theme": content::theme(&row.story),
        "plan": plan,
    } }))
    .into_response())
}

/// `POST /api/campaigns/{id}/media/batch` — the rows queued (202).
///
/// # Errors
///
/// The codes of `media::batch`.
pub async fn batch(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(ask): Body<BatchAsk>,
) -> Result<Response, AppError> {
    let queued = media::batch(&state.pool, &state.ai, &gm, parse_id(&id)?, &ask).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "data": { "queued": queued } })),
    )
        .into_response())
}

/// `POST /api/campaigns/{id}/media` — the new image, pending or (when the
/// model failed) rejected with the reason (201).
///
/// # Errors
///
/// The codes of `media::ask`.
pub async fn ask(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(ask): Body<Ask>,
) -> Result<Response, AppError> {
    let a = media::ask(&state.pool, &state.ai, &gm, parse_id(&id)?, &ask).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": a }))).into_response())
}

/// `GET /api/campaigns/{id}/media/{asset}/image`
///
/// # Errors
///
/// 404 `NO_SUCH_ASSET`.
pub async fn gm_image(
    State(state): State<AppState>,
    gm: CurrentGm,
    headers: HeaderMap,
    Path((id, asset)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let (_, bytes, mime) = media::image(&state.pool, row.id, parse_id(&asset)?).await?;
    Ok(image_response(&headers, bytes, mime))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionBody {
    approve: bool,
}

/// `POST /api/campaigns/{id}/media/{asset}/decision` (204).
///
/// # Errors
///
/// The codes of `media::decide`.
pub async fn decide(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, asset)): Path<(String, String)>,
    Body(body): Body<DecisionBody>,
) -> Result<Response, AppError> {
    media::decide(
        &state.pool,
        &gm,
        parse_id(&id)?,
        parse_id(&asset)?,
        body.approve,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// The images player `p` may see.
async fn visible(
    state: &AppState,
    p: &CurrentPlayer,
) -> Result<(CampaignRow, Vec<media::Asset>), AppError> {
    let row = campaigns::find(&state.pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))?;
    let given = media::given_items(&state.pool, row.id).await?;
    let assets = media::all(&state.pool, row.id)
        .await?
        .into_iter()
        .filter(|a| media::shown_to_players(&row.story, &row.world, &given, a))
        .collect();
    Ok((row, assets))
}

/// `GET /api/play/{campaign}/media`
///
/// # Errors
///
/// 401 `NOT_JOINED`.
pub async fn player_list(
    State(state): State<AppState>,
    p: CurrentPlayer,
) -> Result<Response, AppError> {
    let (row, assets) = visible(&state, &p).await?;
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

/// `GET /api/play/{campaign}/media/{asset}/image`
///
/// # Errors
///
/// 401 `NOT_JOINED`; 404 `NO_SUCH_ASSET` (also for an image the table
/// may not see).
pub async fn player_image(
    State(state): State<AppState>,
    p: CurrentPlayer,
    headers: HeaderMap,
    Path((_, asset)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let id = parse_id(&asset)?;
    let (row, assets) = visible(&state, &p).await?;
    if !assets.iter().any(|a| a.id == id) {
        return Err(AppError::NotFound("NO_SUCH_ASSET"));
    }
    let (_, bytes, mime) = media::image(&state.pool, row.id, id).await?;
    Ok(image_response(&headers, bytes, mime))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asking(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::RANGE, HeaderValue::from_str(value).unwrap());
        h
    }

    #[test]
    fn a_range_is_read_as_a_video_player_asks_for_it() {
        assert_eq!(range(&HeaderMap::new(), 10), Ok(None));
        // Safari's first probe, then the rest.
        assert_eq!(range(&asking("bytes=0-1"), 10), Ok(Some((0, 1))));
        assert_eq!(range(&asking("bytes=4-"), 10), Ok(Some((4, 9))));
        assert_eq!(range(&asking("bytes=-3"), 10), Ok(Some((7, 9))));
        assert_eq!(range(&asking("bytes=8-50"), 10), Ok(Some((8, 9))));
        assert_eq!(range(&asking("bytes=10-"), 10), Err(()));
        assert_eq!(range(&asking("bytes=5-2"), 10), Err(()));
        assert_eq!(range(&asking("bytes=0-1,4-5"), 10), Err(()));
        assert_eq!(range(&asking("items=0-1"), 10), Err(()));
    }

    #[test]
    fn a_partial_answer_says_which_bytes_of_how_many() {
        let r = image_response(
            &asking("bytes=2-4"),
            b"abcdefgh".to_vec(),
            "video/mp4".into(),
        );
        assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(r.headers()[header::CONTENT_RANGE], "bytes 2-4/8");
        assert_eq!(r.headers()[header::CONTENT_TYPE], "video/mp4");
        let r = image_response(
            &asking("bytes=9-"),
            b"abcdefgh".to_vec(),
            "video/mp4".into(),
        );
        assert_eq!(r.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(r.headers()[header::CONTENT_RANGE], "bytes */8");
    }
}
