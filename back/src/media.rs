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
//!
//! media/generate-images-and-video — an act's introduction is a short
//! video (`intro`, prompt `prompts/intro-video.v1.md`), shown once a scene
//! of the act is entered. A video takes minutes, so it is drawn in the
//! background: its row waits as `drawing` and becomes `pending` (or
//! `rejected`, with the model's words) when the model answers. The GM can
//! also draw, in one background batch, every scene, NPC, adversary and
//! place (and, if asked, every act's video) that has no image waiting or
//! approved: the batch is estimated whole and refused before its first
//! call when it would pass the budget.

use std::collections::{BTreeMap, HashSet};
use std::sync::{LazyLock, Mutex};

use chrono::{DateTime, Utc};
use promptus_shared::story::{Campaign, WorldState};
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::ai::ledger::{self, Spending};
use crate::ai::{Ai, ImageRequest, VideoRequest, templates};
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
    /// An act's introduction: a video.
    Intro,
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
            Self::Intro => "intro",
        }
    }

    /// Whether the model makes a video, not an image.
    #[must_use]
    pub fn is_video(self) -> bool {
        self == Self::Intro
    }

    fn parse(s: &str) -> Result<Self, AppError> {
        Ok(match s {
            "scene" => Self::Scene,
            "npc" => Self::Npc,
            "adversary" => Self::Adversary,
            "location" => Self::Location,
            "item" => Self::Item,
            "tileset" => Self::Tileset,
            "intro" => Self::Intro,
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
            Self::Intro => "Opening cinematic of an act",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// The model is still making it.
    Drawing,
    Pending,
    Approved,
    Rejected,
}

impl Status {
    fn parse(s: &str) -> Result<Self, AppError> {
        Ok(match s {
            "drawing" => Self::Drawing,
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
        Kind::Intro => story.acts.iter().find(|a| a.id == subject).map(|a| {
            let scene = [&a.opening, &a.summary]
                .into_iter()
                .find(|t| !t.trim().is_empty())
                .cloned()
                .unwrap_or_default();
            format!("{}: {scene}", a.title)
        }),
    }
}

/// The prompt that draws `subject` of `kind`, with the GM's word.
fn prompt(
    story: &Campaign,
    kind: Kind,
    subject: &str,
    direction: &str,
) -> Result<String, AppError> {
    let described = description(story, &materials(story), kind, subject)
        .ok_or(AppError::BadRequest("UNKNOWN_SUBJECT"))?;
    let described = if direction.is_empty() {
        described
    } else {
        format!("{described} ({direction})")
    };
    let world = story.bible.art_direction.clone();
    let (template, vars): (_, BTreeMap<&str, String>) = if kind.is_video() {
        let (title, scene) = described
            .split_once(": ")
            .map_or((described.as_str(), ""), |(t, s)| (t, s));
        (
            templates::INTRO_VIDEO,
            [
                ("direction", world),
                ("pitch", story.bible.pitch.clone()),
                ("title", title.to_string()),
                ("scene", scene.to_string()),
            ]
            .into_iter()
            .collect(),
        )
    } else {
        (
            templates::PIXEL_ART,
            [
                ("kind", kind.words().to_string()),
                ("direction", world),
                ("subject", described),
            ]
            .into_iter()
            .collect(),
        )
    };
    Ok(template
        .render(&vars)
        .map_err(|e| AppError::internal("media template", e))?
        .into_iter()
        .map(|m| m.content)
        .collect::<Vec<_>>()
        .join("\n"))
}

/// The most one drawing of `kind` costs.
fn estimate(ai: &Ai, kind: Kind) -> i64 {
    if kind.is_video() {
        ai.pricing.video()
    } else {
        ai.pricing.image()
    }
}

/// The bytes, type and failure of one drawing.
type Drawn = (Option<Vec<u8>>, Option<String>, Option<String>);

/// Ask the model for one drawing, counted. The model's failure is kept
/// as the GM's words; the budget's refusal is raised.
async fn draw(
    pool: &PgPool,
    ai: &Ai,
    campaign: Uuid,
    kind: Kind,
    prompt: String,
) -> Result<Drawn, AppError> {
    let purpose = format!("media.{}", kind.as_str());
    let answer = if kind.is_video() {
        ai.video(
            pool,
            campaign,
            &purpose,
            &templates::INTRO_VIDEO,
            &VideoRequest {
                prompt,
                model: None,
                seconds: 8,
            },
        )
        .await?
        .map(|v| (v.bytes, v.mime))
    } else {
        ai.images(
            pool,
            campaign,
            &purpose,
            &templates::PIXEL_ART,
            &[ImageRequest {
                prompt,
                model: None,
            }],
        )
        .await?
        .pop()
        .ok_or(AppError::Internal("no image answer".into()))?
        .map(|i| (i.bytes, i.mime))
    };
    Ok(match answer {
        Ok((bytes, mime)) => (Some(bytes), Some(mime), None),
        Err(e) => (None, None, Some(e.to_string())),
    })
}

/// Rows this server is drawing now. A `drawing` row outside it was cut
/// off by a restart.
static DRAWING: LazyLock<Mutex<HashSet<Uuid>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

fn drawing() -> std::sync::MutexGuard<'static, HashSet<Uuid>> {
    DRAWING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Mark the `drawing` rows of `campaign` that no task draws any more
/// (the server restarted) as rejected, saying so.
///
/// # Errors
///
/// A database error.
pub async fn settle_interrupted(pool: &PgPool, campaign: Uuid) -> Result<(), AppError> {
    let rows: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM media_assets WHERE campaign_id = $1 AND status = 'drawing'",
    )
    .bind(campaign)
    .fetch_all(pool)
    .await?;
    let lost: Vec<Uuid> = {
        let live = drawing();
        rows.into_iter().filter(|id| !live.contains(id)).collect()
    };
    if lost.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "UPDATE media_assets SET status = 'rejected', decided_at = now(),
                error = 'Interrompu : le serveur a redémarré pendant le dessin.'
         WHERE id = ANY($1) AND status = 'drawing'",
    )
    .bind(&lost)
    .execute(pool)
    .await?;
    Ok(())
}

/// One row to draw in the background.
struct Queued {
    id: Uuid,
    kind: Kind,
    subject: String,
    direction: String,
}

/// Draw `queue` one after the other, each row becoming `pending` or
/// `rejected` as soon as its answer comes back. The rows were claimed in
/// [`DRAWING`] before their insert was committed, so no reader ever
/// takes them for interrupted.
fn spawn(pool: PgPool, ai: Ai, campaign: Uuid, queue: Vec<Queued>) {
    tokio::spawn(async move {
        for q in queue {
            let drawn = match draw_queued(&pool, &ai, campaign, &q).await {
                Ok(d) => d,
                Err(AppError::Conflict("AI_BUDGET_EXCEEDED")) => (
                    None,
                    None,
                    Some("Le budget IA de la campagne est épuisé.".to_string()),
                ),
                Err(AppError::Upstream { detail, .. }) => (None, None, Some(detail)),
                Err(e) => {
                    tracing::error!(asset = %q.id, error = ?e, "media drawing failed");
                    (
                        None,
                        None,
                        Some("Le dessin a échoué sur le serveur.".to_string()),
                    )
                }
            };
            if let Err(e) = finish(&pool, campaign, q.id, drawn).await {
                tracing::error!(asset = %q.id, error = ?e, "media drawing not saved");
            }
            drawing().remove(&q.id);
        }
    });
}

async fn draw_queued(
    pool: &PgPool,
    ai: &Ai,
    campaign: Uuid,
    q: &Queued,
) -> Result<Drawn, AppError> {
    let story = campaigns::find(pool, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?
        .story;
    let prompt = prompt(&story, q.kind, &q.subject, &q.direction)?;
    draw(pool, ai, campaign, q.kind, prompt).await
}

/// Save what the model drew for row `id`.
async fn finish(
    pool: &PgPool,
    campaign: Uuid,
    id: Uuid,
    (bytes, mime, error): Drawn,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE media_assets
         SET status = CASE WHEN $2::bytea IS NULL THEN 'rejected' ELSE 'pending' END,
             image = $2, mime = $3, error = $4,
             decided_at = CASE WHEN $2::bytea IS NULL THEN now() END
         WHERE id = $1 AND status = 'drawing'",
    )
    .bind(id)
    .bind(bytes)
    .bind(mime)
    .bind(error)
    .execute(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(())
}

/// Refuse a drawing the budget cannot pay, before anything is written.
async fn afford(
    pool: &PgPool,
    ai: &Ai,
    campaign: Uuid,
    estimate: i64,
) -> Result<Spending, AppError> {
    if !ai.is_configured() {
        return Err(AppError::ServiceUnavailable("AI_NOT_CONFIGURED"));
    }
    let spending = ledger::spending(pool, campaign).await?;
    if estimate > spending.left_micros() {
        return Err(AppError::Conflict("AI_BUDGET_EXCEEDED"));
    }
    Ok(spending)
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

/// Draw one image and keep it pending; a video is drawn in the
/// background and waits as `drawing` meanwhile.
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
    let prompt = prompt(&row.story, ask.kind, &ask.subject, &direction)?;
    afford(pool, ai, campaign, estimate(ai, ask.kind)).await?;
    let drawn = if ask.kind.is_video() {
        None
    } else {
        Some(draw(pool, ai, campaign, ask.kind, prompt).await?)
    };
    let (status, bytes, mime, error) = match drawn {
        None => ("drawing", None, None, None),
        Some((None, _, error)) => ("rejected", None, None, error),
        Some((bytes, mime, _)) => ("pending", bytes, mime, None),
    };
    let mut tx = pool.begin().await?;
    let stored: Row = sqlx::query_as(&format!(
        "INSERT INTO media_assets (campaign_id, kind, subject, direction, status, mime, image, error)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(ask.kind.as_str())
    .bind(&ask.subject)
    .bind(&direction)
    .bind(status)
    .bind(mime)
    .bind(bytes)
    .bind(&error)
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    let stored = asset(stored)?;
    if stored.status == Status::Drawing {
        drawing().insert(stored.id);
    }
    tx.commit().await?;
    if stored.status == Status::Drawing {
        spawn(
            pool.clone(),
            ai.clone(),
            campaign,
            vec![Queued {
                id: stored.id,
                kind: stored.kind,
                subject: stored.subject.clone(),
                direction,
            }],
        );
    }
    // A failed drawing is kept, rejected, with the model's words: the GM
    // sees why and asks again.
    Ok(stored)
}

/// What a batch would draw: every scene, NPC, adversary and place with
/// no image drawing, waiting or approved, and every act with no video.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub images: Vec<Subject>,
    pub videos: Vec<Subject>,
    pub image_micros: i64,
    pub video_micros: i64,
    pub spending: Spending,
    /// A batch is being drawn.
    pub running: bool,
    pub configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Subject {
    pub kind: Kind,
    pub subject: String,
}

/// The subjects of `story` that have nothing drawing, waiting or approved.
fn missing(story: &Campaign, assets: &[Asset]) -> (Vec<Subject>, Vec<Subject>) {
    let covered = |kind: Kind, id: &str| {
        assets
            .iter()
            .any(|a| a.kind == kind && a.subject == id && a.status != Status::Rejected)
    };
    let images = story
        .nodes
        .iter()
        .map(|n| (Kind::Scene, n.id.as_str()))
        .chain(story.npcs.iter().map(|n| (Kind::Npc, n.id.as_str())))
        .chain(
            story
                .adversaries
                .iter()
                .map(|a| (Kind::Adversary, a.id.as_str())),
        )
        .chain(
            story
                .locations
                .iter()
                .map(|l| (Kind::Location, l.id.as_str())),
        )
        .filter(|(k, id)| !covered(*k, id))
        .map(|(kind, id)| Subject {
            kind,
            subject: id.to_string(),
        })
        .collect();
    let videos = story
        .acts
        .iter()
        .filter(|a| !covered(Kind::Intro, &a.id))
        .map(|a| Subject {
            kind: Kind::Intro,
            subject: a.id.clone(),
        })
        .collect();
    (images, videos)
}

/// What a batch would draw now, and what it would cost.
///
/// # Errors
///
/// A database error.
pub async fn plan(pool: &PgPool, ai: &Ai, row: &campaigns::CampaignRow) -> Result<Plan, AppError> {
    let assets = all(pool, row.id).await?;
    let (images, videos) = missing(&row.story, &assets);
    Ok(Plan {
        images,
        videos,
        image_micros: ai.pricing.image(),
        video_micros: ai.pricing.video(),
        spending: ledger::spending(pool, row.id).await?,
        running: assets.iter().any(|a| a.status == Status::Drawing),
        configured: ai.is_configured(),
    })
}

/// The GM asks for every missing image, and the acts' videos if `videos`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchAsk {
    #[serde(default)]
    pub videos: bool,
}

/// Queue a batch and draw it in the background; the rows it queued.
///
/// # Errors
///
/// 404; 400 `NOTHING_TO_DRAW`; 409 `MEDIA_BATCH_RUNNING`,
/// `AI_BUDGET_EXCEEDED`; 503 `AI_NOT_CONFIGURED`.
pub async fn batch(
    pool: &PgPool,
    ai: &Ai,
    gm: &CurrentGm,
    campaign: Uuid,
    ask: &BatchAsk,
) -> Result<Vec<Asset>, AppError> {
    owned_by(campaigns::find(pool, campaign).await?, gm)?;
    settle_interrupted(pool, campaign).await?;
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let assets = all(&mut *tx, campaign).await?;
    if assets.iter().any(|a| a.status == Status::Drawing) {
        return Err(AppError::Conflict("MEDIA_BATCH_RUNNING"));
    }
    let (mut queue, videos) = missing(&row.story, &assets);
    if ask.videos {
        queue.extend(videos);
    }
    if queue.is_empty() {
        return Err(AppError::BadRequest("NOTHING_TO_DRAW"));
    }
    let cost = queue.iter().map(|s| estimate(ai, s.kind)).sum();
    afford(pool, ai, campaign, cost).await?;
    let mut queued = Vec::with_capacity(queue.len());
    for s in &queue {
        let stored: Row = sqlx::query_as(&format!(
            "INSERT INTO media_assets (campaign_id, kind, subject, status)
             VALUES ($1, $2, $3, 'drawing') RETURNING {COLUMNS}"
        ))
        .bind(campaign)
        .bind(s.kind.as_str())
        .bind(&s.subject)
        .fetch_one(&mut *tx)
        .await?;
        queued.push(asset(stored)?);
    }
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    drawing().extend(queued.iter().map(|a| a.id));
    tx.commit().await?;
    spawn(
        pool.clone(),
        ai.clone(),
        campaign,
        queued
            .iter()
            .map(|a| Queued {
                id: a.id,
                kind: a.kind,
                subject: a.subject.clone(),
                direction: String::new(),
            })
            .collect(),
    );
    Ok(queued)
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
        Kind::Intro => story
            .nodes
            .iter()
            .any(|n| n.act == a.subject && entered(&n.id)),
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
