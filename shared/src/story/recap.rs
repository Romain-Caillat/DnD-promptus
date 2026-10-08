//! What a session changed, and the recaps drawn from it without any
//! model (`session/write-recaps`, V1 `continuity/recap.ts`).
//!
//! [`session_facts`] compares the world when the session started with the
//! world when it ended. [`facts_recap`] writes three texts from those
//! facts: the GM's recap (may hold secrets: fronts, revelations), the
//! players' « Précédemment… » and the chronicle entry — the last two only
//! from what the table saw (scenes entered, clue texts found, names
//! learnt, the shared journal lines the caller passes). The co-GM
//! rewrites them in prose; without a model, the factual draft is usable
//! as it is, and the GM rereads either before publishing.
//!
//! [`leaks`] is the check on any text meant for the players, whoever
//! wrote it: a front's name, or an NPC or adversary the table has not
//! met. It flags, it never blocks.
//!
//! [`lines`] cuts « Précédemment… » into the sentences the GM reveals
//! one by one when launching the next session (`gm/launch-session`).

use serde::Serialize;

use super::model::{Campaign, Id};
use super::world::{NodeStatus, WorldState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneFact {
    pub id: Id,
    pub title: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClueFact {
    pub id: Id,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevelationFact {
    pub id: Id,
    /// GM wording: never in a player text.
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontFact {
    pub id: Id,
    pub name: String,
    pub from: u32,
    pub to: u32,
    /// The label of the last step reached.
    pub step: String,
    pub total: usize,
}

/// What one session changed in the world.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecapFacts {
    /// Entered, resolved, or current at either end; campaign order.
    pub scenes: Vec<SceneFact>,
    pub clues: Vec<ClueFact>,
    /// Revelations the table can now draw (a first clue found).
    pub revelations: Vec<RevelationFact>,
    /// GM-only: the players feel a front, they never see it.
    pub fronts: Vec<FrontFact>,
    /// NPCs and adversaries whose name the table learnt.
    pub met: Vec<String>,
}

impl RecapFacts {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// What changed between `before` (the world when the session started)
/// and `after` (when it ended). Ids the campaign no longer has are
/// skipped: the story may have been edited since.
#[must_use]
pub fn session_facts(campaign: &Campaign, before: &WorldState, after: &WorldState) -> RecapFacts {
    let played = |id: &str| {
        before.node_status.get(id) != after.node_status.get(id)
            || before.current_node.as_deref() == Some(id)
            || after.current_node.as_deref() == Some(id)
    };
    let scenes = campaign
        .nodes
        .iter()
        .filter(|n| played(&n.id))
        .map(|n| SceneFact {
            id: n.id.clone(),
            title: n.title.clone(),
            resolved: after.node_status.get(&n.id) == Some(&NodeStatus::Resolved),
        })
        .collect();
    let clues = campaign
        .clues
        .iter()
        .filter(|c| after.found_clues.contains(&c.id) && !before.found_clues.contains(&c.id))
        .map(|c| ClueFact {
            id: c.id.clone(),
            text: c.text.clone(),
        })
        .collect();
    let revelations = campaign
        .revelations
        .iter()
        .filter(|r| {
            after.is_revelation_known(campaign, &r.id)
                && !before.is_revelation_known(campaign, &r.id)
        })
        .map(|r| RevelationFact {
            id: r.id.clone(),
            statement: r.statement.clone(),
        })
        .collect();
    let fronts = campaign
        .fronts
        .iter()
        .filter_map(|f| {
            let from = before.front_progress.get(&f.id).copied().unwrap_or(0);
            let to = after.front_progress.get(&f.id).copied().unwrap_or(0);
            (to > from).then(|| FrontFact {
                id: f.id.clone(),
                name: f.name.clone(),
                from,
                to,
                step: (to as usize)
                    .checked_sub(1)
                    .and_then(|i| f.steps.get(i))
                    .map(|s| s.label.clone())
                    .unwrap_or_default(),
                total: f.steps.len(),
            })
        })
        .collect();
    let met = after
        .revealed
        .iter()
        .filter(|id| !before.revealed.contains(*id))
        .filter_map(|id| {
            campaign
                .npc(id)
                .map(|n| n.name.clone())
                .or_else(|| campaign.adversary(id).map(|a| a.name.clone()))
        })
        .collect();
    RecapFacts {
        scenes,
        clues,
        revelations,
        fronts,
        met,
    }
}

/// The three texts of a session, drafted from its facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FactsRecap {
    /// The GM's recap: may name fronts and revelations.
    pub gm: String,
    /// « Précédemment… »: only what the table saw.
    pub players: String,
    pub chronicle_title: String,
    /// One or two lines for the campaign's chronicle: what the table saw.
    pub chronicle: String,
}

fn bullets<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    items
        .into_iter()
        .map(|i| format!("- {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Join names the French way: « a », « a et b », « a, b et c ».
fn and_list(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} et {last}", rest.join(", ")),
    }
}

/// Draft the recaps of a session from its facts. `table_lines` are the
/// session's journal lines the players already saw (a fight, loot, a
/// promise, a debt): they go into the players' texts as they are.
/// `minutes` is how long it was played, when known.
#[must_use]
pub fn facts_recap(f: &RecapFacts, table_lines: &[String], minutes: Option<i64>) -> FactsRecap {
    let titles: Vec<&str> = f.scenes.iter().map(|s| s.title.as_str()).collect();

    let mut gm = Vec::new();
    if let Some(m) = minutes {
        gm.push(format!("Durée : {} h {:02}.", m / 60, m % 60));
    }
    if !f.scenes.is_empty() {
        gm.push(format!(
            "Scènes :\n{}",
            bullets(f.scenes.iter().map(|s| s.title.as_str()))
        ));
        let resolved: Vec<&str> = f
            .scenes
            .iter()
            .filter(|s| s.resolved)
            .map(|s| s.title.as_str())
            .collect();
        if !resolved.is_empty() {
            gm.push(format!("Résolues : {}.", resolved.join(", ")));
        }
    }
    if !f.clues.is_empty() {
        gm.push(format!(
            "Indices trouvés :\n{}",
            bullets(f.clues.iter().map(|c| c.text.as_str()))
        ));
    }
    if !f.revelations.is_empty() {
        gm.push(format!(
            "Révélations désormais à leur portée :\n{}",
            bullets(f.revelations.iter().map(|r| r.statement.as_str()))
        ));
    }
    if !f.fronts.is_empty() {
        let lines: Vec<String> = f
            .fronts
            .iter()
            .map(|x| format!("{} : {}/{} — {}", x.name, x.to, x.total, x.step))
            .collect();
        gm.push(format!(
            "Menaces :\n{}",
            bullets(lines.iter().map(String::as_str))
        ));
    }
    if !f.met.is_empty() {
        gm.push(format!("Rencontrés : {}.", f.met.join(", ")));
    }
    if !table_lines.is_empty() {
        gm.push(format!(
            "À la table :\n{}",
            bullets(table_lines.iter().map(String::as_str))
        ));
    }

    let mut players = Vec::new();
    players.push(if titles.is_empty() {
        "Précédemment… l’aventure a commencé.".to_string()
    } else {
        format!(
            "Précédemment… votre route vous a menés à {}.",
            titles.join(", puis à ")
        )
    });
    if !f.met.is_empty() {
        let names: Vec<&str> = f.met.iter().map(String::as_str).collect();
        players.push(format!("Vous avez rencontré {}.", and_list(&names)));
    }
    if !f.clues.is_empty() {
        players.push(format!(
            "Vous avez découvert :\n{}",
            bullets(f.clues.iter().map(|c| c.text.as_str()))
        ));
    }
    if !table_lines.is_empty() {
        players.push(format!(
            "À retenir :\n{}",
            bullets(table_lines.iter().map(String::as_str))
        ));
    }

    let chronicle_title = f.scenes.last().map(|s| s.title.clone()).unwrap_or_default();
    let mut chronicle = Vec::new();
    if !titles.is_empty() {
        chronicle.push(format!("{}.", titles.join(", puis ")));
    }
    chronicle.extend(
        f.clues
            .iter()
            .map(|c| c.text.as_str())
            .chain(table_lines.iter().map(String::as_str))
            .take(2)
            .map(str::to_string),
    );

    FactsRecap {
        gm: gm.join("\n\n"),
        players: players.join("\n\n"),
        chronicle_title,
        chronicle: chronicle.join(" "),
    }
}

/// A name a player text must not hold, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leak {
    pub name: String,
    /// `front` or `unmet`.
    pub kind: &'static str,
}

fn mentions(text: &str, name: &str) -> bool {
    let name = name.trim();
    if name.chars().count() < 3 {
        return false;
    }
    let text = text.to_lowercase();
    let name = name.to_lowercase();
    text.match_indices(&name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + name.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// The names in `text` the table should not read, as of `world`: a
/// front, an NPC or adversary not met yet.
#[must_use]
pub fn leaks(campaign: &Campaign, world: &WorldState, text: &str) -> Vec<Leak> {
    let fronts = campaign.fronts.iter().map(|f| (&f.name, "front"));
    let unmet = campaign
        .npcs
        .iter()
        .filter(|n| !world.revealed.contains(&n.id))
        .map(|n| (&n.name, "unmet"))
        .chain(
            campaign
                .adversaries
                .iter()
                .filter(|a| !world.revealed.contains(&a.id))
                .map(|a| (&a.name, "unmet")),
        );
    let mut out: Vec<Leak> = Vec::new();
    for (name, kind) in fronts.chain(unmet) {
        if mentions(text, name) && !out.iter().any(|l| &l.name == name) {
            out.push(Leak {
                name: name.clone(),
                kind,
            });
        }
    }
    out
}

/// « Précédemment… » cut into the sentences read one by one: a line
/// break always cuts; a sentence ends on `.`, `!`, `?` or `…` (closing
/// quotes kept with it) when what follows starts a new sentence.
/// Bullets lose their dash.
#[must_use]
pub fn lines(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.lines() {
        let para = para.trim().trim_start_matches("- ").trim();
        if para.is_empty() {
            continue;
        }
        let chars: Vec<char> = para.chars().collect();
        let mut start = 0;
        let mut i = 0;
        while i < chars.len() {
            if matches!(chars[i], '.' | '!' | '?' | '…') {
                let mut end = i + 1;
                while end < chars.len() && matches!(chars[end], '.' | '»' | '"' | '’' | ')') {
                    end += 1;
                }
                let mut next = end;
                while next < chars.len() && chars[next].is_whitespace() {
                    next += 1;
                }
                let new_sentence = next > end
                    && next < chars.len()
                    && (chars[next].is_uppercase() || matches!(chars[next], '«' | '—' | '-'));
                if new_sentence {
                    out.push(
                        chars[start..end]
                            .iter()
                            .collect::<String>()
                            .trim()
                            .to_string(),
                    );
                    start = next;
                    i = next;
                    continue;
                }
                i = end;
                continue;
            }
            i += 1;
        }
        let rest: String = chars[start..].iter().collect();
        if !rest.trim().is_empty() {
            out.push(rest.trim().to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::from_yaml;

    const CORSAIRES: &str = include_str!("../../../content/campaigns/corsaires/campagne.yaml");
    const BRASIER: &str = include_str!("../../../content/campaigns/brasier/campagne.yaml");

    /// A session on `campaign`: the table walks from the first scene into
    /// the second (resolving the first), finds the first clue of the
    /// second scene, a front moves two steps, and the table meets the
    /// first NPC — while another NPC and an adversary stay unknown.
    fn played(campaign: &Campaign) -> (WorldState, WorldState) {
        let first = &campaign.nodes[0].id;
        let second = &campaign.nodes[1].id;
        let mut before = WorldState::default();
        before.enter_node(campaign, first).unwrap();
        let mut after = before.clone();
        after.resolve_node(campaign, first).unwrap();
        after.enter_node(campaign, second).unwrap();
        let clue = campaign
            .clues
            .iter()
            .find(|c| !after.is_revelation_known(campaign, &c.revelation))
            .unwrap();
        after.reveal_clue(campaign, &clue.id).unwrap();
        after
            .advance_front(campaign, &campaign.fronts[0].id, 2)
            .unwrap();
        after.reveal_entity(campaign, &campaign.npcs[0].id).unwrap();
        (before, after)
    }

    fn worlds() -> Vec<Campaign> {
        vec![from_yaml(CORSAIRES).unwrap(), from_yaml(BRASIER).unwrap()]
    }

    #[test]
    fn the_facts_measure_what_the_session_changed_on_both_worlds() {
        for c in worlds() {
            let (before, after) = played(&c);
            let f = session_facts(&c, &before, &after);
            let scenes: Vec<(&str, bool)> = f
                .scenes
                .iter()
                .map(|s| (s.id.as_str(), s.resolved))
                .collect();
            assert_eq!(
                scenes,
                [
                    (c.nodes[0].id.as_str(), true),
                    (c.nodes[1].id.as_str(), false)
                ],
                "{}",
                c.title
            );
            assert_eq!(f.clues.len(), 1, "{}", c.title);
            assert_eq!(f.revelations.len(), 1, "{}", c.title);
            assert_eq!(f.fronts[0].from, 0);
            assert_eq!(f.fronts[0].to, 2);
            assert_eq!(f.fronts[0].step, c.fronts[0].steps[1].label);
            assert_eq!(f.met, [c.npcs[0].name.clone()]);
        }
    }

    #[test]
    fn only_what_is_new_counts() {
        let c = from_yaml(CORSAIRES).unwrap();
        let (_, after) = played(&c);
        let f = session_facts(&c, &after, &after);
        assert!(f.clues.is_empty() && f.fronts.is_empty() && f.met.is_empty());
        assert!(f.revelations.is_empty());
        // The table is still somewhere: the scene it sits in is played.
        assert_eq!(f.scenes.len(), 1);
    }

    #[test]
    fn the_players_texts_hold_no_front_and_no_unmet_name_on_both_worlds() {
        for c in worlds() {
            let (before, after) = played(&c);
            let f = session_facts(&c, &before, &after);
            let r = facts_recap(&f, &["Le quai a brûlé.".to_string()], Some(150));
            assert!(r.gm.contains(&c.fronts[0].name), "{}", r.gm);
            assert!(r.gm.contains(&f.revelations[0].statement), "{}", r.gm);
            assert!(r.gm.contains("2 h 30"), "{}", r.gm);
            for text in [&r.players, &r.chronicle, &r.chronicle_title] {
                assert!(text.contains(&c.nodes[1].title), "{text}");
                assert!(leaks(&c, &after, text).is_empty(), "{}: {text}", c.title);
                assert!(!text.contains(&f.revelations[0].statement), "{text}");
            }
            assert!(r.players.starts_with("Précédemment…"));
            assert!(r.players.contains(&f.clues[0].text), "{}", r.players);
            assert!(r.players.contains(&c.npcs[0].name), "{}", r.players);
            assert!(r.players.contains("Le quai a brûlé."), "{}", r.players);
            assert_eq!(r.chronicle_title, c.nodes[1].title);
        }
    }

    #[test]
    fn a_front_or_an_unmet_name_in_a_player_text_is_flagged() {
        let c = from_yaml(BRASIER).unwrap();
        let (_, after) = played(&c);
        let unmet = c
            .npcs
            .iter()
            .filter(|n| !after.revealed.contains(&n.id))
            .map(|n| &n.name)
            .chain(c.adversaries.iter().map(|a| &a.name))
            .next()
            .unwrap();
        let text = format!("{unmet} vous attend. {} grandit.", c.fronts[0].name);
        let found = leaks(&c, &after, &text);
        let kinds: Vec<(&str, &str)> = found.iter().map(|l| (l.name.as_str(), l.kind)).collect();
        assert!(
            kinds.contains(&(c.fronts[0].name.as_str(), "front")),
            "{kinds:?}"
        );
        assert!(kinds.contains(&(unmet.as_str(), "unmet")), "{kinds:?}");
        // A met NPC is fine.
        assert!(leaks(&c, &after, &format!("{} sourit.", c.npcs[0].name)).is_empty());
    }

    #[test]
    fn previously_is_read_sentence_by_sentence() {
        let text = "Précédemment… sous l’autel, Borin a trouvé la page. Les Valombre y gardaient « la porte nord ». \
                    Il cria « Partez ! » et s’enfuit.\n\n- Et quelqu’un avait laissé des cendres… encore tièdes.";
        assert_eq!(
            lines(text),
            [
                "Précédemment… sous l’autel, Borin a trouvé la page.",
                "Les Valombre y gardaient « la porte nord ».",
                "Il cria « Partez ! » et s’enfuit.",
                "Et quelqu’un avait laissé des cendres… encore tièdes.",
            ]
        );
        assert!(lines("  \n ").is_empty());
    }
}
