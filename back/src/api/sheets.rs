//! gm/adjust-sheets-fast — the GM's board of the characters in play
//! (behind `require_gm`; another GM's campaign answers 404 like a
//! missing one):
//! - `GET  /api/campaigns/{id}/sheets` → every validated character in
//!   play (the same `PlayView` its player reads), what the rules let the
//!   GM give (items, resources), and the latest history of changes;
//! - `POST /api/campaigns/{id}/characters/{character}/adjust` → one
//!   gesture (`players::play::Adjustment`: `{ kind: "xp", delta }`,
//!   `hitPoints`, `resource`, `giveItem`, `takeItem`): applied, logged,
//!   both screens told; answers the character as the board shows it.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use promptus_shared::rules::RuleSystem;
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::body::Body;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::projection::{PlayView, project_play};
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::players::play::{self, Adjustment, InPlay};
use crate::players::review;
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

async fn owned_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)
}

/// One character on the GM's board.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoardSheet {
    character_id: Uuid,
    player_id: Uuid,
    nickname: String,
    name: String,
    class_name: Option<String>,
    look: Option<Value>,
    /// `None` when the sheet names no class of the rules: nothing to
    /// adjust.
    play: Option<PlayView>,
}

fn board_sheet(row: &CampaignRow, rules: Option<&RuleSystem>, c: &InPlay) -> BoardSheet {
    BoardSheet {
        character_id: c.id,
        player_id: c.player_id,
        nickname: c.nickname.clone(),
        name: c.sheet.name.clone(),
        class_name: review::class_name(row.rules(), c.sheet.class_id.as_deref()),
        look: c
            .sheet
            .look
            .as_ref()
            .and_then(|l| serde_json::to_value(l).ok()),
        play: rules.and_then(|r| project_play(r, &c.sheet, c.state.as_ref())),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GivableItem {
    id: String,
    name: String,
    description: String,
    consumable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceLabel {
    id: String,
    name: String,
    abbr: String,
}

/// `GET /api/campaigns/{id}/sheets`
///
/// # Errors
///
/// 404 when missing or another GM's; a database error.
pub async fn board(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let rules = row.rules();
    let sheets: Vec<BoardSheet> = play::in_play(&state.pool, row.id)
        .await?
        .iter()
        .map(|c| board_sheet(&row, rules, c))
        .collect();
    let history = play::history(&state.pool, row.id).await?;
    let items: Vec<GivableItem> = rules
        .map(|r| {
            r.items
                .iter()
                .map(|i| GivableItem {
                    id: i.id.clone(),
                    name: i.name.clone(),
                    description: i.description.clone(),
                    consumable: i.consumable,
                })
                .collect()
        })
        .unwrap_or_default();
    let resources: Vec<ResourceLabel> = rules
        .map(|r| {
            r.resources
                .iter()
                .map(|x| ResourceLabel {
                    id: x.id.clone(),
                    name: x.name.clone(),
                    abbr: x.abbr.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Json(json!({
        "data": {
            "rulesKnown": rules.is_some(),
            "sheets": sheets,
            "items": items,
            "resources": resources,
            "history": history,
        }
    }))
    .into_response())
}

/// `POST /api/campaigns/{id}/characters/{character}/adjust`
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, or the character
/// does not sit at its table; 409 `RULES_UNKNOWN` when the server does
/// not have the campaign's rules; the errors of `players::play::adjust`;
/// 400 `INVALID_BODY`.
pub async fn adjust(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, character)): Path<(String, String)>,
    Body(adjustment): Body<Adjustment>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let character = parse_id(&character)?;
    let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    play::adjust(&state.pool, row.id, rules, character, adjustment).await?;
    let sheet = play::in_play(&state.pool, row.id)
        .await?
        .into_iter()
        .find(|c| c.id == character)
        .map(|c| board_sheet(&row, Some(rules), &c));
    Ok(Json(json!({ "data": sheet })).into_response())
}
