//! copilot/draft-narration — the co-GM, live.
//!
//! The LLM reads a compact context (the bible, the current scene, the
//! NPCs present with what they want and hide, the clues, the exits, the
//! fronts, what the table knows from the journal, the requests waiting)
//! and drafts a narration, NPC lines and suggestions. Everything it
//! writes is a **draft** (`MEMORY.md` §3): the GM edits it and shows it
//! with a gesture (`show`), or drops it. A suggestion's action keeps its
//! button only when every id it names exists in the campaign — invented
//! ids are dropped, the suggestion stays as text.
//!
//! Port of V1 `copilot.ts` (`tests/unit/copilot.test.ts`).

pub mod drafts;

use std::collections::BTreeMap;

use promptus_shared::rules::RuleSystem;
use promptus_shared::story::{Campaign, WorldState};
use serde::{Deserialize, Serialize};

use crate::ai::{Message, templates};
use crate::evening::knowledge::{JournalKind, JournalLine, Ruling};

/// What the GM asks the co-GM for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Describe the situation to the players.
    Describe,
    /// Make an NPC speak.
    Npc,
    /// What follows from what the players just did.
    Consequence,
    /// « Et ensuite ? »: relaunch a scene that stalls.
    Next,
    /// The GM's own question.
    Free,
}

impl Kind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Describe => "describe",
            Self::Npc => "npc",
            Self::Consequence => "consequence",
            Self::Next => "next",
            Self::Free => "free",
        }
    }

    /// # Errors
    ///
    /// An unknown kind.
    pub fn parse(s: &str) -> Result<Self, String> {
        Ok(match s {
            "describe" => Self::Describe,
            "npc" => Self::Npc,
            "consequence" => Self::Consequence,
            "next" => Self::Next,
            "free" => Self::Free,
            other => return Err(format!("copilot kind {other}")),
        })
    }
}

/// The GM's request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ask {
    pub kind: Kind,
    /// The NPC who speaks (`kind = npc`).
    #[serde(default)]
    pub npc: Option<String>,
    /// What the players say, the GM's question…
    #[serde(default)]
    pub prompt: String,
}

/// Everything the context is built from.
pub struct ContextInput<'a> {
    pub campaign: &'a Campaign,
    pub world: &'a WorldState,
    /// The whole journal, oldest first (GM lines included).
    pub journal: &'a [JournalLine],
    /// The GM's recaps of the ended sessions, oldest first.
    pub recaps: &'a [String],
    /// The requests waiting for the GM, as « Marc : Dextérité (« … ») ».
    pub pending: &'a [String],
    pub rulings: &'a [Ruling],
}

fn cut(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}…", s.chars().take(n).collect::<String>())
    } else {
        s.to_string()
    }
}

/// « - pnj id Name — role [veut : … ; voix : …] ».
fn npc_line(npc: &promptus_shared::story::Npc, role: &str, world: &WorldState) -> String {
    let mut extra = Vec::new();
    if !npc.wants.is_empty() {
        extra.push(format!("veut : {}", cut(&npc.wants, 200)));
    }
    if !npc.hides.is_empty() {
        extra.push(format!("cache : {}", cut(&npc.hides, 200)));
    }
    if !npc.roleplay.is_empty() {
        extra.push(format!("voix : {}", cut(&npc.roleplay, 160)));
    }
    if world.revealed.contains(&npc.id) {
        extra.push("connu des joueurs".into());
    }
    let role = if role.is_empty() {
        String::new()
    } else {
        format!(" — {}", cut(role, 120))
    };
    let extra = if extra.is_empty() {
        String::new()
    } else {
        format!(" [{}]", extra.join(" ; "))
    };
    format!("- pnj {} {}{role}{extra}", npc.id, npc.name)
}

/// How many journal lines the context carries: the most recent.
const JOURNAL_LINES: usize = 40;

/// The compact context: only what serves the scene being played.
#[must_use]
pub fn context(input: &ContextInput<'_>) -> String {
    let story = input.campaign;
    let world = input.world;
    let b = &story.bible;
    let mut l: Vec<String> = Vec::new();
    l.push(format!("# Campagne « {} »", story.title));
    l.push(format!("Pitch : {}", cut(&b.pitch, 600)));
    l.push(format!("Ton : {}", cut(&b.tone, 300)));
    if !b.truths.is_empty() {
        let t: Vec<String> = b.truths.iter().map(|t| cut(t, 200)).collect();
        l.push(format!("Vérités du monde : {}", t.join(" | ")));
    }
    if !b.secrets.is_empty() {
        let t: Vec<String> = b.secrets.iter().map(|t| cut(t, 200)).collect();
        l.push(format!("Secrets (MJ) : {}", t.join(" | ")));
    }
    if !input.recaps.is_empty() {
        l.push(String::new());
        l.push("# Sessions précédentes (résumés MJ)".into());
        for r in input.recaps.iter().filter(|r| !r.trim().is_empty()) {
            l.push(cut(r, 1_500));
        }
    }

    l.push(String::new());
    l.push("# Scène en cours".into());
    match world.current_node.as_deref().and_then(|id| story.node(id)) {
        Some(n) => {
            let status = match world.node_status.get(&n.id) {
                Some(promptus_shared::story::NodeStatus::Resolved) => "résolue",
                _ => "en cours",
            };
            l.push(format!("{} « {} » ({status})", n.id, n.title));
            l.push(format!("Résumé MJ : {}", cut(&n.summary, 600)));
            if !n.read_aloud.is_empty() {
                l.push(format!(
                    "Texte lu aux joueurs : {}",
                    cut(&n.read_aloud, 800)
                ));
            }
            if !n.flow.is_empty() {
                l.push(format!("Déroulé : {}", cut(&n.flow, 600)));
            }
            if !n.gm_notes.is_empty() {
                l.push(format!("Notes MJ : {}", cut(&n.gm_notes, 600)));
            }
            if let Some(loc) = n.location.as_deref() {
                l.push(format!("Lieu : {loc}"));
            }
            let npcs: Vec<_> = n
                .npcs
                .iter()
                .filter_map(|p| story.npc(&p.npc).map(|npc| (p, npc)))
                .collect();
            if !npcs.is_empty() {
                l.push("Présents :".into());
                for (p, npc) in npcs {
                    l.push(npc_line(npc, &p.role, world));
                }
            }
            let clues: Vec<_> = story.clues.iter().filter(|c| c.node == n.id).collect();
            if !clues.is_empty() {
                l.push("Indices de la scène :".into());
                for c in clues {
                    let found = if world.found_clues.contains(&c.id) {
                        "(trouvé)"
                    } else {
                        "(à trouver)"
                    };
                    l.push(format!(
                        "- indice {} {found} : {} — découverte : {}",
                        c.id,
                        cut(&c.text, 200),
                        cut(&c.discovery, 160)
                    ));
                }
            }
            if !n.exits.is_empty() {
                l.push("Sorties :".into());
                for x in &n.exits {
                    let title = story.node(&x.to).map_or("?", |t| t.title.as_str());
                    l.push(format!("- {} « {title} » : {}", x.to, x.label));
                }
            }
        }
        None => {
            l.push("Aucune scène en cours.".into());
            if let Some(start) = b.start_node.as_deref().and_then(|id| story.node(id)) {
                l.push(format!(
                    "Scène d’ouverture : {} « {} »",
                    start.id, start.title
                ));
            }
        }
    }

    // campaign/track-factions-and-goals: the party's companion speaks
    // in every scene.
    let along: Vec<_> = story.npcs.iter().filter(|n| n.permanent).collect();
    if !along.is_empty() {
        l.push(String::new());
        l.push("# Toujours là (accompagne le groupe dans chaque scène)".into());
        for npc in along {
            l.push(npc_line(npc, "", world));
        }
    }

    if !story.factions.is_empty() {
        l.push(String::new());
        l.push("# Factions (affinité du groupe)".into());
        for f in &story.factions {
            let at = world.affinity(story, &f.id).unwrap_or(f.affinity.start);
            let met = if world.met_factions.contains(&f.id) {
                "rencontrée"
            } else {
                "pas encore rencontrée"
            };
            let rivals: Vec<&str> = f
                .rivals
                .iter()
                .filter_map(|r| story.faction(r))
                .map(|r| r.name.as_str())
                .collect();
            let rivals = if rivals.is_empty() {
                String::new()
            } else {
                format!(" ; rivaux : {}", rivals.join(", "))
            };
            l.push(format!(
                "- faction {} {} : {at} ({}..{}), {met}{rivals}",
                f.id, f.name, f.affinity.min, f.affinity.max
            ));
        }
    }
    if !story.goals.is_empty() {
        l.push(String::new());
        l.push("# Objectifs de campagne".into());
        for g in &story.goals {
            let done = if world.goals_done.contains(&g.id) {
                "atteint"
            } else {
                "à atteindre"
            };
            l.push(format!("- objectif {} « {} » : {done}", g.id, g.title));
        }
    }

    if !story.fronts.is_empty() {
        l.push(String::new());
        l.push("# Menaces (fronts)".into());
        for f in &story.fronts {
            let at = world.front_progress.get(&f.id).copied().unwrap_or(0) as usize;
            let next = f.steps.get(at).map_or_else(
                || "catastrophe atteinte".to_string(),
                |s| format!("prochaine : {}", s.label),
            );
            l.push(format!(
                "- front {} « {} » (but : {}) : {at}/{} ; {next}",
                f.id,
                f.name,
                cut(&f.goal, 150),
                f.steps.len()
            ));
        }
    }

    if !story.revelations.is_empty() {
        let (known, hidden): (Vec<_>, Vec<_>) = story
            .revelations
            .iter()
            .partition(|r| world.is_revelation_known(story, &r.id));
        l.push(String::new());
        l.push("# Révélations".into());
        if !known.is_empty() {
            let k: Vec<String> = known.iter().map(|r| cut(&r.statement, 160)).collect();
            l.push(format!("Connues des joueurs : {}", k.join(" | ")));
        }
        if !hidden.is_empty() {
            let h: Vec<String> = hidden
                .iter()
                .map(|r| format!("{} {}", r.id, cut(&r.statement, 160)))
                .collect();
            l.push(format!("Encore cachées : {}", h.join(" | ")));
        }
    }

    let recent = input.journal.len().saturating_sub(JOURNAL_LINES);
    if input.journal.len() > recent {
        l.push(String::new());
        l.push("# Ce que la table sait (journal, du plus ancien au plus récent)".into());
        for j in &input.journal[recent..] {
            let who = if j.shared { "" } else { "(MJ) " };
            let kind = match j.kind {
                JournalKind::Promise => "promesse : ",
                JournalKind::Debt => "dette : ",
                JournalKind::Roll => "jet : ",
                _ => "",
            };
            l.push(format!("- {who}{kind}{}", cut(&j.text, 300)));
        }
    }
    if !input.rulings.is_empty() {
        l.push(String::new());
        l.push("# Décisions de règle déjà prises".into());
        for r in input.rulings.iter().take(10) {
            l.push(format!(
                "- {} → {} contre {}",
                cut(&r.situation, 160),
                r.ability,
                r.difficulty
            ));
        }
    }
    if !input.pending.is_empty() {
        l.push(String::new());
        l.push("# Demandes des joueurs en attente".into());
        for p in input.pending {
            l.push(format!("- {}", cut(p, 300)));
        }
    }
    l.join("\n")
}

/// The instruction for `ask`.
#[must_use]
pub fn instruction(ask: &Ask, campaign: &Campaign) -> String {
    let prompt = ask.prompt.trim();
    let extra = if prompt.is_empty() {
        String::new()
    } else {
        format!("\nPrécision du MJ : {prompt}")
    };
    match ask.kind {
        Kind::Describe => format!(
            "Décris la situation actuelle aux joueurs (narration), en tenant compte des derniers événements.{extra}"
        ),
        Kind::Npc => {
            let who = ask
                .npc
                .as_deref()
                .and_then(|id| campaign.npc(id))
                .map_or_else(
                    || "un PNJ présent".to_string(),
                    |n| format!("{} {}", n.id, n.name),
                );
            format!(
                "Donne 1 à 3 répliques de {who} en réponse à ce qui vient de se passer, selon ce qu’il veut et ce qu’il cache. Narration courte ou vide.{extra}"
            )
        }
        Kind::Consequence => format!(
            "Quelles sont les conséquences des dernières actions des joueurs ? Narration de ce qui se passe, réactions des PNJ, et suggestions (indice à révéler, menace qui avance, scène suivante).{extra}"
        ),
        Kind::Next => format!(
            "Les joueurs hésitent ou la scène s’essouffle. Propose comment relancer : pistes vers les indices manquants, menace qui se rapproche, scène suivante.{extra}"
        ),
        Kind::Free => {
            if prompt.is_empty() {
                "Aide le MJ pour la suite.".to_string()
            } else {
                format!("Question du MJ : {prompt}")
            }
        }
    }
}

/// The co-GM's messages for `ask`: the compact context, the rule system
/// (house rules included: the GM applies them at the table and the
/// co-GM must stay within them), the instruction.
///
/// # Errors
///
/// A template hole left unfilled.
pub fn messages(
    input: &ContextInput<'_>,
    rules: Option<&RuleSystem>,
    ask: &Ask,
) -> Result<Vec<Message>, String> {
    let rules_line = rules.map_or_else(
        || "Système de règles inconnu.".to_string(),
        |r| {
            let mut line = format!(
                "Système de règles : « {} » (test {}).",
                r.name, r.check.dice
            );
            for h in &r.house_rules {
                line.push_str(&format!("\nRègle maison « {} » : {}", h.name, h.text));
            }
            line
        },
    );
    let vars: BTreeMap<&str, String> = [
        ("context", context(input)),
        ("rules", rules_line),
        ("request", instruction(ask, input.campaign)),
    ]
    .into_iter()
    .collect();
    templates::COPILOT.render(&vars)
}

/// A suggestion's gesture, applied by the GM in one tap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    RevealClue { clue: String },
    AdvanceFront { front: String },
    EnterScene { node: String },
    RevealNpc { npc: String },
}

impl Action {
    fn exists_in(&self, c: &Campaign) -> bool {
        match self {
            Self::RevealClue { clue } => c.clue(clue).is_some(),
            Self::AdvanceFront { front } => c.front(front).is_some(),
            Self::EnterScene { node } => c.node(node).is_some(),
            Self::RevealNpc { npc } => c.npc(npc).is_some(),
        }
    }
}

/// The model's answer, as read: missing fields tolerated.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawAnswer {
    #[serde(default)]
    pub narration: String,
    #[serde(default)]
    pub npc_lines: Vec<RawNpcLine>,
    #[serde(default)]
    pub suggestions: Vec<RawSuggestion>,
    #[serde(default)]
    pub gm_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawNpcLine {
    #[serde(default)]
    pub npc: Option<String>,
    #[serde(default)]
    pub speaker: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSuggestion {
    pub label: String,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default)]
    pub action: Option<serde_json::Value>,
}

/// The answer the GM reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub narration: String,
    pub npc_lines: Vec<NpcLine>,
    pub suggestions: Vec<Suggestion>,
    pub gm_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcLine {
    pub npc: Option<String>,
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub label: String,
    pub why: Option<String>,
    /// `None` when the model named an id the campaign does not have, or
    /// an action that does not exist: the suggestion stays, no button.
    pub action: Option<Action>,
}

/// Validate the model's answer against the campaign.
#[must_use]
pub fn sanitize(raw: RawAnswer, campaign: &Campaign) -> Answer {
    Answer {
        narration: raw.narration.trim().to_string(),
        npc_lines: raw
            .npc_lines
            .into_iter()
            .filter(|l| !l.text.trim().is_empty())
            .map(|l| {
                let npc = l.npc.as_deref().and_then(|id| campaign.npc(id));
                NpcLine {
                    npc: npc.map(|n| n.id.clone()),
                    speaker: npc
                        .map(|n| n.name.clone())
                        .or(l.speaker)
                        .unwrap_or_else(|| "PNJ".to_string()),
                    text: l.text.trim().to_string(),
                }
            })
            .collect(),
        suggestions: raw
            .suggestions
            .into_iter()
            .take(5)
            .map(|s| Suggestion {
                label: s.label,
                why: s.why.filter(|w| !w.trim().is_empty()),
                action: s
                    .action
                    .and_then(|a| serde_json::from_value::<Action>(a).ok())
                    .filter(|a| a.exists_in(campaign)),
            })
            .collect(),
        gm_note: raw
            .gm_note
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use promptus_shared::story::from_yaml;

    const FIXTURE: &str = include_str!("../../../content/fixtures/phare-de-kerbrume.yaml");

    fn campaign() -> Campaign {
        from_yaml(FIXTURE).unwrap()
    }

    fn input<'a>(
        c: &'a Campaign,
        w: &'a WorldState,
        journal: &'a [JournalLine],
    ) -> ContextInput<'a> {
        ContextInput {
            campaign: c,
            world: w,
            journal,
            recaps: &[],
            pending: &[],
            rulings: &[],
        }
    }

    fn line(text: &str, shared: bool) -> JournalLine {
        JournalLine {
            id: uuid::Uuid::nil(),
            session_id: None,
            kind: JournalKind::Note,
            r#ref: None,
            text: text.into(),
            shared,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn the_context_holds_the_scene_its_clues_exits_fronts_and_the_journal() {
        let c = campaign();
        let start = c.bible.start_node.clone().unwrap();
        let mut w = WorldState::default();
        w.enter_node(&c, &start).unwrap();
        let journal = [line("Borin fouille le comptoir", true)];
        let pending = ["Marc : Dextérité (« la clé »)".to_string()];
        let ctx = context(&ContextInput {
            pending: &pending,
            ..input(&c, &w, &journal)
        });
        let node = c.node(&start).unwrap();
        assert!(
            ctx.contains(&format!("{} « {} »", node.id, node.title)),
            "{ctx}"
        );
        for cl in c.clues.iter().filter(|cl| cl.node == start) {
            assert!(ctx.contains(&format!("- indice {}", cl.id)), "{ctx}");
        }
        for x in &node.exits {
            assert!(ctx.contains(&x.to), "{ctx}");
        }
        for f in &c.fronts {
            assert!(ctx.contains(&format!("- front {}", f.id)), "{ctx}");
        }
        assert!(ctx.contains("Borin fouille le comptoir"));
        assert!(ctx.contains("Marc : Dextérité"));
        assert!(ctx.chars().count() < 12_000, "{}", ctx.chars().count());
    }

    #[test]
    fn the_context_holds_affinities_goals_and_the_companion_in_every_scene() {
        let mut c = campaign();
        // Corentin goes everywhere with the party, listed or not.
        let along = c.npcs[0].id.clone();
        c.npcs[0].permanent = true;
        let mut w = WorldState::default();
        let elsewhere = c
            .nodes
            .iter()
            .find(|n| !n.npcs.iter().any(|p| p.npc == along))
            .unwrap()
            .id
            .clone();
        w.enter_node(&c, &elsewhere).unwrap();
        w.shift_affinity(&c, "fac_douane", 2).unwrap();
        w.set_goal(&c, "but_lumiere", true).unwrap();
        let ctx = context(&input(&c, &w, &[]));
        assert!(ctx.contains("# Toujours là"), "{ctx}");
        assert!(ctx.contains(&format!("- pnj {along} ")), "{ctx}");
        assert!(
            ctx.contains("- faction fac_douane La douane royale : 2 (-5..5), rencontrée"),
            "{ctx}"
        );
        assert!(
            ctx.contains("- faction fac_contrebandiers Les naufrageurs : -4 (-5..5), pas encore rencontrée ; rivaux : La douane royale"),
            "{ctx}"
        );
        assert!(
            ctx.contains("- objectif but_lumiere « Rallumer le phare » : atteint"),
            "{ctx}"
        );
    }

    #[test]
    fn invented_ids_lose_their_button_and_the_suggestion_stays() {
        let c = campaign();
        let clue = c.clues[0].id.clone();
        let npc = c.npcs[0].id.clone();
        let raw: RawAnswer = serde_json::from_value(serde_json::json!({
            "narration": "  Le vent se lève.  ",
            "npcLines": [{ "npc": npc, "text": "Bonjour" }, { "text": "   " }],
            "suggestions": [
                { "label": "Indice", "action": { "type": "reveal_clue", "clue": clue } },
                { "label": "Inventé", "action": { "type": "reveal_clue", "clue": "cl_nope" } },
                { "label": "Bizarre", "action": { "type": "delete_everything" } },
            ],
        }))
        .unwrap();
        let a = sanitize(raw, &c);
        assert_eq!(a.narration, "Le vent se lève.");
        assert_eq!(a.npc_lines.len(), 1);
        assert_eq!(a.npc_lines[0].speaker, c.npcs[0].name);
        let buttons: Vec<bool> = a.suggestions.iter().map(|s| s.action.is_some()).collect();
        assert_eq!(buttons, [true, false, false]);
    }

    #[test]
    fn an_unreadable_answer_is_refused() {
        assert!(
            serde_json::from_value::<RawAnswer>(serde_json::json!({ "narration": 42 })).is_err()
        );
    }
}
