//! A GM's campaigns in the database (migration `003_campaigns.sql`).
//!
//! The story and the world are JSONB documents typed by
//! `promptus_shared::story`. Reads take the whole row; writes go through
//! [`lock`]: the campaign row is locked with `SELECT … FOR UPDATE` and
//! re-read inside the transaction, so a GM and players acting at the
//! same moment run one after the other instead of overwriting each
//! other (`MEMORY.md` §3). [`update_world`] wraps that for world writes.

pub mod projection;

use chrono::{DateTime, Utc};
use promptus_shared::story::{Campaign, WorldState};
use serde::Serialize;
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, OwnedByGm, owned_by};
use crate::error::AppError;

/// A campaign row: its story, its world, and who owns it.
#[derive(Debug, Clone)]
pub struct CampaignRow {
    pub id: Uuid,
    pub gm_id: Uuid,
    pub story: Campaign,
    pub world: WorldState,
    pub updated_at: DateTime<Utc>,
}

impl OwnedByGm for CampaignRow {
    fn owner_id(&self) -> Uuid {
        self.gm_id
    }
}

/// One line of the GM's campaign list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignSummary {
    pub id: Uuid,
    pub title: String,
    pub world: String,
    pub updated_at: DateTime<Utc>,
}

type Row = (Uuid, Uuid, Json<Campaign>, Json<WorldState>, DateTime<Utc>);

fn from_row((id, gm_id, story, world, updated_at): Row) -> CampaignRow {
    CampaignRow {
        id,
        gm_id,
        story: story.0,
        world: world.0,
        updated_at,
    }
}

const COLUMNS: &str = "id, gm_id, story, world, updated_at";

/// Store a new campaign for `gm`, with an empty world.
///
/// # Errors
///
/// Fails on a database error.
pub async fn create(
    pool: &PgPool,
    gm: &CurrentGm,
    story: &Campaign,
) -> Result<CampaignRow, AppError> {
    let row: Row = sqlx::query_as(&format!(
        "INSERT INTO campaigns (gm_id, story, world) VALUES ($1, $2, $3) RETURNING {COLUMNS}"
    ))
    .bind(gm.id)
    .bind(Json(story))
    .bind(Json(WorldState::default()))
    .fetch_one(pool)
    .await?;
    Ok(from_row(row))
}

/// `gm`'s campaigns, most recently changed first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(pool: &PgPool, gm: &CurrentGm) -> Result<Vec<CampaignSummary>, AppError> {
    let rows: Vec<(Uuid, String, Option<String>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, story->>'title', story->>'world', updated_at
         FROM campaigns WHERE gm_id = $1 ORDER BY updated_at DESC",
    )
    .bind(gm.id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, title, world, updated_at)| CampaignSummary {
            id,
            title,
            world: world.unwrap_or_default(),
            updated_at,
        })
        .collect())
}

/// The campaign `id`, whoever owns it. Callers pass it to `owned_by`.
///
/// # Errors
///
/// Fails on a database error, or when the stored documents no longer
/// match the model.
pub async fn find(db: impl PgExecutor<'_>, id: Uuid) -> Result<Option<CampaignRow>, AppError> {
    let row: Option<Row> =
        sqlx::query_as(&format!("SELECT {COLUMNS} FROM campaigns WHERE id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await?;
    Ok(row.map(from_row))
}

/// Lock the campaign row until `tx` ends and read it again **inside**
/// the transaction: what a write builds on is what is committed now, not
/// what was read before waiting for the lock.
///
/// # Errors
///
/// Fails on a database error.
pub async fn lock(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<CampaignRow>, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM campaigns WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(from_row))
}

/// Write the world of a campaign locked by [`lock`] in `tx`.
///
/// # Errors
///
/// Fails on a database error.
pub async fn save_world(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    world: &WorldState,
) -> Result<(), AppError> {
    sqlx::query("UPDATE campaigns SET world = $2 WHERE id = $1")
        .bind(id)
        .bind(Json(world))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Replace the story of a campaign locked by [`lock`] in `tx`. The world
/// is kept: ids are stable, so what was found stays found.
///
/// # Errors
///
/// Fails on a database error.
pub async fn save_story(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    story: &Campaign,
) -> Result<(), AppError> {
    sqlx::query("UPDATE campaigns SET story = $2 WHERE id = $1")
        .bind(id)
        .bind(Json(story))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// One world write by `gm` on campaign `id`, under the campaign lock:
/// lock and re-read, check ownership, apply `change` to the fresh world,
/// write it back, commit. Returns what `change` returned.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; whatever `change`
/// returns; a database error.
pub async fn update_world<R>(
    pool: &PgPool,
    gm: &CurrentGm,
    id: Uuid,
    change: impl FnOnce(&Campaign, &mut WorldState) -> Result<R, AppError>,
) -> Result<R, AppError> {
    let mut tx = pool.begin().await?;
    let mut row = owned_by(lock(&mut tx, id).await?, gm)?;
    let out = change(&row.story, &mut row.world)?;
    save_world(&mut tx, id, &row.world).await?;
    tx.commit().await?;
    Ok(out)
}
