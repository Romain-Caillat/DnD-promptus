//! What a session changed, read from the data with no model
//! (`session/write-recaps`, V1 `continuity/recap.ts`): the facts, and
//! the two recaps written from them — the GM's, which may hold secrets,
//! and « Précédemment… », which holds only what the table saw. The
//! co-GM rewrites the facts in prose; without it, the factual recaps
//! stand as they are.

use serde::{Deserialize, Serialize};

use super::model::Campaign;
use super::world::{NodeStatus, WorldState};

/// A scene played during the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneFact {
    pub id: String,
    pub title: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClueFact {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevelationFact {
    pub id: String,
    pub statement: String,
}

/// A front's clock that moved (GM-only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontFact {
    pub id: String,
    pub name: String,
    pub from: u32,
    pub to: u32,
    /// The label of the last step reached.
    pub step: String,
    pub total: u32,
}

/// A party member at the end of the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyFact {
    pub name: String,
    pub hit_points: i32,
    pub max_hit_points: i32,
}

impl PartyFact {
    #[must_use]
    pub fn down(&self) -> bool {
        self.hit_points <= 0
    }
}

/// What the session changed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecapFacts {
    pub duration_minutes: i64,
    pub scenes: Vec<SceneFact>,
    pub clues: Vec<ClueFact>,
    pub revelations: Vec<RevelationFact>,
    /// GM-only: never in the players' recap.
    pub fronts: Vec<FrontFact>,
    /// NPCs and adversaries whose name the players learned.
    pub revealed: Vec<String>,
    pub fights: u32,
    /// Opponents fallen in the session's fights, as the players saw
    /// them named.
    pub fallen: Vec<String>,
    pub party: Vec<PartyFact>,
    /// Promises and debts the table took on: what stays open.
    pub open_threads: Vec<String>,
}

/// What the server read about the session, beyond the two worlds.
#[derive(Debug, Clone, Default)]
pub struct SessionRecord {
    pub duration_minutes: i64,
    pub fights: u32,
    pub fallen: Vec<String>,
    pub party: Vec<PartyFact>,
    pub open_threads: Vec<String>,
}

/// The facts of a session that took the world from `before` to `after`.
#[must_use]
pub fn session_facts(
    campaign: &Campaign,
    before: &WorldState,
    after: &WorldState,
    record: SessionRecord,
) -> RecapFacts {
    let changed = |id: &str| after.node_status.get(id) != before.node_status.get(id);
    let current = |id: &str| {
        before.current_node.as_deref() == Some(id) || after.current_node.as_deref() == Some(id)
    };
    let scenes = campaign
        .nodes
        .iter()
        .filter(|n| changed(&n.id) || current(&n.id))
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
                step: f
                    .steps
                    .get(to as usize - 1)
                    .map(|s| s.label.clone())
                    .unwrap_or_default(),
                total: u32::try_from(f.steps.len()).unwrap_or(u32::MAX),
            })
        })
        .collect();
    let name_of = |id: &str| {
        campaign
            .npc(id)
            .map(|n| n.name.clone())
            .or_else(|| campaign.adversary(id).map(|a| a.name.clone()))
    };
    let revealed = after
        .revealed
        .iter()
        .filter(|id| !before.revealed.contains(*id))
        .filter_map(|id| name_of(id))
        .collect();
    RecapFacts {
        duration_minutes: record.duration_minutes.max(0),
        scenes,
        clues,
        revelations,
        fronts,
        revealed,
        fights: record.fights,
        fallen: record.fallen,
        party: record.party,
        open_threads: record.open_threads,
    }
}

fn list<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    items
        .into_iter()
        .map(|i| format!("- {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The factual recaps (no model): the GM's, and « Précédemment… ».
/// The players' never names a front or anything the GM keeps.
#[must_use]
pub fn facts_recap(f: &RecapFacts) -> (String, String) {
    let mut gm = vec![format!(
        "Durée : {} min · {} combat(s).",
        f.duration_minutes, f.fights
    )];
    if !f.scenes.is_empty() {
        let scenes: Vec<String> = f
            .scenes
            .iter()
            .map(|s| {
                if s.resolved {
                    format!("{} (résolue)", s.title)
                } else {
                    s.title.clone()
                }
            })
            .collect();
        gm.push(format!(
            "Scènes :\n{}",
            list(scenes.iter().map(String::as_str))
        ));
    }
    if !f.clues.is_empty() {
        gm.push(format!(
            "Indices trouvés :\n{}",
            list(f.clues.iter().map(|c| c.text.as_str()))
        ));
    }
    if !f.revelations.is_empty() {
        gm.push(format!(
            "Révélations désormais connues :\n{}",
            list(f.revelations.iter().map(|r| r.statement.as_str()))
        ));
    }
    if !f.fronts.is_empty() {
        let fronts: Vec<String> = f
            .fronts
            .iter()
            .map(|x| format!("{} : étape {}/{} — {}", x.name, x.to, x.total, x.step))
            .collect();
        gm.push(format!(
            "Menaces :\n{}",
            list(fronts.iter().map(String::as_str))
        ));
    }
    if !f.revealed.is_empty() {
        gm.push(format!("Révélé aux joueurs : {}.", f.revealed.join(", ")));
    }
    if !f.fallen.is_empty() {
        gm.push(format!("Adversaires vaincus : {}.", f.fallen.join(", ")));
    }
    let hurt: Vec<String> = f
        .party
        .iter()
        .filter(|p| p.hit_points < p.max_hit_points)
        .map(|p| {
            if p.down() {
                format!("{} à terre", p.name)
            } else {
                format!("{} {}/{} PV", p.name, p.hit_points, p.max_hit_points)
            }
        })
        .collect();
    if !hurt.is_empty() {
        gm.push(format!("État du groupe : {}.", hurt.join(", ")));
    }
    if !f.open_threads.is_empty() {
        gm.push(format!(
            "Fils ouverts :\n{}",
            list(f.open_threads.iter().map(String::as_str))
        ));
    }

    let mut players = Vec::new();
    let titles: Vec<&str> = f.scenes.iter().map(|s| s.title.as_str()).collect();
    players.push(if titles.is_empty() {
        "Précédemment : l’aventure a commencé.".to_string()
    } else {
        format!(
            "Précédemment : votre route vous a menés à {}.",
            titles.join(", puis ")
        )
    });
    if !f.clues.is_empty() {
        players.push(format!(
            "Vous avez découvert :\n{}",
            list(f.clues.iter().map(|c| c.text.as_str()))
        ));
    }
    if !f.revelations.is_empty() {
        players.push(format!(
            "Vous savez désormais :\n{}",
            list(f.revelations.iter().map(|r| r.statement.as_str()))
        ));
    }
    if !f.fallen.is_empty() {
        let mut names = f.fallen.clone();
        names.dedup();
        players.push(format!("Au combat, vous avez abattu {}.", names.join(", ")));
    }
    let down: Vec<&str> = f
        .party
        .iter()
        .filter(|p| p.down())
        .map(|p| p.name.as_str())
        .collect();
    if !down.is_empty() {
        players.push(format!(
            "{} {} à terre.",
            down.join(" et "),
            if down.len() > 1 { "sont" } else { "est" }
        ));
    }
    (gm.join("\n\n"), players.join("\n\n"))
}

/// The chronicle's entry title when the GM wrote none: the last scene
/// played.
#[must_use]
pub fn default_title(f: &RecapFacts) -> String {
    f.scenes.last().map(|s| s.title.clone()).unwrap_or_default()
}
