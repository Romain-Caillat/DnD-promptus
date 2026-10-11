//! The co-GM's workshop (`story_proposals`, migration 016): the GM asks
//! for a change in their own words — « rends le maire plus ambigu »,
//! or « résous cette alerte » from the coherence check — and the co-GM
//! proposes edits (`story::edit`). Invented ids and edits that break
//! the campaign are dropped before the GM sees them; nothing changes
//! until the GM accepts, and accepting re-applies the edits to the
//! campaign as it is then.

use std::collections::BTreeMap;

use promptus_shared::story::edit::{self, Edit};
use promptus_shared::story::{Campaign, Issue, to_yaml, validate};
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use crate::ai::{self, Ai, LlmRequest, ledger, templates};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::revisions::Author;
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::live::{self, Topic};

/// What the GM asks.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ask {
    #[serde(default)]
    pub prompt: String,
    /// The scene the GM is looking at.
    #[serde(default)]
    pub node: Option<String>,
    /// An alert of the coherence check to resolve, by code and path.
    #[serde(default)]
    pub issue: Option<IssueRef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueRef {
    pub code: String,
    pub path: String,
}

pub use super::proposals::Proposal;
use super::proposals::{COLUMNS, Decision, ItemStatus, Row, stored, view};

/// A stored row of the workshop as the GM's screen reads it.
fn proposal(row: &CampaignRow, r: Row) -> Result<Proposal, AppError> {
    Ok(view(row, stored(r)?, &[]))
}

/// The co-GM's proposals, newest first (Claude's batches are read
/// from the atlas and `prep::claude::list`).
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; a database error.
pub async fn list(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
) -> Result<Vec<Proposal>, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM story_proposals WHERE campaign_id = $1 AND author = 'copilot'
         ORDER BY created_at DESC LIMIT 30"
    ))
    .bind(campaign)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(|r| proposal(&row, r)).collect()
}

const PROMPT_MAX: usize = 2_000;

/// The co-GM's answer, as written.
#[derive(Debug, Deserialize)]
struct RawAnswer {
    #[serde(default)]
    reply: String,
    #[serde(default)]
    edits: Vec<serde_json::Value>,
}

/// What the co-GM reads: the whole campaign, and what the validator
/// says of it.
fn context(story: &Campaign, issues: &[Issue]) -> Result<String, AppError> {
    let yaml = to_yaml(story).map_err(|e| AppError::internal("workshop yaml", e))?;
    let mut out = format!("# La campagne (YAML)\n```yaml\n{}\n```\n", yaml.trim_end());
    out.push_str("\n# Alertes du validateur\n");
    if issues.is_empty() {
        out.push_str("Aucune.\n");
    }
    for i in issues {
        out.push_str(&format!("- {} {} : {}\n", i.code, i.path, i.detail));
    }
    Ok(out)
}

fn request(
    ask: &Ask,
    prompt: &str,
    story: &Campaign,
    issues: &[Issue],
) -> Result<String, AppError> {
    let mut out = String::new();
    if let Some(node) = &ask.node {
        let n = story
            .node(node)
            .ok_or(AppError::BadRequest("UNKNOWN_NODE"))?;
        out.push_str(&format!("Scène sélectionnée : {} « {} »\n", n.id, n.title));
    }
    if let Some(r) = &ask.issue {
        let issue = issues
            .iter()
            .find(|i| i.code == r.code && i.path == r.path)
            .ok_or(AppError::Conflict("ISSUE_GONE"))?;
        out.push_str(&format!(
            "Alerte à résoudre : {} {} — {}\n",
            issue.code, issue.path, issue.detail
        ));
    }
    if !prompt.is_empty() {
        out.push_str(&format!("Demande du MJ : {prompt}\n"));
    }
    if out.is_empty() {
        return Err(AppError::BadRequest("EMPTY_TEXT"));
    }
    Ok(out)
}

/// Ask the co-GM for a change. The call is counted against the
/// campaign's AI budget; the proposal is stored pending.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 400 `EMPTY_TEXT`,
/// `TEXT_TOO_LONG`, `UNKNOWN_NODE`; 409 `ISSUE_GONE`,
/// `AI_BUDGET_EXCEEDED`; 503 `AI_NOT_CONFIGURED`; 502 `AI_UNAVAILABLE`,
/// `AI_OUTPUT_INVALID`.
pub async fn ask(
    pool: &PgPool,
    ai: &Ai,
    gm: &CurrentGm,
    campaign: Uuid,
    ask: &Ask,
) -> Result<Proposal, AppError> {
    let prompt = crate::evening::clean_text(&ask.prompt, PROMPT_MAX)?;
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    // What the co-GM may fix: an alert it cannot act on is neither shown
    // to it nor askable (`ISSUE_GONE`), before any AI is spent.
    let issues: Vec<Issue> = validate(&row.story)
        .into_iter()
        .filter(super::for_the_model)
        .collect();
    let vars: BTreeMap<&str, String> = [
        ("context", context(&row.story, &issues)?),
        ("request", request(ask, &prompt, &row.story, &issues)?),
    ]
    .into_iter()
    .collect();
    let messages = templates::WORKSHOP
        .render(&vars)
        .map_err(|e| AppError::internal("workshop template", e))?;
    let mut req = LlmRequest::json(messages);
    req.max_tokens = 4_000;
    let reply = ai
        .complete(pool, campaign, "workshop", &templates::WORKSHOP, &req)
        .await?;
    let raw: RawAnswer = ai::parse_json(&reply.text).map_err(|e| ledger::app_error(&e))?;
    // An edit the format cannot read counts as dropped, like one that
    // points at nothing.
    let total = raw.edits.len();
    let readable: Vec<Edit> = raw
        .edits
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();
    let unreadable = total - readable.len();
    let (edits, dropped) = edit::sanitize_with(&row.story, readable, &super::edit_library(&row));
    let dropped = i32::try_from(dropped + unreadable).unwrap_or(i32::MAX);

    let mut tx = pool.begin().await?;
    let stored: Row = sqlx::query_as(&format!(
        "INSERT INTO story_proposals (campaign_id, prompt, reply, edits, dropped)
         VALUES ($1, $2, $3, $4, $5) RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(if prompt.is_empty() {
        ask.issue
            .as_ref()
            .map_or_else(String::new, |i| format!("{} {}", i.code, i.path))
    } else {
        prompt
    })
    .bind(raw.reply.trim())
    .bind(Json(&edits))
    .bind(dropped)
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Proposals).await?;
    tx.commit().await?;
    proposal(&row, stored)
}

async fn locked(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    campaign: Uuid,
    id: Uuid,
) -> Result<Row, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM story_proposals
         WHERE id = $1 AND campaign_id = $2 AND author = 'copilot' FOR UPDATE"
    ))
    .bind(id)
    .bind(campaign)
    .fetch_optional(&mut **tx)
    .await?;
    let row = row.ok_or(AppError::NotFound("NO_SUCH_PROPOSAL"))?;
    if row.5 != "pending" {
        return Err(AppError::Conflict("PROPOSAL_ALREADY_DECIDED"));
    }
    Ok(row)
}

/// Accept or reject a proposal. Accepting applies its edits to the
/// campaign as it is now, under the campaign lock.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, `NO_SUCH_PROPOSAL`;
/// 409 `PROPOSAL_ALREADY_DECIDED`, `PROPOSAL_STALE` (the campaign changed
/// and the edits no longer apply); a database error.
pub async fn decide(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
    accept: bool,
) -> Result<(CampaignRow, Proposal), AppError> {
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let stored = locked(&mut tx, campaign, id).await?;
    if accept {
        let (story, _) = edit::apply_with(&row.story, &stored.3.0, &super::edit_library(&row))
            .map_err(|_| AppError::Conflict("PROPOSAL_STALE"))?;
        campaigns::save_story(&mut tx, campaign, &row.story, &story, Author::Copilot).await?;
    }
    // Taken whole: every edit gets the same word.
    let decision = Decision {
        status: if accept {
            ItemStatus::Kept
        } else {
            ItemStatus::Discarded
        },
        word: None,
        code: None,
    };
    let decisions: std::collections::BTreeMap<String, Decision> = (0..stored.3.0.len())
        .map(|i| (super::proposals::edit_key(i), decision.clone()))
        .collect();
    let decided: Row = sqlx::query_as(&format!(
        "UPDATE story_proposals SET status = $2, decisions = $3, decided_at = now()
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(if accept { "accepted" } else { "rejected" })
    .bind(Json(&decisions))
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Proposals).await?;
    let row = campaigns::find(&mut *tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    tx.commit().await?;
    let p = proposal(&row, decided)?;
    Ok((row, p))
}
