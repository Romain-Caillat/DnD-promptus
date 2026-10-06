//! The co-GM beside the character sheets:
//!
//! - `copilot/check-character-sheets` — for a submitted sheet, a word
//!   to the player drawn from the rule checks ([`draft_note`]). The GM
//!   reads it, edits it, and decides: nothing reaches the player until
//!   the GM returns the sheet with it.
//! - `copilot/co-write-backstory` — the player's three answers made a
//!   paragraph ([`write_backstory`]), with no name the player did not
//!   write; and, for the GM, secret hooks drawn from that story
//!   ([`propose_hooks`]), tied only to scenes and fronts that exist.
//!   Nothing is stored: the player saves the paragraph in their draft,
//!   the GM keeps the hooks they want through `review::add_hook`.
//!
//! Every call is counted against the campaign's AI budget.

use std::collections::BTreeMap;

use promptus_shared::issue::Severity;
use promptus_shared::rules::RuleSystem;
use promptus_shared::story::{Campaign, to_yaml};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::review::{self, HOOK_BODY_MAX, HOOK_TITLE_MAX, NOTE_MAX};
use super::{
    ANSWER_MAX, Backstory, CharacterSheet, CharacterStatus, Player, TEXT_MAX, character_of,
};
use crate::ai::{self, Ai, AiError, LlmRequest, Message, Role, ledger, templates};
use crate::campaigns::CampaignRow;
use crate::error::AppError;

fn invalid(why: &str) -> AppError {
    ledger::app_error(&AiError::Schema(why.to_string()))
}

/// At most `max` characters of `s`, trimmed.
fn bounded(s: &str, max: usize) -> String {
    s.trim()
        .chars()
        .take(max)
        .collect::<String>()
        .trim()
        .to_string()
}

fn people_name(rules: Option<&RuleSystem>, id: Option<&str>) -> Option<String> {
    let id = id?;
    rules?
        .peoples
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.name.clone())
}

/// Who the character is, in a few lines: name, people, class.
fn character_lines(rules: Option<&RuleSystem>, sheet: &CharacterSheet) -> String {
    let mut out = format!(
        "Nom : {}\n",
        if sheet.name.is_empty() {
            "(pas encore de nom)"
        } else {
            &sheet.name
        }
    );
    if let Some(p) = people_name(rules, sheet.people_id.as_deref()) {
        out.push_str(&format!("Peuple : {p}\n"));
    }
    if let Some(c) = review::class_name(rules, sheet.class_id.as_deref()) {
        out.push_str(&format!("Classe : {c}\n"));
    }
    out
}

/// The story as the player wrote it: the paragraph, else the answers.
fn story_lines(b: &Backstory) -> String {
    let mut out = String::new();
    for (q, a) in [
        ("D’où vient-il ?", &b.origin),
        ("Qui a-t-il perdu ?", &b.loss),
        ("Que cherche-t-il ?", &b.quest),
    ] {
        if !a.trim().is_empty() {
            out.push_str(&format!("- {q} {}\n", a.trim()));
        }
    }
    if !b.text.trim().is_empty() {
        out.push_str(&format!("Paragraphe : {}\n", b.text.trim()));
    }
    out
}

async fn complete_json<T: for<'de> Deserialize<'de>>(
    pool: &PgPool,
    ai: &Ai,
    campaign: Uuid,
    purpose: &str,
    template: &templates::Template,
    messages: Vec<Message>,
    max_tokens: u32,
) -> Result<T, AppError> {
    let mut req = LlmRequest::json(messages);
    req.max_tokens = max_tokens;
    let reply = ai.complete(pool, campaign, purpose, template, &req).await?;
    ai::parse_json(&reply.text).map_err(|e| ledger::app_error(&e))
}

// --- copilot/check-character-sheets ------------------------------------------

#[derive(Deserialize)]
struct RawNote {
    #[serde(default)]
    note: String,
}

/// The co-GM's word to the player who sent character `id`, drawn from
/// the rule checks. Not stored: the GM edits it and returns the sheet
/// with it, or validates the sheet anyway.
///
/// # Errors
///
/// 404 `NOT_FOUND`; 409 `CHARACTER_NOT_SUBMITTED`, `AI_BUDGET_EXCEEDED`;
/// 503 `AI_NOT_CONFIGURED`; 502 `AI_UNAVAILABLE`, `AI_OUTPUT_INVALID`.
pub async fn draft_note(
    pool: &PgPool,
    ai: &Ai,
    campaign: &CampaignRow,
    id: Uuid,
) -> Result<String, AppError> {
    let r = review::review(pool, campaign, id).await?;
    if r.status != CharacterStatus::Submitted {
        return Err(AppError::Conflict("CHARACTER_NOT_SUBMITTED"));
    }
    let rules = campaign.rules();
    let sheet: CharacterSheet = serde_json::from_value(r.sheet.clone()).unwrap_or_default();
    let mut lines = format!("Joueur : {}\n", r.nickname);
    lines.push_str(&character_lines(rules, &sheet));
    if !sheet.abilities.is_empty() {
        let named: Vec<String> = sheet
            .abilities
            .iter()
            .map(|(k, v)| {
                let name = r
                    .abilities
                    .iter()
                    .find(|a| &a.id == k)
                    .map_or(k.as_str(), |a| a.name.as_str());
                format!("{name} {v}")
            })
            .collect();
        lines.push_str(&format!("Caractéristiques : {}\n", named.join(", ")));
    }
    let story = story_lines(&sheet.backstory);
    if !story.is_empty() {
        lines.push_str(&format!("Histoire :\n{story}"));
    }
    let mut checks = String::new();
    for i in &r.checks {
        let level = match i.severity {
            Severity::Error => "dépasse les règles",
            Severity::Warning => "à signaler",
            Severity::Info => "pour info",
        };
        checks.push_str(&format!(
            "- {level} : {}\n",
            i.message.as_deref().unwrap_or(&i.detail)
        ));
    }
    if checks.is_empty() {
        checks.push_str("Rien à redire.\n");
    }
    let s = &campaign.story;
    let vars: BTreeMap<&str, String> = [
        (
            "campaign",
            format!(
                "{}{}{}",
                s.title,
                if s.world.is_empty() { "" } else { " · " },
                s.world
            ),
        ),
        ("sheet", lines),
        ("checks", checks),
    ]
    .into_iter()
    .collect();
    let messages = templates::SHEET_NOTE
        .render(&vars)
        .map_err(|e| AppError::internal("sheet-note template", e))?;
    let raw: RawNote = complete_json(
        pool,
        ai,
        campaign.id,
        "sheet.note",
        &templates::SHEET_NOTE,
        messages,
        800,
    )
    .await?;
    let note = bounded(&raw.note, NOTE_MAX);
    if note.is_empty() {
        return Err(invalid("le mot au joueur est vide"));
    }
    Ok(note)
}

// --- copilot/co-write-backstory: the player's paragraph -----------------------

/// The three answers the player asks the co-GM to write up.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Answers {
    pub origin: String,
    pub loss: String,
    pub quest: String,
}

#[derive(Deserialize)]
struct RawText {
    #[serde(default)]
    text: String,
}

/// Lowercase, without accents' trouble: the words of `s`, for matching.
fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The capitalised words of `text` that do not start a sentence (or a
/// quotation) and that `known` never uses: names the model made up.
fn invented_names(text: &str, known: &str) -> Vec<String> {
    let known = words(known);
    let mut out: Vec<String> = Vec::new();
    let mut sentence_start = true;
    for token in text.split_whitespace() {
        let word: String = token
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '-')
            .to_string();
        let opens = token.starts_with('«') || token.starts_with('"') || token.starts_with('“');
        if !word.is_empty() {
            let capital = word.chars().next().is_some_and(char::is_uppercase);
            if capital && !sentence_start && !opens {
                let lower = word.to_lowercase();
                // « Mont-Rouge » is known when the player wrote it whole.
                if !known.contains(&lower) && !out.contains(&word) {
                    out.push(word);
                }
            }
            sentence_start = false;
        }
        if token.ends_with(['.', '!', '?', '…', ':']) || token == "«" || token == "—" {
            sentence_start = true;
        }
    }
    out
}

/// The player's three answers made a paragraph, with no name they did
/// not write: a paragraph naming someone or somewhere new is sent back
/// once with the names to drop, then refused. Allowed while the sheet is
/// the player's to edit; the player saves it in their draft.
///
/// # Errors
///
/// 404 `NO_CHARACTER`; 409 `CHARACTER_LOCKED`, `AI_BUDGET_EXCEEDED`; 400
/// `EMPTY_TEXT` (no answer), `TEXT_TOO_LONG`; 503 `AI_NOT_CONFIGURED`;
/// 502 `AI_UNAVAILABLE`, `AI_OUTPUT_INVALID`.
pub async fn write_backstory(
    pool: &PgPool,
    ai: &Ai,
    campaign: &CampaignRow,
    player: &Player,
    answers: &Answers,
) -> Result<String, AppError> {
    let character = character_of(pool, player)
        .await?
        .ok_or(AppError::NotFound("NO_CHARACTER"))?;
    if !character.status.editable() {
        return Err(AppError::Conflict("CHARACTER_LOCKED"));
    }
    let given = [
        ("D’où vient-il ?", answers.origin.trim()),
        ("Qui a-t-il perdu ?", answers.loss.trim()),
        ("Que cherche-t-il ?", answers.quest.trim()),
    ];
    if given.iter().any(|(_, a)| a.chars().count() > ANSWER_MAX) {
        return Err(AppError::BadRequest("TEXT_TOO_LONG"));
    }
    if given.iter().all(|(_, a)| a.is_empty()) {
        return Err(AppError::BadRequest("EMPTY_TEXT"));
    }
    let rules = campaign.rules();
    let sheet = &character.sheet;
    let mut who = character_lines(rules, sheet);
    let s = &campaign.story;
    who.push_str(&format!("Campagne : {}", s.title));
    if !s.world.is_empty() {
        who.push_str(&format!(" · {}", s.world));
    }
    who.push('\n');
    let answer_lines: String = given
        .iter()
        .map(|(q, a)| {
            format!(
                "- {q} {}\n",
                if a.is_empty() { "(sans réponse)" } else { a }
            )
        })
        .collect();
    // What the paragraph may name: the player's words and who the
    // character is.
    let known = format!("{who}\n{answer_lines}");
    let vars: BTreeMap<&str, String> = [("character", who), ("answers", answer_lines)]
        .into_iter()
        .collect();
    let mut messages = templates::BACKSTORY
        .render(&vars)
        .map_err(|e| AppError::internal("backstory template", e))?;
    for attempt in 0..2 {
        let raw: RawText = complete_json(
            pool,
            ai,
            campaign.id,
            "backstory.write",
            &templates::BACKSTORY,
            messages.clone(),
            700,
        )
        .await?;
        let text = bounded(&raw.text, TEXT_MAX);
        if text.is_empty() {
            return Err(invalid("le paragraphe est vide"));
        }
        let invented = invented_names(&text, &known);
        if invented.is_empty() {
            return Ok(text);
        }
        if attempt == 0 {
            messages.push(Message {
                role: Role::Assistant,
                content: serde_json::json!({ "text": text }).to_string(),
            });
            messages.push(Message::user(format!(
                "Ces noms ne viennent pas du joueur : {}. Réécris le paragraphe sans eux, \
                 avec seulement ce qu’il a écrit.",
                invented.join(", ")
            )));
        }
    }
    Err(invalid(
        "le paragraphe invente des noms que le joueur n’a pas écrits",
    ))
}

// --- copilot/co-write-backstory: the GM's hooks -------------------------------

/// One hook the co-GM proposes; the GM keeps it with `review::add_hook`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookProposal {
    pub title: String,
    pub body: String,
    pub links: Vec<String>,
}

/// The hooks proposed, and how many links or hooks were dropped.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookProposals {
    pub hooks: Vec<HookProposal>,
    pub dropped: usize,
}

#[derive(Deserialize)]
struct RawHooks {
    #[serde(default)]
    hooks: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawHook {
    #[serde(default)]
    title: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    links: Vec<String>,
}

/// The co-GM's hooks as the GM sees them: at most three, bounded, their
/// links only to scenes and fronts of the story. A link to nothing is
/// dropped and counted; a hook without a title or that cannot be read
/// too.
fn sanitize_hooks(story: &Campaign, raw: Vec<serde_json::Value>) -> HookProposals {
    let mut dropped = 0;
    let mut hooks = Vec::new();
    for v in raw {
        let Ok(h) = serde_json::from_value::<RawHook>(v) else {
            dropped += 1;
            continue;
        };
        let title = bounded(&h.title, HOOK_TITLE_MAX);
        if title.is_empty() || hooks.len() == 3 {
            dropped += 1;
            continue;
        }
        let mut links: Vec<String> = Vec::new();
        for l in h.links {
            let known =
                story.nodes.iter().any(|n| n.id == l) || story.fronts.iter().any(|f| f.id == l);
            if !known {
                dropped += 1;
            } else if !links.contains(&l) {
                links.push(l);
            }
        }
        hooks.push(HookProposal {
            title,
            body: bounded(&h.body, HOOK_BODY_MAX),
            links,
        });
    }
    HookProposals { hooks, dropped }
}

/// Hooks drawn from character `id`'s backstory, for the GM to keep or
/// not. Nothing is stored.
///
/// # Errors
///
/// 404 `NOT_FOUND`; 400 `NO_BACKSTORY`; 409 `AI_BUDGET_EXCEEDED`; 503
/// `AI_NOT_CONFIGURED`; 502 `AI_UNAVAILABLE`, `AI_OUTPUT_INVALID`.
pub async fn propose_hooks(
    pool: &PgPool,
    ai: &Ai,
    campaign: &CampaignRow,
    id: Uuid,
) -> Result<HookProposals, AppError> {
    let r = review::review(pool, campaign, id).await?;
    let sheet: CharacterSheet = serde_json::from_value(r.sheet).unwrap_or_default();
    let story = story_lines(&sheet.backstory);
    if story.is_empty() {
        return Err(AppError::BadRequest("NO_BACKSTORY"));
    }
    let existing: String = review::hooks(pool, campaign.id)
        .await?
        .into_iter()
        .filter(|h| h.character_id == id)
        .map(|h| format!("- {} : {}\n", h.title, h.body))
        .collect();
    let targets: String = review::hook_targets(&campaign.story)
        .into_iter()
        .map(|t| {
            let kind = if t.kind == "front" { "front" } else { "scène" };
            format!("- `{}` ({kind}) {}\n", t.id, t.title)
        })
        .collect();
    let yaml = to_yaml(&campaign.story).map_err(|e| AppError::internal("hooks yaml", e))?;
    let vars: BTreeMap<&str, String> = [
        ("campaign", yaml.trim_end().to_string()),
        ("targets", targets),
        (
            "character",
            format!(
                "Joueur : {}\n{}",
                r.nickname,
                character_lines(campaign.rules(), &sheet)
            ),
        ),
        ("backstory", story),
        (
            "existing",
            if existing.is_empty() {
                "Aucune.\n".into()
            } else {
                existing
            },
        ),
    ]
    .into_iter()
    .collect();
    let messages = templates::HOOKS
        .render(&vars)
        .map_err(|e| AppError::internal("hooks template", e))?;
    let raw: RawHooks = complete_json(
        pool,
        ai,
        campaign.id,
        "hooks.propose",
        &templates::HOOKS,
        messages,
        1_500,
    )
    .await?;
    let proposals = sanitize_hooks(&campaign.story, raw.hooks);
    if proposals.hooks.is_empty() {
        return Err(invalid("aucune accroche lisible"));
    }
    Ok(proposals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_the_player_never_wrote_is_caught_but_not_a_sentence_start() {
        let known = "Nom : Borin\nPeuple : Nain\n- D’où vient-il ? De la mine d’argent de Valombre.\n- Qui a-t-il perdu ? Son frère Dorn.";
        let good = "Borin a travaillé vingt ans dans la mine de Valombre. Son frère Dorn y est descendu un soir.";
        assert!(invented_names(good, known).is_empty());
        let bad = "Borin a quitté Valombre avec Thrain. « Jamais », dit-il au baron Mordec.";
        assert_eq!(invented_names(bad, known), vec!["Thrain", "Mordec"]);
    }
}
