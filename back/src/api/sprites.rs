//! Character sprites, drawn by the server (characters/render-layered-sprite).
//!
//! One implementation draws every sprite: the phone, the GM screen and
//! the TV all load the same PNG for the same description, so they show
//! the same pixels. The starter packs and the looks of the two witness
//! worlds are compiled into the binary (`content/sprites/`).
//!
//! Every route is public: a description is not a secret (the caller
//! sends it), and drawing a 22 x 28 sprite costs next to nothing; the
//! renders are cached by hash all the same.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use axum::Json;
use axum::extract::{Path, Query};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use promptus_shared::sprite::{CharacterLook, Direction, render};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::content;
use crate::error::AppError;

/// Renders kept in memory; past this many the cache starts over.
const CACHE_ENTRIES: usize = 1024;

struct Sprites {
    /// Hash of the pack files: a pack edit changes every ETag.
    version: String,
    cache: Mutex<HashMap<String, Arc<Vec<u8>>>>,
}

static SPRITES: LazyLock<Sprites> = LazyLock::new(|| {
    let mut hash = Sha256::new();
    for p in content::PACK_FILES {
        hash.update(p.as_bytes());
    }
    Sprites {
        version: hex(&hash.finalize()[..8]),
        cache: Mutex::new(HashMap::new()),
    }
});

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Deserialize)]
pub struct RenderQuery {
    /// The `CharacterLook`, as JSON.
    look: String,
    #[serde(default)]
    facing: Direction,
}

/// `GET /api/sprites/render.png?look=<json>&facing=east|west` — the PNG
/// of a description, unscaled (the client enlarges it pixelated).
///
/// 400 `SPRITE_LOOK_INVALID` when `look` is not a description, and the
/// renderer's code (`SPRITE_UNKNOWN_PIECE`, `SPRITE_UNKNOWN_COLOUR`…)
/// when it names something no pack has.
///
/// # Errors
///
/// `AppError::Invalid` as above.
pub async fn render_png(
    Query(q): Query<RenderQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let look: CharacterLook = serde_json::from_str(&q.look).map_err(|e| AppError::Invalid {
        code: "SPRITE_LOOK_INVALID",
        detail: e.to_string(),
    })?;
    let sprites = &*SPRITES;
    // Hash the description as re-serialised, so two spellings of one
    // look share a cache entry.
    let canonical = serde_json::to_string(&look).map_err(|e| AppError::internal("look", e))?;
    let mut hash = Sha256::new();
    hash.update(sprites.version.as_bytes());
    hash.update(q.facing.as_str().as_bytes());
    hash.update(canonical.as_bytes());
    let key = hex(&hash.finalize()[..16]);
    let etag = format!("\"{key}\"");

    if headers
        .get(IF_NONE_MATCH)
        .is_some_and(|v| v.as_bytes() == etag.as_bytes())
    {
        return Ok((StatusCode::NOT_MODIFIED, cache_headers(&etag)).into_response());
    }

    let cached = sprites.cache.lock().ok().and_then(|c| c.get(&key).cloned());
    let png = if let Some(png) = cached {
        png
    } else {
        let image = render(content::packs(), &look, q.facing).map_err(|e| AppError::Invalid {
            code: e.code(),
            detail: e.to_string(),
        })?;
        let png = Arc::new(image.to_png());
        if let Ok(mut cache) = sprites.cache.lock() {
            if cache.len() >= CACHE_ENTRIES {
                cache.clear();
            }
            cache.insert(key, Arc::clone(&png));
        }
        png
    };
    let mut response = (cache_headers(&etag), png.as_ref().clone()).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    Ok(response)
}

fn cache_headers(etag: &str) -> [(axum::http::HeaderName, HeaderValue); 2] {
    [
        // A day, then revalidated: a pack update reaches clients by ETag.
        (
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=86400"),
        ),
        (
            ETAG,
            HeaderValue::from_str(etag).unwrap_or(HeaderValue::from_static("\"\"")),
        ),
    ]
}

/// `GET /api/sprites/looks` — the looks of the two witness worlds: their
/// party slots (drawn facing east) and their foes (facing west).
pub async fn looks() -> Json<Value> {
    Json(json!({ "data": { "worlds": content::look_books() } }))
}

/// `GET /api/sprites/packs/{pack}` — what the character creator offers
/// from a pack: each slot's pieces (id, French name, whether they take
/// colours) and the palettes. No pixels: previews go through
/// `render.png`.
///
/// # Errors
///
/// 404 `SPRITE_UNKNOWN_PACK` for a pack this server does not have.
pub async fn pack(Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    let pack = content::packs()
        .get(&id)
        .ok_or(AppError::NotFound("SPRITE_UNKNOWN_PACK"))?;
    Ok(Json(json!({ "data": pack.catalogue() })))
}
