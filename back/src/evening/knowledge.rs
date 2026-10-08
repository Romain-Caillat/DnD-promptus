//! session/track-table-knowledge — what the table knows.
//!
//! - **The journal** (`table_journal`): every scene shown, clue found,
//!   NPC met, promise, debt, key item, roll and narration, in order. A
//!   `shared` line is the players' journal; the others stay the GM's.
//!   Each line may name the story id it is about (`ref`), so the GM
//!   finds what the players know of an NPC in one gesture ([`about`]).
//! - **The rulings** (`rulings`): the difficulty the GM gave for a
//!   situation in play, offered again when the same situation comes
//!   back ([`similar_rulings`]).
//! - **The gaps** ([`gaps`]): what the scenes reachable from here need
//!   the players to know and they do not know yet.

use chrono::{DateTime, Utc};
use promptus_shared::story::{Campaign, WorldState};
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, Postgres, Transaction};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalKind {
    Scene,
    Clue,
    Npc,
    Promise,
    Debt,
    Item,
    Note,
    Roll,
    Narration,
    Fight,
    Loot,
}

impl JournalKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Scene => "scene",
            Self::Clue => "clue",
            Self::Npc => "npc",
            Self::Promise => "promise",
            Self::Debt => "debt",
            Self::Item => "item",
            Self::Note => "note",
            Self::Roll => "roll",
            Self::Narration => "narration",
            Self::Fight => "fight",
            Self::Loot => "loot",
        }
    }

    fn parse(s: &str) -> Result<Self, AppError> {
        Ok(match s {
            "scene" => Self::Scene,
            "clue" => Self::Clue,
            "npc" => Self::Npc,
            "promise" => Self::Promise,
            "debt" => Self::Debt,
            "item" => Self::Item,
            "note" => Self::Note,
            "roll" => Self::Roll,
            "narration" => Self::Narration,
            "fight" => Self::Fight,
            "loot" => Self::Loot,
            other => return Err(AppError::Internal(format!("journal kind {other}"))),
        })
    }
}

/// One line of the journal.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalLine {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub kind: JournalKind,
    pub r#ref: Option<String>,
    pub text: String,
    pub shared: bool,
    pub created_at: DateTime<Utc>,
}

type LineRow = (
    Uuid,
    Option<Uuid>,
    String,
    Option<String>,
    String,
    bool,
    DateTime<Utc>,
);

fn line(r: LineRow) -> Result<JournalLine, AppError> {
    Ok(JournalLine {
        id: r.0,
        session_id: r.1,
        kind: JournalKind::parse(&r.2)?,
        r#ref: r.3,
        text: r.4,
        shared: r.5,
        created_at: r.6,
    })
}

/// Add a line, inside the caller's (locked) transaction.
///
/// # Errors
///
/// A database error.
pub async fn write(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    session: Option<Uuid>,
    kind: JournalKind,
    r#ref: Option<&str>,
    text: &str,
    shared: bool,
) -> Result<(), AppError> {
    write_about(tx, campaign, session, kind, r#ref, text, shared, None).await
}

/// [`write`], naming the character the line is about (loot handed to
/// them, a purchase, a gift): their end-of-evening screen lists it.
///
/// # Errors
///
/// A database error.
#[allow(clippy::too_many_arguments)]
pub async fn write_about(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    session: Option<Uuid>,
    kind: JournalKind,
    r#ref: Option<&str>,
    text: &str,
    shared: bool,
    character: Option<Uuid>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO table_journal (campaign_id, session_id, kind, ref, text, shared, character_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(campaign)
    .bind(session)
    .bind(kind.as_str())
    .bind(r#ref)
    .bind(text)
    .bind(shared)
    .bind(character)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The shared lines of `session` about `character`, oldest first.
///
/// # Errors
///
/// A database error.
pub async fn shared_about_character(
    db: impl PgExecutor<'_>,
    session: Uuid,
    character: Uuid,
) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT text FROM table_journal
         WHERE session_id = $1 AND character_id = $2 AND shared ORDER BY created_at, id",
    )
    .bind(session)
    .bind(character)
    .fetch_all(db)
    .await?)
}

/// The journal of `campaign`, oldest first: every line for the GM, the
/// shared ones only for players.
///
/// # Errors
///
/// A database error.
pub async fn journal(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    shared_only: bool,
) -> Result<Vec<JournalLine>, AppError> {
    let rows: Vec<LineRow> = sqlx::query_as(
        "SELECT id, session_id, kind, ref, text, shared, created_at FROM table_journal
         WHERE campaign_id = $1 AND (shared OR NOT $2) ORDER BY created_at, id",
    )
    .bind(campaign)
    .bind(shared_only)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(line).collect()
}

/// Every line about story id `ref` (an NPC, a clue, a node), oldest first.
///
/// # Errors
///
/// A database error.
pub async fn about(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    r#ref: &str,
) -> Result<Vec<JournalLine>, AppError> {
    let rows: Vec<LineRow> = sqlx::query_as(
        "SELECT id, session_id, kind, ref, text, shared, created_at FROM table_journal
         WHERE campaign_id = $1 AND ref = $2 ORDER BY created_at, id",
    )
    .bind(campaign)
    .bind(r#ref)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(line).collect()
}

/// A difficulty the GM gave in play.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ruling {
    pub id: Uuid,
    pub situation: String,
    pub ability: String,
    pub difficulty: i32,
    pub created_at: DateTime<Utc>,
}

/// Remember a ruling, inside the caller's transaction.
///
/// # Errors
///
/// A database error.
pub async fn rule(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    session: Uuid,
    situation: &str,
    ability: &str,
    difficulty: i32,
) -> Result<(), AppError> {
    if situation.trim().is_empty() {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO rulings (campaign_id, session_id, situation, ability, difficulty)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(campaign)
    .bind(session)
    .bind(situation.trim())
    .bind(ability)
    .bind(difficulty)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Words that say nothing about the situation.
const STOP_WORDS: [&str; 34] = [
    "le", "la", "les", "un", "une", "des", "de", "du", "au", "aux", "et", "ou", "en", "sur",
    "sous", "pour", "par", "avec", "dans", "je", "tu", "il", "elle", "on", "nous", "vous", "son",
    "sa", "ses", "mon", "ma", "mes", "ce", "qui",
];

fn fold(c: char) -> char {
    match c {
        'à' | 'â' | 'ä' | 'á' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' => 'i',
        'ô' | 'ö' | 'ó' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ç' => 'c',
        'ÿ' => 'y',
        'œ' => 'o',
        other => other,
    }
}

/// French endings stripped so « escalade », « escalader » and
/// « escaladent » are one word. Longest first.
const ENDINGS: [&str; 8] = ["ement", "ent", "ons", "ez", "er", "es", "e", "s"];

fn stem(word: &str) -> String {
    let word = match word.strip_suffix('s') {
        Some(root) if root.chars().count() >= 3 => root,
        _ => word,
    };
    for ending in ENDINGS {
        if let Some(root) = word.strip_suffix(ending)
            && root.chars().count() >= 4
        {
            return root.to_string();
        }
    }
    word.to_string()
}

/// The words of a situation that carry meaning: lowercased, accents
/// folded, short and empty words dropped, French endings stripped.
#[must_use]
pub fn keywords(text: &str) -> Vec<String> {
    let folded: String = text.to_lowercase().chars().map(fold).collect();
    let mut out: Vec<String> = folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3 && !STOP_WORDS.contains(w))
        .map(stem)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// How much `past` covers `now`: shared keywords over `now`'s keywords.
#[must_use]
pub fn likeness(now: &[String], past: &[String]) -> f64 {
    if now.is_empty() {
        return 0.0;
    }
    let shared = now.iter().filter(|w| past.contains(w)).count();
    shared as f64 / now.len() as f64
}

/// Past rulings of `campaign` for a situation like `situation`, the most
/// alike first, then the latest.
///
/// # Errors
///
/// A database error.
pub async fn similar_rulings(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    situation: &str,
) -> Result<Vec<Ruling>, AppError> {
    let now = keywords(situation);
    if now.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(Uuid, String, String, i32, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, situation, ability, difficulty, created_at FROM rulings
         WHERE campaign_id = $1 ORDER BY created_at DESC LIMIT 500",
    )
    .bind(campaign)
    .fetch_all(db)
    .await?;
    let mut scored: Vec<(f64, Ruling)> = rows
        .into_iter()
        .filter_map(|(id, situation, ability, difficulty, created_at)| {
            let score = likeness(&now, &keywords(&situation));
            (score >= 0.5).then_some((
                score,
                Ruling {
                    id,
                    situation,
                    ability,
                    difficulty,
                    created_at,
                },
            ))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.1.created_at.cmp(&a.1.created_at))
    });
    Ok(scored.into_iter().map(|(_, r)| r).take(5).collect())
}

/// Every ruling of `campaign`, the latest first.
///
/// # Errors
///
/// A database error.
pub async fn rulings(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<Ruling>, AppError> {
    let rows: Vec<(Uuid, String, String, i32, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, situation, ability, difficulty, created_at FROM rulings
         WHERE campaign_id = $1 ORDER BY created_at DESC LIMIT 200",
    )
    .bind(campaign)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, situation, ability, difficulty, created_at)| Ruling {
            id,
            situation,
            ability,
            difficulty,
            created_at,
        })
        .collect())
}

/// Something a reachable scene needs the table to know, that it does not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gap {
    /// The scene that needs it.
    pub node: String,
    pub node_title: String,
    pub revelation: String,
    /// The revelation, in the GM's words.
    pub statement: String,
    /// Clues not found yet that would give it, and where they are.
    pub clues: Vec<GapClue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GapClue {
    pub clue: String,
    pub node: String,
    pub node_title: String,
}

/// What the scenes reachable from the current one (its exits, and the
/// start scene when none is current) require that the table does not
/// know: flagged to the GM, never blocking.
#[must_use]
pub fn gaps(campaign: &Campaign, world: &WorldState) -> Vec<Gap> {
    let next: Vec<&str> = match world.current_node.as_deref().and_then(|n| campaign.node(n)) {
        Some(node) => node.exits.iter().map(|e| e.to.as_str()).collect(),
        None => campaign.bible.start_node.as_deref().into_iter().collect(),
    };
    let title = |id: &str| {
        campaign
            .node(id)
            .map(|n| n.title.clone())
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    for id in next {
        let Some(node) = campaign.node(id) else {
            continue;
        };
        for rev in &node.requires {
            if world.is_revelation_known(campaign, rev) {
                continue;
            }
            let statement = campaign
                .revelations
                .iter()
                .find(|r| &r.id == rev)
                .map(|r| r.statement.clone())
                .unwrap_or_default();
            out.push(Gap {
                node: node.id.clone(),
                node_title: node.title.clone(),
                revelation: rev.clone(),
                statement,
                clues: campaign
                    .clues_for(rev)
                    .filter(|c| !world.found_clues.contains(&c.id))
                    .map(|c| GapClue {
                        clue: c.id.clone(),
                        node: c.node.clone(),
                        node_title: title(&c.node),
                    })
                    .collect(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_situation_in_other_words_is_found_and_another_is_not() {
        let past = keywords("Escalader le grand mât");
        assert!(likeness(&keywords("j'escalade le mât"), &past) >= 0.99);
        assert!(likeness(&keywords("Escalader les mâts"), &past) >= 0.99);
        assert!(likeness(&keywords("Escalader le mur de l'entrepôt"), &past) < 0.5);
        assert_eq!(likeness(&keywords("Crocheter la serrure"), &past), 0.0);
    }

    #[test]
    fn keywords_fold_accents_and_drop_small_words() {
        assert_eq!(
            keywords("Crocheter la serrure de l'Entrepôt"),
            ["crochet", "entrepot", "serrur"]
        );
    }
}
