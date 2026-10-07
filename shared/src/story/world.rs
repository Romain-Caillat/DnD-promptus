//! The living world: what changed since the campaign started. The
//! prepared story (`Campaign`) is never mutated in play; this is.
//!
//! Every write to it on the server goes through the per-campaign lock
//! (`MEMORY.md` §3): read under `SELECT … FOR UPDATE`, apply one of
//! these pure operations, write back, commit.
//!
//! Fields are added as the tickets that need them land (faction
//! affinity, goals reached, table knowledge); every field defaults, so
//! a stored state from before a new field still loads.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{Campaign, Id};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorldState {
    /// The node the table is in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_node: Option<Id>,
    /// Nodes entered (and resolved); absent = not entered yet.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub node_status: BTreeMap<Id, NodeStatus>,
    /// Clues the players found.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub found_clues: BTreeSet<Id>,
    /// Filled segments of each front's clock (0 = untouched).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub front_progress: BTreeMap<Id, u32>,
    /// NPCs and adversaries whose name the players know.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub revealed: BTreeSet<Id>,
    /// Free flags the GM sets (« pont_coupe »).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub flags: BTreeMap<String, FlagValue>,
    /// Each faction's affinity once it moved (absent = its `start`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub faction_affinity: BTreeMap<Id, i32>,
    /// Factions the players have met: only those reach their screen.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub met_factions: BTreeSet<Id>,
    /// Campaign goals ticked off.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub goals_done: BTreeSet<Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Visited,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlagValue {
    Bool(bool),
    Int(i64),
    Text(String),
}

/// A world operation named something the campaign does not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    UnknownNode(Id),
    UnknownClue(Id),
    UnknownFront(Id),
    UnknownEntity(Id),
    UnknownFaction(Id),
    UnknownGoal(Id),
}

impl std::fmt::Display for WorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownNode(id) => write!(f, "unknown node `{id}`"),
            Self::UnknownClue(id) => write!(f, "unknown clue `{id}`"),
            Self::UnknownFront(id) => write!(f, "unknown front `{id}`"),
            Self::UnknownEntity(id) => write!(f, "unknown NPC or adversary `{id}`"),
            Self::UnknownFaction(id) => write!(f, "unknown faction `{id}`"),
            Self::UnknownGoal(id) => write!(f, "unknown goal `{id}`"),
        }
    }
}

impl std::error::Error for WorldError {}

/// What revealing a clue did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClueReveal {
    /// The first clue of its revelation: the players can now draw it.
    NewRevelation(Id),
    /// The revelation was already known; one more path to it.
    Reinforced(Id),
    /// Already found: nothing changed.
    AlreadyFound,
}

/// Where a front's clock went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontAdvance {
    pub from: u32,
    pub to: u32,
    /// Steps newly reached (0-based), in order; empty when going back.
    pub reached: Vec<usize>,
}

/// One faction's affinity moving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffinityShift {
    pub faction: Id,
    pub from: i32,
    pub to: i32,
}

impl WorldState {
    /// The table enters `node`: it becomes current and visited (a
    /// resolved node stays resolved).
    ///
    /// # Errors
    ///
    /// `UnknownNode` when the campaign has no such node.
    pub fn enter_node(&mut self, campaign: &Campaign, node: &str) -> Result<(), WorldError> {
        if campaign.node(node).is_none() {
            return Err(WorldError::UnknownNode(node.to_string()));
        }
        self.current_node = Some(node.to_string());
        self.node_status
            .entry(node.to_string())
            .or_insert(NodeStatus::Visited);
        Ok(())
    }

    /// # Errors
    ///
    /// `UnknownNode` when the campaign has no such node.
    pub fn resolve_node(&mut self, campaign: &Campaign, node: &str) -> Result<(), WorldError> {
        if campaign.node(node).is_none() {
            return Err(WorldError::UnknownNode(node.to_string()));
        }
        self.node_status
            .insert(node.to_string(), NodeStatus::Resolved);
        Ok(())
    }

    /// Entered at some point — a resolved node counts as visited.
    #[must_use]
    pub fn is_visited(&self, node: &str) -> bool {
        self.node_status.contains_key(node)
    }

    /// A revelation is known as soon as one of its clues is found.
    #[must_use]
    pub fn is_revelation_known(&self, campaign: &Campaign, revelation: &str) -> bool {
        campaign
            .clues_for(revelation)
            .any(|c| self.found_clues.contains(&c.id))
    }

    /// The players find `clue`.
    ///
    /// # Errors
    ///
    /// `UnknownClue` when the campaign has no such clue.
    pub fn reveal_clue(
        &mut self,
        campaign: &Campaign,
        clue: &str,
    ) -> Result<ClueReveal, WorldError> {
        let c = campaign
            .clue(clue)
            .ok_or_else(|| WorldError::UnknownClue(clue.to_string()))?;
        if self.found_clues.contains(clue) {
            return Ok(ClueReveal::AlreadyFound);
        }
        let known = self.is_revelation_known(campaign, &c.revelation);
        self.found_clues.insert(clue.to_string());
        Ok(if known {
            ClueReveal::Reinforced(c.revelation.clone())
        } else {
            ClueReveal::NewRevelation(c.revelation.clone())
        })
    }

    /// Move a front's clock by `delta` segments, bounded to `0..=steps`.
    ///
    /// # Errors
    ///
    /// `UnknownFront` when the campaign has no such front.
    pub fn advance_front(
        &mut self,
        campaign: &Campaign,
        front: &str,
        delta: i32,
    ) -> Result<FrontAdvance, WorldError> {
        let f = campaign
            .front(front)
            .ok_or_else(|| WorldError::UnknownFront(front.to_string()))?;
        let len = i64::try_from(f.steps.len()).unwrap_or(i64::MAX);
        let from = self.front_progress.get(front).copied().unwrap_or(0);
        let to_wide = (i64::from(from) + i64::from(delta)).clamp(0, len);
        let to = u32::try_from(to_wide).unwrap_or(from);
        self.front_progress.insert(front.to_string(), to);
        let reached = (from..to).map(|i| i as usize).collect();
        Ok(FrontAdvance { from, to, reached })
    }

    /// The players learn the name of an NPC or adversary.
    ///
    /// # Errors
    ///
    /// `UnknownEntity` when it is neither an NPC nor an adversary.
    pub fn reveal_entity(&mut self, campaign: &Campaign, id: &str) -> Result<(), WorldError> {
        if campaign.npc(id).is_none() && campaign.adversary(id).is_none() {
            return Err(WorldError::UnknownEntity(id.to_string()));
        }
        self.revealed.insert(id.to_string());
        Ok(())
    }

    pub fn set_flag(&mut self, flag: &str, value: FlagValue) {
        self.flags.insert(flag.to_string(), value);
    }

    /// A faction's affinity now: its `start` until it moved.
    ///
    /// # Errors
    ///
    /// `UnknownFaction` when the campaign has no such faction.
    pub fn affinity(&self, campaign: &Campaign, faction: &str) -> Result<i32, WorldError> {
        let f = campaign
            .faction(faction)
            .ok_or_else(|| WorldError::UnknownFaction(faction.to_string()))?;
        Ok(self
            .faction_affinity
            .get(faction)
            .copied()
            .unwrap_or(f.affinity.start))
    }

    /// The players meet `faction`: it now shows on their screen.
    /// Returns whether it was new.
    ///
    /// # Errors
    ///
    /// `UnknownFaction` when the campaign has no such faction.
    pub fn meet_faction(&mut self, campaign: &Campaign, faction: &str) -> Result<bool, WorldError> {
        if campaign.faction(faction).is_none() {
            return Err(WorldError::UnknownFaction(faction.to_string()));
        }
        Ok(self.met_factions.insert(faction.to_string()))
    }

    /// Move a faction's affinity by `delta`, within its bounds; dealing
    /// with a faction means meeting it. Winning a faction's favour
    /// (`delta > 0`) costs as much with each of its rivals; losing it
    /// moves no one else. The shifts that actually happened come back,
    /// the faction first.
    ///
    /// # Errors
    ///
    /// `UnknownFaction` when the campaign has no such faction (a rival
    /// missing from the campaign is skipped: the validator reports it).
    pub fn shift_affinity(
        &mut self,
        campaign: &Campaign,
        faction: &str,
        delta: i32,
    ) -> Result<Vec<AffinityShift>, WorldError> {
        let f = campaign
            .faction(faction)
            .ok_or_else(|| WorldError::UnknownFaction(faction.to_string()))?;
        self.met_factions.insert(faction.to_string());
        let mut shifts = Vec::new();
        self.move_affinity(campaign, &f.id, delta, &mut shifts);
        if delta > 0 {
            for rival in &f.rivals {
                if rival != faction {
                    self.move_affinity(campaign, rival, -delta, &mut shifts);
                }
            }
        }
        Ok(shifts)
    }

    fn move_affinity(
        &mut self,
        campaign: &Campaign,
        faction: &str,
        delta: i32,
        shifts: &mut Vec<AffinityShift>,
    ) {
        let Some(f) = campaign.faction(faction) else {
            return;
        };
        let from = self
            .faction_affinity
            .get(faction)
            .copied()
            .unwrap_or(f.affinity.start);
        let (lo, hi) = (
            f.affinity.min.min(f.affinity.max),
            f.affinity.max.max(f.affinity.min),
        );
        let to = from.saturating_add(delta).clamp(lo, hi);
        if to != from {
            self.faction_affinity.insert(faction.to_string(), to);
            shifts.push(AffinityShift {
                faction: faction.to_string(),
                from,
                to,
            });
        }
    }

    /// Tick (or untick) a campaign goal. Returns whether it changed.
    ///
    /// # Errors
    ///
    /// `UnknownGoal` when the campaign has no such goal.
    pub fn set_goal(
        &mut self,
        campaign: &Campaign,
        goal: &str,
        done: bool,
    ) -> Result<bool, WorldError> {
        if campaign.goal(goal).is_none() {
            return Err(WorldError::UnknownGoal(goal.to_string()));
        }
        Ok(if done {
            self.goals_done.insert(goal.to_string())
        } else {
            self.goals_done.remove(goal)
        })
    }
}
