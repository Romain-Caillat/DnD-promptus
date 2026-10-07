//! player/buy-and-trade (Corsaires acte 1, « le marché noir de
//! Kerjean »): the purse is the rules' first resource. The GM prepares a
//! shop — its lines, each an item of the rules or one the GM names, a
//! price and maybe a stock, some kept « sous le comptoir » until the GM
//! reveals them after a roll or a talk — and opens it to the table. A
//! player buys from it, or haggles once per line: the server rolls the
//! ability the GM chose against the GM's difficulty, and a success
//! takes the GM's discount off that line for that player. Players share
//! what they carry: coins or a bag line, to another character in play.
//!
//! Every purchase, haggle and gift is a shared journal line; the sheets
//! move through `players::play`, logged as the player's own gestures.
//! The player sees the shop through `campaigns::projection::shop`, never
//! a hidden line.

use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::check::{self, Advantage, OutcomeBand, RollBreakdown, RollTarget};
use promptus_shared::rules::dice::SeededDice;
use promptus_shared::rules::model::{ResourceDef, RollScope};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;
use crate::evening::knowledge::{self, JournalKind};
use crate::evening::session;
use crate::players::play::{self, Actor, Adjustment, ITEM_NAME_MAX, ITEM_TEXT_MAX, QTY_MAX};
use crate::players::{Player, Role};

/// Longest shop name, in characters.
pub const NAME_MAX: usize = 80;
/// Most lines a shop holds.
pub const LINES_MAX: usize = 40;
/// Highest price and stock the GM may set.
pub const PRICE_MAX: u32 = 100_000;
pub const STOCK_MAX: u32 = 999;

/// One line of a shop, as the GM sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopLine {
    pub id: String,
    /// An item of the rules, by id; `None` for one the GM names.
    #[serde(default)]
    pub item: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub price: u32,
    /// `None`: as many as the table wants.
    #[serde(default)]
    pub stock: Option<u32>,
    /// « Sous le comptoir »: the table does not see it yet.
    #[serde(default)]
    pub hidden: bool,
}

impl ShopLine {
    /// The name the table reads.
    #[must_use]
    pub fn display_name(&self, rules: &RuleSystem) -> String {
        match &self.item {
            Some(id) if self.name.is_empty() => rules
                .item(id)
                .map_or_else(|| id.clone(), |i| i.name.clone()),
            _ => self.name.clone(),
        }
    }

    /// The description the table reads.
    #[must_use]
    pub fn display_description(&self, rules: &RuleSystem) -> String {
        match &self.item {
            Some(id) if self.description.is_empty() => rules
                .item(id)
                .map(|i| i.description.clone())
                .unwrap_or_default(),
            _ => self.description.clone(),
        }
    }
}

/// How the table may haggle in a shop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HaggleRule {
    pub ability: String,
    pub difficulty: i32,
    /// Percent off the line for a success.
    pub discount: u32,
}

/// One player's try at haggling a line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Haggle {
    pub line: String,
    pub player: Uuid,
    pub success: bool,
}

/// A shop, as the GM sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shop {
    pub id: Uuid,
    pub name: String,
    pub open: bool,
    pub lines: Vec<ShopLine>,
    pub haggle: Option<HaggleRule>,
    pub haggles: Vec<Haggle>,
}

impl Shop {
    /// What `line` costs `player`: the GM's price, less the discount
    /// their haggle earned.
    #[must_use]
    pub fn price_for(&self, line: &ShopLine, player: Uuid) -> u32 {
        let won = self
            .haggles
            .iter()
            .any(|h| h.line == line.id && h.player == player && h.success);
        match (&self.haggle, won) {
            (Some(rule), true) => line.price * (100 - rule.discount.min(100)) / 100,
            _ => line.price,
        }
    }

    /// `player`'s haggle on `line`, if they tried.
    #[must_use]
    pub fn haggle_of(&self, line: &str, player: Uuid) -> Option<&Haggle> {
        self.haggles
            .iter()
            .find(|h| h.line == line && h.player == player)
    }
}

/// A line as the GM writes it; a line without an id is new.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineInput {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub item: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub price: u32,
    #[serde(default)]
    pub stock: Option<u32>,
    #[serde(default)]
    pub hidden: bool,
}

/// A shop as the GM writes it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShopInput {
    pub name: String,
    #[serde(default)]
    pub lines: Vec<LineInput>,
    #[serde(default)]
    pub haggle: Option<HaggleRule>,
}

type Row = (
    Uuid,
    String,
    bool,
    Json<Vec<ShopLine>>,
    Option<String>,
    Option<i32>,
    Option<i32>,
    Json<Vec<Haggle>>,
);

const COLUMNS: &str =
    "id, name, is_open, lines, haggle_ability, haggle_difficulty, haggle_discount, haggles";

fn from_row((id, name, open, lines, ability, difficulty, discount, haggles): Row) -> Shop {
    Shop {
        id,
        name,
        open,
        lines: lines.0,
        haggle: match (ability, difficulty, discount) {
            (Some(ability), Some(difficulty), Some(discount)) => Some(HaggleRule {
                ability,
                difficulty,
                discount: u32::try_from(discount).unwrap_or(0),
            }),
            _ => None,
        },
        haggles: haggles.0,
    }
}

fn new_line_id() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}

/// The GM's shop checked against the rules; lines keep their ids, new
/// lines get one.
fn checked(rules: &RuleSystem, input: &ShopInput) -> Result<(String, Vec<ShopLine>), AppError> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > NAME_MAX {
        return Err(AppError::BadRequest("INVALID_SHOP_NAME"));
    }
    if input.lines.len() > LINES_MAX {
        return Err(AppError::BadRequest("TOO_MANY_LINES"));
    }
    if input.haggle.as_ref().is_some_and(|h| {
        !rules.abilities.iter().any(|a| a.id == h.ability)
            || !(1..=90).contains(&h.discount)
            || !(1..=99).contains(&h.difficulty)
    }) {
        return Err(AppError::BadRequest("INVALID_HAGGLE"));
    }
    let mut lines = Vec::with_capacity(input.lines.len());
    for l in &input.lines {
        if let Some(item) = &l.item {
            if rules.item(item).is_none() {
                return Err(AppError::BadRequest("UNKNOWN_ITEM"));
            }
        } else if l.name.trim().is_empty() {
            return Err(AppError::BadRequest("INVALID_ITEM_NAME"));
        }
        if l.name.trim().chars().count() > ITEM_NAME_MAX
            || l.description.trim().chars().count() > ITEM_TEXT_MAX
        {
            return Err(AppError::BadRequest("TEXT_TOO_LONG"));
        }
        if l.price > PRICE_MAX || l.stock.is_some_and(|s| s > STOCK_MAX) {
            return Err(AppError::BadRequest("INVALID_PRICE"));
        }
        let id =
            l.id.clone()
                .filter(|id| !id.is_empty() && id.len() <= 40)
                .unwrap_or_else(new_line_id);
        if lines.iter().any(|x: &ShopLine| x.id == id) {
            return Err(AppError::BadRequest("DUPLICATE_LINE"));
        }
        lines.push(ShopLine {
            id,
            item: l.item.clone(),
            name: l.name.trim().to_string(),
            description: l.description.trim().to_string(),
            price: l.price,
            stock: l.stock,
            hidden: l.hidden,
        });
    }
    Ok((name.to_string(), lines))
}

/// The shops of `campaign`, oldest first.
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

/// The shop open at `campaign`'s table, if any.
///
/// # Errors
///
/// A database error.
pub async fn open_shop(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Option<Shop>, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM shops WHERE campaign_id = $1 AND is_open"
    ))
    .bind(campaign)
    .fetch_optional(db)
    .await?;
    Ok(row.map(from_row))
}

async fn locked(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    shop: Uuid,
) -> Result<Shop, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM shops WHERE id = $1 AND campaign_id = $2 FOR UPDATE"
    ))
    .bind(shop)
    .bind(campaign)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(from_row).ok_or(AppError::NotFound("NO_SUCH_SHOP"))
}

async fn store(tx: &mut Transaction<'_, Postgres>, shop: &Shop) -> Result<(), AppError> {
    let h = shop.haggle.as_ref();
    sqlx::query(
        "UPDATE shops SET name = $2, is_open = $3, lines = $4, haggle_ability = $5,
                haggle_difficulty = $6, haggle_discount = $7, haggles = $8
         WHERE id = $1",
    )
    .bind(shop.id)
    .bind(&shop.name)
    .bind(shop.open)
    .bind(Json(&shop.lines))
    .bind(h.map(|h| h.ability.clone()))
    .bind(h.map(|h| h.difficulty))
    .bind(h.map(|h| i32::try_from(h.discount).unwrap_or(0)))
    .bind(Json(&shop.haggles))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The campaign `gm` owns, locked, with its rules.
async fn gm_lock(
    tx: &mut Transaction<'_, Postgres>,
    gm: &CurrentGm,
    campaign: Uuid,
) -> Result<campaigns::CampaignRow, AppError> {
    owned_by(campaigns::lock(tx, campaign).await?, gm)
}

/// A shop the GM prepares, closed.
///
/// # Errors
///
/// 404; 409 `RULES_UNKNOWN`; 400 the codes of a bad shop.
pub async fn create(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    input: &ShopInput,
) -> Result<Shop, AppError> {
    let mut tx = pool.begin().await?;
    let row = gm_lock(&mut tx, gm, campaign).await?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let (name, lines) = checked(rules, input)?;
    let id: Uuid =
        sqlx::query_scalar("INSERT INTO shops (campaign_id, name) VALUES ($1, $2) RETURNING id")
            .bind(campaign)
            .bind(&name)
            .fetch_one(&mut *tx)
            .await?;
    let shop = Shop {
        id,
        name,
        open: false,
        lines,
        haggle: input.haggle.clone(),
        haggles: Vec::new(),
    };
    store(&mut tx, &shop).await?;
    tx.commit().await?;
    Ok(shop)
}

/// The GM rewrites a shop. Haggles on lines still there are kept.
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`; 400 the codes of a bad shop.
pub async fn save(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    shop: Uuid,
    input: &ShopInput,
) -> Result<Shop, AppError> {
    let mut tx = pool.begin().await?;
    let row = gm_lock(&mut tx, gm, campaign).await?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let mut s = locked(&mut tx, campaign, shop).await?;
    let (name, lines) = checked(rules, input)?;
    s.haggles.retain(|h| lines.iter().any(|l| l.id == h.line));
    s.name = name;
    s.lines = lines;
    s.haggle = input.haggle.clone();
    store(&mut tx, &s).await?;
    if s.open {
        crate::evening::touch(&mut tx, campaign).await?;
    }
    tx.commit().await?;
    Ok(s)
}

/// Open a shop to the table (closing the one open before), or close it.
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`.
pub async fn set_open(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    shop: Uuid,
    open: bool,
) -> Result<Shop, AppError> {
    let mut tx = pool.begin().await?;
    gm_lock(&mut tx, gm, campaign).await?;
    let mut s = locked(&mut tx, campaign, shop).await?;
    if s.open != open {
        if open {
            sqlx::query("UPDATE shops SET is_open = false WHERE campaign_id = $1 AND is_open")
                .bind(campaign)
                .execute(&mut *tx)
                .await?;
            let current = session::current(&mut *tx, campaign).await?;
            knowledge::write(
                &mut tx,
                campaign,
                current.map(|c| c.id),
                JournalKind::Narration,
                None,
                &format!("La boutique « {} » est ouverte.", s.name),
                true,
            )
            .await?;
        }
        s.open = open;
        store(&mut tx, &s).await?;
        crate::evening::touch(&mut tx, campaign).await?;
    }
    tx.commit().await?;
    Ok(s)
}

/// The GM brings a line out from under the counter: the table sees it.
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`, `NO_SUCH_LINE`; 409 `NOT_HIDDEN`.
pub async fn reveal(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    shop: Uuid,
    line: &str,
) -> Result<Shop, AppError> {
    let mut tx = pool.begin().await?;
    let row = gm_lock(&mut tx, gm, campaign).await?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let mut s = locked(&mut tx, campaign, shop).await?;
    let l = s
        .lines
        .iter_mut()
        .find(|l| l.id == line)
        .ok_or(AppError::NotFound("NO_SUCH_LINE"))?;
    if !l.hidden {
        return Err(AppError::Conflict("NOT_HIDDEN"));
    }
    l.hidden = false;
    let name = l.display_name(rules);
    store(&mut tx, &s).await?;
    if s.open {
        let current = session::current(&mut *tx, campaign).await?;
        knowledge::write(
            &mut tx,
            campaign,
            current.map(|c| c.id),
            JournalKind::Item,
            None,
            &format!("Sous le comptoir de « {} » : {name}.", s.name),
            true,
        )
        .await?;
        crate::evening::touch(&mut tx, campaign).await?;
    }
    tx.commit().await?;
    Ok(s)
}

/// The GM drops a shop.
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`.
pub async fn delete(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    shop: Uuid,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    gm_lock(&mut tx, gm, campaign).await?;
    let s = locked(&mut tx, campaign, shop).await?;
    sqlx::query("DELETE FROM shops WHERE id = $1")
        .bind(s.id)
        .execute(&mut *tx)
        .await?;
    if s.open {
        crate::evening::touch(&mut tx, campaign).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// The character `player` plays and has in play.
async fn character_of(
    tx: &mut Transaction<'_, Postgres>,
    player: &Player,
) -> Result<Uuid, AppError> {
    let id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM characters WHERE player_id = $1 AND status <> 'dead'")
            .bind(player.id)
            .fetch_optional(&mut **tx)
            .await?;
    id.ok_or(AppError::NotFound("NO_CHARACTER"))
}

fn purse(rules: &RuleSystem) -> Result<&ResourceDef, AppError> {
    rules
        .resources
        .first()
        .ok_or(AppError::Conflict("NO_CURRENCY"))
}

/// The open shop and the visible `line` of it, under the campaign lock.
async fn shop_line(
    tx: &mut Transaction<'_, Postgres>,
    player: &Player,
    line: &str,
) -> Result<(Shop, ShopLine), AppError> {
    let s = open_shop(&mut **tx, player.campaign_id)
        .await?
        .ok_or(AppError::Conflict("NO_SHOP"))?;
    let l = s
        .lines
        .iter()
        .find(|l| l.id == line && !l.hidden)
        .cloned()
        .ok_or(AppError::NotFound("NO_SUCH_LINE"))?;
    Ok((s, l))
}

/// The player buys `qty` of `line` from the open shop, at their price.
///
/// # Errors
///
/// 403 `SPECTATOR`; 409 `NO_SHOP`, `OUT_OF_STOCK`, `NOT_ENOUGH` (the
/// purse), `NO_CURRENCY`, `CHARACTER_NOT_VALIDATED`; 404
/// `NO_SUCH_LINE`, `NO_CHARACTER`; 400 `INVALID_QUANTITY`.
pub async fn buy(
    pool: &PgPool,
    player: &Player,
    rules: &RuleSystem,
    line: &str,
    qty: u32,
) -> Result<(), AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    if qty == 0 || qty > QTY_MAX {
        return Err(AppError::BadRequest("INVALID_QUANTITY"));
    }
    let mut tx = pool.begin().await?;
    campaigns::lock(&mut tx, player.campaign_id)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    let (mut shop, l) = shop_line(&mut tx, player, line).await?;
    if l.stock.is_some_and(|s| s < qty) {
        return Err(AppError::Conflict("OUT_OF_STOCK"));
    }
    let character = character_of(&mut tx, player).await?;
    let total = shop.price_for(&l, player.id) * qty;
    let currency = purse(rules)?;
    if total > 0 {
        play::adjust_in(
            &mut tx,
            player.campaign_id,
            rules,
            character,
            Adjustment::Resource {
                resource: currency.id.clone(),
                delta: -i32::try_from(total).map_err(|_| AppError::Conflict("NOT_ENOUGH"))?,
            },
            Actor::Player,
        )
        .await?;
    }
    let (sheet, _) = play::adjust_in(
        &mut tx,
        player.campaign_id,
        rules,
        character,
        Adjustment::GiveItem {
            item: l.item.clone().filter(|_| l.name.is_empty()),
            name: if l.item.is_some() && l.name.is_empty() {
                String::new()
            } else {
                l.display_name(rules)
            },
            description: if l.item.is_some() && l.name.is_empty() {
                String::new()
            } else {
                l.display_description(rules)
            },
            qty,
        },
        Actor::Player,
    )
    .await?;
    if let Some(stock) = shop
        .lines
        .iter_mut()
        .find(|x| x.id == l.id)
        .and_then(|x| x.stock.as_mut())
    {
        *stock -= qty;
    }
    store(&mut tx, &shop).await?;
    let name = l.display_name(rules);
    let what = if qty > 1 {
        format!("{name} ×{qty}")
    } else {
        name
    };
    let current = session::current(&mut *tx, player.campaign_id).await?;
    knowledge::write(
        &mut tx,
        player.campaign_id,
        current.map(|c| c.id),
        JournalKind::Item,
        l.item.as_deref(),
        &format!("{} achète {what} ({total} {}).", sheet.name, currency.abbr),
        true,
    )
    .await?;
    crate::evening::touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(())
}

/// The player haggles over `line`, once: the server rolls the shop's
/// ability against its difficulty; a success takes the discount off
/// that line for them.
///
/// # Errors
///
/// 403 `SPECTATOR`; 409 `NO_SHOP`, `NO_HAGGLING`, `ALREADY_HAGGLED`,
/// `NO_PLAY_SHEET`, `CHARACTER_NOT_VALIDATED`; 404 `NO_SUCH_LINE`,
/// `NO_CHARACTER`.
pub async fn haggle(
    pool: &PgPool,
    player: &Player,
    rules: &RuleSystem,
    line: &str,
) -> Result<RollBreakdown, AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let mut tx = pool.begin().await?;
    campaigns::lock(&mut tx, player.campaign_id)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    let (mut shop, l) = shop_line(&mut tx, player, line).await?;
    let rule = shop
        .haggle
        .clone()
        .ok_or(AppError::Conflict("NO_HAGGLING"))?;
    if shop.haggle_of(&l.id, player.id).is_some() {
        return Err(AppError::Conflict("ALREADY_HAGGLED"));
    }
    let character = character_of(&mut tx, player).await?;
    let (sheet, state) =
        play::in_play_locked(&mut tx, player.campaign_id, character, rules).await?;
    let c = play::combatant(rules, &sheet, &state).ok_or(AppError::Conflict("NO_PLAY_SHEET"))?;
    let breakdown = check::ability_check(
        rules,
        &c,
        &rule.ability,
        RollScope::Checks,
        Some(RollTarget::Difficulty {
            id: rules
                .difficulties
                .iter()
                .find(|d| d.value == rule.difficulty)
                .map(|d| d.id.clone()),
            value: rule.difficulty,
        }),
        Advantage::Normal,
        &mut SeededDice::from_os(),
    )
    .map_err(|e| AppError::Internal(format!("haggle: {e:?}")))?;
    let success = breakdown.band.is_some_and(OutcomeBand::is_success);
    shop.haggles.push(Haggle {
        line: l.id.clone(),
        player: player.id,
        success,
    });
    store(&mut tx, &shop).await?;
    let name = l.display_name(rules);
    let text = if success {
        format!(
            "{} marchande {name} : {} contre {}, réussi (−{} %).",
            sheet.name, breakdown.total, rule.difficulty, rule.discount
        )
    } else {
        format!(
            "{} marchande {name} : {} contre {}, raté.",
            sheet.name, breakdown.total, rule.difficulty
        )
    };
    let current = session::current(&mut *tx, player.campaign_id).await?;
    knowledge::write(
        &mut tx,
        player.campaign_id,
        current.map(|c| c.id),
        JournalKind::Roll,
        None,
        &text,
        true,
    )
    .await?;
    crate::evening::touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(breakdown)
}

/// What a player hands to a friend: coins, or some of a bag line.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Gift {
    /// The character who receives it.
    pub to: Uuid,
    #[serde(default)]
    pub coins: Option<u32>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub qty: Option<u32>,
}

/// The player shares what they carry with another character in play.
///
/// # Errors
///
/// 404 `NO_CHARACTER`, `NO_SUCH_CHARACTER`, `NO_SUCH_ENTRY`; 409
/// `NOT_ENOUGH`, `CHARACTER_NOT_VALIDATED`, `NO_CURRENCY`; 400
/// `INVALID_GIFT`, `INVALID_QUANTITY`.
pub async fn give(
    pool: &PgPool,
    player: &Player,
    rules: &RuleSystem,
    gift: &Gift,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    campaigns::lock(&mut tx, player.campaign_id)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    let me = character_of(&mut tx, player).await?;
    let friend: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(sheet->>'name', '') FROM characters
         WHERE id = $1 AND campaign_id = $2 AND status = 'validated'",
    )
    .bind(gift.to)
    .bind(player.campaign_id)
    .fetch_optional(&mut *tx)
    .await?;
    let friend = friend
        .filter(|_| gift.to != me)
        .ok_or(AppError::NotFound("NO_SUCH_CHARACTER"))?;
    let (take, put, what) = match (gift.coins, &gift.entry) {
        (Some(n), None) if n > 0 => {
            let currency = purse(rules)?;
            let delta = i32::try_from(n).map_err(|_| AppError::BadRequest("INVALID_GIFT"))?;
            (
                Adjustment::Resource {
                    resource: currency.id.clone(),
                    delta: -delta,
                },
                Adjustment::Resource {
                    resource: currency.id.clone(),
                    delta,
                },
                format!("{n} {}", currency.abbr),
            )
        }
        (None, Some(entry)) => {
            let qty = gift.qty.unwrap_or(1);
            let (_, state) = play::in_play_locked(&mut tx, player.campaign_id, me, rules).await?;
            let line = state
                .inventory
                .iter()
                .find(|e| &e.key == entry)
                .cloned()
                .ok_or(AppError::NotFound("NO_SUCH_ENTRY"))?;
            let name = line.display_name(rules);
            (
                Adjustment::TakeItem {
                    entry: entry.clone(),
                    qty,
                },
                Adjustment::GiveItem {
                    item: line.item.clone(),
                    name: line.name.clone(),
                    description: line.description.clone(),
                    qty,
                },
                if qty > 1 {
                    format!("{name} ×{qty}")
                } else {
                    name
                },
            )
        }
        _ => return Err(AppError::BadRequest("INVALID_GIFT")),
    };
    let (sheet, _) =
        play::adjust_in(&mut tx, player.campaign_id, rules, me, take, Actor::Player).await?;
    play::adjust_in(
        &mut tx,
        player.campaign_id,
        rules,
        gift.to,
        put,
        Actor::Player,
    )
    .await?;
    let current = session::current(&mut *tx, player.campaign_id).await?;
    knowledge::write(
        &mut tx,
        player.campaign_id,
        current.map(|c| c.id),
        JournalKind::Loot,
        None,
        &format!("{} donne {what} à {friend}.", sheet.name),
        true,
    )
    .await?;
    crate::evening::touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shop(haggle: Option<HaggleRule>, haggles: Vec<Haggle>) -> Shop {
        Shop {
            id: Uuid::nil(),
            name: "Le marché noir".into(),
            open: true,
            lines: vec![],
            haggle,
            haggles,
        }
    }

    fn line(price: u32) -> ShopLine {
        ShopLine {
            id: "l1".into(),
            item: None,
            name: "Longue-vue".into(),
            description: String::new(),
            price,
            stock: None,
            hidden: false,
        }
    }

    #[test]
    fn a_won_haggle_takes_the_discount_off_for_that_player_only() {
        let marc = Uuid::new_v4();
        let lea = Uuid::new_v4();
        let rule = HaggleRule {
            ability: "CHA".into(),
            difficulty: 12,
            discount: 20,
        };
        let won = Haggle {
            line: "l1".into(),
            player: marc,
            success: true,
        };
        let s = shop(Some(rule.clone()), vec![won]);
        assert_eq!(s.price_for(&line(15), marc), 12);
        assert_eq!(s.price_for(&line(15), lea), 15);
        let lost = Haggle {
            line: "l1".into(),
            player: marc,
            success: false,
        };
        assert_eq!(shop(Some(rule), vec![lost]).price_for(&line(15), marc), 15);
    }
}
