//! player/buy-and-trade.
//!
//! Player, behind `require_player` (answers built by
//! `campaigns::projection::shop`):
//! - `GET /api/play/{campaign}/shop` → the open shop, or `null`;
//! - `POST /api/play/{campaign}/shop/buy` → `{ line, qty? }`;
//! - `POST /api/play/{campaign}/shop/haggle` → `{ line }`: the roll, and
//!   the shop with the caller's new price;
//! - `GET /api/play/{campaign}/party` → the other characters in play,
//!   to share with;
//! - `POST /api/play/{campaign}/character/give` → `{ to, coins }` or
//!   `{ to, entry, qty? }`.
//!
//! GM, behind `require_gm`:
//! - `GET /api/campaigns/{id}/shops` → the shops, with the rules' items,
//!   abilities and currency to write them;
//! - `POST /api/campaigns/{id}/shops`, `PUT|DELETE …/shops/{shop}`;
//! - `POST …/shops/{shop}/open` → `{ open }`;
//! - `POST …/shops/{shop}/reveal` → `{ line }`: out from under the
//!   counter.

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
use crate::campaigns::projection::shop::{ShopView, project_shop};
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::players::{self, CharacterStatus, Role, play};
use crate::shops::{self, Gift, ShopInput};
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

async fn campaign_of(state: &AppState, p: &CurrentPlayer) -> Result<CampaignRow, AppError> {
    campaigns::find(&state.pool, p.0.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))
}

/// The open shop as the caller sees it.
async fn shop_view(
    state: &AppState,
    p: &CurrentPlayer,
    row: &CampaignRow,
) -> Result<Option<ShopView>, AppError> {
    let Some(rules) = row.rules() else {
        return Ok(None);
    };
    let Some(shop) = shops::open_shop(&state.pool, row.id).await? else {
        return Ok(None);
    };
    let coins = match players::character_of(&state.pool, &p.0).await? {
        Some(c) if p.0.role == Role::Player && c.status == CharacterStatus::Validated => {
            let play = c
                .play
                .unwrap_or_else(|| play::PlayState::start(rules, &c.sheet));
            rules.resources.first().map(|r| play.resource(rules, &r.id))
        }
        _ => None,
    };
    Ok(Some(project_shop(&shop, rules, p.0.id, coins)))
}

/// `GET /api/play/{campaign}/shop`
///
/// # Errors
///
/// 401 `NOT_JOINED`; a database error.
pub async fn shop(State(state): State<AppState>, p: CurrentPlayer) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let view = shop_view(&state, &p, &row).await?;
    Ok(Json(json!({ "data": view })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuyBody {
    line: String,
    #[serde(default = "one")]
    qty: u32,
}

fn one() -> u32 {
    1
}

/// `POST /api/play/{campaign}/shop/buy`
///
/// # Errors
///
/// As [`shops::buy`]; 409 `RULES_UNKNOWN`.
pub async fn buy(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(body): Body<BuyBody>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    shops::buy(&state.pool, &p.0, rules, &body.line, body.qty).await?;
    let view = shop_view(&state, &p, &row).await?;
    Ok(Json(json!({ "data": view })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HaggleBody {
    line: String,
}

/// `POST /api/play/{campaign}/shop/haggle`
///
/// # Errors
///
/// As [`shops::haggle`]; 409 `RULES_UNKNOWN`.
pub async fn haggle(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(body): Body<HaggleBody>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let roll = shops::haggle(&state.pool, &p.0, rules, &body.line).await?;
    let view = shop_view(&state, &p, &row).await?;
    Ok(Json(json!({ "data": { "roll": roll, "shop": view } })).into_response())
}

/// A character of the table to share with.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Friend {
    id: Uuid,
    name: String,
    nickname: String,
}

/// `GET /api/play/{campaign}/party`
///
/// # Errors
///
/// 401 `NOT_JOINED`; a database error.
pub async fn party(State(state): State<AppState>, p: CurrentPlayer) -> Result<Response, AppError> {
    let friends: Vec<Friend> = play::in_play(&state.pool, p.0.campaign_id)
        .await?
        .into_iter()
        .filter(|c| c.player_id != p.0.id)
        .map(|c| Friend {
            id: c.id,
            name: c.sheet.name,
            nickname: c.nickname,
        })
        .collect();
    Ok(Json(json!({ "data": friends })).into_response())
}

/// `POST /api/play/{campaign}/character/give`
///
/// # Errors
///
/// As [`shops::give`]; 409 `RULES_UNKNOWN`.
pub async fn give(
    State(state): State<AppState>,
    p: CurrentPlayer,
    Body(gift): Body<Gift>,
) -> Result<Response, AppError> {
    let row = campaign_of(&state, &p).await?;
    let Some(rules) = row.rules() else {
        players::character_of(&state.pool, &p.0)
            .await?
            .ok_or(AppError::NotFound("NO_CHARACTER"))?;
        return Err(AppError::Conflict("RULES_UNKNOWN"));
    };
    shops::give(&state.pool, &p.0, rules, &gift).await?;
    let character = players::character_of(&state.pool, &p.0)
        .await?
        .ok_or(AppError::NotFound("NO_CHARACTER"))?;
    let view = campaigns::projection::project_character(Some(rules), &character);
    Ok(Json(json!({ "data": view })).into_response())
}

async fn gm_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)
}

/// `GET /api/campaigns/{id}/shops`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn list(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = gm_row(&state, &gm, &id).await?;
    let list = shops::list(&state.pool, row.id).await?;
    let rules = row.rules();
    Ok(Json(json!({ "data": {
        "shops": list,
        "items": rules.map(|r| r.items.iter().map(|i| json!({
            "id": i.id, "name": i.name, "price": i.price,
        })).collect::<Vec<_>>()).unwrap_or_default(),
        "abilities": rules.map(|r| r.abilities.iter().map(|a| (a.id.clone(), a.name.clone()))
            .collect::<Vec<_>>()).unwrap_or_default(),
        "currency": rules.and_then(|r| r.resources.first()).map(|c| json!({
            "name": c.name, "abbr": c.abbr,
        })),
    } }))
    .into_response())
}

/// `POST /api/campaigns/{id}/shops`
///
/// # Errors
///
/// As [`shops::create`].
pub async fn create(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(input): Body<ShopInput>,
) -> Result<Response, AppError> {
    let shop = shops::create(&state.pool, &gm, parse_id(&id)?, &input).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": shop }))).into_response())
}

/// `PUT /api/campaigns/{id}/shops/{shop}`
///
/// # Errors
///
/// As [`shops::save`].
pub async fn save(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, shop)): Path<(String, String)>,
    Body(input): Body<ShopInput>,
) -> Result<Response, AppError> {
    let shop = shops::save(&state.pool, &gm, parse_id(&id)?, parse_id(&shop)?, &input).await?;
    Ok(Json(json!({ "data": shop })).into_response())
}

/// `DELETE /api/campaigns/{id}/shops/{shop}`
///
/// # Errors
///
/// As [`shops::delete`].
pub async fn delete(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, shop)): Path<(String, String)>,
) -> Result<Response, AppError> {
    shops::delete(&state.pool, &gm, parse_id(&id)?, parse_id(&shop)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
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
/// As [`shops::set_open`].
pub async fn open(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, shop)): Path<(String, String)>,
    Body(body): Body<OpenBody>,
) -> Result<Response, AppError> {
    let shop = shops::set_open(
        &state.pool,
        &gm,
        parse_id(&id)?,
        parse_id(&shop)?,
        body.open,
    )
    .await?;
    Ok(Json(json!({ "data": shop })).into_response())
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
/// As [`shops::reveal`].
pub async fn reveal(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, shop)): Path<(String, String)>,
    Body(body): Body<RevealBody>,
) -> Result<Response, AppError> {
    let shop = shops::reveal(
        &state.pool,
        &gm,
        parse_id(&id)?,
        parse_id(&shop)?,
        &body.line,
    )
    .await?;
    Ok(Json(json!({ "data": shop })).into_response())
}
