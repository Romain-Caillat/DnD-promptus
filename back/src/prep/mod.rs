//! Preparing a campaign (`campaign/review-story-graph`): the GM's edits
//! on the review screen, the co-GM's workshop, the readiness gauge of
//! each act (`campaign/check-act-readiness`), and the validation that
//! makes the campaign playable; and the campaign generated from a pitch
//! (`ai/generate-campaign`), which lands here to be reviewed.
//!
//! Every write takes the campaign lock (`campaigns::lock`). Edits never
//! wait on the validator: it reports, the GM decides — except at the
//! very end, where a campaign with errors (a reference to nothing, a
//! duplicate id) is not declared playable.

pub mod generation;
pub mod workshop;

use promptus_shared::maps::Map;
use promptus_shared::story::edit::{self, Change, Edit};
use promptus_shared::story::{ActReadiness, Library, Severity, readiness, validate};
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;

/// The most edits one save carries: a typo guard.
pub const EDITS_MAX: usize = 200;

/// `edit::EditError` as the API says it.
pub(crate) fn edit_error(e: &edit::EditError) -> AppError {
    AppError::Invalid {
        code: e.code,
        detail: format!("edit {}: {}", e.index, e.detail),
    }
}

/// The GM's edits, applied whole under the campaign lock. Returns the
/// campaign as stored and what changed.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 400 `EDIT_*` (the
/// first edit that does not fit, `EDIT_SCENE_INVALID` when it leaves a
/// scene's fight, sound, checks, exits or loot naming nothing, checked
/// against the campaign's rule system) or `TOO_MANY_EDITS`; a database
/// error.
pub async fn apply(
    pool: &sqlx::PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    edits: &[Edit],
) -> Result<(CampaignRow, Vec<Change>), AppError> {
    if edits.len() > EDITS_MAX {
        return Err(AppError::BadRequest("TOO_MANY_EDITS"));
    }
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let library = Library {
        rules: row.rules(),
        maps: None,
    };
    let (story, changes) =
        edit::apply_with(&row.story, edits, &library).map_err(|e| edit_error(&e))?;
    campaigns::save_story(&mut tx, campaign, &story).await?;
    let row = campaigns::find(&mut *tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    tx.commit().await?;
    Ok((row, changes))
}

/// Declare the campaign playable. A campaign with errors is not: the
/// GM fixes them first (warnings are theirs to weigh).
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 409
/// `CAMPAIGN_HAS_ERRORS`; a database error.
pub async fn validate_campaign(
    pool: &sqlx::PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
) -> Result<CampaignRow, AppError> {
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    if validate(&row.story)
        .iter()
        .any(|i| i.severity == Severity::Error)
    {
        return Err(AppError::Conflict("CAMPAIGN_HAS_ERRORS"));
    }
    sqlx::query("UPDATE campaigns SET validated_at = now() WHERE id = $1")
        .bind(campaign)
        .execute(&mut *tx)
        .await?;
    crate::live::touch(&mut tx, campaign, &crate::live::Topic::Story).await?;
    let row = campaigns::find(&mut *tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    tx.commit().await?;
    Ok(row)
}

/// The readiness gauge of every act, its planned fights simulated with
/// the campaign's rule system on the maps a scene may be played on: the
/// campaign's validated maps, then the world's. CPU work: run off the
/// async runtime.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; a database error.
pub async fn act_readiness(
    pool: &sqlx::PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
) -> Result<Vec<ActReadiness>, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let mut maps: Vec<Map> = crate::campaign_maps::validated(pool, campaign).await?;
    let world: Vec<Map> = crate::content::maps(&row.story)
        .filter(|w| !maps.iter().any(|m| m.id == w.id))
        .cloned()
        .collect();
    maps.extend(world);
    tokio::task::spawn_blocking(move || {
        readiness(
            &row.story,
            &Library {
                rules: row.rules(),
                maps: Some(&maps),
            },
        )
    })
    .await
    .map_err(|e| AppError::internal("readiness", e))
}
