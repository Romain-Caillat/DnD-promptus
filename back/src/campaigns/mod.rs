//! A GM's campaigns in the database (migration `003_campaigns.sql`).
//!
//! The story and the world are JSONB documents typed by
//! `promptus_shared::story`. Reads take the whole row; writes go through
//! [`lock`]: the campaign row is locked with `SELECT … FOR UPDATE` and
//! re-read inside the transaction, so a GM and players acting at the
//! same moment run one after the other instead of overwriting each
//! other (`MEMORY.md` §3). [`update_world`] wraps that for world writes.

pub mod projection;
pub mod rules_seen;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::story::{Campaign, RuleSystemRef, WorldState};
use serde::Serialize;
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, OwnedByGm, owned_by};
use crate::error::AppError;
use crate::live::{self, Topic};

/// A campaign row: its story, its world, who owns it, and the GM's
/// planning (migration `006_campaign_settings.sql`).
#[derive(Debug, Clone)]
pub struct CampaignRow {
    pub id: Uuid,
    pub gm_id: Uuid,
    pub story: Campaign,
    pub world: WorldState,
    pub settings: Settings,
    /// When the GM shelved it; `None` while it is in use.
    pub archived_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    /// The rule system `story.rules` names: the campaign's own locked
    /// version, or the preset (`crate::rules`). `None` when the server
    /// has neither.
    pub rules: Option<Arc<RuleSystem>>,
}

impl CampaignRow {
    /// The campaign's rule system, if the server has it.
    #[must_use]
    pub fn rules(&self) -> Option<&RuleSystem> {
        self.rules.as_deref()
    }
}

/// The GM's planning of a campaign: never sent to a player, never in
/// the YAML export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// How many players the campaign is prepared for.
    pub player_count: i16,
    /// The most the campaign may spend on AI calls, in US cents. 0: none.
    pub ai_budget_cents: i32,
}

impl Settings {
    pub const PLAYERS: std::ops::RangeInclusive<i16> = 1..=12;
    /// 10 000 $: a typo guard, not a policy.
    pub const MAX_BUDGET_CENTS: i32 = 1_000_000;

    /// The settings, or the code of the first field out of range (the
    /// database refuses the same ranges, migration 006).
    ///
    /// # Errors
    ///
    /// 400 `INVALID_PLAYER_COUNT` or `INVALID_AI_BUDGET`.
    pub fn checked(self) -> Result<Self, AppError> {
        if !Self::PLAYERS.contains(&self.player_count) {
            return Err(AppError::BadRequest("INVALID_PLAYER_COUNT"));
        }
        if !(0..=Self::MAX_BUDGET_CENTS).contains(&self.ai_budget_cents) {
            return Err(AppError::BadRequest("INVALID_AI_BUDGET"));
        }
        Ok(self)
    }
}

impl Default for Settings {
    /// The column defaults of migration 006: Romain's table of six, and
    /// no AI spending until the GM sets a budget.
    fn default() -> Self {
        Self {
            player_count: 6,
            ai_budget_cents: 0,
        }
    }
}

impl OwnedByGm for CampaignRow {
    fn owner_id(&self) -> Uuid {
        self.gm_id
    }
}

/// One card of the GM's campaign list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignSummary {
    pub id: Uuid,
    pub title: String,
    pub world: String,
    pub rules: RuleSystemRef,
    pub player_count: i16,
    /// Players who joined to play (spectators are not seated).
    pub players_seated: i64,
    pub archived_at: Option<DateTime<Utc>>,
    /// The latest of: a change to the campaign, a player seen, a
    /// character sheet written.
    pub last_activity_at: DateTime<Utc>,
}

type Row = (
    Uuid,
    Uuid,
    Json<Campaign>,
    Json<WorldState>,
    i16,
    i32,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
    Option<String>,
);

fn from_row(
    (id, gm_id, story, world, player_count, ai_budget_cents, archived_at, updated_at, stored_rules): Row,
) -> CampaignRow {
    let rules = crate::rules::resolve(id, &story.0.rules, stored_rules.as_deref());
    CampaignRow {
        id,
        gm_id,
        story: story.0,
        world: world.0,
        settings: Settings {
            player_count,
            ai_budget_cents,
        },
        archived_at,
        updated_at,
        rules,
    }
}

/// Every column of a campaign row, and the text of the locked rule
/// version its story names, when the campaign has its own.
const COLUMNS: &str =
    "id, gm_id, story, world, player_count, ai_budget_cents, archived_at, updated_at,
     (SELECT rv.system FROM rule_versions rv
       WHERE rv.campaign_id = campaigns.id
         AND rv.version = (campaigns.story->'rules'->>'version')::int
         AND rv.locked_at IS NOT NULL)";

/// Store a new campaign for `gm`, with an empty world and the default
/// [`Settings`].
///
/// # Errors
///
/// Fails on a database error.
pub async fn create(
    pool: &PgPool,
    gm: &CurrentGm,
    story: &Campaign,
) -> Result<CampaignRow, AppError> {
    create_planned(pool, gm, story, Settings::default()).await
}

/// [`create`], with the GM's [`Settings`] (already [`Settings::checked`]).
///
/// # Errors
///
/// Fails on a database error.
pub async fn create_planned(
    pool: &PgPool,
    gm: &CurrentGm,
    story: &Campaign,
    settings: Settings,
) -> Result<CampaignRow, AppError> {
    let row: Row = sqlx::query_as(&format!(
        "INSERT INTO campaigns (gm_id, story, world, player_count, ai_budget_cents)
         VALUES ($1, $2, $3, $4, $5) RETURNING {COLUMNS}"
    ))
    .bind(gm.id)
    .bind(Json(story))
    .bind(Json(WorldState::default()))
    .bind(settings.player_count)
    .bind(settings.ai_budget_cents)
    .fetch_one(pool)
    .await?;
    Ok(from_row(row))
}

type SummaryRow = (
    Uuid,
    String,
    Option<String>,
    Json<RuleSystemRef>,
    i16,
    i64,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
);

/// `gm`'s campaigns, archived ones included, most recent activity first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(pool: &PgPool, gm: &CurrentGm) -> Result<Vec<CampaignSummary>, AppError> {
    // GREATEST ignores NULLs: a campaign nobody joined is as recent as
    // its last change.
    let rows: Vec<SummaryRow> = sqlx::query_as(
        "SELECT c.id, c.story->>'title', c.story->>'world', c.story->'rules',
                c.player_count,
                (SELECT COUNT(*) FROM players p
                  WHERE p.campaign_id = c.id AND p.role = 'player'),
                c.archived_at,
                GREATEST(
                  c.updated_at,
                  (SELECT MAX(p.last_seen_at) FROM players p WHERE p.campaign_id = c.id),
                  (SELECT MAX(ch.updated_at) FROM characters ch WHERE ch.campaign_id = c.id)
                ) AS last_activity
         FROM campaigns c WHERE c.gm_id = $1
         ORDER BY last_activity DESC, c.id",
    )
    .bind(gm.id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, title, world, rules, player_count, seated, archived_at, last)| CampaignSummary {
                id,
                title,
                world: world.unwrap_or_default(),
                rules: rules.0,
                player_count,
                players_seated: seated,
                archived_at,
                last_activity_at: last,
            },
        )
        .collect())
}

/// Write the [`Settings`] of a campaign locked by [`lock`] in `tx`.
/// Players never see them, so no live topic moves.
///
/// # Errors
///
/// Fails on a database error.
pub async fn save_settings(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    settings: Settings,
) -> Result<(), AppError> {
    sqlx::query("UPDATE campaigns SET player_count = $2, ai_budget_cents = $3 WHERE id = $1")
        .bind(id)
        .bind(settings.player_count)
        .bind(settings.ai_budget_cents)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Shelve campaign `id` of `gm`, or take it off the shelf. Shelving an
/// archived campaign keeps its first date. Players are not told: the
/// shelf is the GM's.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; a database error.
pub async fn set_archived(
    pool: &PgPool,
    gm: &CurrentGm,
    id: Uuid,
    archived: bool,
) -> Result<CampaignRow, AppError> {
    let mut tx = pool.begin().await?;
    owned_by(lock(&mut tx, id).await?, gm)?;
    let row: Row = sqlx::query_as(&format!(
        "UPDATE campaigns
         SET archived_at = CASE WHEN $2 THEN COALESCE(archived_at, now()) END
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(archived)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(from_row(row))
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

/// The display name of the GM who runs `row`, as players see it.
///
/// # Errors
///
/// Fails on a database error.
pub async fn gm_name(db: impl PgExecutor<'_>, row: &CampaignRow) -> Result<String, AppError> {
    Ok(
        sqlx::query_scalar("SELECT display_name FROM gms WHERE id = $1")
            .bind(row.gm_id)
            .fetch_one(db)
            .await?,
    )
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

/// Write the world of a campaign locked by [`lock`] in `tx`, and tell
/// its live sockets when `tx` commits (`live::touch`).
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
    live::touch(tx, id, &Topic::World).await?;
    Ok(())
}

/// Replace the story of a campaign locked by [`lock`] in `tx`. The world
/// is kept: ids are stable, so what was found stays found. Its live
/// sockets hear of it when `tx` commits (`live::touch`).
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
    live::touch(tx, id, &Topic::Story).await?;
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
