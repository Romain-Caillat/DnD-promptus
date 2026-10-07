//! player/buy-and-trade — shops the GM opens (migration `033_shops.sql`).
//!
//! - **The GM** opens a shop from an NPC of the story who sells
//!   (Dents-de-Fer: his `sells` at their prices, what he hides in his
//!   `inventory` « sous le comptoir », the haggling check of his scene)
//!   or from nothing, edits its lines (an item of the rules or of the
//!   story, or one they name; a price; a stock; hidden or not) and its
//!   haggling terms, opens and closes it, and reveals a hidden line
//!   (after a discussion, or a search the GM asked a check for).
//! - **A player** buys one unit at a time: the server takes the price
//!   from their purse (the rules' currency) and puts the item in their
//!   bag, through `players::play` like every other change. Each
//!   character may **haggle once** per shop: the server rolls the
//!   shop's check, gives the outcome's XP, and applies what it won
//!   (`promptus_shared::rules::trade`) — one purchase at a discount, a
//!   surcharge on every price after a natural 1, the hidden stock out
//!   after a natural 20.
//!
//! Every write takes the campaign row lock and re-reads inside the
//! transaction (`MEMORY.md` §3) and touches the `session` topic, so the
//! phones refetch `GET /api/play/…/trade` (built by
//! `campaigns::projection::trade`; hidden lines never leave).

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::check::{self, Advantage, OutcomeBand, RollBreakdown, RollTarget};
use promptus_shared::rules::dice::SeededDice;
use promptus_shared::rules::model::RollScope;
use promptus_shared::rules::trade::{self, haggle_outcome};
use promptus_shared::story::Campaign;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, owned_by};
use crate::board::rewards::item_adjustment;
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::evening::knowledge::{self, JournalKind};
use crate::players::play::{self, Actor, Adjustment, combatant};
use crate::players::{Player, Role};

/// Longest shop, keeper or item name, and item description.
pub const NAME_MAX: usize = 80;
pub const TEXT_MAX: usize = 500;
/// Highest price and stock of a line, and most lines in a shop.
pub const PRICE_MAX: u32 = 100_000;
pub const STOCK_MAX: u32 = 999;
pub const LINES_MAX: usize = 60;

/// One line of the shop's counter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Line {
    /// Stable within the shop; given by the server when empty.
    #[serde(default)]
    pub key: String,
    /// An item of the rule system, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// An item of the story (the admiral's compass), by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story_item: Option<String>,
    /// The name of an item the GM names; empty for the others, whose
    /// name comes from the rules or the story.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// In whole units of the rules' currency.
    pub price: u32,
    /// Units left; `None` for as many as the players want.
    #[serde(default)]
    pub stock: Option<u32>,
    /// « Sous le comptoir »: players do not see it until revealed.
    #[serde(default)]
    pub hidden: bool,
}

impl Line {
    /// The name and description players read: the rules', the story's
    /// (never its GM notes nor its effect in rules terms), or the GM's.
    #[must_use]
    pub fn display(&self, rules: &RuleSystem, story: &Campaign) -> (String, String) {
        if let Some(def) = self.item.as_deref().and_then(|id| rules.item(id)) {
            return (def.name.clone(), def.description.clone());
        }
        if let Some(it) = self
            .story_item
            .as_deref()
            .and_then(|id| story.items.iter().find(|i| i.id == id))
        {
            return (it.name.clone(), it.description.clone());
        }
        (self.name.clone(), self.description.clone())
    }
}

/// How the keeper haggles: the check, and what its outcomes do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Haggle {
    /// An ability of the rules (`CHA`).
    pub ability: String,
    pub difficulty: i32,
    /// Off one purchase after a won haggle (Kerjean: half price).
    #[serde(default = "half")]
    pub discount_percent: u32,
    /// Added to every price after a natural 1 (Kerjean: 2 PO).
    #[serde(default)]
    pub fumble_surcharge: u32,
    /// A natural 20 brings out what is under the counter.
    #[serde(default)]
    pub critical_reveals: bool,
}

fn half() -> u32 {
    50
}

/// A shop as the GM holds it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shop {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub name: String,
    pub keeper: String,
    pub npc: Option<String>,
    pub open: bool,
    pub currency: String,
    pub lines: Vec<Line>,
    pub haggle: Option<Haggle>,
    pub surcharge: u32,
    pub created_at: DateTime<Utc>,
}

type Row = (
    Uuid,
    Uuid,
    String,
    String,
    Option<String>,
    bool,
    String,
    Json<Vec<Line>>,
    Option<Json<Haggle>>,
    i32,
    DateTime<Utc>,
);

const COLUMNS: &str =
    "id, campaign_id, name, keeper, npc, open, currency, lines, haggle, surcharge, created_at";

fn from_row(r: Row) -> Shop {
    Shop {
        id: r.0,
        campaign_id: r.1,
        name: r.2,
        keeper: r.3,
        npc: r.4,
        open: r.5,
        currency: r.6,
        lines: r.7.0,
        haggle: r.8.map(|h| h.0),
        surcharge: u32::try_from(r.9).unwrap_or(0),
        created_at: r.10,
    }
}

/// Every shop of `campaign`, the oldest first.
///
/// # Errors
///
/// A database error.
pub async fn list(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<Shop>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM shops WHERE campaign_id = $1 ORDER BY created_at, id"
    ))
    .bind(campaign)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(from_row).collect())
}

async fn locked(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    id: Uuid,
) -> Result<Shop, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM shops WHERE campaign_id = $1 AND id = $2 FOR UPDATE"
    ))
    .bind(campaign)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(from_row).ok_or(AppError::NotFound("NO_SUCH_SHOP"))
}

async fn store(tx: &mut Transaction<'_, Postgres>, shop: &Shop) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE shops SET name = $2, keeper = $3, open = $4, lines = $5, haggle = $6,
                          surcharge = $7
         WHERE id = $1",
    )
    .bind(shop.id)
    .bind(&shop.name)
    .bind(&shop.keeper)
    .bind(shop.open)
    .bind(Json(&shop.lines))
    .bind(shop.haggle.as_ref().map(Json))
    .bind(i32::try_from(shop.surcharge).unwrap_or(i32::MAX))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn clean_name(raw: &str, code: &'static str) -> Result<String, AppError> {
    let name = raw.trim().to_string();
    if name.is_empty() || name.chars().count() > NAME_MAX || name.chars().any(char::is_control) {
        return Err(AppError::BadRequest(code));
    }
    Ok(name)
}

/// A line the GM wrote, checked against the rules and the story, with
/// a key.
fn check_line(rules: &RuleSystem, story: &Campaign, mut line: Line) -> Result<Line, AppError> {
    match (&line.item, &line.story_item) {
        (Some(id), None) => {
            rules.item(id).ok_or(AppError::BadRequest("UNKNOWN_ITEM"))?;
            line.name.clear();
            line.description.clear();
        }
        (None, Some(id)) => {
            story
                .items
                .iter()
                .find(|i| &i.id == id)
                .ok_or(AppError::BadRequest("UNKNOWN_ITEM"))?;
            line.name.clear();
            line.description.clear();
        }
        (None, None) => {
            line.name = clean_name(&line.name, "INVALID_ITEM_NAME")?;
            line.description = line.description.trim().to_string();
            if line.description.chars().count() > TEXT_MAX {
                return Err(AppError::BadRequest("TEXT_TOO_LONG"));
            }
        }
        (Some(_), Some(_)) => return Err(AppError::BadRequest("UNKNOWN_ITEM")),
    }
    if line.price > PRICE_MAX || line.stock.is_some_and(|s| s > STOCK_MAX) {
        return Err(AppError::BadRequest("INVALID_PRICE"));
    }
    if line.key.trim().is_empty() {
        line.key = Uuid::new_v4().to_string();
    }
    Ok(line)
}

fn check_lines(
    rules: &RuleSystem,
    story: &Campaign,
    lines: Vec<Line>,
) -> Result<Vec<Line>, AppError> {
    if lines.len() > LINES_MAX {
        return Err(AppError::BadRequest("TOO_MANY_LINES"));
    }
    let lines: Vec<Line> = lines
        .into_iter()
        .map(|l| check_line(rules, story, l))
        .collect::<Result<_, _>>()?;
    let mut keys: Vec<&str> = lines.iter().map(|l| l.key.as_str()).collect();
    keys.sort_unstable();
    keys.dedup();
    if keys.len() != lines.len() {
        return Err(AppError::BadRequest("DUPLICATE_LINE"));
    }
    Ok(lines)
}

fn check_haggle(rules: &RuleSystem, h: &Haggle) -> Result<(), AppError> {
    rules
        .ability(&h.ability)
        .ok_or(AppError::BadRequest("UNKNOWN_ABILITY"))?;
    if !(1..=40).contains(&h.difficulty) {
        return Err(AppError::BadRequest("INVALID_DIFFICULTY"));
    }
    if h.discount_percent > 100 || h.fumble_surcharge > PRICE_MAX {
        return Err(AppError::BadRequest("INVALID_PRICE"));
    }
    Ok(())
}

/// The shop `npc` of `story` keeps, as it starts: what they sell at
/// their prices, what they hold and do not sell hidden under the counter
/// at the item's value, and the haggling check of a scene they are in
/// (an action that names haggling), with Kerjean's terms: half price on
/// one purchase, +2 on every price after a natural 1, the hidden stock
/// out after a natural 20.
///
/// # Errors
///
/// 400 `UNKNOWN_NPC`, `NOTHING_TO_SELL` when they neither sell nor hold
/// anything.
pub fn from_npc(
    rules: &RuleSystem,
    story: &Campaign,
    npc: &str,
) -> Result<(String, String, Vec<Line>, Option<Haggle>), AppError> {
    let n = story.npc(npc).ok_or(AppError::BadRequest("UNKNOWN_NPC"))?;
    let line = |id: &str, price: u32, hidden: bool| {
        let story_item = story.items.iter().find(|i| i.id == id);
        // The rules item carries the effects (the market's pistol rolls
        // 1d8); the story's own item otherwise (the compass).
        let rules_id = if rules.item(id).is_some() {
            Some(id.to_string())
        } else {
            story_item
                .and_then(|i| i.from_rules.clone())
                .filter(|r| rules.item(r).is_some())
        };
        Line {
            key: id.to_string(),
            story_item: rules_id.is_none().then(|| id.to_string()),
            item: rules_id,
            name: String::new(),
            description: String::new(),
            price,
            stock: None,
            hidden,
        }
    };
    let mut lines: Vec<Line> = n
        .sells
        .iter()
        .map(|s| line(&s.item, s.price, false))
        .collect();
    for h in &n.inventory {
        if lines.iter().any(|l| l.key == h.item) {
            continue;
        }
        let value = story
            .items
            .iter()
            .find(|i| i.id == h.item)
            .and_then(|i| i.value)
            .unwrap_or(0);
        let mut l = line(&h.item, value, true);
        l.stock = Some(h.quantity);
        lines.push(l);
    }
    if lines.is_empty() {
        return Err(AppError::BadRequest("NOTHING_TO_SELL"));
    }
    let planned = story
        .nodes
        .iter()
        .filter(|node| node.npcs.iter().any(|p| p.npc == n.id))
        .flat_map(|node| &node.checks)
        .find(|c| c.action.to_lowercase().contains("marchand"));
    let haggle = planned
        .filter(|c| rules.ability(&c.stat).is_some())
        .map(|c| Haggle {
            ability: c.stat.clone(),
            difficulty: i32::try_from(c.difficulty).unwrap_or(10),
            discount_percent: 50,
            fumble_surcharge: 2,
            critical_reveals: true,
        });
    let name = n
        .location
        .as_deref()
        .and_then(|l| story.location(l))
        .map_or_else(|| n.name.clone(), |l| l.name.clone());
    let keeper = if n.title.is_empty() {
        n.name.clone()
    } else {
        n.title.clone()
    };
    Ok((name, keeper, lines, haggle))
}

/// What the GM sends to open a new shop.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewShop {
    /// Start from what this NPC of the story sells and hides.
    #[serde(default)]
    pub from_npc: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub keeper: String,
}

async fn gm_campaign(
    tx: &mut Transaction<'_, Postgres>,
    gm: &CurrentGm,
    campaign: Uuid,
) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::lock(tx, campaign).await?, gm)
}

/// The currency of the campaign's rules, or the reason there is none.
fn currency(rules: Option<&RuleSystem>) -> Result<(&RuleSystem, String), AppError> {
    let rules = rules.ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let c = rules.currency().ok_or(AppError::Conflict("NO_CURRENCY"))?;
    Ok((rules, c.id.clone()))
}

async fn touch(tx: &mut Transaction<'_, Postgres>, campaign: Uuid) -> Result<(), AppError> {
    crate::evening::touch(tx, campaign).await
}

/// A new shop, closed, in the rules' currency.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 409 `RULES_UNKNOWN`,
/// `NO_CURRENCY` (the rules declare no resource to pay with: add one in
/// the rules editor); 400 the codes of [`from_npc`], `INVALID_NAME`.
pub async fn create(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    new: &NewShop,
) -> Result<Shop, AppError> {
    let mut tx = pool.begin().await?;
    let row = gm_campaign(&mut tx, gm, campaign).await?;
    let (rules, currency) = currency(row.rules())?;
    let (name, keeper, lines, haggle) = match &new.from_npc {
        Some(npc) => from_npc(rules, &row.story, npc)?,
        None => (
            clean_name(&new.name, "INVALID_NAME")?,
            new.keeper.trim().chars().take(NAME_MAX).collect(),
            Vec::new(),
            None,
        ),
    };
    let saved: Row = sqlx::query_as(&format!(
        "INSERT INTO shops (campaign_id, name, keeper, npc, currency, lines, haggle)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(&name)
    .bind(&keeper)
    .bind(&new.from_npc)
    .bind(&currency)
    .bind(Json(&lines))
    .bind(haggle.as_ref().map(Json))
    .fetch_one(&mut *tx)
    .await?;
    touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(from_row(saved))
}

/// What the GM changes in a shop: everything but its currency.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShopEdit {
    pub name: String,
    #[serde(default)]
    pub keeper: String,
    pub lines: Vec<Line>,
    #[serde(default)]
    pub haggle: Option<Haggle>,
    /// The keeper's mood, set back by hand if the GM wants.
    #[serde(default)]
    pub surcharge: Option<u32>,
}

/// Replace the shop's name, keeper, lines and haggling terms.
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`; 400 `INVALID_NAME`, the line codes
/// (`UNKNOWN_ITEM`, `INVALID_ITEM_NAME`, `TEXT_TOO_LONG`,
/// `INVALID_PRICE`, `TOO_MANY_LINES`, `DUPLICATE_LINE`) and the haggle
/// codes (`UNKNOWN_ABILITY`, `INVALID_DIFFICULTY`).
pub async fn edit(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
    edit: ShopEdit,
) -> Result<Shop, AppError> {
    let mut tx = pool.begin().await?;
    let row = gm_campaign(&mut tx, gm, campaign).await?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let mut shop = locked(&mut tx, campaign, id).await?;
    shop.name = clean_name(&edit.name, "INVALID_NAME")?;
    shop.keeper = edit.keeper.trim().chars().take(NAME_MAX).collect();
    shop.lines = check_lines(rules, &row.story, edit.lines)?;
    if let Some(h) = &edit.haggle {
        check_haggle(rules, h)?;
    }
    shop.haggle = edit.haggle;
    if let Some(s) = edit.surcharge {
        shop.surcharge = s.min(PRICE_MAX);
    }
    store(&mut tx, &shop).await?;
    touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(shop)
}

/// Open the shop to the players, or close it.
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`.
pub async fn set_open(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
    open: bool,
) -> Result<Shop, AppError> {
    let mut tx = pool.begin().await?;
    gm_campaign(&mut tx, gm, campaign).await?;
    let mut shop = locked(&mut tx, campaign, id).await?;
    shop.open = open;
    store(&mut tx, &shop).await?;
    touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(shop)
}

/// Bring out line `key` from under the counter (a discussion, a search
/// the GM asked a check for); the table's journal says so.
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`, `NO_SUCH_LINE`.
pub async fn reveal(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
    key: &str,
) -> Result<Shop, AppError> {
    let mut tx = pool.begin().await?;
    let row = gm_campaign(&mut tx, gm, campaign).await?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let mut shop = locked(&mut tx, campaign, id).await?;
    let line = shop
        .lines
        .iter_mut()
        .find(|l| l.key == key)
        .ok_or(AppError::NotFound("NO_SUCH_LINE"))?;
    if line.hidden {
        line.hidden = false;
        let (name, _) = line.display(rules, &row.story);
        let session = session_id(&mut tx, campaign).await?;
        knowledge::write(
            &mut tx,
            campaign,
            session,
            JournalKind::Item,
            None,
            &format!("{} sort de sous le comptoir : {name}", keeper_of(&shop)),
            true,
        )
        .await?;
        store(&mut tx, &shop).await?;
        touch(&mut tx, campaign).await?;
    }
    tx.commit().await?;
    Ok(shop)
}

/// Close and forget the shop.
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`.
pub async fn delete(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    gm_campaign(&mut tx, gm, campaign).await?;
    locked(&mut tx, campaign, id).await?;
    sqlx::query("DELETE FROM shops WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(())
}

fn keeper_of(shop: &Shop) -> &str {
    if shop.keeper.is_empty() {
        &shop.name
    } else {
        &shop.keeper
    }
}

async fn session_id(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
) -> Result<Option<Uuid>, AppError> {
    Ok(crate::evening::session::current(&mut **tx, campaign)
        .await?
        .map(|s| s.id))
}

/// A character's haggle at a shop.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HaggleResult {
    pub shop_id: Uuid,
    pub character_id: Uuid,
    pub band: OutcomeBand,
    pub roll: RollBreakdown,
    pub discount_left: bool,
}

/// Every haggle at the shops of `campaign`.
///
/// # Errors
///
/// A database error.
pub async fn haggles(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
) -> Result<Vec<HaggleResult>, AppError> {
    type HRow = (Uuid, Uuid, String, Json<RollBreakdown>, bool);
    let rows: Vec<HRow> = sqlx::query_as(
        "SELECT h.shop_id, h.character_id, h.band, h.roll, h.discount_left
         FROM shop_haggles h JOIN shops s ON s.id = h.shop_id
         WHERE s.campaign_id = $1 ORDER BY h.created_at",
    )
    .bind(campaign)
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|(shop_id, character_id, band, roll, discount_left)| {
            Ok(HaggleResult {
                shop_id,
                character_id,
                band: serde_json::from_value(serde_json::Value::String(band))
                    .map_err(|e| AppError::internal("haggle band", e))?,
                roll: roll.0,
                discount_left,
            })
        })
        .collect()
}

/// Under the campaign lock: the player's validated character, the open
/// shop `id`, and the campaign.
async fn player_at_shop(
    tx: &mut Transaction<'_, Postgres>,
    player: &Player,
    id: Uuid,
) -> Result<(CampaignRow, Uuid, Shop), AppError> {
    if player.role == Role::Spectator {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let row = campaigns::lock(tx, player.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))?;
    let character: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM characters WHERE player_id = $1")
            .bind(player.id)
            .fetch_optional(&mut **tx)
            .await?;
    let character = character.ok_or(AppError::NotFound("NO_CHARACTER"))?;
    let shop = locked(tx, player.campaign_id, id).await?;
    // A closed shop is not there for players: the same answer as none.
    if !shop.open {
        return Err(AppError::NotFound("NO_SUCH_SHOP"));
    }
    Ok((row, character, shop))
}

/// The player's own haggle at `shop`, locked.
async fn my_haggle(
    tx: &mut Transaction<'_, Postgres>,
    shop: Uuid,
    character: Uuid,
) -> Result<Option<bool>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT discount_left FROM shop_haggles WHERE shop_id = $1 AND character_id = $2 FOR UPDATE",
    )
    .bind(shop)
    .bind(character)
    .fetch_optional(&mut **tx)
    .await?)
}

/// What a player asks to buy.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Purchase {
    pub line: String,
    /// Spend the haggle won on this purchase.
    #[serde(default)]
    pub discounted: bool,
}

/// One unit of line `purchase.line` of open shop `id`, for the player's
/// character: paid from their purse, put in their bag, the stock and
/// the won discount spent. Both the purse and the bag change through
/// `players::play` (logged « par le joueur »), and the table's journal
/// says who bought what.
///
/// # Errors
///
/// 403 `SPECTATOR`; 404 `NO_CHARACTER`, `NO_SUCH_SHOP`, `NO_SUCH_LINE`;
/// 409 `RULES_UNKNOWN`, `SOLD_OUT`, `NO_DISCOUNT`, `NOT_ENOUGH` (the
/// purse is short), `CHARACTER_NOT_VALIDATED`.
pub async fn buy(
    pool: &PgPool,
    player: &Player,
    id: Uuid,
    purchase: &Purchase,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let (row, character, mut shop) = player_at_shop(&mut tx, player, id).await?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let at = shop
        .lines
        .iter()
        .position(|l| l.key == purchase.line && !l.hidden)
        .ok_or(AppError::NotFound("NO_SUCH_LINE"))?;
    if shop.lines[at].stock == Some(0) {
        return Err(AppError::Conflict("SOLD_OUT"));
    }
    let discount = if purchase.discounted {
        if my_haggle(&mut tx, shop.id, character).await? != Some(true) {
            return Err(AppError::Conflict("NO_DISCOUNT"));
        }
        Some(shop.haggle.as_ref().map_or(0, |h| h.discount_percent))
    } else {
        None
    };
    let price = trade::price(shop.lines[at].price, shop.surcharge, discount);
    let line = shop.lines[at].clone();
    let (name, _) = line.display(rules, &row.story);
    if price > 0 {
        play::adjust_in(
            &mut tx,
            player.campaign_id,
            rules,
            character,
            Adjustment::Resource {
                resource: shop.currency.clone(),
                delta: -i32::try_from(price).unwrap_or(i32::MAX),
            },
            Actor::Player,
        )
        .await?;
    }
    let given = match (&line.item, &line.story_item) {
        (Some(id), _) | (None, Some(id)) => item_adjustment(rules, &row.story, id, &name, 1),
        (None, None) => Adjustment::GiveItem {
            item: None,
            name: line.name.clone(),
            description: line.description.clone(),
            qty: 1,
        },
    };
    let (sheet, _) = play::adjust_in(
        &mut tx,
        player.campaign_id,
        rules,
        character,
        given,
        Actor::Player,
    )
    .await?;
    if let Some(stock) = shop.lines[at].stock.as_mut() {
        *stock -= 1;
    }
    if discount.is_some() {
        sqlx::query(
            "UPDATE shop_haggles SET discount_left = false WHERE shop_id = $1 AND character_id = $2",
        )
        .bind(shop.id)
        .bind(character)
        .execute(&mut *tx)
        .await?;
    }
    store(&mut tx, &shop).await?;
    let abbr = rules
        .resources
        .iter()
        .find(|r| r.id == shop.currency)
        .map_or_else(String::new, |r| r.abbr.clone());
    let session = session_id(&mut tx, player.campaign_id).await?;
    knowledge::write_about(
        &mut tx,
        player.campaign_id,
        session,
        JournalKind::Item,
        None,
        &format!(
            "{} achète {name} à {} ({price} {abbr})",
            sheet.name,
            keeper_of(&shop)
        ),
        true,
        Some(character),
    )
    .await?;
    touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(())
}

/// The player's one haggle at open shop `id`: the server rolls the
/// shop's check for their character, grants the outcome's XP, and
/// applies what it won — a discount on one purchase, a surcharge on
/// every price after a natural 1, the hidden stock out after a natural
/// 20. The table's journal says how it went.
///
/// # Errors
///
/// 403 `SPECTATOR`; 404 `NO_CHARACTER`, `NO_SUCH_SHOP`; 409
/// `RULES_UNKNOWN`, `NO_HAGGLE` (the keeper does not haggle),
/// `ALREADY_HAGGLED`, `CHARACTER_NOT_VALIDATED`, `NO_PLAY_SHEET`.
pub async fn haggle(pool: &PgPool, player: &Player, id: Uuid) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let (row, character, mut shop) = player_at_shop(&mut tx, player, id).await?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let terms = shop.haggle.clone().ok_or(AppError::Conflict("NO_HAGGLE"))?;
    if my_haggle(&mut tx, shop.id, character).await?.is_some() {
        return Err(AppError::Conflict("ALREADY_HAGGLED"));
    }
    let (sheet, state) =
        play::in_play_locked(&mut tx, player.campaign_id, character, rules).await?;
    let c = combatant(rules, &sheet, &state).ok_or(AppError::Conflict("NO_PLAY_SHEET"))?;
    let difficulty = rules
        .difficulties
        .iter()
        .find(|d| d.value == terms.difficulty);
    let roll = check::ability_check(
        rules,
        &c,
        &terms.ability,
        RollScope::Checks,
        Some(RollTarget::Difficulty {
            id: difficulty.map(|d| d.id.clone()),
            value: terms.difficulty,
        }),
        Advantage::Normal,
        &mut SeededDice::from_os(),
    )
    .map_err(|e| AppError::Internal(format!("haggle check: {e:?}")))?;
    let band = roll.band.unwrap_or(OutcomeBand::Failure);
    let won = haggle_outcome(band, terms.fumble_surcharge, terms.critical_reveals);
    let xp = band.xp(rules);
    if xp > 0 {
        play::adjust_in(
            &mut tx,
            player.campaign_id,
            rules,
            character,
            Adjustment::Xp {
                delta: i32::try_from(xp).unwrap_or(i32::MAX),
            },
            Actor::Rules,
        )
        .await?;
    }
    sqlx::query(
        "INSERT INTO shop_haggles (shop_id, character_id, band, roll, discount_left)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(shop.id)
    .bind(character)
    .bind(
        serde_json::to_value(band)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default(),
    )
    .bind(Json(&roll))
    .bind(won.discount)
    .execute(&mut *tx)
    .await?;
    let session = session_id(&mut tx, player.campaign_id).await?;
    let ability = rules
        .ability(&terms.ability)
        .map_or(terms.ability.clone(), |a| a.name.clone());
    let against = difficulty.map_or_else(|| terms.difficulty.to_string(), |d| d.name.clone());
    knowledge::write(
        &mut tx,
        player.campaign_id,
        session,
        JournalKind::Roll,
        None,
        &format!(
            "{} marchande avec {} : {ability} {} contre {against} → {}",
            sheet.name,
            keeper_of(&shop),
            roll.total,
            band.name(rules)
        ),
        true,
    )
    .await?;
    shop.surcharge = shop.surcharge.saturating_add(won.surcharge).min(PRICE_MAX);
    if won.reveals && shop.lines.iter().any(|l| l.hidden) {
        for l in &mut shop.lines {
            l.hidden = false;
        }
        knowledge::write(
            &mut tx,
            player.campaign_id,
            session,
            JournalKind::Item,
            None,
            &format!("{} sort ce qu'il cache sous le comptoir", keeper_of(&shop)),
            true,
        )
        .await?;
    }
    store(&mut tx, &shop).await?;
    touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(())
}
