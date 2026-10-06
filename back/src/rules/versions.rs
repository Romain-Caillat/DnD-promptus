//! campaign/edit-rule-system — the GM's versions of a campaign's rule
//! system (migration `015_rule_versions.sql`).
//!
//! - **Draft**: the first edit copies the version the campaign plays —
//!   its newest locked version, or the preset — into a draft numbered
//!   one above everything the campaign has. The GM saves it as often as
//!   they like; a save that does not load is refused with the loader's
//!   errors, and every save answers with what the draft touches
//!   ([`super::report`]).
//! - **Lock**: the draft becomes a version that never changes again. The
//!   campaign plays it from its next session (`evening::session::open`
//!   moves `story.rules` to the newest locked version), and players read
//!   what changed before they play.
//! - **Compare**: any two versions, as the players will read the change
//!   and line by line ([`super::diff`]).
//!
//! The text kept is the YAML the GM wrote (comments included). The
//! structured editor sends a JSON document instead: it is written back
//! as YAML, without comments.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use promptus_shared::maps::Map;
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::changes::{RuleChange, rule_changes};
use promptus_shared::story::RuleSystemRef;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::diff::{self, Line};
use super::report::{self, Report, Version};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::{self, CampaignRow};
use crate::content;
use crate::error::AppError;
use crate::live::{self, Topic};

/// One version in the history.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub version: u32,
    pub note: String,
    /// `None` for the draft.
    pub locked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// A compiled-in preset, not one of the campaign's own.
    pub preset: bool,
}

/// The editor's whole screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Editor {
    pub rules_id: String,
    pub name: String,
    /// The version the campaign plays now.
    pub current: u32,
    /// The newest locked version above `current`: the campaign moves to
    /// it when its next session opens.
    pub next: Option<u32>,
    /// Newest first, the draft included.
    pub versions: Vec<VersionInfo>,
    pub draft: Option<Draft>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub version: u32,
    pub note: String,
    pub yaml: String,
    /// The same text as a JSON tree, for the structured editor.
    pub document: serde_json::Value,
    pub report: Report,
}

type Row = (
    i32,
    String,
    String,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
    DateTime<Utc>,
);

fn as_version(v: i32) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

fn as_db(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

async fn rows(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<Row>, AppError> {
    Ok(sqlx::query_as(
        "SELECT version, system, note, locked_at, created_at, updated_at
         FROM rule_versions WHERE campaign_id = $1 ORDER BY version DESC",
    )
    .bind(campaign)
    .fetch_all(db)
    .await?)
}

/// The text of version `version` of the campaign's rules: its own
/// (draft or locked), or the preset.
fn text_of<'a>(rows: &'a [Row], rules_id: &str, version: u32) -> Option<&'a str> {
    rows.iter()
        .find(|r| as_version(r.0) == version)
        .map(|r| r.1.as_str())
        .or_else(|| {
            content::preset_yaml(&RuleSystemRef {
                id: rules_id.to_string(),
                version,
            })
        })
}

fn load(text: &str) -> Result<RuleSystem, AppError> {
    RuleSystem::from_yaml(text).map_err(|e| AppError::Invalid {
        code: "RULES_INVALID",
        detail: e
            .0
            .iter()
            .map(|r| format!("{} — {}", r.path, r.detail))
            .collect::<Vec<_>>()
            .join("\n"),
    })
}

/// The maps a campaign's fights are played on.
fn maps_of(row: &CampaignRow) -> Vec<Map> {
    content::maps(&row.story).cloned().collect()
}

/// The report of `draft_text` against the version the campaign plays.
async fn report_of(row: &CampaignRow, rows: &[Row], draft_text: &str) -> Result<Report, AppError> {
    let draft = load(draft_text)?;
    let current_text = text_of(rows, &row.story.rules.id, row.story.rules.version)
        .map(str::to_string)
        .ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let current: Arc<RuleSystem> = row
        .rules
        .clone()
        .ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let campaign = row.story.clone();
    let maps = maps_of(row);
    let draft_text = draft_text.to_string();
    tokio::task::spawn_blocking(move || {
        report::report(
            &campaign,
            &maps,
            &Version {
                text: &current_text,
                system: &current,
            },
            &Version {
                text: &draft_text,
                system: &draft,
            },
        )
    })
    .await
    .map_err(|e| AppError::internal("rule report", e))
}

/// The YAML text as a JSON tree (one-key maps stay one-key maps).
fn document(text: &str) -> Result<serde_json::Value, AppError> {
    serde_yaml_ng::from_str(text).map_err(|e| AppError::internal("rule yaml to json", e))
}

async fn editor(pool: &PgPool, row: &CampaignRow) -> Result<Editor, AppError> {
    let rows = rows(pool, row.id).await?;
    let current = row.story.rules.version;
    let next = rows
        .iter()
        .filter(|r| r.3.is_some() && as_version(r.0) > current)
        .map(|r| as_version(r.0))
        .max();
    let mut versions: Vec<VersionInfo> = rows
        .iter()
        .map(
            |(v, _, note, locked_at, created_at, updated_at)| VersionInfo {
                version: as_version(*v),
                note: note.clone(),
                locked_at: *locked_at,
                created_at: *created_at,
                updated_at: *updated_at,
                preset: false,
            },
        )
        .collect();
    // The presets of this system the campaign went through.
    for preset in content::presets().filter(|p| p.id == row.story.rules.id) {
        if !versions.iter().any(|v| v.version == preset.version) {
            versions.push(VersionInfo {
                version: preset.version,
                note: String::new(),
                locked_at: None,
                created_at: row.updated_at,
                updated_at: row.updated_at,
                preset: true,
            });
        }
    }
    versions.sort_by_key(|v| std::cmp::Reverse(v.version));
    let draft = match rows.iter().find(|r| r.3.is_none()) {
        Some((v, text, note, ..)) => Some(Draft {
            version: as_version(*v),
            note: note.clone(),
            yaml: text.clone(),
            document: document(text)?,
            report: report_of(row, &rows, text).await?,
        }),
        None => None,
    };
    Ok(Editor {
        rules_id: row.story.rules.id.clone(),
        name: row
            .rules()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| row.story.rules.id.clone()),
        current,
        next,
        versions,
        draft,
    })
}

/// `GET /api/campaigns/{id}/rules`.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; a database error.
pub async fn get(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Editor, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    editor(pool, &row).await
}

/// Set the top-level `version:` line of `text` to `version`, keeping
/// every other line (comments included).
fn with_version(text: &str, version: u32) -> Option<String> {
    let mut done = false;
    let lines: Vec<String> = text
        .lines()
        .map(|l| {
            if !done && l.starts_with("version:") {
                done = true;
                format!("version: {version}")
            } else {
                l.to_string()
            }
        })
        .collect();
    done.then(|| {
        let mut out = lines.join("\n");
        out.push('\n');
        out
    })
}

async fn locked_row(
    tx: &mut Transaction<'_, Postgres>,
    gm: &CurrentGm,
    campaign: Uuid,
) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::lock(tx, campaign).await?, gm)
}

/// Start a draft (or return the one in progress): a copy of the newest
/// locked version, or of the version the campaign plays.
///
/// # Errors
///
/// 404 as [`get`]; 409 `RULES_UNKNOWN` when the server does not have the
/// version the campaign plays; a database error.
pub async fn start(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Editor, AppError> {
    let mut tx = pool.begin().await?;
    let row = locked_row(&mut tx, gm, campaign).await?;
    let rows = rows(&mut *tx, campaign).await?;
    if !rows.iter().any(|r| r.3.is_none()) {
        let current = row.story.rules.version;
        let newest_locked = rows
            .iter()
            .filter(|r| r.3.is_some())
            .map(|r| as_version(r.0))
            .max()
            .filter(|v| *v > current)
            .unwrap_or(current);
        let base = text_of(&rows, &row.story.rules.id, newest_locked)
            .ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
        let top = rows
            .iter()
            .map(|r| as_version(r.0))
            .chain(std::iter::once(current))
            .chain(
                content::presets()
                    .filter(|p| p.id == row.story.rules.id)
                    .map(|p| p.version),
            )
            .max()
            .unwrap_or(current);
        let text = with_version(base, top + 1).ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
        sqlx::query("INSERT INTO rule_versions (campaign_id, version, system) VALUES ($1, $2, $3)")
            .bind(campaign)
            .bind(as_db(top + 1))
            .bind(&text)
            .execute(&mut *tx)
            .await?;
        live::touch(&mut tx, campaign, &Topic::Desk).await?;
    }
    tx.commit().await?;
    editor(pool, &row).await
}

/// What the GM saves: their YAML, or the structured editor's document.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Save {
    #[serde(default)]
    pub yaml: Option<String>,
    #[serde(default)]
    pub document: Option<serde_json::Value>,
    #[serde(default)]
    pub note: String,
}

const NOTE_MAX: usize = 2_000;
const TEXT_MAX: usize = 400_000;

/// Save the draft. The text must load, and keep the system's id and the
/// draft's version.
///
/// # Errors
///
/// 404 as [`get`]; 409 `NO_DRAFT`; 400 `RULES_INVALID` (with the
/// loader's errors), `RULES_ID_CHANGED`, `TEXT_TOO_LONG`, `EMPTY_TEXT`;
/// a database error.
pub async fn save(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    input: &Save,
) -> Result<Editor, AppError> {
    let text = match (&input.yaml, &input.document) {
        (Some(yaml), None) => yaml.clone(),
        (None, Some(doc)) => {
            serde_yaml_ng::to_string(doc).map_err(|e| AppError::internal("rule json to yaml", e))?
        }
        _ => return Err(AppError::BadRequest("EMPTY_TEXT")),
    };
    if text.len() > TEXT_MAX || input.note.chars().count() > NOTE_MAX {
        return Err(AppError::BadRequest("TEXT_TOO_LONG"));
    }
    let mut tx = pool.begin().await?;
    let row = locked_row(&mut tx, gm, campaign).await?;
    let rows = rows(&mut *tx, campaign).await?;
    let version = rows
        .iter()
        .find(|r| r.3.is_none())
        .map(|r| as_version(r.0))
        .ok_or(AppError::Conflict("NO_DRAFT"))?;
    let system = load(&text)?;
    if system.id != row.story.rules.id || system.version != version {
        return Err(AppError::BadRequest("RULES_ID_CHANGED"));
    }
    sqlx::query(
        "UPDATE rule_versions SET system = $3, note = $4
         WHERE campaign_id = $1 AND version = $2 AND locked_at IS NULL",
    )
    .bind(campaign)
    .bind(as_db(version))
    .bind(&text)
    .bind(input.note.trim())
    .execute(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    editor(pool, &row).await
}

/// Lock the draft: it never changes again, and the campaign plays it
/// from its next session.
///
/// # Errors
///
/// 404 as [`get`]; 409 `NO_DRAFT`; 400 `RULES_INVALID` when the draft no
/// longer loads; a database error.
pub async fn lock(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Editor, AppError> {
    let mut tx = pool.begin().await?;
    let row = locked_row(&mut tx, gm, campaign).await?;
    let rows = rows(&mut *tx, campaign).await?;
    let (version, text) = rows
        .iter()
        .find(|r| r.3.is_none())
        .map(|r| (r.0, r.1.clone()))
        .ok_or(AppError::Conflict("NO_DRAFT"))?;
    load(&text)?;
    sqlx::query(
        "UPDATE rule_versions SET locked_at = now() WHERE campaign_id = $1 AND version = $2",
    )
    .bind(campaign)
    .bind(version)
    .execute(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    editor(pool, &row).await
}

/// Drop the draft, if there is one.
///
/// # Errors
///
/// 404 as [`get`]; a database error.
pub async fn discard(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Editor, AppError> {
    let mut tx = pool.begin().await?;
    let row = locked_row(&mut tx, gm, campaign).await?;
    let gone =
        sqlx::query("DELETE FROM rule_versions WHERE campaign_id = $1 AND locked_at IS NULL")
            .bind(campaign)
            .execute(&mut *tx)
            .await?;
    if gone.rows_affected() > 0 {
        live::touch(&mut tx, campaign, &Topic::Desk).await?;
    }
    tx.commit().await?;
    editor(pool, &row).await
}

/// Two versions side by side.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub from: u32,
    pub to: u32,
    /// What a player reads as changed.
    pub changes: Vec<RuleChange>,
    /// Every line, GM-only parts included.
    pub lines: Vec<Line>,
}

/// Compare versions `from` and `to` of the campaign's rules (its own,
/// draft included, or the presets of its system).
///
/// # Errors
///
/// 404 as [`get`], or `NO_SUCH_VERSION`; 400 `RULES_INVALID` when one no
/// longer loads; a database error.
pub async fn compare(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    from: u32,
    to: u32,
) -> Result<Comparison, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let rows = rows(pool, campaign).await?;
    let text =
        |v| text_of(&rows, &row.story.rules.id, v).ok_or(AppError::NotFound("NO_SUCH_VERSION"));
    let (a, b) = (text(from)?, text(to)?);
    Ok(Comparison {
        from,
        to,
        changes: rule_changes(&load(a)?, &load(b)?),
        lines: diff::lines(a, b),
    })
}

/// When a session opens: the campaign moves to its newest locked
/// version, if it is above the one it plays. Returns the new version.
/// The caller holds the campaign lock.
///
/// # Errors
///
/// A database error.
pub async fn adopt_newest(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
) -> Result<Option<u32>, AppError> {
    let newest: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(version) FROM rule_versions WHERE campaign_id = $1 AND locked_at IS NOT NULL",
    )
    .bind(row.id)
    .fetch_one(&mut **tx)
    .await?;
    let Some(newest) = newest.map(as_version) else {
        return Ok(None);
    };
    if newest <= row.story.rules.version {
        return Ok(None);
    }
    let mut story = row.story.clone();
    story.rules.version = newest;
    campaigns::save_story(tx, row.id, &story).await?;
    Ok(Some(newest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_line_changes_and_the_comments_stay() {
        let text = "# Mes règles\nid: corsaires\nversion: 1\nname: X\n  version: 7\n";
        assert_eq!(
            with_version(text, 2).unwrap(),
            "# Mes règles\nid: corsaires\nversion: 2\nname: X\n  version: 7\n"
        );
        assert!(with_version("id: x\n", 2).is_none());
    }
}
