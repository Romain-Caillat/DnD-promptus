//! gm/run-combat (« valider le butin »), player/receive-rewards — the
//! scene's loot goes where the GM says, through the sheet in play
//! (`players::play`): an item of the story becomes a named bag line,
//! coins the rules' first resource. Each line given is a shared journal
//! line, and the player's toast comes from their history.

use promptus_shared::rules::RuleSystem;
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use super::fight::LootLine;
use crate::auth::guard::CurrentGm;
use crate::campaigns;
use crate::error::AppError;
use crate::evening::knowledge::{self, JournalKind};
use crate::live::{self, Topic};
use crate::players::play::{self, Actor, Adjustment};

/// Who receives one loot line.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Give {
    pub index: usize,
    pub character: Uuid,
}

/// What giving `qty` of item `id` puts in a bag: the rules item when the
/// rules have it (or the one the story's item carries its effects with,
/// `from_rules`), else the story's own item by its name and description
/// (`fallback` when the story has no such item either). Shared by loot
/// and shops (`crate::shops`).
#[must_use]
pub fn item_adjustment(
    rules: &RuleSystem,
    story: &promptus_shared::story::Campaign,
    id: &str,
    fallback: &str,
    qty: u32,
) -> Adjustment {
    let story_item = story.items.iter().find(|i| i.id == id);
    let rules_id = if rules.item(id).is_some() {
        Some(id.to_string())
    } else {
        story_item
            .and_then(|i| i.from_rules.clone())
            .filter(|r| rules.item(r).is_some())
    };
    match rules_id {
        Some(item) => Adjustment::GiveItem {
            item: Some(item),
            name: String::new(),
            description: String::new(),
            qty,
        },
        None => Adjustment::GiveItem {
            item: None,
            name: story_item.map_or_else(|| fallback.to_string(), |i| i.name.clone()),
            description: story_item
                .map(|i| i.description.clone())
                .unwrap_or_default(),
            qty,
        },
    }
}

fn adjustment(
    rules: &RuleSystem,
    story: &promptus_shared::story::Campaign,
    line: &LootLine,
) -> Result<Adjustment, AppError> {
    match (&line.item, line.coins) {
        (Some(id), _) => Ok(item_adjustment(rules, story, id, &line.name, 1)),
        (None, Some(n)) => {
            let resource = rules.currency().ok_or(AppError::Conflict("NO_CURRENCY"))?;
            Ok(Adjustment::Resource {
                resource: resource.id.clone(),
                delta: i32::try_from(n).unwrap_or(i32::MAX),
            })
        }
        (None, None) => Ok(Adjustment::GiveItem {
            item: None,
            name: line.name.clone(),
            description: String::new(),
            qty: 1,
        }),
    }
}

/// Hand out loot lines of the last encounter of `campaign` (it may still
/// be live: a pocket searched mid-fight).
///
/// # Errors
///
/// 404; 409 `NO_FIGHT`, `ALREADY_GIVEN`, `NO_CURRENCY`,
/// `CHARACTER_NOT_VALIDATED`; 400 `UNKNOWN_LOOT`.
pub async fn give(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    gives: &[Give],
) -> Result<Vec<LootLine>, AppError> {
    let mut tx = pool.begin().await?;
    let row = crate::auth::guard::owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let found: Option<(Uuid, Option<Uuid>, Json<Vec<LootLine>>)> = sqlx::query_as(
        "SELECT id, session_id, loot FROM encounters WHERE campaign_id = $1
         ORDER BY (status = 'live') DESC, started_at DESC LIMIT 1 FOR UPDATE",
    )
    .bind(campaign)
    .fetch_optional(&mut *tx)
    .await?;
    let (id, session, Json(mut loot)) = found.ok_or(AppError::Conflict("NO_FIGHT"))?;
    for g in gives {
        let line = loot
            .iter_mut()
            .find(|l| l.index == g.index)
            .ok_or(AppError::BadRequest("UNKNOWN_LOOT"))?;
        if line.given_to.is_some() {
            return Err(AppError::Conflict("ALREADY_GIVEN"));
        }
        let adj = adjustment(rules, &row.story, line)?;
        let (sheet, _) =
            play::adjust_in(&mut tx, campaign, rules, g.character, adj, Actor::Gm).await?;
        line.given_to = Some(g.character);
        knowledge::write_about(
            &mut tx,
            campaign,
            session,
            JournalKind::Loot,
            line.item.as_deref(),
            &format!("{} — {}", line.name, sheet.name),
            true,
            Some(g.character),
        )
        .await?;
    }
    sqlx::query("UPDATE encounters SET loot = $2 WHERE id = $1")
        .bind(id)
        .bind(Json(&loot))
        .execute(&mut *tx)
        .await?;
    crate::evening::touch(&mut tx, campaign).await?;
    live::touch(&mut tx, campaign, &Topic::Fight).await?;
    tx.commit().await?;
    Ok(loot)
}
