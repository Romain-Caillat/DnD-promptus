//! A scene's planned fight, staged as a simulator scenario: the party
//! slots of the campaign against the scene's opponents, on the scene's
//! map. The readiness gauge plays it (`readiness`), and so does the rule
//! system editor on every draft (`campaign/edit-rule-system`).
//!
//! What can be staged is what the rules engine can play: a party slot
//! needs a class of the rule system, an opponent a stat block of the
//! rule system (`stats: { from_rules }`). Anything else is said, never
//! guessed.

use crate::combat::scenario::{FighterSpec, PolicyKind, Scenario, SideSpec, SimulationSpec};
use crate::rules::RuleSystem;

use super::model::{Campaign, Node, StatBlock};

/// Why a planned fight cannot be played by the simulator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagingError {
    /// The scene plans no fight.
    NoEncounter,
    /// The scene names no map.
    NoMap,
    /// No party slot names a class of the rule system.
    NoParty,
    /// This opponent has no stat block in the rule system.
    NotInRules(String),
}

impl std::fmt::Display for StagingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEncounter => write!(f, "the scene plans no fight"),
            Self::NoMap => write!(f, "the scene names no map"),
            Self::NoParty => write!(f, "no party slot has a class of the rule system"),
            Self::NotInRules(who) => write!(f, "`{who}` has no stat block in the rule system"),
        }
    }
}

/// The stat block `who` (an adversary or an NPC of the campaign) fights
/// with, as a rule-system adversary id.
fn rules_block<'a>(campaign: &'a Campaign, who: &str) -> Option<&'a str> {
    let stats: Option<&StatBlock> = campaign
        .adversary(who)
        .map(|a| &a.stats)
        .or_else(|| campaign.npc(who).and_then(|n| n.stats.as_ref()));
    stats.and_then(|s| s.from_rules.as_deref())
}

/// The scene's fight as a scenario of `system`.
///
/// # Errors
///
/// A [`StagingError`] saying what is missing.
pub fn encounter_scenario(
    campaign: &Campaign,
    node: &Node,
    system: &RuleSystem,
) -> Result<Scenario, StagingError> {
    let encounter = node.encounter.as_ref().ok_or(StagingError::NoEncounter)?;
    let map = node.map.clone().ok_or(StagingError::NoMap)?;
    let party: Vec<FighterSpec> = campaign
        .party
        .iter()
        .filter_map(|p| {
            let class = p.class.as_deref().filter(|c| system.class(c).is_some())?;
            Some(FighterSpec {
                id: p.id.clone(),
                name: Some(p.name.clone()),
                class: Some(class.to_string()),
                adversary: None,
                level: 1,
                at: None,
            })
        })
        .collect();
    if party.is_empty() {
        return Err(StagingError::NoParty);
    }
    let mut opposition = Vec::new();
    for group in &encounter.opponents {
        let block = rules_block(campaign, &group.who)
            .filter(|b| system.adversary(b).is_some())
            .ok_or_else(|| StagingError::NotInRules(group.who.clone()))?;
        let name = campaign
            .adversary(&group.who)
            .map(|a| a.name.clone())
            .or_else(|| campaign.npc(&group.who).map(|n| n.name.clone()))
            .unwrap_or_else(|| group.who.clone());
        for i in 0..group.count.max(1) {
            let (id, name) = if group.count > 1 {
                (
                    format!("{}_{}", group.who, i + 1),
                    format!("{name} {}", i + 1),
                )
            } else {
                (group.who.clone(), name.clone())
            };
            opposition.push(FighterSpec {
                id,
                name: Some(name),
                class: None,
                adversary: Some(block.to_string()),
                level: 1,
                at: None,
            });
        }
    }
    Ok(Scenario {
        id: node.id.clone(),
        name: node.title.clone(),
        description: String::new(),
        source: format!("{} · {}", campaign.id, node.id),
        rules: system.id.clone(),
        version: system.version,
        map,
        context: system
            .turn_contexts
            .first()
            .map(|c| c.id.clone())
            .unwrap_or_default(),
        doors: Default::default(),
        party: SideSpec {
            policies: vec![PolicyKind::Brawler],
            morale: None,
            fighters: party,
        },
        opposition: SideSpec {
            policies: vec![PolicyKind::Brawler],
            morale: None,
            fighters: opposition,
        },
        adversaries: Vec::new(),
        simulation: SimulationSpec::default(),
        variants: Vec::new(),
    })
}
