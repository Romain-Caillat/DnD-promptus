//! The co-GM's drafts (`copilot_drafts`): asked, read and edited by the
//! GM, then shown to the table — or dropped. Asking calls the LLM
//! outside any lock (the call is counted by `ai::ledger`); showing is
//! one write under the campaign lock that puts the GM's text, not the
//! draft, in the shared journal.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use super::{Answer, Ask, ContextInput, Kind, RawAnswer, context, instruction, sanitize};
use crate::ai::{self, Ai, LlmRequest, ledger, templates};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;
use crate::evening::knowledge::{self, JournalKind};
use crate::evening::requests::{self, RequestStatus};
use crate::evening::session::{self, Status};
use crate::live::{self, Topic};
use crate::players;

/// What the GM's screen shows of a draft.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub id: Uuid,
    pub kind: Kind,
    pub prompt: String,
    pub answer: Answer,
    pub status: DraftStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftStatus {
    Draft,
    Shown,
    Dismissed,
}

impl DraftStatus {
    fn parse(s: &str) -> Result<Self, AppError> {
        Ok(match s {
            "draft" => Self::Draft,
            "shown" => Self::Shown,
            "dismissed" => Self::Dismissed,
            other => return Err(AppError::Internal(format!("draft status {other}"))),
        })
    }
}

type Row = (Uuid, String, String, Json<Answer>, String, DateTime<Utc>);

const COLUMNS: &str = "id, kind, prompt, answer, status, created_at";

fn draft((id, kind, prompt, Json(answer), status, created_at): Row) -> Result<Draft, AppError> {
    Ok(Draft {
        id,
        kind: Kind::parse(&kind).map_err(AppError::Internal)?,
        prompt,
        answer,
        status: DraftStatus::parse(&status)?,
        created_at,
    })
}

/// The drafts of `session`, newest first.
///
/// # Errors
///
/// A database error.
pub async fn of_session(pool: &PgPool, session: Uuid) -> Result<Vec<Draft>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM copilot_drafts WHERE session_id = $1
         ORDER BY created_at DESC LIMIT 30"
    ))
    .bind(session)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(draft).collect()
}

const PROMPT_MAX: usize = 1_000;

/// Ask the co-GM. The session must be live; the call is counted
/// against the campaign's AI budget before it leaves.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 409 `NO_SESSION`,
/// `SESSION_NOT_LIVE`, `AI_BUDGET_EXCEEDED`; 400 `TEXT_TOO_LONG`,
/// `UNKNOWN_NPC`; 503 `AI_NOT_CONFIGURED`; 502 `AI_UNAVAILABLE`,
/// `AI_OUTPUT_INVALID`.
pub async fn ask(
    pool: &PgPool,
    ai: &Ai,
    gm: &CurrentGm,
    campaign: Uuid,
    ask: &Ask,
) -> Result<Draft, AppError> {
    let prompt = crate::evening::clean_text(&ask.prompt, PROMPT_MAX)?;
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let live = session::current(pool, campaign)
        .await?
        .filter(|s| s.status == Status::Live)
        .ok_or(AppError::Conflict("SESSION_NOT_LIVE"))?;
    if let Some(npc) = &ask.npc
        && row.story.npc(npc).is_none()
    {
        return Err(AppError::BadRequest("UNKNOWN_NPC"));
    }
    let rules = row.rules();
    let journal = knowledge::journal(pool, campaign, false).await?;
    let recaps: Vec<String> = session::all(pool, campaign)
        .await?
        .into_iter()
        .filter(|s| s.status == Status::Ended)
        .map(|s| s.recap)
        .collect();
    let seats = players::seats(pool, campaign).await?;
    let pending: Vec<String> = requests::of_session(pool, live.id)
        .await?
        .into_iter()
        .filter(|r| r.status == RequestStatus::Pending)
        .map(|r| {
            let who = seats
                .iter()
                .find(|s| s.id == r.player_id)
                .map_or("?", |s| s.nickname.as_str());
            let card = r.card.name(rules).unwrap_or_else(|| "Autre".into());
            format!("{who} : {card} (« {} »)", r.text)
        })
        .collect();
    let rulings = knowledge::rulings(pool, campaign).await?;
    let ctx = context(&ContextInput {
        campaign: &row.story,
        world: &row.world,
        journal: &journal,
        recaps: &recaps,
        pending: &pending,
        rulings: &rulings,
    });
    let rules_line = rules.map_or_else(
        || "Système de règles inconnu.".to_string(),
        |r| {
            let mut line = format!(
                "Système de règles : « {} » (test {}).",
                r.name, r.check.dice
            );
            // The GM applies house rules at the table; the co-GM must
            // know them to stay within the rules.
            for h in &r.house_rules {
                line.push_str(&format!("\nRègle maison « {} » : {}", h.name, h.text));
            }
            line
        },
    );
    let ask_with_prompt = Ask {
        prompt: prompt.clone(),
        ..ask.clone()
    };
    let vars: BTreeMap<&str, String> = [
        ("context", ctx),
        ("rules", rules_line),
        ("request", instruction(&ask_with_prompt, &row.story)),
    ]
    .into_iter()
    .collect();
    let messages = templates::COPILOT
        .render(&vars)
        .map_err(|e| AppError::internal("copilot template", e))?;
    let reply = ai
        .complete(
            pool,
            campaign,
            &format!("copilot.{}", ask.kind.as_str()),
            &templates::COPILOT,
            &LlmRequest::json(messages),
        )
        .await?;
    let raw: RawAnswer = ai::parse_json(&reply.text).map_err(|e| ledger::app_error(&e))?;
    let answer = sanitize(raw, &row.story);

    let mut tx = pool.begin().await?;
    let stored: Row = sqlx::query_as(&format!(
        "INSERT INTO copilot_drafts (campaign_id, session_id, kind, prompt, answer)
         VALUES ($1, $2, $3, $4, $5) RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(live.id)
    .bind(ask.kind.as_str())
    .bind(&prompt)
    .bind(Json(&answer))
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    draft(stored)
}

/// What the GM shows, after editing the draft.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Show {
    #[serde(default)]
    pub narration: String,
    #[serde(default)]
    pub npc_lines: Vec<ShownLine>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShownLine {
    pub speaker: String,
    pub text: String,
}

const SHOWN_MAX: usize = 2_000;

async fn locked_draft(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: Uuid,
    id: Uuid,
) -> Result<Draft, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM copilot_drafts WHERE id = $1 AND session_id = $2 FOR UPDATE"
    ))
    .bind(id)
    .bind(session)
    .fetch_optional(&mut **tx)
    .await?;
    let d = draft(row.ok_or(AppError::NotFound("NO_SUCH_DRAFT"))?)?;
    if d.status != DraftStatus::Draft {
        return Err(AppError::Conflict("DRAFT_ALREADY_DECIDED"));
    }
    Ok(d)
}

/// The GM's gesture: their edited text goes to the shared journal, the
/// draft is marked shown.
///
/// # Errors
///
/// 404 `NO_SUCH_DRAFT`; 409 `DRAFT_ALREADY_DECIDED`, `SESSION_NOT_LIVE`;
/// 400 `EMPTY_TEXT`, `TEXT_TOO_LONG`.
pub async fn show(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
    show: &Show,
) -> Result<(), AppError> {
    let narration = crate::evening::clean_text(&show.narration, SHOWN_MAX)?;
    let mut lines = Vec::new();
    for l in &show.npc_lines {
        let speaker = crate::evening::clean_text(&l.speaker, 80)?;
        let text = crate::evening::clean_text(&l.text, SHOWN_MAX)?;
        if !text.is_empty() {
            lines.push(format!("{speaker} : « {text} »"));
        }
    }
    if narration.is_empty() && lines.is_empty() {
        return Err(AppError::BadRequest("EMPTY_TEXT"));
    }
    let mut tx = pool.begin().await?;
    let (_, live) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    locked_draft(&mut tx, live.id, id).await?;
    if !narration.is_empty() {
        knowledge::write(
            &mut tx,
            campaign,
            Some(live.id),
            JournalKind::Narration,
            None,
            &narration,
            true,
        )
        .await?;
    }
    for line in &lines {
        knowledge::write(
            &mut tx,
            campaign,
            Some(live.id),
            JournalKind::Narration,
            None,
            line,
            true,
        )
        .await?;
    }
    sqlx::query("UPDATE copilot_drafts SET status = 'shown', decided_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    crate::evening::touch(&mut tx, campaign).await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(())
}

/// The GM drops a draft: nothing reaches the table.
///
/// # Errors
///
/// 404 `NO_SUCH_DRAFT`; 409 `DRAFT_ALREADY_DECIDED`, `SESSION_NOT_LIVE`.
pub async fn dismiss(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let (_, live) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    locked_draft(&mut tx, live.id, id).await?;
    sqlx::query("UPDATE copilot_drafts SET status = 'dismissed', decided_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(())
}
