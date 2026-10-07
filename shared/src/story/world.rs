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
    /// Each faction's affinity with the party once it moved; absent =
    /// the faction's `affinity.start` ([`WorldState::affinity_of`]).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub affinity: BTreeMap<Id, i32>,
    /// Factions the players know of: only these reach their screens.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub known_factions: BTreeSet<Id>,
    /// Campaign goals the players know of, and whether they are reached;
    /// absent = not known to the table yet.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub goals: BTreeMap<Id, GoalStatus>,
}

/// Where the table stands with a campaign goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    /// The players know it is to be done.
    Known,
    /// Ticked off.
    Done,
}

/// One gauge an affinity shift moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffinityShift {
    pub faction: Id,
    pub from: i32,
    pub to: i32,
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

    /// Where the party stands with `faction` now: its starting affinity
    /// until something moved it. `None` for an unknown faction.
    #[must_use]
    pub fn affinity_of(&self, campaign: &Campaign, faction: &str) -> Option<i32> {
        let f = campaign.faction(faction)?;
        Some(
            self.affinity
                .get(faction)
                .copied()
                .unwrap_or(f.affinity.start)
                .clamp(f.affinity.min, f.affinity.max),
        )
    }

    /// The players learn that `faction` exists (its name, what it is).
    ///
    /// # Errors
    ///
    /// `UnknownFaction` when the campaign has no such faction.
    pub fn meet_faction(&mut self, campaign: &Campaign, faction: &str) -> Result<bool, WorldError> {
        if campaign.faction(faction).is_none() {
            return Err(WorldError::UnknownFaction(faction.to_string()));
        }
        Ok(self.known_factions.insert(faction.to_string()))
    }

    /// The party's standing with `faction` moves by `delta`, within the
    /// faction's bounds. The tension rule of the Brasier (`Univers.md`):
    /// winning favour with a faction **lowers each of its rivals** by as
    /// much, each within its own bounds; losing favour moves no one
    /// else. Rivalries are read as declared on `faction` — they need not
    /// be mutual. The faction becomes known to the players; a rival that
    /// drops stays as known (or unknown) as it was.
    ///
    /// Returns every gauge that moved, the faction first; a gauge already
    /// at its bound is left out.
    ///
    /// # Errors
    ///
    /// `UnknownFaction` when the campaign has no such faction.
    pub fn shift_affinity(
        &mut self,
        campaign: &Campaign,
        faction: &str,
        delta: i32,
    ) -> Result<Vec<AffinityShift>, WorldError> {
        let f = campaign
            .faction(faction)
            .ok_or_else(|| WorldError::UnknownFaction(faction.to_string()))?;
        self.known_factions.insert(faction.to_string());
        let mut moved = Vec::new();
        self.move_gauge(campaign, faction, delta, &mut moved);
        if delta > 0 {
            for rival in &f.rivals {
                if rival != faction {
                    self.move_gauge(campaign, rival, -delta, &mut moved);
                }
            }
        }
        Ok(moved)
    }

    fn move_gauge(
        &mut self,
        campaign: &Campaign,
        faction: &str,
        delta: i32,
        moved: &mut Vec<AffinityShift>,
    ) {
        let (Some(f), Some(from)) = (
            campaign.faction(faction),
            self.affinity_of(campaign, faction),
        ) else {
            return;
        };
        let to = from
            .saturating_add(delta)
            .clamp(f.affinity.min, f.affinity.max);
        if to != from {
            self.affinity.insert(faction.to_string(), to);
            moved.push(AffinityShift {
                faction: faction.to_string(),
                from,
                to,
            });
        }
    }

    /// Set what the table knows of `goal`: `None` hides it again (a
    /// mistake undone), `Known` shows it to do, `Done` ticks it off. A
    /// goal ticked off makes the faction holding it known.
    ///
    /// # Errors
    ///
    /// `UnknownGoal` when the campaign has no such goal.
    pub fn set_goal(
        &mut self,
        campaign: &Campaign,
        goal: &str,
        status: Option<GoalStatus>,
    ) -> Result<(), WorldError> {
        let g = campaign
            .goal(goal)
            .ok_or_else(|| WorldError::UnknownGoal(goal.to_string()))?;
        match status {
            None => {
                self.goals.remove(goal);
            }
            Some(s) => {
                self.goals.insert(goal.to_string(), s);
                if s == GoalStatus::Done
                    && let Some(holder) = &g.held_by
                    && campaign.faction(holder).is_some()
                {
                    self.known_factions.insert(holder.clone());
                }
            }
        }
        Ok(())
    }
}
