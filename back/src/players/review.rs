//! session/validate-characters — the GM's review of the sheets
//! (migration `007_character_review.sql`).
//!
//! A submitted sheet is read with the rule system's checks
//! ([`promptus_shared::rules::check_character`]: they report, never
//! block), then **validated** or **returned** with a word for the player
//! (`gm_note`). Both take a snapshot of the sheet (`reviewed_sheet`):
//! when the player sends it again, [`review`] lists only what changed
//! since ([`changes`]). The GM keeps **secret hooks** drawn from the
//! backstories, tied to nodes and fronts of the story.
//!
//! All of it is GM-side. Nothing here is read by a player route: the
//! player projection (`campaigns::projection`) only knows `sheet`,
//! `status` and `gm_note`.
//!
//! The player's side — saving a draft, submitting it — belongs to
//! `characters/build-character-creator`; its writes must call
//! [`super::touch_character`] so the GM's table follows live.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use promptus_shared::issue::{Issue, Severity};
use promptus_shared::rules::{CharacterInput, RuleSystem, check_character};
use promptus_shared::story::Campaign;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use super::{CharacterSheet, CharacterStatus, touch_character};
use crate::campaigns::CampaignRow;
use crate::error::AppError;

/// Longest word to a player, in characters.
pub const NOTE_MAX: usize = 2000;
/// Longest hook title and body, in characters.
pub const HOOK_TITLE_MAX: usize = 120;
pub const HOOK_BODY_MAX: usize = 2000;

/// One field that moved between the last review and the sheet now.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    /// Dotted path in the sheet: `name`, `abilities.FOR`, `look.hair`.
    pub path: String,
    /// `None` when the field did not exist.
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// The fields of `after` that differ from `before`, objects compared
/// key by key (`abilities.FOR`), anything else whole. In path order.
#[must_use]
pub fn changes(before: &Value, after: &Value) -> Vec<Change> {
    fn flatten(prefix: &str, v: &Value, out: &mut BTreeMap<String, Value>) {
        match v {
            Value::Object(map) if !map.is_empty() || prefix.is_empty() => {
                for (k, v) in map {
                    let path = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{prefix}.{k}")
                    };
                    flatten(&path, v, out);
                }
            }
            // An empty value is no value: a field the player cleared
            // reads as removed, like one never written.
            Value::Null => {}
            Value::String(s) if s.is_empty() => {}
            _ => {
                out.insert(prefix.to_string(), v.clone());
            }
        }
    }
    let (mut a, mut b) = (BTreeMap::new(), BTreeMap::new());
    flatten("", before, &mut a);
    flatten("", after, &mut b);
    let paths: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    paths
        .into_iter()
        .filter(|p| a.get(*p) != b.get(*p))
        .map(|p| Change {
            path: p.clone(),
            before: a.get(p).cloned(),
            after: b.get(p).cloned(),
        })
        .collect()
}

/// An ability as the review labels it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbilityLabel {
    pub id: String,
    pub name: String,
}

/// What the GM reads to decide on one character.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: Uuid,
    pub player_id: Uuid,
    pub nickname: String,
    pub status: CharacterStatus,
    /// The sheet as the player stored it, every field kept (what the
    /// creator adds — look, people, backstory — included).
    pub sheet: Value,
    pub gm_note: Option<String>,
    pub updated_at: DateTime<Utc>,
    /// The sheet as of the GM's last decision, and when.
    pub reviewed_sheet: Option<Value>,
    pub reviewed_at: Option<DateTime<Utc>>,
    /// What moved since `reviewed_sheet`; empty without one.
    pub changes: Vec<Change>,
    /// The rule system's checks on the sheet. They never block.
    pub checks: Vec<Issue>,
    /// The campaign's rule system, to label the sheet.
    pub rules_name: Option<String>,
    pub class_name: Option<String>,
    pub abilities: Vec<AbilityLabel>,
}

/// The sheet fields the rules check, read leniently: a sheet the rules
/// cannot read is checked as empty, and the checks say so.
fn rule_input(sheet: &Value) -> CharacterSheet {
    serde_json::from_value(sheet.clone()).unwrap_or_default()
}

/// The checks of `sheet` under `system`, or one issue saying the rules
/// are unknown to this server.
#[must_use]
pub fn checks(campaign: &Campaign, system: Option<&RuleSystem>, sheet: &Value) -> Vec<Issue> {
    let Some(system) = system else {
        let r = &campaign.rules;
        return vec![Issue {
            severity: Severity::Warning,
            code: "RULES_UNKNOWN",
            path: "rules".into(),
            detail: format!("rule system {} v{} is not on this server", r.id, r.version),
            message: Some(format!(
                "Les règles « {} » v{} sont inconnues du serveur : rien n'a été vérifié.",
                r.id, r.version
            )),
        }];
    };
    let s = rule_input(sheet);
    check_character(
        system,
        &CharacterInput {
            name: &s.name,
            class_id: s.class_id.as_deref(),
            abilities: &s.abilities,
        },
    )
}

/// The name of class `id` in the campaign's rules, if both are known.
#[must_use]
pub fn class_name(system: Option<&RuleSystem>, id: Option<&str>) -> Option<String> {
    system?.class(id?).map(|c| c.name.clone())
}

type ReviewRow = (
    Uuid,
    Uuid,
    String,
    String,
    Value,
    Option<String>,
    DateTime<Utc>,
    Option<Value>,
    Option<DateTime<Utc>>,
);

/// Character `id` of `campaign`, as the GM reviews it. The caller has
/// checked the GM owns the campaign.
///
/// # Errors
///
/// 404 `NOT_FOUND` when no such character sits at this table.
pub async fn review(pool: &PgPool, campaign: &CampaignRow, id: Uuid) -> Result<Review, AppError> {
    let row: Option<ReviewRow> = sqlx::query_as(
        "SELECT c.id, p.id, p.nickname, c.status, c.sheet, c.gm_note, c.updated_at,
                c.reviewed_sheet, c.reviewed_at
         FROM characters c JOIN players p ON p.id = c.player_id
         WHERE c.id = $1 AND c.campaign_id = $2",
    )
    .bind(id)
    .bind(campaign.id)
    .fetch_optional(pool)
    .await?;
    let Some((id, player_id, nickname, status, sheet, gm_note, updated_at, reviewed, reviewed_at)) =
        row
    else {
        return Err(AppError::NotFound("NOT_FOUND"));
    };
    let story = &campaign.story;
    let system = campaign.rules();
    let class_id = sheet.get("classId").and_then(Value::as_str);
    Ok(Review {
        id,
        player_id,
        nickname,
        status: CharacterStatus::parse(&status)?,
        changes: reviewed
            .as_ref()
            .map(|before| changes(before, &sheet))
            .unwrap_or_default(),
        checks: checks(story, system, &sheet),
        rules_name: system.map(|s| s.name.clone()),
        class_name: class_name(system, class_id),
        abilities: system
            .map(|s| {
                s.abilities
                    .iter()
                    .map(|a| AbilityLabel {
                        id: a.id.clone(),
                        name: a.name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        sheet,
        gm_note,
        updated_at,
        reviewed_sheet: reviewed,
        reviewed_at,
    })
}

/// The GM's decision on a submitted sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Validate,
    /// Back to the player with this word.
    Return(String),
}

/// A word to a player as stored: trimmed, 1 to [`NOTE_MAX`] characters.
///
/// # Errors
///
/// 400 `NOTE_REQUIRED` or `NOTE_TOO_LONG`.
pub fn clean_note(raw: &str) -> Result<String, AppError> {
    let note = raw.trim();
    if note.is_empty() {
        return Err(AppError::BadRequest("NOTE_REQUIRED"));
    }
    if note.chars().count() > NOTE_MAX {
        return Err(AppError::BadRequest("NOTE_TOO_LONG"));
    }
    Ok(note.to_string())
}

/// Validate or return character `id` of `campaign_id`, as the GM read it
/// at `seen` (its `updatedAt`). The sheet is snapshotted for the next
/// review, and the character's and the table's topics move. The caller
/// has checked the GM owns the campaign.
///
/// # Errors
///
/// 404 `NOT_FOUND` when no such character sits at this table; 409
/// `CHARACTER_NOT_SUBMITTED` when it is not waiting for review; 409
/// `CHARACTER_CHANGED` when the player changed it after `seen` — the GM
/// would decide on a sheet they have not read.
pub async fn decide(
    pool: &PgPool,
    campaign_id: Uuid,
    id: Uuid,
    seen: DateTime<Utc>,
    decision: Decision,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let row: Option<(String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT status, updated_at FROM characters
         WHERE id = $1 AND campaign_id = $2 FOR UPDATE",
    )
    .bind(id)
    .bind(campaign_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((status, updated_at)) = row else {
        return Err(AppError::NotFound("NOT_FOUND"));
    };
    if CharacterStatus::parse(&status)? != CharacterStatus::Submitted {
        return Err(AppError::Conflict("CHARACTER_NOT_SUBMITTED"));
    }
    if updated_at != seen {
        return Err(AppError::Conflict("CHARACTER_CHANGED"));
    }
    let (status, note) = match decision {
        Decision::Validate => ("validated", None),
        Decision::Return(note) => ("returned", Some(note)),
    };
    sqlx::query(
        "UPDATE characters
         SET status = $2, gm_note = $3, reviewed_sheet = sheet, reviewed_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(note)
    .execute(&mut *tx)
    .await?;
    touch_character(&mut tx, campaign_id, id).await?;
    tx.commit().await?;
    Ok(())
}

// --- Secret hooks ------------------------------------------------------------

/// A hook the GM draws from a player's backstory. GM-only.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hook {
    pub id: Uuid,
    pub character_id: Uuid,
    pub title: String,
    pub body: String,
    /// Ids of the story's nodes and fronts it ties into.
    pub links: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What the GM writes for a hook.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HookInput {
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub links: Vec<String>,
}

/// Something of the story a hook can tie into.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookTarget {
    pub id: String,
    /// `node` or `front`.
    pub kind: &'static str,
    pub title: String,
}

/// The story's nodes, then its fronts.
#[must_use]
pub fn hook_targets(campaign: &Campaign) -> Vec<HookTarget> {
    campaign
        .nodes
        .iter()
        .map(|n| HookTarget {
            id: n.id.clone(),
            kind: "node",
            title: n.title.clone(),
        })
        .chain(campaign.fronts.iter().map(|f| HookTarget {
            id: f.id.clone(),
            kind: "front",
            title: f.name.clone(),
        }))
        .collect()
}

/// `input` as stored: trimmed, bounded, its links existing in the story
/// and each once.
fn clean_hook(campaign: &Campaign, input: HookInput) -> Result<HookInput, AppError> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::BadRequest("HOOK_TITLE_REQUIRED"));
    }
    let body = input.body.trim().to_string();
    if title.chars().count() > HOOK_TITLE_MAX || body.chars().count() > HOOK_BODY_MAX {
        return Err(AppError::BadRequest("HOOK_TOO_LONG"));
    }
    let mut links: Vec<String> = Vec::new();
    for l in input.links {
        let known =
            campaign.nodes.iter().any(|n| n.id == l) || campaign.fronts.iter().any(|f| f.id == l);
        if !known {
            return Err(AppError::BadRequest("HOOK_LINK_UNKNOWN"));
        }
        if !links.contains(&l) {
            links.push(l);
        }
    }
    Ok(HookInput { title, body, links })
}

type HookRow = (
    Uuid,
    Uuid,
    String,
    String,
    Json<Vec<String>>,
    DateTime<Utc>,
    DateTime<Utc>,
);

fn hook_from_row((id, character_id, title, body, links, created_at, updated_at): HookRow) -> Hook {
    Hook {
        id,
        character_id,
        title,
        body,
        links: links.0,
        created_at,
        updated_at,
    }
}

const HOOK_COLUMNS: &str = "id, character_id, title, body, links, created_at, updated_at";

/// Every hook of `campaign_id`, oldest first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn hooks(pool: &PgPool, campaign_id: Uuid) -> Result<Vec<Hook>, AppError> {
    let rows: Vec<HookRow> = sqlx::query_as(&format!(
        "SELECT {HOOK_COLUMNS} FROM secret_hooks WHERE campaign_id = $1 ORDER BY created_at, id"
    ))
    .bind(campaign_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(hook_from_row).collect())
}

/// A new hook drawn from character `character_id`'s backstory.
///
/// # Errors
///
/// 404 `NOT_FOUND` when that character does not sit at this table; 400
/// `HOOK_TITLE_REQUIRED`, `HOOK_TOO_LONG`, `HOOK_LINK_UNKNOWN`.
pub async fn add_hook(
    pool: &PgPool,
    campaign: &CampaignRow,
    character_id: Uuid,
    input: HookInput,
) -> Result<Hook, AppError> {
    let h = clean_hook(&campaign.story, input)?;
    let row: Option<HookRow> = sqlx::query_as(&format!(
        "INSERT INTO secret_hooks (campaign_id, character_id, title, body, links)
         SELECT $1, c.id, $3, $4, $5 FROM characters c WHERE c.id = $2 AND c.campaign_id = $1
         RETURNING {HOOK_COLUMNS}"
    ))
    .bind(campaign.id)
    .bind(character_id)
    .bind(&h.title)
    .bind(&h.body)
    .bind(Json(&h.links))
    .fetch_optional(pool)
    .await?;
    row.map(hook_from_row)
        .ok_or(AppError::NotFound("NOT_FOUND"))
}

/// Rewrite hook `id`.
///
/// # Errors
///
/// 404 `NOT_FOUND` when no such hook belongs to this campaign; the
/// errors of [`add_hook`].
pub async fn edit_hook(
    pool: &PgPool,
    campaign: &CampaignRow,
    id: Uuid,
    input: HookInput,
) -> Result<Hook, AppError> {
    let h = clean_hook(&campaign.story, input)?;
    let row: Option<HookRow> = sqlx::query_as(&format!(
        "UPDATE secret_hooks SET title = $3, body = $4, links = $5
         WHERE id = $1 AND campaign_id = $2
         RETURNING {HOOK_COLUMNS}"
    ))
    .bind(id)
    .bind(campaign.id)
    .bind(&h.title)
    .bind(&h.body)
    .bind(Json(&h.links))
    .fetch_optional(pool)
    .await?;
    row.map(hook_from_row)
        .ok_or(AppError::NotFound("NOT_FOUND"))
}

/// Delete hook `id`.
///
/// # Errors
///
/// 404 `NOT_FOUND` when no such hook belongs to this campaign.
pub async fn delete_hook(pool: &PgPool, campaign_id: Uuid, id: Uuid) -> Result<(), AppError> {
    let done = sqlx::query("DELETE FROM secret_hooks WHERE id = $1 AND campaign_id = $2")
        .bind(id)
        .bind(campaign_id)
        .execute(pool)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound("NOT_FOUND"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_what_moved_is_listed() {
        let before = json!({
            "name": "Borin", "classId": "bretteur",
            "abilities": { "FOR": 18, "SAG": 12, "DEX": 10 },
            "backstory": "La mine.", "look": { "body": "nain", "hair": { "colour": "roux" } }
        });
        let after = json!({
            "name": "Borin", "classId": "bretteur",
            "abilities": { "FOR": 17, "SAG": 13, "DEX": 10 },
            "backstory": "La mine.", "look": { "body": "nain", "hair": { "colour": "noir" } },
            "appearance": "Une barbe tressée."
        });
        let c = changes(&before, &after);
        let paths: Vec<&str> = c.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "abilities.FOR",
                "abilities.SAG",
                "appearance",
                "look.hair.colour"
            ]
        );
        assert_eq!(c[0].before, Some(json!(18)));
        assert_eq!(c[0].after, Some(json!(17)));
        assert_eq!(c[2].before, None);
        assert!(changes(&after, &after).is_empty());
    }

    #[test]
    fn a_cleared_field_reads_as_removed() {
        let c = changes(
            &json!({ "appearance": "Grand." }),
            &json!({ "appearance": "" }),
        );
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].after, None);
    }

    #[test]
    fn a_note_is_a_word_not_an_essay() {
        assert_eq!(clean_note("  Force 17 max. ").unwrap(), "Force 17 max.");
        assert!(clean_note(" \n ").is_err());
        assert!(clean_note(&"a".repeat(NOTE_MAX + 1)).is_err());
    }
}
