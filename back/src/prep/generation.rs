//! A campaign generated from the GM's pitch (`ai/generate-campaign`,
//! migration 017), ported from V1's `generation/pipeline.ts`.
//!
//! The GM writes an idea; a background job asks the model, step after
//! step, for:
//! 1. **cast** — the bible, the acts, the fronts, the NPCs, adversaries,
//!    places, items and factions;
//! 2. **scenes** — the nodes, the revelations and their clues;
//! 3. **check** — the validator reads the result; while it finds errors
//!    or a broken structure (three-clue rule, unreachable scene…), the
//!    model proposes repair edits (`story::edit`), at most
//!    [`MAX_REPAIRS`] times, and a repair is kept only when it does not
//!    make things worse.
//!
//! An answer out of format is sent back once with what was wrong. Ids
//! the model invented are removed ([`story::prune`]) and repair edits
//! that break the campaign are dropped (`edit::sanitize`), and the job
//! says how many. Every call is counted against the campaign's budget
//! (`ai::ledger`): the job is refused before its first call when its
//! estimate does not fit, and stops when a call would pass it.
//!
//! The job's progress is followed live (the `desk` topic). Its result is
//! a draft: nothing reaches the campaign until the GM applies it, and
//! the applied campaign is still to review and validate
//! (`campaign/review-story-graph`) before a session opens.
//!
//! [`story::prune`]: promptus_shared::story::prune

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::story::edit::{self, Edit};
use promptus_shared::story::prune::prune;
use promptus_shared::story::{
    Act, Adversary, Bible, Campaign, Clue, Faction, Front, Issue, Item, Library, Location, Node,
    Npc, Revelation, Severity, to_yaml, validate_with,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use crate::ai::templates::{self, Template};
use crate::ai::{self, Ai, LlmRequest, Message, Pricing, Role, ledger};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::{self, CampaignRow};
use crate::error::AppError;
use crate::live::{self, Topic};

/// Repairs at most, after the first check.
pub const MAX_REPAIRS: usize = 2;
/// What each step may answer, in tokens: the estimate bills it whole.
const CAST_TOKENS: u32 = 6_000;
const SCENES_TOKENS: u32 = 10_000;
const REPAIR_TOKENS: u32 = 4_000;
/// A job `running` without news for this long was interrupted.
const STALE_MINUTES: i64 = 15;
const PITCH_MIN: usize = 10;
const PITCH_MAX: usize = 4_000;
const FIELD_MAX: usize = 500;
const CONSTRAINTS_MAX: usize = 2_000;
/// The validator findings worth a repair, besides errors.
const STRUCTURAL: [&str; 6] = [
    "THREE_CLUE_RULE",
    "NODE_UNREACHABLE",
    "REVELATION_NO_CLUE",
    "KNOWLEDGE_NEVER_GIVEN",
    "KNOWLEDGE_ONLY_OPTIONAL",
    "START_NODE_MISSING",
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Length {
    /// One evening (3–4 h).
    #[default]
    OneShot,
    /// 3 to 5 sessions.
    Short,
    /// 8 sessions or more.
    Long,
}

impl Length {
    fn label(self) -> &'static str {
        match self {
            Self::OneShot => "Partie unique (3-4 h)",
            Self::Short => "Courte (3-5 sessions)",
            Self::Long => "Longue (8 sessions et plus)",
        }
    }

    fn cast_sizes(self) -> &'static str {
        match self {
            Self::OneShot => {
                "1 acte ; 2 menaces ; PNJ 3 à 5 ; adversaires 2 à 4 ; lieux 3 à 5 ; objets 2 à 4 ; factions 1 à 2"
            }
            Self::Short => {
                "2 actes ; 2 à 3 menaces ; PNJ 5 à 8 ; adversaires 4 à 6 ; lieux 5 à 8 ; objets 3 à 6 ; factions 2 à 3"
            }
            Self::Long => {
                "3 actes ; 3 menaces ; PNJ 8 à 12 ; adversaires 6 à 10 ; lieux 7 à 10 ; objets 4 à 8 ; factions 3 à 4"
            }
        }
    }

    fn scene_sizes(self) -> &'static str {
        match self {
            Self::OneShot => "scènes 5 à 8 ; révélations 2 à 3, dont au moins 2 critiques",
            Self::Short => {
                "scènes 9 à 14, réparties entre les actes ; révélations 3 à 5, dont au moins 2 critiques"
            }
            Self::Long => {
                "scènes 14 à 20, réparties entre les actes ; révélations 4 à 6, dont au moins 3 critiques"
            }
        }
    }
}

/// What the GM asks for.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub pitch: String,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub themes: String,
    #[serde(default)]
    pub constraints: String,
    #[serde(default)]
    pub length: Length,
}

impl Input {
    /// Trimmed, within bounds.
    ///
    /// # Errors
    ///
    /// 400 `PITCH_TOO_SHORT`, `TEXT_TOO_LONG`.
    pub fn checked(self) -> Result<Self, AppError> {
        let pitch = crate::evening::clean_text(&self.pitch, PITCH_MAX)?;
        if pitch.chars().count() < PITCH_MIN {
            return Err(AppError::BadRequest("PITCH_TOO_SHORT"));
        }
        Ok(Self {
            pitch,
            tone: crate::evening::clean_text(&self.tone, FIELD_MAX)?,
            themes: crate::evening::clean_text(&self.themes, FIELD_MAX)?,
            constraints: crate::evening::clean_text(&self.constraints, CONSTRAINTS_MAX)?,
            length: self.length,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepId {
    Cast,
    Scenes,
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Done,
    Failed,
}

/// A step and where it stands; `counts` say what it produced (`npcs`,
/// `nodes`, `errors`…), for the screen to word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub id: StepId,
    pub status: StepStatus,
    #[serde(default)]
    pub counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Running,
    Succeeded,
    Failed,
    Applied,
}

/// Something the model invented that the server removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Removed {
    pub code: String,
    pub path: String,
    pub detail: String,
}

/// A job as the GM's screen shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: Uuid,
    pub status: JobStatus,
    pub input: Input,
    pub steps: Vec<Step>,
    /// The generated campaign, once the job succeeded.
    pub draft: Option<Campaign>,
    /// What the validator says of the draft (rule system included).
    pub issues: Vec<Issue>,
    pub removed: Vec<Removed>,
    pub repairs: i32,
    /// Repair edits refused.
    pub dropped: i32,
    pub cost_micros: i64,
    pub error: Option<String>,
    pub detail: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The generation desk of a campaign: its jobs, and what one more costs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Desk {
    pub jobs: Vec<Job>,
    /// The most a job costs (longest pitch, one repair, no retry).
    pub estimate_micros: i64,
    pub spending: ledger::Spending,
    pub validated: bool,
}

type Row = (
    Uuid,
    String,
    Json<Input>,
    Json<Vec<Step>>,
    Option<Json<Campaign>>,
    Json<Vec<Removed>>,
    i32,
    i32,
    i64,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
    DateTime<Utc>,
);

const COLUMNS: &str = "id, status, input, steps, draft, pruned, repairs, dropped, cost_micros, \
                       error, detail, created_at, updated_at";

fn job(rules: Option<&RuleSystem>, row: Row) -> Job {
    let (
        id,
        status,
        Json(input),
        Json(steps),
        draft,
        Json(removed),
        repairs,
        dropped,
        cost_micros,
        mut error,
        detail,
        created_at,
        updated_at,
    ) = row;
    let mut status = match status.as_str() {
        "succeeded" => JobStatus::Succeeded,
        "failed" => JobStatus::Failed,
        "applied" => JobStatus::Applied,
        _ => JobStatus::Running,
    };
    if status == JobStatus::Running
        && Utc::now() - updated_at > chrono::Duration::minutes(STALE_MINUTES)
    {
        status = JobStatus::Failed;
        error = Some("INTERRUPTED".into());
    }
    let draft = draft.map(|Json(c)| c);
    let issues = draft.as_ref().map_or_else(Vec::new, |c| {
        validate_with(c, &Library { rules, maps: None })
    });
    Job {
        id,
        status,
        input,
        steps,
        draft,
        issues,
        removed,
        repairs,
        dropped,
        cost_micros,
        error,
        detail,
        created_at,
        updated_at,
    }
}

/// The campaign's generation desk: its latest jobs, newest first.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; a database error.
pub async fn desk(
    pool: &PgPool,
    ai: &Ai,
    gm: &CurrentGm,
    campaign: Uuid,
) -> Result<Desk, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM generation_jobs WHERE campaign_id = $1
         ORDER BY created_at DESC LIMIT 10"
    ))
    .bind(campaign)
    .fetch_all(pool)
    .await?;
    let longest = Input {
        pitch: "x".repeat(PITCH_MAX),
        tone: "x".repeat(FIELD_MAX),
        themes: "x".repeat(FIELD_MAX),
        constraints: "x".repeat(CONSTRAINTS_MAX),
        length: Length::Long,
    };
    Ok(Desk {
        jobs: rows.into_iter().map(|r| job(row.rules(), r)).collect(),
        estimate_micros: estimate(&ai.pricing, &row, &longest)?,
        spending: ledger::spending(pool, campaign).await?,
        validated: row.validated_at.is_some(),
    })
}

// ---------------------------------------------------------------------
// Prompts
// ---------------------------------------------------------------------

fn or(text: &str, default: &str) -> String {
    if text.is_empty() {
        default.to_string()
    } else {
        text.to_string()
    }
}

/// The GM's request, as every step reads it.
fn brief(row: &CampaignRow, input: &Input) -> String {
    format!(
        "# Demande du MJ\n- Titre : {}\n- Univers : {}\n- Idée : {}\n- Ton : {}\n- Thèmes : {}\n- Joueurs : {}\n- Format : {}\n- Contraintes : {}",
        row.story.title,
        or(&row.story.world, "libre"),
        input.pitch,
        or(&input.tone, "libre"),
        or(&input.themes, "libres"),
        row.settings.player_count,
        input.length.label(),
        or(&input.constraints, "aucune"),
    )
}

/// What the model must know of the rule system: ids it may use.
fn rules_text(rules: Option<&RuleSystem>) -> String {
    let Some(r) = rules else {
        return "Aucun système de règles connu : donne aux adversaires des statistiques explicites, et aucun test.".into();
    };
    let list = |items: Vec<String>| {
        if items.is_empty() {
            "aucun".to_string()
        } else {
            items.join(", ")
        }
    };
    let mut out = format!("« {} »\n", r.name);
    out.push_str(&format!(
        "Caractéristiques (id → nom) : {}\n",
        list(
            r.abilities
                .iter()
                .map(|a| format!("{} → {}", a.id, a.name))
                .collect()
        )
    ));
    out.push_str(&format!(
        "Difficultés : {}\n",
        list(
            r.difficulties
                .iter()
                .map(|d| format!("{} {}", d.name, d.value))
                .collect()
        )
    ));
    out.push_str(&format!(
        "Adversaires du système (id → nom) : {}\n",
        list(
            r.adversaries
                .iter()
                .map(|a| match &a.tier {
                    Some(t) => format!("{} → {} ({t})", a.id, a.name),
                    None => format!("{} → {}", a.id, a.name),
                })
                .collect()
        )
    ));
    for h in &r.house_rules {
        out.push_str(&format!("Règle maison : {} — {}\n", h.name, h.text));
    }
    out.trim_end().to_string()
}

fn render(template: &Template, vars: &[(&'static str, String)]) -> Result<Vec<Message>, AppError> {
    let vars: BTreeMap<&str, String> = vars.iter().cloned().collect();
    template
        .render(&vars)
        .map_err(|e| AppError::internal("generation template", e))
}

fn request(messages: Vec<Message>, max_tokens: u32) -> LlmRequest {
    let mut req = LlmRequest::json(messages);
    req.max_tokens = max_tokens;
    req
}

/// The most a job costs: each step's prompt and whole answer, the
/// scenes and repair prompts counted with everything the steps before
/// them may have written, one repair, no retry.
fn estimate(pricing: &Pricing, row: &CampaignRow, input: &Input) -> Result<i64, AppError> {
    let brief = brief(row, input);
    let rules = rules_text(row.rules());
    let cast = render(
        &templates::GENERATION_CAST,
        &[
            ("brief", brief.clone()),
            ("rules", rules.clone()),
            ("sizes", input.length.cast_sizes().into()),
        ],
    )?;
    let written = |tokens: u32| "x".repeat(tokens as usize * 4);
    let scenes = render(
        &templates::GENERATION_SCENES,
        &[
            ("brief", brief),
            ("cast", written(CAST_TOKENS)),
            ("rules", rules),
            ("sizes", input.length.scene_sizes().into()),
        ],
    )?;
    let repair = render(
        &templates::GENERATION_REPAIR,
        &[
            ("campaign", written(CAST_TOKENS + SCENES_TOKENS)),
            ("issues", String::new()),
        ],
    )?;
    Ok(pricing.llm(&request(cast, CAST_TOKENS))
        + pricing.llm(&request(scenes, SCENES_TOKENS))
        + pricing.llm(&request(repair, REPAIR_TOKENS)))
}

// ---------------------------------------------------------------------
// Starting a job
// ---------------------------------------------------------------------

/// Start a job on `campaign` and run it in the background. Refused
/// before any call when the campaign is already validated, a job is
/// running, or the job's estimate does not fit the budget.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 400 `PITCH_TOO_SHORT`,
/// `TEXT_TOO_LONG`; 409 `CAMPAIGN_ALREADY_VALIDATED`, `GENERATION_RUNNING`,
/// `AI_BUDGET_EXCEEDED`; 503 `AI_NOT_CONFIGURED`; a database error.
pub async fn start(
    pool: &PgPool,
    ai: &Ai,
    gm: &CurrentGm,
    campaign: Uuid,
    input: Input,
) -> Result<Job, AppError> {
    let input = input.checked()?;
    ai.provider().map_err(|e| ledger::app_error(&e))?;
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    if row.validated_at.is_some() {
        return Err(AppError::Conflict("CAMPAIGN_ALREADY_VALIDATED"));
    }
    sqlx::query(&format!(
        "UPDATE generation_jobs SET status = 'failed', error = 'INTERRUPTED'
         WHERE campaign_id = $1 AND status = 'running'
           AND updated_at < now() - interval '{STALE_MINUTES} minutes'"
    ))
    .bind(campaign)
    .execute(&mut *tx)
    .await?;
    let running: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM generation_jobs WHERE campaign_id = $1 AND status = 'running')",
    )
    .bind(campaign)
    .fetch_one(&mut *tx)
    .await?;
    if running {
        return Err(AppError::Conflict("GENERATION_RUNNING"));
    }
    let spending = ledger::spending(pool, campaign).await?;
    if estimate(&ai.pricing, &row, &input)? > spending.left_micros() {
        return Err(AppError::Conflict("AI_BUDGET_EXCEEDED"));
    }
    let steps: Vec<Step> = [StepId::Cast, StepId::Scenes, StepId::Check]
        .into_iter()
        .map(|id| Step {
            id,
            status: StepStatus::Pending,
            counts: BTreeMap::new(),
        })
        .collect();
    let stored: Row = sqlx::query_as(&format!(
        "INSERT INTO generation_jobs (campaign_id, input, steps) VALUES ($1, $2, $3)
         RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(Json(&input))
    .bind(Json(&steps))
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    let job = job(row.rules(), stored);
    tokio::spawn(run(pool.clone(), ai.clone(), campaign, job.id));
    Ok(job)
}

// ---------------------------------------------------------------------
// Running a job
// ---------------------------------------------------------------------

/// Why a job stopped: a code for the screen, a detail for the GM.
#[derive(Debug)]
struct Failure {
    code: String,
    detail: Option<String>,
}

impl From<AppError> for Failure {
    fn from(e: AppError) -> Self {
        let (code, detail) = match e {
            AppError::Conflict(code) | AppError::ServiceUnavailable(code) => (code, None),
            AppError::Upstream { code, detail } => (code, Some(detail)),
            AppError::NotFound(code) => (code, None),
            other => {
                tracing::error!(error = ?other, "generation failed");
                ("INTERNAL_ERROR", None)
            }
        };
        Self {
            code: code.to_string(),
            detail,
        }
    }
}

impl From<sqlx::Error> for Failure {
    fn from(e: sqlx::Error) -> Self {
        AppError::from(e).into()
    }
}

/// What the cast step answers.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cast {
    bible: Bible,
    #[serde(default)]
    acts: Vec<Act>,
    #[serde(default)]
    fronts: Vec<Front>,
    #[serde(default)]
    npcs: Vec<Npc>,
    #[serde(default)]
    adversaries: Vec<Adversary>,
    #[serde(default)]
    locations: Vec<Location>,
    #[serde(default)]
    items: Vec<Item>,
    #[serde(default)]
    factions: Vec<Faction>,
}

/// What the scenes step answers.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plot {
    start_node: String,
    nodes: Vec<Node>,
    #[serde(default)]
    revelations: Vec<Revelation>,
    #[serde(default)]
    clues: Vec<Clue>,
}

/// What a repair answers.
#[derive(Debug, Deserialize)]
struct Repair {
    #[serde(default)]
    edits: Vec<serde_json::Value>,
}

/// One running job.
struct Runner {
    pool: PgPool,
    ai: Ai,
    campaign: Uuid,
    id: Uuid,
    steps: Vec<Step>,
}

impl Runner {
    async fn save(&self) -> Result<(), Failure> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE generation_jobs SET steps = $2 WHERE id = $1")
            .bind(self.id)
            .bind(Json(&self.steps))
            .execute(&mut *tx)
            .await?;
        live::touch(&mut tx, self.campaign, &Topic::Desk).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn step(
        &mut self,
        id: StepId,
        status: StepStatus,
        counts: &[(&str, usize)],
    ) -> Result<(), Failure> {
        if let Some(s) = self.steps.iter_mut().find(|s| s.id == id) {
            s.status = status;
            s.counts = counts.iter().map(|(k, v)| ((*k).to_string(), *v)).collect();
        }
        self.save().await
    }

    /// One answer of type `T`, counted; an answer out of format is sent
    /// back once with what was wrong.
    async fn ask<T: DeserializeOwned>(
        &self,
        template: &Template,
        mut messages: Vec<Message>,
        max_tokens: u32,
    ) -> Result<T, Failure> {
        let mut problem = String::new();
        for _ in 0..2 {
            let req = request(messages.clone(), max_tokens);
            let answer = self
                .ai
                .complete(&self.pool, self.campaign, "generation", template, &req)
                .await?;
            sqlx::query("UPDATE generation_jobs SET cost_micros = cost_micros + $2 WHERE id = $1")
                .bind(self.id)
                .bind(answer.usage.cost_micros.max(0))
                .execute(&self.pool)
                .await?;
            match ai::parse_json::<T>(&answer.text) {
                Ok(v) => return Ok(v),
                Err(e) => {
                    problem = e.to_string();
                    messages.push(Message {
                        role: Role::Assistant,
                        content: answer.text,
                    });
                    messages.push(Message::user(format!(
                        "Ta réponse ne respecte pas le format attendu : {problem}\nRenvoie l’objet JSON complet corrigé."
                    )));
                }
            }
        }
        Err(Failure {
            code: "AI_OUTPUT_INVALID".into(),
            detail: Some(problem),
        })
    }

    async fn fail(&mut self, failure: Failure) {
        for s in &mut self.steps {
            if s.status == StepStatus::Running {
                s.status = StepStatus::Failed;
            }
        }
        let saved = sqlx::query(
            "UPDATE generation_jobs SET status = 'failed', steps = $2, error = $3, detail = $4
             WHERE id = $1",
        )
        .bind(self.id)
        .bind(Json(&self.steps))
        .bind(&failure.code)
        .bind(&failure.detail)
        .execute(&self.pool)
        .await;
        let touched = async {
            let mut tx = self.pool.begin().await?;
            live::touch(&mut tx, self.campaign, &Topic::Desk).await?;
            tx.commit().await?;
            Ok::<_, AppError>(())
        }
        .await;
        if let Err(e) = saved {
            tracing::error!(error = %e, job = %self.id, "generation: could not record the failure");
        }
        if let Err(e) = touched {
            tracing::error!(error = ?e, job = %self.id, "generation: could not signal the failure");
        }
    }
}

fn errors(issues: &[Issue]) -> usize {
    issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count()
}

fn needs_repair(issues: &[Issue]) -> bool {
    issues
        .iter()
        .any(|i| i.severity == Severity::Error || STRUCTURAL.contains(&i.code))
}

fn issue_lines(issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|i| {
            let severity = match i.severity {
                Severity::Error => "erreur",
                Severity::Warning => "avertissement",
                Severity::Info => "à vérifier",
            };
            format!("- [{severity}] {} {} : {}", i.code, i.path, i.detail)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Run job `id` to its end, recording each step; a failure is recorded
/// on the job, never lost.
pub async fn run(pool: PgPool, ai: Ai, campaign: Uuid, id: Uuid) {
    let mut runner = Runner {
        pool,
        ai,
        campaign,
        id,
        steps: Vec::new(),
    };
    if let Err(failure) = drive(&mut runner).await {
        runner.fail(failure).await;
    }
}

async fn drive(r: &mut Runner) -> Result<(), Failure> {
    let (Json(input), Json(steps)): (Json<Input>, Json<Vec<Step>>) =
        sqlx::query_as("SELECT input, steps FROM generation_jobs WHERE id = $1")
            .bind(r.id)
            .fetch_one(&r.pool)
            .await?;
    r.steps = steps;
    let row = campaigns::find(&r.pool, r.campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    let library = Library {
        rules: row.rules(),
        maps: None,
    };
    let brief = brief(&row, &input);
    let rules = rules_text(row.rules());

    // 1. Bible, acts, fronts, cast.
    r.step(StepId::Cast, StepStatus::Running, &[]).await?;
    let messages = render(
        &templates::GENERATION_CAST,
        &[
            ("brief", brief.clone()),
            ("rules", rules.clone()),
            ("sizes", input.length.cast_sizes().into()),
        ],
    )?;
    let cast: Cast = r
        .ask(&templates::GENERATION_CAST, messages, CAST_TOKENS)
        .await?;
    r.step(
        StepId::Cast,
        StepStatus::Done,
        &[
            ("acts", cast.acts.len()),
            ("fronts", cast.fronts.len()),
            ("npcs", cast.npcs.len()),
            ("adversaries", cast.adversaries.len()),
            ("locations", cast.locations.len()),
            ("items", cast.items.len()),
        ],
    )
    .await?;

    // 2. Scenes, revelations, clues.
    r.step(StepId::Scenes, StepStatus::Running, &[]).await?;
    let mut story = row.story.clone();
    story.bible = cast.bible;
    story.acts = cast.acts;
    if story.acts.is_empty() {
        story.acts.push(Act {
            id: "acte_1".into(),
            title: "Acte I".into(),
            summary: String::new(),
            opening: String::new(),
            closing: String::new(),
            music: Vec::new(),
            gm_notes: String::new(),
        });
    }
    story.fronts = cast.fronts;
    story.npcs = cast.npcs;
    story.adversaries = cast.adversaries;
    story.locations = cast.locations;
    story.items = cast.items;
    story.factions = cast.factions;
    story.goals = Vec::new();
    let written = serde_json::to_string_pretty(&serde_json::json!({
        "bible": story.bible,
        "acts": story.acts,
        "fronts": story.fronts,
        "npcs": story.npcs,
        "adversaries": story.adversaries,
        "locations": story.locations,
        "items": story.items,
        "factions": story.factions,
    }))
    .map_err(|e| AppError::internal("generation cast", e))?;
    let messages = render(
        &templates::GENERATION_SCENES,
        &[
            ("brief", brief),
            ("cast", written),
            ("rules", rules),
            ("sizes", input.length.scene_sizes().into()),
        ],
    )?;
    let plot: Plot = r
        .ask(&templates::GENERATION_SCENES, messages, SCENES_TOKENS)
        .await?;
    story.bible.start_node = Some(plot.start_node);
    story.nodes = plot.nodes;
    story.revelations = plot.revelations;
    story.clues = plot.clues;
    r.step(
        StepId::Scenes,
        StepStatus::Done,
        &[
            ("nodes", story.nodes.len()),
            ("revelations", story.revelations.len()),
            ("clues", story.clues.len()),
        ],
    )
    .await?;

    // 3. Check, and repair what the validator finds.
    r.step(StepId::Check, StepStatus::Running, &[]).await?;
    let (pruned_story, pruned) = prune(&story, &library);
    let mut story = pruned_story;
    let mut removed: Vec<Removed> = pruned
        .into_iter()
        .map(|p| Removed {
            code: p.code.into(),
            path: p.path,
            detail: p.detail,
        })
        .collect();
    let mut issues = validate_with(&story, &library);
    let mut repairs = 0usize;
    let mut dropped = 0usize;
    while repairs < MAX_REPAIRS && needs_repair(&issues) {
        repairs += 1;
        let yaml = to_yaml(&story).map_err(|e| AppError::internal("generation yaml", e))?;
        let messages = render(
            &templates::GENERATION_REPAIR,
            &[
                ("campaign", yaml.trim_end().to_string()),
                ("issues", issue_lines(&issues)),
            ],
        )?;
        let answer: Repair = r
            .ask(&templates::GENERATION_REPAIR, messages, REPAIR_TOKENS)
            .await?;
        let total = answer.edits.len();
        let readable: Vec<Edit> = answer
            .edits
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        dropped += total - readable.len();
        let (kept, refused) = edit::sanitize(&story, readable);
        dropped += refused;
        let Ok((fixed, _)) = edit::apply(&story, &kept) else {
            continue;
        };
        let (fixed, pruned) = prune(&fixed, &library);
        let fixed_issues = validate_with(&fixed, &library);
        // Kept only when it makes nothing worse.
        if errors(&fixed_issues) <= errors(&issues) && fixed_issues.len() <= issues.len() {
            story = fixed;
            issues = fixed_issues;
            removed.extend(pruned.into_iter().map(|p| Removed {
                code: p.code.into(),
                path: p.path,
                detail: p.detail,
            }));
        }
    }
    let errors = errors(&issues);
    if let Some(s) = r.steps.iter_mut().find(|s| s.id == StepId::Check) {
        s.status = StepStatus::Done;
        s.counts = [
            ("errors".to_string(), errors),
            ("warnings".to_string(), issues.len() - errors),
            ("repairs".to_string(), repairs),
        ]
        .into_iter()
        .collect();
    }
    let mut tx = r.pool.begin().await?;
    sqlx::query(
        "UPDATE generation_jobs SET status = 'succeeded', steps = $2, draft = $3, pruned = $4,
                repairs = $5, dropped = $6
         WHERE id = $1",
    )
    .bind(r.id)
    .bind(Json(&r.steps))
    .bind(Json(&story))
    .bind(Json(&removed))
    .bind(i32::try_from(repairs).unwrap_or(i32::MAX))
    .bind(i32::try_from(dropped).unwrap_or(i32::MAX))
    .execute(&mut *tx)
    .await?;
    live::touch(&mut tx, r.campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------------
// Applying a draft
// ---------------------------------------------------------------------

/// Install a succeeded job's draft as the campaign's story, keeping the
/// campaign's own id, title, world, rule system and party. The campaign
/// stays to review and validate.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, `NO_SUCH_GENERATION`;
/// 409 `GENERATION_ALREADY_APPLIED`, `GENERATION_NOT_READY`,
/// `CAMPAIGN_ALREADY_VALIDATED`; a database error.
pub async fn apply(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
) -> Result<(CampaignRow, Job), AppError> {
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let stored: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM generation_jobs WHERE id = $1 AND campaign_id = $2 FOR UPDATE"
    ))
    .bind(id)
    .bind(campaign)
    .fetch_optional(&mut *tx)
    .await?;
    let current = job(
        row.rules(),
        stored.ok_or(AppError::NotFound("NO_SUCH_GENERATION"))?,
    );
    match current.status {
        JobStatus::Applied => return Err(AppError::Conflict("GENERATION_ALREADY_APPLIED")),
        JobStatus::Succeeded => {}
        JobStatus::Running | JobStatus::Failed => {
            return Err(AppError::Conflict("GENERATION_NOT_READY"));
        }
    }
    if row.validated_at.is_some() {
        return Err(AppError::Conflict("CAMPAIGN_ALREADY_VALIDATED"));
    }
    let mut story = current
        .draft
        .ok_or(AppError::Conflict("GENERATION_NOT_READY"))?;
    story.format = row.story.format;
    story.id = row.story.id.clone();
    story.title = row.story.title.clone();
    story.world = row.story.world.clone();
    story.rules = row.story.rules.clone();
    story.party = row.story.party.clone();
    campaigns::save_story(&mut tx, campaign, &story).await?;
    let applied: Row = sqlx::query_as(&format!(
        "UPDATE generation_jobs SET status = 'applied' WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    let row = campaigns::find(&mut *tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    tx.commit().await?;
    let applied = job(row.rules(), applied);
    Ok((row, applied))
}
