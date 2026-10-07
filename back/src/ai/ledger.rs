//! ai/count-ai-calls — every call counted, every batch checked against
//! the campaign's budget before anything leaves (`MEMORY.md` §3).
//!
//! A batch runs in three steps:
//! 1. **reserve**: under the campaign row lock, what was spent plus the
//!    batch's estimate is compared with `campaigns.ai_budget_cents`; over
//!    it, the batch is refused whole and no call is made. Within it, one
//!    `ai_calls` row per call is written with its estimate, unsettled;
//! 2. **call** the provider, outside any transaction;
//! 3. **settle** each row with what the provider reported.
//!
//! An unsettled row counts its estimate, so two batches started at the
//! same moment cannot both spend the last of a budget, and a call whose
//! server died in flight keeps counting what it may have cost.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::templates::Template;
use super::{
    AiError, ImageRequest, ImageResponse, LlmRequest, LlmResponse, TranscribeRequest,
    TranscribeResponse, Usage, VideoRequest, VideoResponse,
};
use crate::ai::Ai;
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CallKind {
    Llm,
    Image,
    Video,
}

impl CallKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Llm => "llm",
            Self::Image => "image",
            Self::Video => "video",
        }
    }
}

/// A campaign's AI money, in millionths of a dollar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spending {
    pub budget_micros: i64,
    pub spent_micros: i64,
}

impl Spending {
    #[must_use]
    pub fn left_micros(&self) -> i64 {
        (self.budget_micros - self.spent_micros).max(0)
    }
}

/// One line of the campaign's AI history, for the GM.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallRecord {
    pub id: Uuid,
    pub kind: String,
    pub purpose: String,
    pub template: Option<String>,
    pub provider: String,
    pub model: String,
    pub cost_micros: i64,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// What the campaign may spend and has spent (settled or reserved).
///
/// # Errors
///
/// 404 when the campaign is gone; a database error.
pub async fn spending(pool: &PgPool, campaign: Uuid) -> Result<Spending, AppError> {
    let row: Option<(i32, i64)> = sqlx::query_as(
        "SELECT c.ai_budget_cents,
                COALESCE((SELECT SUM(cost_micros) FROM ai_calls a WHERE a.campaign_id = c.id), 0)::BIGINT
         FROM campaigns c WHERE c.id = $1",
    )
    .bind(campaign)
    .fetch_optional(pool)
    .await?;
    let (budget, spent) = row.ok_or(AppError::NotFound("NOT_FOUND"))?;
    Ok(Spending {
        budget_micros: i64::from(budget) * 10_000,
        spent_micros: spent,
    })
}

/// The latest calls of a campaign, newest first.
///
/// # Errors
///
/// A database error.
pub async fn history(
    pool: &PgPool,
    campaign: Uuid,
    limit: i64,
) -> Result<Vec<CallRecord>, AppError> {
    type Row = (
        Uuid,
        String,
        String,
        Option<String>,
        String,
        String,
        i64,
        Option<String>,
        DateTime<Utc>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, kind, purpose, template, provider, model, cost_micros, error, created_at
         FROM ai_calls WHERE campaign_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(campaign)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, kind, purpose, template, provider, model, cost_micros, error, created_at)| {
                CallRecord {
                    id,
                    kind,
                    purpose,
                    template,
                    provider,
                    model,
                    cost_micros,
                    error,
                    created_at,
                }
            },
        )
        .collect())
}

/// One call of a batch, before it leaves.
struct Planned<'a> {
    kind: CallKind,
    purpose: &'a str,
    template: Option<&'a Template>,
    model: Option<&'a str>,
    estimate: i64,
}

/// Reserve a batch: refused whole when it would pass the budget.
async fn reserve(
    pool: &PgPool,
    campaign: Uuid,
    provider: &str,
    batch: &[Planned<'_>],
) -> Result<Vec<Uuid>, AppError> {
    let mut tx = pool.begin().await?;
    let budget: Option<i32> =
        sqlx::query_scalar("SELECT ai_budget_cents FROM campaigns WHERE id = $1 FOR UPDATE")
            .bind(campaign)
            .fetch_optional(&mut *tx)
            .await?;
    let budget = i64::from(budget.ok_or(AppError::NotFound("NOT_FOUND"))?) * 10_000;
    let spent: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(cost_micros), 0)::BIGINT FROM ai_calls WHERE campaign_id = $1",
    )
    .bind(campaign)
    .fetch_one(&mut *tx)
    .await?;
    let estimate: i64 = batch.iter().map(|p| p.estimate).sum();
    if spent + estimate > budget {
        return Err(AppError::Conflict("AI_BUDGET_EXCEEDED"));
    }
    let mut ids = Vec::with_capacity(batch.len());
    for p in batch {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO ai_calls (campaign_id, kind, purpose, template, provider, model, cost_micros)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(campaign)
        .bind(p.kind.as_str())
        .bind(p.purpose)
        .bind(p.template.map(Template::key))
        .bind(provider)
        .bind(p.model.unwrap_or("default"))
        .bind(p.estimate)
        .fetch_one(&mut *tx)
        .await?;
        ids.push(id);
    }
    tx.commit().await?;
    Ok(ids)
}

/// Settle a reserved row with what the provider reported.
async fn settle(
    pool: &PgPool,
    id: Uuid,
    model: Option<&str>,
    usage: Usage,
    error: Option<&AiError>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE ai_calls SET settled = true, cost_micros = $2, prompt_tokens = $3,
                completion_tokens = $4, model = COALESCE($5, model), error = $6
         WHERE id = $1",
    )
    .bind(id)
    .bind(usage.cost_micros.max(0))
    .bind(i32::try_from(usage.prompt_tokens).unwrap_or(i32::MAX))
    .bind(i32::try_from(usage.completion_tokens).unwrap_or(i32::MAX))
    .bind(model)
    .bind(error.map(ToString::to_string))
    .execute(pool)
    .await?;
    Ok(())
}

/// What the server answers when a call fails.
#[must_use]
pub fn app_error(e: &AiError) -> AppError {
    match e {
        AiError::NotConfigured => AppError::ServiceUnavailable("AI_NOT_CONFIGURED"),
        AiError::Schema(_) => AppError::Upstream {
            code: "AI_OUTPUT_INVALID",
            detail: e.to_string(),
        },
        AiError::Unreachable(_) | AiError::Refused { .. } => AppError::Upstream {
            code: "AI_UNAVAILABLE",
            detail: e.to_string(),
        },
    }
}

impl Ai {
    /// One completion for `campaign`, counted: refused before it leaves
    /// when its estimate would pass the budget.
    ///
    /// # Errors
    ///
    /// 409 `AI_BUDGET_EXCEEDED` (no call made); 503 `AI_NOT_CONFIGURED`;
    /// 502 `AI_UNAVAILABLE`; a database error.
    pub async fn complete(
        &self,
        pool: &PgPool,
        campaign: Uuid,
        purpose: &str,
        template: &Template,
        req: &LlmRequest,
    ) -> Result<LlmResponse, AppError> {
        let provider = self.provider().map_err(|e| app_error(&e))?;
        let planned = [Planned {
            kind: CallKind::Llm,
            purpose,
            template: Some(template),
            model: req.model.as_deref(),
            estimate: self.pricing.llm(req),
        }];
        let ids = reserve(pool, campaign, provider.name(), &planned).await?;
        let answer = provider.complete(req).await;
        match &answer {
            Ok(r) => settle(pool, ids[0], Some(&r.model), r.usage, None).await?,
            Err(e) => settle(pool, ids[0], None, Usage::default(), Some(e)).await?,
        }
        answer.map_err(|e| app_error(&e))
    }

    /// A batch of images for `campaign`, estimated and checked whole
    /// before the first leaves. Each image answers on its own: one failure
    /// does not cancel the others.
    ///
    /// # Errors
    ///
    /// 409 `AI_BUDGET_EXCEEDED` (no call made); 503 `AI_NOT_CONFIGURED`;
    /// a database error.
    pub async fn images(
        &self,
        pool: &PgPool,
        campaign: Uuid,
        purpose: &str,
        template: &Template,
        batch: &[ImageRequest],
    ) -> Result<Vec<Result<ImageResponse, AiError>>, AppError> {
        let provider = self.provider().map_err(|e| app_error(&e))?;
        let planned: Vec<Planned<'_>> = batch
            .iter()
            .map(|r| Planned {
                kind: CallKind::Image,
                purpose,
                template: Some(template),
                model: r.model.as_deref(),
                estimate: self.pricing.image(),
            })
            .collect();
        let ids = reserve(pool, campaign, provider.name(), &planned).await?;
        let mut out = Vec::with_capacity(batch.len());
        for (req, id) in batch.iter().zip(ids) {
            let answer = provider.image(req).await;
            match &answer {
                Ok(r) => settle(pool, id, Some(&r.model), r.usage, None).await?,
                Err(e) => settle(pool, id, None, Usage::default(), Some(e)).await?,
            }
            out.push(answer);
        }
        Ok(out)
    }

    /// One video for `campaign`, counted like an image: refused before it
    /// leaves when its estimate would pass the budget. The provider's
    /// failure is answered, not raised, so the caller can keep it with
    /// the draft it was for.
    ///
    /// # Errors
    ///
    /// 409 `AI_BUDGET_EXCEEDED` (no call made); 503 `AI_NOT_CONFIGURED`;
    /// a database error.
    pub async fn video(
        &self,
        pool: &PgPool,
        campaign: Uuid,
        purpose: &str,
        template: &Template,
        req: &VideoRequest,
    ) -> Result<Result<VideoResponse, AiError>, AppError> {
        let provider = self.provider().map_err(|e| app_error(&e))?;
        let planned = [Planned {
            kind: CallKind::Video,
            purpose,
            template: Some(template),
            model: req.model.as_deref(),
            estimate: self.pricing.video(),
        }];
        let ids = reserve(pool, campaign, provider.name(), &planned).await?;
        let answer = provider.video(req).await;
        match &answer {
            Ok(r) => settle(pool, ids[0], Some(&r.model), r.usage, None).await?,
            Err(e) => settle(pool, ids[0], None, Usage::default(), Some(e)).await?,
        }
        Ok(answer)
    }

    /// One transcription for `campaign` (`copilot/listen-by-voice`),
    /// counted like a completion: a model call whose input is audio, so
    /// it is recorded as an `llm` call (its purpose says it was voice).
    ///
    /// # Errors
    ///
    /// 409 `AI_BUDGET_EXCEEDED` (no call made); 503 `AI_NOT_CONFIGURED`;
    /// 502 `AI_UNAVAILABLE`; a database error.
    pub async fn transcribe(
        &self,
        pool: &PgPool,
        campaign: Uuid,
        purpose: &str,
        template: &Template,
        req: &TranscribeRequest,
    ) -> Result<TranscribeResponse, AppError> {
        let provider = self.provider().map_err(|e| app_error(&e))?;
        let planned = [Planned {
            kind: CallKind::Llm,
            purpose,
            template: Some(template),
            model: req.model.as_deref(),
            estimate: self
                .pricing
                .transcribe(req, super::openrouter::TRANSCRIPT_TOKENS),
        }];
        let ids = reserve(pool, campaign, provider.name(), &planned).await?;
        let answer = provider.transcribe(req).await;
        match &answer {
            Ok(r) => settle(pool, ids[0], Some(&r.model), r.usage, None).await?,
            Err(e) => settle(pool, ids[0], None, Usage::default(), Some(e)).await?,
        }
        answer.map_err(|e| app_error(&e))
    }
}
