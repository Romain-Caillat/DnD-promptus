//! campaign/import-pixellab-assets — Claude generates at PixelLab and
//! drops the image into Promptus.
//!
//! The GM's Claude holds both MCP servers. For a place or an NPC the GM
//! kept, it asks what the theme still lacks ([`needs`]), announces the
//! generations, generates at PixelLab once the GM agrees, and hands
//! Promptus each image's address ([`import`]). The server fetches the
//! bytes itself — from PixelLab's hosts only (`PIXELLAB_HOSTS`), or
//! from a `data:` URL — checks they are a PNG, brings an element's
//! colours onto the theme's palette (`shared::elements`), and stores the
//! image in `media_assets` as `pending`: the GM keeps or discards it like
//! every other image (`media::decide`). Nothing reaches players before.
//!
//! What the theme already holds, waiting or kept, is never generated
//! again: importing over it is refused (`ELEMENT_EXISTS`) unless the
//! call says it replaces it.

use std::sync::LazyLock;
use std::time::Duration;

use base64::Engine as _;
use promptus_shared::elements::{self, Form, Slot};
use promptus_shared::story::Campaign;
use promptus_shared::theme::Tileset;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::live::{self, Topic};
use crate::media::{self, Asset, Kind, Status};

/// The most bytes one image may weigh.
const MAX_BYTES: usize = 4 * 1024 * 1024;
/// The most frames a character brings: 4 directions × 5 animations × 4 frames.
pub const MAX_FRAMES: usize = 80;

/// The hosts the server fetches from, and their subdomains:
/// `PIXELLAB_HOSTS` (comma-separated), PixelLab's own by default.
static HOSTS: LazyLock<Vec<String>> = LazyLock::new(|| {
    std::env::var("PIXELLAB_HOSTS")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "pixellab.ai".to_string())
        .split(',')
        .map(|h| h.trim().trim_start_matches('.').to_lowercase())
        .filter(|h| !h.is_empty())
        .collect()
});

fn allowed_host(host: &str) -> bool {
    let host = host.to_lowercase();
    HOSTS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")))
}

/// The client that fetches PixelLab's images. If it cannot be built, no
/// image is fetched at all: never a default client, which would follow
/// redirects anywhere.
static HTTP: LazyLock<Result<reqwest::Client, String>> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        // A redirect may only stay among the allowed hosts.
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let ok = attempt.url().scheme() == "https"
                && attempt.url().host_str().is_some_and(allowed_host)
                && attempt.previous().len() < 5;
            if ok { attempt.follow() } else { attempt.stop() }
        }))
        .build()
        .map_err(|e| e.to_string())
});

/// How many images of one character are fetched at once.
const FETCHES_AT_ONCE: usize = 8;

fn refused(code: &'static str, detail: impl Into<String>) -> AppError {
    AppError::Invalid {
        code,
        detail: detail.into(),
    }
}

/// The bytes at `url`: a `data:image/png;base64,` URL, or an HTTPS
/// address on an allowed host.
///
/// # Errors
///
/// 400 `IMAGE_URL_REFUSED`, `IMAGE_TOO_LARGE`; 502 `IMAGE_FETCH_FAILED`.
pub async fn fetch(url: &str) -> Result<Vec<u8>, AppError> {
    let url = url.trim();
    if let Some(rest) = url.strip_prefix("data:") {
        let (head, payload) = rest
            .split_once(',')
            .ok_or_else(|| refused("IMAGE_URL_REFUSED", "a data URL needs `,` before its bytes"))?;
        if !head.ends_with(";base64") {
            return Err(refused("IMAGE_URL_REFUSED", "a data URL must be base64"));
        }
        if payload.len() > MAX_BYTES * 4 / 3 + 4 {
            return Err(refused("IMAGE_TOO_LARGE", "at most 4 MB"));
        }
        return base64::engine::general_purpose::STANDARD
            .decode(payload.trim())
            .map_err(|_| refused("IMAGE_URL_REFUSED", "the data URL is not valid base64"));
    }
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| refused("IMAGE_URL_REFUSED", format!("`{url}` is not an address")))?;
    let host = parsed.host_str().unwrap_or_default();
    if parsed.scheme() != "https" || !allowed_host(host) {
        return Err(refused(
            "IMAGE_URL_REFUSED",
            format!(
                "only https addresses on {} (PIXELLAB_HOSTS) or a data URL",
                HOSTS.join(", ")
            ),
        ));
    }
    let failed = |e: String| AppError::Upstream {
        code: "IMAGE_FETCH_FAILED",
        detail: e,
    };
    let client = HTTP
        .as_ref()
        .map_err(|e| failed(format!("no HTTP client: {e}")))?;
    let mut response = client
        .get(parsed)
        .send()
        .await
        .map_err(|e| failed(e.to_string()))?;
    if !response.status().is_success() {
        return Err(failed(format!("PixelLab answered {}", response.status())));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| failed(e.to_string()))? {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > MAX_BYTES {
            return Err(refused("IMAGE_TOO_LARGE", "at most 4 MB"));
        }
    }
    Ok(bytes)
}

fn image_error(e: &elements::ImageError) -> AppError {
    refused("IMAGE_INVALID", e.to_string())
}

fn slot_error(e: &elements::SlotError) -> AppError {
    refused("ELEMENT_UNKNOWN", e.to_string())
}

fn tilesets(story: &Campaign) -> &'static [Tileset] {
    crate::content::theme(story).map_or(&[], |t| t.tilesets.as_slice())
}

/// One element or figure the scene needs, and where it stands.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Need {
    pub kind: Kind,
    pub subject: String,
    pub name: String,
    pub form: Form,
    /// `missing`, or the status of the image that fills it.
    pub status: &'static str,
    pub asset: Option<Uuid>,
    /// PixelLab generations it would cost, when missing.
    pub generations: u32,
}

/// What a scene needs from PixelLab, and the palette to draw it in.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Needs {
    pub tileset: Option<String>,
    pub palette: Vec<String>,
    pub needs: Vec<Need>,
    /// Generations for everything missing.
    pub generations: u32,
}

/// What to list (`?map=`, `?npc=`); both empty: the whole theme and cast.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedsQuery {
    pub map: Option<String>,
    pub npc: Option<String>,
}

fn status_of(assets: &[Asset], kind: Kind, subject: &str) -> (&'static str, Option<Uuid>) {
    // Newest first: an approved image wins, else the one waiting.
    let live = |s: Status| {
        assets
            .iter()
            .find(|a| a.kind == kind && a.subject == subject && a.status == s)
    };
    if let Some(a) = live(Status::Approved) {
        ("approved", Some(a.id))
    } else if let Some(a) = live(Status::Pending) {
        ("pending", Some(a.id))
    } else {
        ("missing", None)
    }
}

fn need(assets: &[Asset], kind: Kind, slot: Slot) -> Need {
    let (status, asset) = status_of(assets, kind, &slot.subject);
    Need {
        kind,
        generations: if status == "missing" {
            slot.form.generations()
        } else {
            0
        },
        subject: slot.subject,
        name: slot.name,
        form: slot.form,
        status,
        asset,
    }
}

fn figure(story: &Campaign, id: &str) -> Option<Slot> {
    let name = story
        .npc(id)
        .map(|n| n.name.clone())
        .or_else(|| story.adversary(id).map(|a| a.name.clone()))?;
    Some(Slot {
        subject: id.to_string(),
        name,
        form: Form::Character,
    })
}

/// What the theme and the cast still lack for `query`.
///
/// # Errors
///
/// 404 (campaign, `NO_SUCH_MAP`, `NO_SUCH_NPC`); 400 `NO_THEME`.
pub async fn needs(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    query: &NeedsQuery,
) -> Result<Needs, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let assets = media::all(pool, campaign).await?;
    let sets = tilesets(&row.story);
    let mut slots: Vec<Slot> = Vec::new();
    let mut figures: Vec<Slot> = Vec::new();
    let mut tileset: Option<&Tileset> = None;
    if let Some(map_id) = query.map.as_deref() {
        let map = crate::campaign_maps::survey::map_of(pool, &row, map_id).await?;
        let t = sets.iter().find(|t| t.id == map.theme).ok_or_else(|| {
            refused(
                "NO_THEME",
                format!("no tileset `{}` in the theme", map.theme),
            )
        })?;
        tileset = Some(t);
        slots.extend(elements::needs_for_map(t, &map));
        // Who stands on the map: its NPCs' figures too.
        let cast = promptus_shared::story::cast::on_map(&row.story, map_id, &[]);
        // The opponents of the fights that break out there, too.
        let opponents = cast.fights.iter().flat_map(|f| {
            row.story
                .node(&f.scene)
                .and_then(|n| n.encounter.as_ref())
                .map(|e| {
                    e.opponents
                        .iter()
                        .map(|o| o.who.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        });
        let who: Vec<String> = cast
            .npcs
            .iter()
            .map(|n| n.npc.clone())
            .chain(opponents)
            .collect();
        for id in who {
            if !figures.iter().any(|f| f.subject == id) {
                figures.extend(figure(&row.story, &id));
            }
        }
    }
    if let Some(npc) = query.npc.as_deref() {
        figures.push(figure(&row.story, npc).ok_or(AppError::NotFound("NO_SUCH_NPC"))?);
    }
    if query.map.is_none() && query.npc.is_none() {
        if sets.is_empty() {
            return Err(refused("NO_THEME", "the campaign's rules have no theme"));
        }
        tileset = sets.first();
        for t in sets {
            slots.extend(elements::every_slot(t));
        }
        let story = &row.story;
        figures.extend(story.npcs.iter().filter_map(|n| figure(story, &n.id)));
        figures.extend(
            story
                .adversaries
                .iter()
                .filter_map(|a| figure(story, &a.id)),
        );
    }
    let list: Vec<Need> = slots
        .into_iter()
        .map(|s| need(&assets, Kind::Element, s))
        .chain(
            figures
                .into_iter()
                .map(|f| need(&assets, Kind::Character, f)),
        )
        .collect();
    Ok(Needs {
        tileset: tileset.map(|t| t.id.clone()),
        palette: tileset.map(elements::palette).unwrap_or_default(),
        generations: list.iter().map(|n| n.generations).sum(),
        needs: list,
    })
}

/// One frame of a character.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameAsk {
    pub url: String,
    /// `south`, `east`, `north`, `west` (or PixelLab's eight).
    pub direction: String,
    /// `idle`, `walk`, `attack`, `hit`, `fall`…
    #[serde(default = "idle")]
    pub animation: String,
    #[serde(default)]
    pub index: u32,
}

fn idle() -> String {
    "idle".into()
}

/// Claude drops one PixelLab image (an element) or figure (a character).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportAsk {
    pub kind: Kind,
    /// `<tileset>/<slot>` for an element; an NPC or adversary id.
    pub subject: String,
    /// An element's image.
    #[serde(default)]
    pub url: Option<String>,
    /// A character's frames.
    #[serde(default)]
    pub frames: Vec<FrameAsk>,
    /// What Claude asked PixelLab, for the GM.
    #[serde(default)]
    pub description: String,
    /// The generations it cost at PixelLab.
    #[serde(default)]
    pub generations: Option<u32>,
    /// Bring the colours onto the theme's palette (an element; default on).
    #[serde(default = "yes")]
    pub palette: bool,
    /// Replace an image already waiting or kept for the same subject.
    #[serde(default)]
    pub replace: bool,
}

fn yes() -> bool {
    true
}

fn word(raw: &str, what: &str) -> Result<String, AppError> {
    let w = raw.trim().to_lowercase();
    if w.is_empty()
        || w.len() > 24
        || !w
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(refused("BAD_FRAME", format!("`{raw}` is not a {what}")));
    }
    Ok(w)
}

/// Fetch, check, store as `pending`.
///
/// # Errors
///
/// 404; 400 `ELEMENT_UNKNOWN`, `NO_SUCH_NPC`, `IMAGE_URL_REFUSED`,
/// `IMAGE_TOO_LARGE`, `IMAGE_INVALID`, `BAD_FRAME`, `TOO_MANY_FRAMES`,
/// `IMPORT_MALFORMED`, `TEXT_TOO_LONG`; 409 `ELEMENT_EXISTS`; 502
/// `IMAGE_FETCH_FAILED`.
pub async fn import(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    ask: &ImportAsk,
) -> Result<Asset, AppError> {
    let description = crate::evening::clean_text(&ask.description, 500)?;
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let (form, name) = check_subject(&row, ask)?;
    // Checked before any fetch: the theme's existing work is never redone.
    exists(pool, campaign, ask).await?;

    let (image, frames) = match ask.kind {
        Kind::Element => {
            let url = ask
                .url
                .as_deref()
                .ok_or_else(|| refused("IMPORT_MALFORMED", "an element needs `url`"))?;
            if !ask.frames.is_empty() {
                return Err(refused("IMPORT_MALFORMED", "an element has no frames"));
            }
            let bytes = fetch(url).await?;
            let palette = if ask.palette {
                tilesets(&row.story)
                    .iter()
                    .find(|t| ask.subject.starts_with(&format!("{}/", t.id)))
                    .map(elements::palette)
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let png = elements::reduce_to_palette(&bytes, &palette).map_err(|e| image_error(&e))?;
            (png, Vec::new())
        }
        _ => {
            if ask.frames.is_empty() {
                return Err(refused("IMPORT_MALFORMED", "a character needs `frames`"));
            }
            if ask.frames.len() > MAX_FRAMES {
                return Err(refused("TOO_MANY_FRAMES", format!("at most {MAX_FRAMES}")));
            }
            let mut named = Vec::with_capacity(ask.frames.len());
            for f in &ask.frames {
                if f.index as usize >= MAX_FRAMES {
                    return Err(refused(
                        "BAD_FRAME",
                        format!("index {} is past {MAX_FRAMES}", f.index),
                    ));
                }
                named.push((
                    word(&f.animation, "animation")?,
                    word(&f.direction, "direction")?,
                    f.index,
                    f.url.clone(),
                ));
            }
            let frames = fetch_frames(named).await?;
            let mut frames = frames;
            frames.sort_by(|a, b| (&a.0, &a.1, a.2).cmp(&(&b.0, &b.1, b.2)));
            frames.dedup_by(|a, b| (&a.0, &a.1, a.2) == (&b.0, &b.1, b.2));
            // The figure shown before any animation: resting, facing us.
            let first = frames
                .iter()
                .find(|f| f.0 == "idle" && f.1 == "south")
                .or_else(|| frames.first())
                .map(|f| f.3.clone())
                .unwrap_or_default();
            (first, frames)
        }
    };

    let meta = json!({
        "form": form,
        "name": name,
        "generations": ask.generations,
        "description": description,
        "frames": frames.iter().enumerate().map(|(i, f)| json!({
            "n": i, "animation": f.0, "direction": f.1, "index": f.2,
        })).collect::<Vec<_>>(),
    });
    let origin = ask
        .url
        .clone()
        .or_else(|| ask.frames.first().map(|f| f.url.clone()))
        .filter(|u| !u.starts_with("data:"));

    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    exists(&mut *tx, campaign, ask).await?;
    if ask.replace {
        // The one waiting gives way; a kept one is retired when the GM
        // keeps the new one (`media::decide`).
        sqlx::query(
            "UPDATE media_assets SET status = 'rejected', decided_at = now()
             WHERE campaign_id = $1 AND kind = $2 AND subject = $3 AND status = 'pending'",
        )
        .bind(campaign)
        .bind(ask.kind.as_str())
        .bind(&ask.subject)
        .execute(&mut *tx)
        .await?;
    }
    let stored: media::Row = sqlx::query_as(&format!(
        "INSERT INTO media_assets
           (campaign_id, kind, subject, direction, status, mime, image, source, origin, meta)
         VALUES ($1, $2, $3, $4, 'pending', 'image/png', $5, 'pixellab', $6, $7)
         RETURNING {}",
        media::COLUMNS
    ))
    .bind(campaign)
    .bind(ask.kind.as_str())
    .bind(&ask.subject)
    .bind(&description)
    .bind(&image)
    .bind(&origin)
    .bind(&meta)
    .fetch_one(&mut *tx)
    .await?;
    let stored = media::asset(stored)?;
    for (animation, direction, index, png) in &frames {
        sqlx::query(
            "INSERT INTO media_frames (asset_id, animation, direction, idx, mime, image)
             VALUES ($1, $2, $3, $4, 'image/png', $5)",
        )
        .bind(stored.id)
        .bind(animation)
        .bind(direction)
        // Below MAX_FRAMES, checked above.
        .bind(i32::try_from(*index).unwrap_or_default())
        .bind(png)
        .execute(&mut *tx)
        .await?;
    }
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(stored)
}

/// The form and name of the subject, checked against the story and theme.
fn check_subject(row: &CampaignRow, ask: &ImportAsk) -> Result<(Form, String), AppError> {
    match ask.kind {
        Kind::Element => {
            let slot =
                elements::slot(tilesets(&row.story), &ask.subject).map_err(|e| slot_error(&e))?;
            Ok((slot.form, slot.name))
        }
        Kind::Character => figure(&row.story, &ask.subject)
            .map(|f| (f.form, f.name))
            .ok_or(AppError::NotFound("NO_SUCH_NPC")),
        _ => Err(refused(
            "IMPORT_MALFORMED",
            "`kind` is `element` or `character`",
        )),
    }
}

/// 409 `ELEMENT_EXISTS` when an image already waits or is kept for the
/// subject and the call does not replace it.
async fn exists(
    db: impl sqlx::PgExecutor<'_>,
    campaign: Uuid,
    ask: &ImportAsk,
) -> Result<(), AppError> {
    if ask.replace {
        return Ok(());
    }
    let there: Option<String> = sqlx::query_scalar(
        "SELECT status FROM media_assets
         WHERE campaign_id = $1 AND kind = $2 AND subject = $3 AND status IN ('pending', 'approved')
         LIMIT 1",
    )
    .bind(campaign)
    .bind(ask.kind.as_str())
    .bind(&ask.subject)
    .fetch_optional(db)
    .await?;
    match there {
        Some(_) => Err(AppError::Conflict("ELEMENT_EXISTS")),
        None => Ok(()),
    }
}

/// A character's frames, fetched a few at a time, each checked as a PNG:
/// `(animation, direction, index, png)`, in no particular order.
async fn fetch_frames(
    named: Vec<(String, String, u32, String)>,
) -> Result<Vec<(String, String, u32, Vec<u8>)>, AppError> {
    let gate = std::sync::Arc::new(tokio::sync::Semaphore::new(FETCHES_AT_ONCE));
    let mut set = tokio::task::JoinSet::new();
    for (animation, direction, index, url) in named {
        let gate = gate.clone();
        set.spawn(async move {
            let _permit = gate
                .acquire_owned()
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let bytes = fetch(&url).await?;
            // Characters keep PixelLab's colours: they are the colour of
            // the board (`MEMORY.md` §2), never the theme's.
            let png = elements::read_png(&bytes)
                .and_then(|i| elements::write_png(&i))
                .map_err(|e| image_error(&e))?;
            Ok::<_, AppError>((animation, direction, index, png))
        });
    }
    let mut out = Vec::new();
    while let Some(done) = set.join_next().await {
        // The first failure answers; the others are dropped with the set.
        out.push(done.map_err(|e| AppError::Internal(e.to_string()))??);
    }
    Ok(out)
}

/// The bytes of frame `n` of character `asset`.
///
/// # Errors
///
/// 404 `NO_SUCH_ASSET`.
pub async fn frame(
    pool: &PgPool,
    campaign: Uuid,
    asset: Uuid,
    n: usize,
) -> Result<Vec<u8>, AppError> {
    let n = i64::try_from(n).map_err(|_| AppError::NotFound("NO_SUCH_ASSET"))?;
    // `COLLATE "C"`: byte order, the order `meta.frames` was numbered in
    // (Rust's), whatever the database's collation.
    let row: Option<Vec<u8>> = sqlx::query_scalar(
        r#"SELECT f.image FROM media_frames f JOIN media_assets a ON a.id = f.asset_id
           WHERE f.asset_id = $1 AND a.campaign_id = $2
           ORDER BY f.animation COLLATE "C", f.direction COLLATE "C", f.idx
           OFFSET $3 LIMIT 1"#,
    )
    .bind(asset)
    .bind(campaign)
    .bind(n)
    .fetch_optional(pool)
    .await?;
    row.ok_or(AppError::NotFound("NO_SUCH_ASSET"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_pixellab_hosts_are_fetched() {
        assert!(allowed_host("pixellab.ai"));
        assert!(allowed_host("api.PixelLab.ai"));
        assert!(!allowed_host("evilpixellab.ai"));
        assert!(!allowed_host("pixellab.ai.example.com"));
        assert!(!allowed_host("localhost"));
    }

    #[tokio::test]
    async fn an_address_off_the_list_is_refused_before_any_call() {
        for url in [
            "http://api.pixellab.ai/x.png",
            "https://169.254.169.254/latest",
            "https://localhost/x.png",
            "file:///etc/passwd",
            "data:text/plain,hello",
        ] {
            let e = fetch(url).await.unwrap_err();
            assert!(
                matches!(
                    e,
                    AppError::Invalid {
                        code: "IMAGE_URL_REFUSED",
                        ..
                    }
                ),
                "{url}: {e:?}"
            );
        }
        let png = fetch("data:image/png;base64,aGVsbG8=").await.unwrap();
        assert_eq!(png, b"hello");
    }
}
