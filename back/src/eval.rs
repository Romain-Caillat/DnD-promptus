//! ai/evaluate-on-real-campaigns — a prompt or a model that changes must
//! not quietly break what worked.
//!
//! The cases (`content/evals/*.yaml`) come from the two games: real
//! situations of the Corsaires' act 1 replayed to the co-GM, and a
//! campaign generated from a pitch held to the scene standard. Each case
//! lists written criteria, each one a check a machine can make:
//!
//! | Check | Passes when |
//! |-------|-------------|
//! | `speaks: <npc>` | that NPC has a line |
//! | `never: [words]` | none of the words reaches the players (narration, NPC lines) |
//! | `mentions: [words]` | one of the words is in the answer |
//! | `suggests: <action>` | one suggestion carries that action |
//! | `no_invented_ids` | nothing the model named had to be dropped |
//! | `no_dice` | no dice formula reaches the players (rules are the engine's) |
//! | `french` | what reaches the players reads as French |
//! | `no_errors` | the generated campaign has no validator error |
//! | `three_clue_rule` | no three-clue alert in the generated campaign |
//! | `scene_standard` | every generated scene has the fields of the scene standard |
//!
//! A run calls the provider directly, without a database or a budget
//! (it is the developer's key and the developer's money), and is saved
//! as JSON (`evals/runs/`); its report compares it, case by case and
//! criterion by criterion, with the previous run (`bun run eval`).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::story::readiness::CheckKind;
use promptus_shared::story::{
    Campaign, Library, NodeStatus, Severity, WorldState, from_yaml, readiness,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ai::templates::Template;
use crate::ai::{self, BoxFuture, LlmRequest, LlmResponse, Provider, ledger};
use crate::copilot::{self, Action, Answer, Ask, ContextInput, RawAnswer};
use crate::error::AppError;
use crate::evening::knowledge::{JournalKind, JournalLine};
use crate::prep::generation::{self, Calls, Input, Length, Outcome, Step};

/// One case of the evaluation set.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub title: String,
    /// Where the situation comes from.
    pub origin: String,
    /// The campaign, relative to `content/`.
    pub campaign: String,
    #[serde(default)]
    pub copilot: Option<CopilotCase>,
    #[serde(default)]
    pub generation: Option<GenerationCase>,
    pub criteria: Vec<Criterion>,
}

/// A situation replayed to the co-GM.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopilotCase {
    /// The scene the table is in.
    pub scene: String,
    /// Scenes entered so far.
    #[serde(default)]
    pub entered: Vec<String>,
    /// Clues the table found.
    #[serde(default)]
    pub found: Vec<String>,
    /// NPCs and adversaries whose name the table knows.
    #[serde(default)]
    pub revealed: Vec<String>,
    /// The journal, oldest first: what the table knows.
    #[serde(default)]
    pub journal: Vec<String>,
    pub ask: Ask,
}

/// A campaign generated from a pitch, on the case campaign's rule system.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationCase {
    pub pitch: String,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub length: Length,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub id: String,
    /// Why it matters, in French, as the report says it.
    pub why: String,
    /// `check: { never: [...] }`, or `check: french` for a check
    /// without a value.
    #[serde(with = "serde_yaml_ng::with::singleton_map")]
    pub check: Check,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    Speaks(String),
    Never(Vec<String>),
    Mentions(Vec<String>),
    Suggests(Action),
    NoInventedIds,
    NoDice,
    French,
    NoErrors,
    ThreeClueRule,
    SceneStandard,
}

/// What a criterion gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub id: String,
    pub why: String,
    pub passed: bool,
    /// What was seen, when it failed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// What a case gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseResult {
    pub id: String,
    pub title: String,
    pub verdicts: Vec<Verdict>,
    /// The call failed (unreachable, out of format…): every criterion
    /// fails with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub cost_micros: i64,
}

impl CaseResult {
    #[must_use]
    pub fn passed(&self) -> usize {
        self.verdicts.iter().filter(|v| v.passed).count()
    }
}

/// A whole run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub at: DateTime<Utc>,
    pub provider: String,
    /// The models that answered.
    pub models: Vec<String>,
    /// The templates used, as `id@version`.
    pub prompts: Vec<String>,
    pub cases: Vec<CaseResult>,
}

/// Every case of `dir`, sorted by id.
///
/// # Errors
///
/// The file that does not read, and why.
pub fn load_cases(dir: &Path) -> Result<Vec<Case>, String> {
    let mut cases = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let case: Case =
            serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        cases.push(case);
    }
    cases.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(cases)
}

/// What a run records of each call.
#[derive(Default)]
struct Tally {
    cost_micros: i64,
    models: BTreeSet<String>,
}

/// The provider alone, counted for the report.
struct Direct<'a> {
    provider: &'a dyn Provider,
    tally: Mutex<Tally>,
}

impl Direct<'_> {
    async fn call(&self, req: &LlmRequest) -> Result<LlmResponse, AppError> {
        let answer = self
            .provider
            .complete(req)
            .await
            .map_err(|e| ledger::app_error(&e))?;
        let mut t = self
            .tally
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        t.cost_micros += answer.usage.cost_micros.max(0);
        t.models.insert(answer.model.clone());
        Ok(answer)
    }
}

impl Calls for Direct<'_> {
    fn complete<'a>(
        &'a self,
        _template: &'a Template,
        req: &'a LlmRequest,
    ) -> BoxFuture<'a, Result<LlmResponse, AppError>> {
        Box::pin(self.call(req))
    }

    fn progress<'a>(&'a self, _steps: &'a [Step]) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async { Ok(()) })
    }
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

fn contains_any(text: &str, words: &[String]) -> Option<String> {
    let text = lower(text);
    words.iter().find(|w| text.contains(&lower(w))).cloned()
}

/// A dice formula: digits, `d`, digits (`1d20`, `2d6+1`), or `d20`.
fn has_dice(text: &str) -> bool {
    let chars: Vec<char> = lower(text).chars().collect();
    chars.iter().enumerate().any(|(i, c)| {
        *c == 'd'
            && chars.get(i + 1).is_some_and(char::is_ascii_digit)
            && (i == 0 || !chars[i - 1].is_alphabetic())
    })
}

const FRENCH_WORDS: [&str; 10] = [
    "le", "la", "les", "de", "des", "et", "un", "une", "vous", "est",
];

fn reads_french(text: &str) -> bool {
    let words: BTreeSet<String> = lower(text)
        .split(|c: char| !c.is_alphabetic())
        .map(str::to_string)
        .collect();
    FRENCH_WORDS.iter().filter(|w| words.contains(**w)).count() >= 3
}

fn verdict(c: &Criterion, passed: bool, note: impl Into<String>) -> Verdict {
    Verdict {
        id: c.id.clone(),
        why: c.why.clone(),
        passed,
        note: if passed { String::new() } else { note.into() },
    }
}

/// The co-GM's answer against the criteria.
fn judge_copilot(criteria: &[Criterion], raw: &RawAnswer, answer: &Answer) -> Vec<Verdict> {
    let heard: String = std::iter::once(answer.narration.clone())
        .chain(answer.npc_lines.iter().map(|l| l.text.clone()))
        .collect::<Vec<_>>()
        .join("\n");
    let all: String = std::iter::once(heard.clone())
        .chain(
            answer
                .suggestions
                .iter()
                .map(|s| format!("{} {}", s.label, s.why.clone().unwrap_or_default())),
        )
        .chain(answer.gm_note.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let named = raw
        .suggestions
        .iter()
        .filter(|s| s.action.is_some())
        .count();
    let kept = answer
        .suggestions
        .iter()
        .filter(|s| s.action.is_some())
        .count();
    criteria
        .iter()
        .map(|c| match &c.check {
            Check::Speaks(npc) => {
                let speakers: Vec<&str> = answer
                    .npc_lines
                    .iter()
                    .map(|l| l.speaker.as_str())
                    .collect();
                verdict(
                    c,
                    answer
                        .npc_lines
                        .iter()
                        .any(|l| l.npc.as_deref() == Some(npc)),
                    format!("répliques de : {}", speakers.join(", ")),
                )
            }
            Check::Never(words) => {
                let hit = contains_any(&heard, words);
                verdict(
                    c,
                    hit.is_none(),
                    format!("« {} » dit aux joueurs", hit.unwrap_or_default()),
                )
            }
            Check::Mentions(words) => verdict(
                c,
                contains_any(&all, words).is_some(),
                "aucun des mots attendus",
            ),
            Check::Suggests(action) => verdict(
                c,
                answer
                    .suggestions
                    .iter()
                    .any(|s| s.action.as_ref() == Some(action)),
                "suggestion absente",
            ),
            Check::NoInventedIds => verdict(
                c,
                named == kept,
                format!("{} action(s) sur un identifiant inexistant", named - kept),
            ),
            Check::NoDice => verdict(c, !has_dice(&heard), "une formule de dés aux joueurs"),
            Check::French => verdict(
                c,
                reads_french(&heard),
                "le texte ne se lit pas en français",
            ),
            Check::NoErrors | Check::ThreeClueRule | Check::SceneStandard => {
                verdict(c, false, "critère de génération sur un cas du co-MJ")
            }
        })
        .collect()
}

/// A generated campaign against the criteria.
fn judge_generation(
    criteria: &[Criterion],
    out: &Outcome,
    rules: Option<&RuleSystem>,
) -> Vec<Verdict> {
    let errors = out
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let three = out
        .issues
        .iter()
        .filter(|i| i.code == "THREE_CLUE_RULE")
        .count();
    let gaps: Vec<String> = readiness(&out.story, &Library { rules, maps: None })
        .iter()
        .flat_map(|a| a.checks.iter())
        .filter(|c| c.kind == CheckKind::SceneFields)
        .flat_map(|c| c.gaps.iter())
        .map(|g| format!("{} ({})", g.id, g.fields.join(", ")))
        .collect();
    criteria
        .iter()
        .map(|c| match &c.check {
            Check::NoErrors => verdict(c, errors == 0, format!("{errors} erreur(s)")),
            Check::ThreeClueRule => verdict(c, three == 0, format!("{three} alerte(s)")),
            Check::SceneStandard => verdict(c, gaps.is_empty(), gaps.join(" ; ")),
            Check::NoInventedIds => verdict(
                c,
                out.removed.is_empty() && out.dropped == 0,
                format!(
                    "{} écarté(s), {} correction(s) refusée(s)",
                    out.removed.len(),
                    out.dropped
                ),
            ),
            _ => verdict(c, false, "critère du co-MJ sur un cas de génération"),
        })
        .collect()
}

fn failed(case: &Case, error: String, cost_micros: i64) -> CaseResult {
    CaseResult {
        id: case.id.clone(),
        title: case.title.clone(),
        verdicts: case
            .criteria
            .iter()
            .map(|c| verdict(c, false, error.clone()))
            .collect(),
        error: Some(error),
        cost_micros,
    }
}

fn app_error_text(e: &AppError) -> String {
    match e {
        AppError::Upstream { code, detail } => format!("{code} : {detail}"),
        other => format!("{other:?}"),
    }
}

/// Run one case. `content` is the `content/` directory.
pub async fn run_case(
    provider: &dyn Provider,
    case: &Case,
    content: &Path,
) -> (CaseResult, BTreeSet<String>) {
    let direct = Direct {
        provider,
        tally: Mutex::new(Tally::default()),
    };
    let result = run_case_with(&direct, case, content).await;
    let tally = direct
        .tally
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let result = match result {
        Ok(verdicts) => CaseResult {
            id: case.id.clone(),
            title: case.title.clone(),
            verdicts,
            error: None,
            cost_micros: tally.cost_micros,
        },
        Err(e) => failed(case, e, tally.cost_micros),
    };
    (result, tally.models)
}

async fn run_case_with(
    direct: &Direct<'_>,
    case: &Case,
    content: &Path,
) -> Result<Vec<Verdict>, String> {
    let path = content.join(&case.campaign);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let campaign = from_yaml(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let rules = crate::content::preset(&campaign.rules).map(AsRef::as_ref);
    match (&case.copilot, &case.generation) {
        (Some(c), None) => copilot_case(direct, case, c, &campaign, rules).await,
        (None, Some(g)) => generation_case(direct, case, g, &campaign, rules).await,
        _ => Err("un cas est soit `copilot`, soit `generation`".into()),
    }
}

async fn copilot_case(
    direct: &Direct<'_>,
    case: &Case,
    c: &CopilotCase,
    campaign: &Campaign,
    rules: Option<&RuleSystem>,
) -> Result<Vec<Verdict>, String> {
    let world = WorldState {
        current_node: Some(c.scene.clone()),
        node_status: c
            .entered
            .iter()
            .map(|n| (n.clone(), NodeStatus::Visited))
            .collect(),
        found_clues: c.found.iter().cloned().collect(),
        revealed: c.revealed.iter().cloned().collect(),
        ..WorldState::default()
    };
    let journal: Vec<JournalLine> = c
        .journal
        .iter()
        .map(|text| JournalLine {
            id: Uuid::nil(),
            session_id: None,
            kind: JournalKind::Note,
            r#ref: None,
            text: text.clone(),
            shared: true,
            created_at: Utc::now(),
        })
        .collect();
    let messages = copilot::messages(
        &ContextInput {
            campaign,
            world: &world,
            journal: &journal,
            recaps: &[],
            pending: &[],
            rulings: &[],
        },
        rules,
        &c.ask,
    )?;
    let reply = direct
        .call(&LlmRequest::json(messages))
        .await
        .map_err(|e| app_error_text(&e))?;
    let raw: RawAnswer = ai::parse_json(&reply.text).map_err(|e| e.to_string())?;
    let answer = copilot::sanitize(raw.clone(), campaign);
    Ok(judge_copilot(&case.criteria, &raw, &answer))
}

async fn generation_case(
    direct: &Direct<'_>,
    case: &Case,
    g: &GenerationCase,
    campaign: &Campaign,
    rules: Option<&RuleSystem>,
) -> Result<Vec<Verdict>, String> {
    let base = Campaign::empty(
        &case.id,
        &case.title,
        &campaign.world,
        campaign.rules.clone(),
    );
    let input = Input {
        pitch: g.pitch.clone(),
        tone: g.tone.clone(),
        themes: String::new(),
        constraints: String::new(),
        length: g.length,
    };
    let mut steps = generation::new_steps();
    let out = generation::generate(direct, &base, rules, 6, &input, &mut steps)
        .await
        .map_err(|f| format!("{} {}", f.code, f.detail.unwrap_or_default()))?;
    Ok(judge_generation(&case.criteria, &out, rules))
}

/// Run every case, one after the other.
pub async fn run(provider: &dyn Provider, cases: &[Case], content: &Path) -> Run {
    let mut results = Vec::new();
    let mut models = BTreeSet::new();
    for case in cases {
        let (r, m) = run_case(provider, case, content).await;
        models.extend(m);
        results.push(r);
    }
    Run {
        at: Utc::now(),
        provider: provider.name().to_string(),
        models: models.into_iter().collect(),
        prompts: crate::ai::templates::ALL
            .iter()
            .map(Template::key)
            .collect(),
        cases: results,
    }
}

/// How a case moved since the previous run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    New,
    Better,
    Worse,
    Same,
}

/// One case compared with the previous run: its trend, and the criteria
/// that changed (`+id` now passes, `-id` now fails).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    pub id: String,
    pub trend: Trend,
    pub changed: Vec<String>,
}

/// `current` against `previous`, case by case.
#[must_use]
pub fn compare(previous: Option<&Run>, current: &Run) -> Vec<Delta> {
    current
        .cases
        .iter()
        .map(|case| {
            let Some(before) = previous.and_then(|p| p.cases.iter().find(|c| c.id == case.id))
            else {
                return Delta {
                    id: case.id.clone(),
                    trend: Trend::New,
                    changed: Vec::new(),
                };
            };
            let mut changed = Vec::new();
            for v in &case.verdicts {
                let was = before
                    .verdicts
                    .iter()
                    .find(|b| b.id == v.id)
                    .map(|b| b.passed);
                match (was, v.passed) {
                    (Some(false) | None, true) => changed.push(format!("+{}", v.id)),
                    (Some(true), false) => changed.push(format!("-{}", v.id)),
                    _ => {}
                }
            }
            let trend = match case.passed().cmp(&before.passed()) {
                std::cmp::Ordering::Greater => Trend::Better,
                std::cmp::Ordering::Less => Trend::Worse,
                std::cmp::Ordering::Equal if changed.is_empty() => Trend::Same,
                // As many pass, not the same ones: a regression to read.
                std::cmp::Ordering::Equal => Trend::Worse,
            };
            Delta {
                id: case.id.clone(),
                trend,
                changed,
            }
        })
        .collect()
}

fn dollars(micros: i64) -> String {
    format!("{:.3}", micros as f64 / 1_000_000.0).replace('.', ",")
}

/// The report, in French, as Markdown.
#[must_use]
pub fn report(previous: Option<&Run>, current: &Run) -> String {
    let deltas = compare(previous, current);
    let mut out = format!(
        "# Évaluation du {}\n\nFournisseur : {} · modèles : {} · gabarits : {}\n",
        current.at.format("%d/%m/%Y %H:%M"),
        current.provider,
        if current.models.is_empty() {
            "aucun".to_string()
        } else {
            current.models.join(", ")
        },
        current.prompts.join(", "),
    );
    match previous {
        Some(p) => out.push_str(&format!(
            "Comparée à l'évaluation du {} ({}).\n",
            p.at.format("%d/%m/%Y %H:%M"),
            p.models.join(", ")
        )),
        None => out.push_str("Première évaluation : rien à comparer.\n"),
    }
    let passed: usize = current.cases.iter().map(CaseResult::passed).sum();
    let total: usize = current.cases.iter().map(|c| c.verdicts.len()).sum();
    let cost: i64 = current.cases.iter().map(|c| c.cost_micros).sum();
    out.push_str(&format!(
        "\n**{passed} critères sur {total}** · coût {} $\n",
        dollars(cost)
    ));
    for (case, delta) in current.cases.iter().zip(&deltas) {
        let trend = match delta.trend {
            Trend::New => "nouveau",
            Trend::Better => "en progrès",
            Trend::Worse => "en recul",
            Trend::Same => "inchangé",
        };
        out.push_str(&format!(
            "\n## {} — {}/{} · {trend}\n\n",
            case.title,
            case.passed(),
            case.verdicts.len()
        ));
        if let Some(e) = &case.error {
            out.push_str(&format!("Échec de l'appel : {e}\n\n"));
        }
        for v in &case.verdicts {
            let moved = if delta.changed.contains(&format!("+{}", v.id)) {
                " (corrigé)"
            } else if delta.changed.contains(&format!("-{}", v.id)) {
                " (cassé)"
            } else {
                ""
            };
            let mark = if v.passed { "✓" } else { "✗" };
            out.push_str(&format!("- {mark} {}{moved}", v.why));
            if !v.note.is_empty() && case.error.is_none() {
                out.push_str(&format!(" — {}", v.note));
            }
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fake::FakeProvider;

    fn content() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../content")
    }

    #[test]
    fn every_case_reads() {
        let cases = load_cases(&content().join("evals")).unwrap();
        assert!(cases.len() >= 4, "{cases:?}");
        for c in &cases {
            assert!(
                c.copilot.is_some() != c.generation.is_some(),
                "{}: copilot or generation",
                c.id
            );
            assert!(!c.criteria.is_empty(), "{}", c.id);
        }
    }

    #[tokio::test]
    async fn a_run_with_the_fake_provider_scores_every_case() {
        let cases = load_cases(&content().join("evals")).unwrap();
        let run = run(&FakeProvider::default(), &cases, &content()).await;
        assert_eq!(run.cases.len(), cases.len());
        for c in &run.cases {
            assert!(c.error.is_none(), "{}: {:?}", c.id, c.error);
        }
        // The fake always adds an action on an invented clue, and a
        // generated campaign with invented ids: the criterion bites.
        let vaubernier = run
            .cases
            .iter()
            .find(|c| c.id == "refuser-offre-vaubernier")
            .unwrap();
        let invented = vaubernier
            .verdicts
            .iter()
            .find(|v| v.id == "aucun_id_invente")
            .unwrap();
        assert!(!invented.passed);
        let generated = run
            .cases
            .iter()
            .find(|c| c.id == "generer-un-acte-corsaire")
            .unwrap();
        let passed: Vec<&str> = generated
            .verdicts
            .iter()
            .filter(|v| v.passed)
            .map(|v| v.id.as_str())
            .collect();
        assert_eq!(
            passed,
            ["aucune_erreur", "trois_indices", "standard_de_scene"]
        );
        assert!(report(None, &run).contains("Première évaluation"));
    }

    fn result(id: &str, verdicts: &[(&str, bool)]) -> CaseResult {
        CaseResult {
            id: id.into(),
            title: id.into(),
            verdicts: verdicts
                .iter()
                .map(|(v, passed)| Verdict {
                    id: (*v).into(),
                    why: format!("Pourquoi {v}"),
                    passed: *passed,
                    note: String::new(),
                })
                .collect(),
            error: None,
            cost_micros: 1_000,
        }
    }

    fn a_run(cases: Vec<CaseResult>, model: &str) -> Run {
        Run {
            at: Utc::now(),
            provider: "openrouter".into(),
            models: vec![model.into()],
            prompts: vec!["copilot@1".into()],
            cases,
        }
    }

    #[test]
    fn a_new_model_is_compared_case_by_case() {
        let before = a_run(
            vec![
                result("a", &[("x", true), ("y", false)]),
                result("b", &[("x", true), ("y", true)]),
                result("c", &[("x", true), ("y", false)]),
            ],
            "ancien",
        );
        let after = a_run(
            vec![
                result("a", &[("x", true), ("y", true)]),
                result("b", &[("x", false), ("y", true)]),
                result("c", &[("x", false), ("y", true)]),
                result("d", &[("x", true)]),
            ],
            "nouveau",
        );
        let deltas = compare(Some(&before), &after);
        let trends: Vec<(&str, Trend)> = deltas.iter().map(|d| (d.id.as_str(), d.trend)).collect();
        assert_eq!(
            trends,
            [
                ("a", Trend::Better),
                ("b", Trend::Worse),
                ("c", Trend::Worse),
                ("d", Trend::New)
            ]
        );
        assert_eq!(deltas[2].changed, ["-x", "+y"]);
        let text = report(Some(&before), &after);
        assert!(text.contains("## a — 2/2 · en progrès"), "{text}");
        assert!(text.contains("- ✗ Pourquoi x (cassé)"), "{text}");
        assert!(text.contains("- ✓ Pourquoi y (corrigé)"), "{text}");
    }

    #[test]
    fn dice_and_french_are_recognised() {
        assert!(has_dice("Lancez 1d20 + 3"));
        assert!(has_dice("un d6 de dégâts"));
        assert!(!has_dice("Le vieux Jacquot titube, d'un pas lourd."));
        assert!(reads_french("Le vent tombe et la brume couvre les quais."));
        assert!(!reads_french("The wind drops and fog covers the docks."));
    }
}
