//! player/buy-and-trade over HTTP.
//!
//! GM, behind `require_gm` (another GM's campaign answers 404 like a
//! missing one); each answers the GM's shops screen:
//! - `GET    /api/campaigns/{id}/shops` → every shop with its hidden
//!   lines and the haggles tried, what may be sold (the rules' and the
//!   story's items) and the NPCs who keep a shop;
//! - `POST   /api/campaigns/{id}/shops` → `{ fromNpc }` or `{ name,
//!   keeper }`: a new closed shop (201);
//! - `PUT    /api/campaigns/{id}/shops/{shop}` → `{ name, keeper, lines,
//!   haggle, surcharge? }`;
//! - `POST   /api/campaigns/{id}/shops/{shop}/open` → `{ open }`;
//! - `POST   /api/campaigns/{id}/shops/{shop}/reveal` → `{ line }`;
//! - `DELETE /api/campaigns/{id}/shops/{shop}`.
//!
//! Player, behind `require_player`; each answers the trade view built
//! by `campaigns::projection::trade`:
//! - `GET  /api/play/{campaign}/trade` → open shops, my purse, my
//!   companions;
//! - `POST /api/play/{campaign}/shops/{shop}/buy` → `{ line, discounted }`;
//! - `POST /api/play/{campaign}/shops/{shop}/haggle`;
//! - `POST /api/play/{campaign}/character/give` → `{ to, entry, qty }`
//!   or `{ to, amount }`: hand an item or money to a companion.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::auth::player::CurrentPlayer;
use crate::campaigns::projection::project_play;
use crate::campaigns::projection::trade::{TradeInput, project_trade};
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::players::play::{self, Gift};
use crate::players::{self, CharacterStatus};
use crate::shops::{self, HaggleResult, Line, NewShop, Purchase, Shop, ShopEdit};
use crate::state::AppState;

fn parse_id(raw: &str, code: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound(code))
}

async fn owned_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(
        campaigns::find(&state.pool, parse_id(raw, "NOT_FOUND")?).await?,
        gm,
    )
}

// --- The GM's screen ------------------------------------------------------------

/// A shop line with the name the players read, for the GM.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GmLine {
    #[serde(flatten)]
    line: Line,
    display_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GmHaggle {
    character_id: Uuid,
    name: String,
    outcome: String,
    total: i32,
    discount_left: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GmShop {
    id: Uuid,
    name: String,
    keeper: String,
    npc: Option<String>,
    open: bool,
    currency: String,
    lines: Vec<GmLine>,
    haggle: Option<shops::Haggle>,
    surcharge: u32,
    haggles: Vec<GmHaggle>,
}

/// Something a shop may sell.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Sellable {
    /// `rules` or `story`.
    kind: &'static str,
    id: String,
    name: String,
    /// The rules' price, or the story's value.
    price: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Keeper {
    id: String,
    name: String,
    title: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Currency {
    id: String,
    name: String,
    abbr: String,
}

async fn gm_screen(state: &AppState, row: &CampaignRow) -> Result<serde_json::Value, AppError> {
    let pool = &state.pool;
    let rules = row.rules();
    let all = shops::list(pool, row.id).await?;
    let haggles = shops::haggles(pool, row.id).await?;
    let names: Vec<(Uuid, String)> = play::in_play(pool, row.id)
        .await?
        .into_iter()
        .map(|c| (c.id, c.sheet.name))
        .collect();
    let gm_shops: Vec<GmShop> = all
        .into_iter()
        .map(|s| GmShop {
            lines: s
                .lines
                .iter()
                .map(|l| GmLine {
                    display_name: rules
                        .map_or_else(|| l.name.clone(), |r| l.display(r, &row.story).0),
                    line: l.clone(),
                })
                .collect(),
            haggles: haggles
                .iter()
                .filter(|h| h.shop_id == s.id)
                .map(|h| GmHaggle {
                    character_id: h.character_id,
                    name: names
                        .iter()
                        .find(|(id, _)| *id == h.character_id)
                        .map_or_else(String::new, |(_, n)| n.clone()),
                    outcome: rules.map_or_else(String::new, |r| h.band.name(r).to_string()),
                    total: h.roll.total,
                    discount_left: h.discount_left,
                })
                .collect(),
            id: s.id,
            name: s.name,
            keeper: s.keeper,
            npc: s.npc,
            open: s.open,
            currency: s.currency,
            haggle: s.haggle,
            surcharge: s.surcharge,
        })
        .collect();
    let mut sellable: Vec<Sellable> = rules
        .map(|r| {
            r.items
                .iter()
                .map(|i| Sellable {
                    kind: "rules",
                    id: i.id.clone(),
                    name: i.name.clone(),
                    price: i.price,
                })
                .collect()
        })
        .unwrap_or_default();
    sellable.extend(row.story.items.iter().map(|i| Sellable {
        kind: "story",
        id: i.id.clone(),
        name: i.name.clone(),
        price: i.value,
    }));
    let keepers: Vec<Keeper> = row
        .story
        .npcs
        .iter()
        .filter(|n| !n.sells.is_empty())
        .map(|n| Keeper {
            id: n.id.clone(),
            name: n.name.clone(),
            title: n.title.clone(),
        })
        .collect();
    let currency = rules.and_then(|r| r.currency()).map(|c| Currency {
        id: c.id.clone(),
        name: c.name.clone(),
        abbr: c.abbr.clone(),
    });
    Ok(json!({
        "shops": gm_shops,
        "sellable": sellable,
        "keepers": keepers,
        "currency": currency,
        "abilities": rules.map(|r| r.abilities.iter().map(|a| json!({ "id": a.id, "name": a.name })).collect::<Vec<_>>()),
        "difficulties": rules.map(|r| r.difficulties.iter().map(|d| json!({ "name": d.name, "value": d.value })).collect::<Vec<_>>()),
    }))
}

async fn gm_answer(
    state: &AppState,
    gm: &CurrentGm,
    raw: &str,
    status: StatusCode,
) -> Result<Response, AppError> {
    let row = owned_row(state, gm, raw).await?;
    let screen = gm_screen(state, &row).await?;
    Ok((status, Json(json!({ "data": screen }))).into_response())
}

/// `GET /api/campaigns/{id}/shops`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn gm_shops(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    gm_answer(&state, &gm, &id, StatusCode::OK).await
}

/// `POST /api/campaigns/{id}/shops` (201)
///
/// # Errors
///
/// The codes of `shops::create`.
pub async fn create(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(new): Body<NewShop>,
) -> Result<Response, AppError> {
    shops::create(&state.pool, &gm, parse_id(&id, "NOT_FOUND")?, &new).await?;
    gm_answer(&state, &gm, &id, StatusCode::CREATED).await
}

/// `PUT /api/campaigns/{id}/shops/{shop}`
///
/// # Errors
///
/// The codes of `shops::edit`.
pub async fn edit(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, shop)): Path<(String, String)>,
    Body(edit): Body<ShopEdit>,
) -> Result<Response, AppError> {
    shops::edit(
        &state.pool,
        &gm,
        parse_id(&id, "NOT_FOUND")?,
        parse_id(&shop, "NO_SUCH_SHOP")?,
        edit,
    )
    .await?;
    gm_answer(&state, &gm, &id, StatusCode::OK).await
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenBody {
    open: bool,
}

/// `POST /api/campaigns/{id}/shops/{shop}/open`
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`.
pub async fn set_open(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, shop)): Path<(String, String)>,
    Body(body): Body<OpenBody>,
) -> Result<Response, AppError> {
    shops::set_open(
        &state.pool,
        &gm,
        parse_id(&id, "NOT_FOUND")?,
        parse_id(&shop, "NO_SUCH_SHOP")?,
        body.open,
    )
    .await?;
    gm_answer(&state, &gm, &id, StatusCode::OK).await
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevealBody {
    line: String,
}

/// `POST /api/campaigns/{id}/shops/{shop}/reveal`
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`, `NO_SUCH_LINE`.
pub async fn reveal(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, shop)): Path<(String, String)>,
    Body(body): Body<RevealBody>,
) -> Result<Response, AppError> {
    shops::reveal(
        &state.pool,
        &gm,
        parse_id(&id, "NOT_FOUND")?,
        parse_id(&shop, "NO_SUCH_SHOP")?,
        &body.line,
    )
    .await?;
    gm_answer(&state, &gm, &id, StatusCode::OK).await
}

/// `DELETE /api/campaigns/{id}/shops/{shop}`
///
/// # Errors
///
/// 404 `NO_SUCH_SHOP`.
pub async fn delete(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, shop)): Path<(String, String)>,
) -> Result<Response, AppError> {
    shops::delete(
        &state.pool,
        &gm,
        parse_id(&id, "NOT_FOUND")?,
        parse_id(&shop, "NO_SUCH_SHOP")?,
    )
    .await?;
    gm_answer(&state, &gm, &id, StatusCode::OK).await
}

// --- The player's side ----------------------------------------------------------

async fn campaign_of(state: &AppState, p: &CurrentPlayer) -> Result<CampaignRow, AppError> {
    campaigns::find(&state.pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))
}

async fn trade_json(state: &AppState, p: &CurrentPlayer) -> Result<serde_json::Value, AppError> {
    let pool = &state.pool;
    let row = campaign_of(state, p).await?;
    let Some(rules) = row.rules() else {
        return Ok(json!({ "purse": null, "shops": [], "companions": [] }));
    };
    let all: Vec<Shop> = shops::list(pool, row.id).await?;
    let haggles: Vec<HaggleResult> = shops::haggles(pool, row.id).await?;
    let table: Vec<(Uuid, String)> = play::in_play(pool, row.id)
        .await?
        .into_iter()
        .map(|c| (c.id, c.sheet.name))
        .collect();
    let character = players::character_of(pool, &p.0).await?;
    let mine = match &character {
        Some(c) if c.status == CharacterStatus::Validated => {
            project_play(rules, &c.sheet, c.play.as_ref()).map(|v| (c.id, v))
        }
        _ => None,
    };
    let view = project_trade(&TradeInput {
        rules,
        story: &row.story,
        shops: &all,
        mine: mine.as_ref().map(|(id, v)| (*id, v)),
        haggles: &haggles,
        table: &table,
    });
    serde_json::to_value(view).map_err(|e| AppError::internal("trade view", e))
}

async fn player_answer(state: &AppState, p: &CurrentPlayer) -> Result<Response, AppError> {
    let view = trade_json(state, p).await?;
    Ok(Json(json!({ "data": view })).into_response())
}

/// `GET /api/play/{campaign}/trade`
///
/// # Errors
///
/// 401 `NOT_JOINED`; a database error.
pub async fn trade(State(state): State<AppState>, p: CurrentPlayer) -> Result<Response, AppError> {
    player_answer(&state, &p).await
}

/// `POST /api/play/{campaign}/shops/{shop}/buy`
///
/// # Errors
///
/// The codes of `shops::buy`.
pub async fn buy(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Path((_, shop)): Path<(String, String)>,
    Body(purchase): Body<Purchase>,
) -> Result<Response, AppError> {
    shops::buy(
        &state.pool,
        &p.0,
        parse_id(&shop, "NO_SUCH_SHOP")?,
        &purchase,
    )
    .await?;
    player_answer(&state, &p).await
}

/// `POST /api/play/{campaign}/shops/{shop}/haggle`
///
/// # Errors
///
/// The codes of `shops::haggle`.
pub async fn haggle(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Path((_, shop)): Path<(String, String)>,
) -> Result<Response, AppError> {
    shops::haggle(&state.pool, &p.0, parse_id(&shop, "NO_SUCH_SHOP")?).await?;
    player_answer(&state, &p).await
}

/// `POST /api/play/{campaign}/character/give`
///
/// # Errors
///
/// The codes of `players::play::give`; 409 `RULES_UNKNOWN`.
pub async fn give(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(gift): Body<Gift>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    play::give(&state.pool, &p.0, rules, &gift).await?;
    player_answer(&state, &p).await
}
