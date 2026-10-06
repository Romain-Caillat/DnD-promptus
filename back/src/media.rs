//! media/draw-pixel-art-assets, maps/build-tileset-packs — images drawn
//! by the image model in the world's pixel-art style, each one reviewed
//! by the GM (migration `014_media.sql`).
//!
//! The GM asks for one subject — a scene, an NPC, an adversary, a place,
//! an item, or a material of a tileset — with an optional word of
//! direction. The prompt is the subject's own description for pixel art
//! (`art`, `portrait`), the world's art direction and the template
//! (`prompts/pixel-art.v1.md`); the call is counted against the
//! campaign's AI budget. The image waits as `pending` until the GM
//! approves it (it then replaces the subject's previous one) or rejects
//! it and asks again.
//!
//! Players receive an approved image only once its subject is theirs to
//! see: a scene entered, an NPC or adversary whose name they know, a
//! place of a scene entered, an item handed out, any tileset.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use promptus_shared::story::{Campaign, WorldState};
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::ai::{Ai, ImageRequest, templates};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;
use crate::live::{self, Topic};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Scene,
    Npc,
    Adversary,
    Location,
    Item,
    /// A material of the world's tilesets: an atlas of its 16 tiles.
    Tileset,
}

impl Kind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scene => "scene",
            Self::Npc => "npc",
            Self::Adversary => "adversary",
            Self::Location => "location",
            Self::Item => "item",
            Self::Tileset => "tileset",
        }
    }

    fn parse(s: &str) -> Result<Self, AppError> {
        Ok(match s {
            "scene" => Self::Scene,
            "npc" => Self::Npc,
            "adversary" => Self::Adversary,
            "location" => Self::Location,
            "item" => Self::Item,
            "tileset" => Self::Tileset,
            other => return Err(AppError::Internal(format!("media kind {other}"))),
        })
    }

    /// The kind as the prompt names it.
    fn words(self) -> &'static str {
        match self {
            Self::Scene => "Illustration of a scene, three-quarter view",
            Self::Npc => "Portrait bust of a character",
            Self::Adversary => "Full-body sprite of an adversary, profile facing left",
            Self::Location => "Illustration of a place, three-quarter view",
            Self::Item => "Item icon, 12x12 sprite scaled up, centred",
            Self::Tileset => {
                "Tileset atlas: a 4x4 grid of 16 seamless top-down floor tiles of one material, \
                 each tile showing one combination of edges (autotiling), three-quarter view lighting"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    Approved,
    Rejected,
}

impl Status {
    fn parse(s: &str) -> Result<Self, AppError> {
        Ok(match s {
            "pending" => Self::Pending,
            "approved" => Self::Approved,
            "rejected" => Self::Rejected,
            other => return Err(AppError::Internal(format!("media status {other}"))),
        })
    }
}

/// An image, without its bytes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: Uuid,
    pub kind: Kind,
    pub subject: String,
    pub direction: String,
    pub status: Status,
    /// Why the model failed, when it did (French, for the GM).
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

type Row = (
    Uuid,
    String,
    String,
    String,
    String,
    Option<String>,
    DateTime<Utc>,
);

/// A row with its bytes and type.
type ImageRow = (
    Uuid,
    String,
    String,
    String,
    String,
    Option<String>,
    DateTime<Utc>,
    Option<Vec<u8>>,
    Option<String>,
);

const COLUMNS: &str = "id, kind, subject, direction, status, error, created_at";

fn asset(r: Row) -> Result<Asset, AppError> {
    Ok(Asset {
        id: r.0,
        kind: Kind::parse(&r.1)?,
        subject: r.2,
        direction: r.3,
        status: Status::parse(&r.4)?,
        error: r.5,
        created_at: r.6,
    })
}

/// Every image of `campaign`, newest first.
///
/// # Errors
///
/// A database error.
pub async fn all(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<Asset>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM media_assets WHERE campaign_id = $1 ORDER BY created_at DESC"
    ))
    .bind(campaign)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(asset).collect()
}

/// What to draw for `subject` of `kind`, from the campaign.
fn description(
    story: &Campaign,
    theme_materials: &BTreeMap<String, String>,
    kind: Kind,
    subject: &str,
) -> Option<String> {
    let pick = |art: &str, fallback: &str| {
        if art.trim().is_empty() {
            fallback.to_string()
        } else {
            art.to_string()
        }
    };
    match kind {
        Kind::Scene => story.node(subject).map(|n| pick(&n.art, &n.read_aloud)),
        Kind::Npc => story
            .npc(subject)
            .map(|n| pick(&n.portrait, &format!("{}, {}", n.name, n.appearance))),
        Kind::Adversary => story
            .adversary(subject)
            .map(|a| pick(&a.art, &a.description)),
        Kind::Location => story
            .locations
            .iter()
            .find(|l| l.id == subject)
            .map(|l| pick(&l.art, &l.description)),
        Kind::Item => story
            .items
            .iter()
            .find(|i| i.id == subject)
            .map(|i| pick(&i.art, &format!("{}, {}", i.name, i.description))),
        Kind::Tileset => theme_materials.get(subject).cloned(),
    }
}

/// The materials of the campaign's theme, by id, as words for the model.
fn materials(story: &Campaign) -> BTreeMap<String, String> {
    crate::content::theme(story)
        .map(|t| {
            t.tilesets
                .iter()
                .flat_map(|ts| {
                    ts.materials.iter().map(move |(id, m)| {
                        (id.clone(), format!("{} ({}), {}", m.name, ts.name, m.base))
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The GM asks for an image.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ask {
    pub kind: Kind,
    pub subject: String,
    /// The GM's word for this try (« plus sombre », « de face »).
    #[serde(default)]
    pub direction: String,
}

/// Draw one image and keep it pending.
///
/// # Errors
///
/// 404; 400 `UNKNOWN_SUBJECT`, `TEXT_TOO_LONG`; 409 `AI_BUDGET_EXCEEDED`;
/// 503 `AI_NOT_CONFIGURED`.
pub async fn ask(
    pool: &PgPool,
    ai: &Ai,
    gm: &CurrentGm,
    campaign: Uuid,
    ask: &Ask,
) -> Result<Asset, AppError> {
    let direction = crate::evening::clean_text(&ask.direction, 300)?;
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let subject = description(&row.story, &materials(&row.story), ask.kind, &ask.subject)
        .ok_or(AppError::BadRequest("UNKNOWN_SUBJECT"))?;
    let world = row.story.bible.art_direction.clone();
    let subject = if direction.is_empty() {
        subject
    } else {
        format!("{subject} ({direction})")
    };
    let vars: BTreeMap<&str, String> = [
        ("kind", ask.kind.words().to_string()),
        ("direction", world),
        ("subject", subject),
    ]
    .into_iter()
    .collect();
    let prompt = templates::PIXEL_ART
        .render(&vars)
        .map_err(|e| AppError::internal("pixel-art template", e))?
        .into_iter()
        .map(|m| m.content)
        .collect::<Vec<_>>()
        .join("\n");
    let mut answers = ai
        .images(
            pool,
            campaign,
            &format!("media.{}", ask.kind.as_str()),
            &templates::PIXEL_ART,
            &[ImageRequest {
                prompt,
                model: None,
            }],
        )
        .await?;
    let answer = answers
        .pop()
        .ok_or(AppError::Internal("no image answer".into()))?;
    let (bytes, mime, error) = match answer {
        Ok(img) => (Some(img.bytes), Some(img.mime), None),
        Err(e) => (None, None, Some(e.to_string())),
    };
    let mut tx = pool.begin().await?;
    let stored: Row = sqlx::query_as(&format!(
        "INSERT INTO media_assets (campaign_id, kind, subject, direction, status, mime, image, error)
         VALUES ($1, $2, $3, $4, CASE WHEN $7::bytea IS NULL THEN 'rejected' ELSE 'pending' END, $5, $7, $6)
         RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(ask.kind.as_str())
    .bind(&ask.subject)
    .bind(&direction)
    .bind(mime)
    .bind(&error)
    .bind(bytes)
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    // A failed drawing is kept, rejected, with the model's words: the GM
    // sees why and asks again.
    asset(stored)
}

/// The GM's review: approve (the subject's previous approved image is
/// retired) or reject.
///
/// # Errors
///
/// 404 `NO_SUCH_ASSET`; 409 `ALREADY_DECIDED`.
pub async fn decide(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
    approve: bool,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let found: Option<(String, String, String)> = sqlx::query_as(
        "SELECT kind, subject, status FROM media_assets WHERE id = $1 AND campaign_id = $2 FOR UPDATE",
    )
    .bind(id)
    .bind(campaign)
    .fetch_optional(&mut *tx)
    .await?;
    let (kind, subject, status) = found.ok_or(AppError::NotFound("NO_SUCH_ASSET"))?;
    if status != "pending" {
        return Err(AppError::Conflict("ALREADY_DECIDED"));
    }
    if approve {
        sqlx::query(
            "UPDATE media_assets SET status = 'rejected', decided_at = now()
             WHERE campaign_id = $1 AND kind = $2 AND subject = $3 AND status = 'approved'",
        )
        .bind(campaign)
        .bind(&kind)
        .bind(&subject)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("UPDATE media_assets SET status = $2, decided_at = now() WHERE id = $1")
        .bind(id)
        .bind(if approve { "approved" } else { "rejected" })
        .execute(&mut *tx)
        .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    // Approved images may reach the players' screens.
    live::touch(&mut tx, campaign, &Topic::Session).await?;
    tx.commit().await?;
    Ok(())
}

/// The bytes and type of image `id` of `campaign`, whatever its status.
///
/// # Errors
///
/// 404 `NO_SUCH_ASSET`.
pub async fn image(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    id: Uuid,
) -> Result<(Asset, Vec<u8>, String), AppError> {
    let row: Option<ImageRow> = sqlx::query_as(&format!(
        "SELECT {COLUMNS}, image, mime FROM media_assets WHERE id = $1 AND campaign_id = $2"
    ))
    .bind(id)
    .bind(campaign)
    .fetch_optional(db)
    .await?;
    let r = row.ok_or(AppError::NotFound("NO_SUCH_ASSET"))?;
    let bytes = r.7.clone().ok_or(AppError::NotFound("NO_SUCH_ASSET"))?;
    let mime = r.8.clone().unwrap_or_else(|| "image/png".into());
    Ok((asset((r.0, r.1, r.2, r.3, r.4, r.5, r.6))?, bytes, mime))
}

/// Whether the players may see the subject of `a`.
#[must_use]
pub fn shown_to_players(
    story: &Campaign,
    world: &WorldState,
    given_items: &[String],
    a: &Asset,
) -> bool {
    if a.status != Status::Approved {
        return false;
    }
    let entered = |node: &str| {
        world.node_status.contains_key(node) || world.current_node.as_deref() == Some(node)
    };
    match a.kind {
        Kind::Scene => entered(&a.subject),
        Kind::Npc | Kind::Adversary => world.revealed.contains(&a.subject),
        Kind::Location => story
            .nodes
            .iter()
            .any(|n| n.location.as_deref() == Some(a.subject.as_str()) && entered(&n.id)),
        Kind::Item => given_items.contains(&a.subject),
        Kind::Tileset => true,
    }
}

/// Items of the story handed to the table (the shared journal's loot
/// lines name them).
///
/// # Errors
///
/// A database error.
pub async fn given_items(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT ref FROM table_journal
         WHERE campaign_id = $1 AND shared AND kind IN ('loot', 'item') AND ref IS NOT NULL",
    )
    .bind(campaign)
    .fetch_all(db)
    .await?)
}
