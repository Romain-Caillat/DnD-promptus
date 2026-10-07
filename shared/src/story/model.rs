//! The campaign document: everything the GM prepares, as one value.
//!
//! The story is a graph, not a script (`MEMORY.md` §1): a bible, fronts
//! with a clock, nodes (scenes) linked by transitions, and clues that
//! lead to revelations. Entities (NPCs, adversaries, locations, items,
//! factions, goals) are declared once and referenced by id.
//!
//! **Ids are stable slugs** written by hand or by the LLM
//! (`sc_taverne`, `pnj_morel`). They share **one namespace** across the
//! whole campaign, so any reference can be resolved without knowing its
//! kind first, and a reference to the wrong kind is caught
//! (`validate::validate`).
//!
//! The serialized form (YAML for people, JSON in the database) is
//! snake_case and is the format documented in `docs/campaign-format.md`.
//! Unknown keys are refused, so a typo in a hand-written file is an
//! import error rather than a field silently dropped. Empty optional
//! fields are left out on export, which keeps the YAML short and makes
//! export → import an identity.
//!
//! What players may see of this document is decided in one place only:
//! the server's player projection. Every field is GM material until that
//! function copies it out.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A stable id (see the module docs).
pub type Id = String;

/// The format version this crate reads and writes.
pub const FORMAT_VERSION: u32 = 1;

fn format_version() -> u32 {
    FORMAT_VERSION
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn one() -> u32 {
    1
}

fn is_one(n: &u32) -> bool {
    *n == 1
}

/// A whole campaign.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Campaign {
    #[serde(default = "format_version")]
    pub format: u32,
    /// The campaign's own stable id (`corsaires`, `brasier`).
    pub id: Id,
    pub title: String,
    /// The universe it is set in, in a few words.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub world: String,
    /// Which rule system, at which locked version, the campaign plays.
    pub rules: RuleSystemRef,
    pub bible: Bible,
    /// The player characters the prep writes hooks for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub party: Vec<PartyMember>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acts: Vec<Act>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fronts: Vec<Front>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<Node>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revelations: Vec<Revelation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clues: Vec<Clue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub npcs: Vec<Npc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adversaries: Vec<Adversary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<Location>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Item>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub factions: Vec<Faction>,
    /// Campaign goals to tick off (the Brasier's four components).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub goals: Vec<Goal>,
}

impl Campaign {
    /// An empty campaign: a title, a rule system, nothing prepared yet.
    #[must_use]
    pub fn empty(id: &str, title: &str, world: &str, rules: RuleSystemRef) -> Self {
        Self {
            format: FORMAT_VERSION,
            id: id.to_string(),
            title: title.to_string(),
            world: world.to_string(),
            rules,
            bible: Bible::default(),
            party: Vec::new(),
            acts: Vec::new(),
            fronts: Vec::new(),
            nodes: Vec::new(),
            revelations: Vec::new(),
            clues: Vec::new(),
            npcs: Vec::new(),
            adversaries: Vec::new(),
            locations: Vec::new(),
            items: Vec::new(),
            factions: Vec::new(),
            goals: Vec::new(),
        }
    }

    #[must_use]
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    #[must_use]
    pub fn clue(&self, id: &str) -> Option<&Clue> {
        self.clues.iter().find(|c| c.id == id)
    }

    #[must_use]
    pub fn faction(&self, id: &str) -> Option<&Faction> {
        self.factions.iter().find(|f| f.id == id)
    }

    #[must_use]
    pub fn goal(&self, id: &str) -> Option<&Goal> {
        self.goals.iter().find(|g| g.id == id)
    }

    #[must_use]
    pub fn front(&self, id: &str) -> Option<&Front> {
        self.fronts.iter().find(|f| f.id == id)
    }

    #[must_use]
    pub fn npc(&self, id: &str) -> Option<&Npc> {
        self.npcs.iter().find(|n| n.id == id)
    }

    #[must_use]
    pub fn adversary(&self, id: &str) -> Option<&Adversary> {
        self.adversaries.iter().find(|a| a.id == id)
    }

    #[must_use]
    pub fn location(&self, id: &str) -> Option<&Location> {
        self.locations.iter().find(|l| l.id == id)
    }

    /// The clues that lead to `revelation`.
    pub fn clues_for<'a>(&'a self, revelation: &'a str) -> impl Iterator<Item = &'a Clue> + 'a {
        self.clues
            .iter()
            .filter(move |c| c.revelation == revelation)
    }
}

/// A pointer to a rule system version (`engine/model-rule-system`).
/// Only stored here: the rules engine resolves and checks it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSystemRef {
    pub id: Id,
    pub version: u32,
}

/// What the campaign is about.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bible {
    /// The pitch, two or three sentences (GM).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pitch: String,
    /// Tone and mood (GM).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tone: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub themes: Vec<String>,
    /// Pixel-art direction for every generated visual: palette, light,
    /// mood (`media/draw-pixel-art-assets`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub art_direction: String,
    /// What is true in the world, known or not (GM).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub truths: Vec<String>,
    /// What only the GM knows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
    /// What the players are told when the campaign starts.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub player_hook: String,
    /// The opening node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_node: Option<Id>,
}

/// A player character slot the prep writes for. The character sheet
/// itself is created and validated later (`session/validate-characters`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyMember {
    pub id: Id,
    pub name: String,
    /// Who plays it, if known.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub player: String,
    /// Class, background, the one line that says who they are.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub concept: String,
    /// The rule system's class this slot is written for, when the prep
    /// assumes one (`validate::validate_with` checks it exists).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<Id>,
}

/// An act groups nodes; a node names its act.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Act {
    pub id: Id,
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub summary: String,
    /// Read aloud when the act opens.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub opening: String,
    /// Read aloud when it closes, towards the next act.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub closing: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub music: Vec<MusicTrack>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gm_notes: String,
}

/// A threat that advances when nobody stops it. GM only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Front {
    pub id: Id,
    pub name: String,
    /// What the threat wants.
    pub goal: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// The clock: four to six steps, the last is the catastrophe.
    pub steps: Vec<FrontStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontStep {
    pub label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// A node of the story graph: a scene, with everything the GM needs to
/// run it (Romain's prep format, completed by `docs/lecons-des-parties.md`
/// §3 chain 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: Id,
    pub act: Id,
    pub title: String,
    /// Optional scenes may be skipped: critical knowledge must not live
    /// only there.
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    /// What the scene is about, for the GM, in one or two sentences.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub summary: String,
    /// Where it happens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Id>,
    /// The grid map the scene is played on: a map file's `id`
    /// (`content/maps/`), not a campaign id. Checked against the maps
    /// supplied to `validate::validate_with`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<String>,
    /// Shown or read to the players when the scene opens.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub read_aloud: String,
    #[serde(default, skip_serializing_if = "Ambience::is_empty")]
    pub ambience: Ambience,
    /// How the scene unfolds (GM).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub flow: String,
    /// The event that sets the scene in motion (GM).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hook: String,
    /// Checks the GM expects to call.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<PlannedCheck>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub npcs: Vec<NpcPresence>,
    /// What the GM must not forget.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_points: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Encounter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loot: Vec<Loot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub xp: Vec<XpAward>,
    /// How the scene hands over to the next one, narrated.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub transition: String,
    /// Where the players can go from here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exits: Vec<Exit>,
    /// Revelations the players must know when they enter. What they
    /// learn here is the clues placed in this node.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Id>,
    /// For each player character, a reason to act in this scene.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player_hooks: Vec<PlayerHook>,
    /// If the players skip this scene, where its critical information
    /// can still be found — the GM's own note. The validator checks the
    /// clues independently.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub if_skipped: String,
    /// The scene illustration, described for pixel art.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub art: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gm_notes: String,
}

/// Mood and music of a scene. Several tracks, one per mood the scene
/// may go through (calm, then tension, then combat).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ambience {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mood: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sounds: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub music: Vec<MusicTrack>,
}

impl Ambience {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.mood.is_empty() && self.sounds.is_empty() && self.music.is_empty()
    }
}

/// A YouTube track, chosen by the GM (`media/play-youtube-music`).
/// A track still to choose has no `url`, only a `search` hint; the
/// validator reports it and players never receive it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicTrack {
    pub mood: MusicMood,
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    /// What to search on YouTube while no link is chosen (GM).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub search: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MusicMood {
    Calm,
    Exploration,
    Tension,
    Mystery,
    Combat,
    Epic,
}

/// A check the GM plans to call: which stat, how hard, and what the
/// extremes do. `stat` and `difficulty` are rule-system data
/// (`engine/model-rule-system` checks them).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedCheck {
    /// What the player tries (« Calmer Jacquot »).
    pub action: String,
    pub stat: String,
    pub difficulty: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub success: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub failure: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub natural_1: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub natural_20: String,
}

/// An NPC in a scene, and what they are there for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcPresence {
    pub npc: Id,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role: String,
}

/// A fight the scene may hold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    pub opponents: Vec<Opponents>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tactics: Vec<String>,
    /// When the opponents break (« si 3 marins tombent, les autres
    /// fuient »).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub morale: Vec<MoraleRule>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub on_victory: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub on_defeat: String,
}

/// A group of identical opponents: an adversary or an NPC with stats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Opponents {
    pub who: Id,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoraleRule {
    pub when: String,
    pub then: String,
}

/// Something to find: an item, coins, or both.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Loot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coins: Option<u32>,
    /// Where or on whom it is found.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub found: String,
    /// Only found by searching (or asking well): never shown before.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XpAward {
    pub amount: u32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exit {
    pub to: Id,
    pub label: String,
}

/// Why this player character, in particular, acts in a scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerHook {
    pub character: Id,
    pub reason: String,
}

/// A conclusion the players must be able to draw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revelation {
    pub id: Id,
    /// The conclusion, in the GM's words.
    pub statement: String,
    pub importance: Importance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Importance {
    /// Needed to go on: reachable through at least three clues placed
    /// in three different nodes.
    Critical,
    Optional,
}

/// One way to reach a revelation, placed in one node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clue {
    pub id: Id,
    pub revelation: Id,
    pub node: Id,
    /// What the players learn — the only part they ever see.
    pub text: String,
    /// How it is found (GM).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub discovery: String,
    /// Who or what gives it: an NPC, an item or a location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<ClueCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClueCheck {
    pub stat: String,
    pub difficulty: u32,
}

/// A non-player character, as Romain's NPC sheet (`template_pnj.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    pub id: Id,
    pub name: String,
    /// Nickname or title.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub age: String,
    /// What the players see of them.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub appearance: String,
    /// The portrait, described for pixel art.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub portrait: String,
    /// One line to play them by (GM).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub roleplay: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub flaw: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub motivation: String,
    pub disposition: Disposition,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub wants: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hides: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<StatBlock>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inventory: Vec<Holding>,
    /// What they sell, when they keep a shop.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sells: Vec<ShopEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Id>,
    /// Always with the party (the Brasier's ship AI): the co-GM may
    /// speak for them in any scene.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub permanent: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gm_notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Friendly,
    Neutral,
    Hostile,
}

/// Game statistics as written in the prep. Ability keys and damage
/// formulas are rule-system data: the rules engine reads and checks
/// them, this model only carries them.
///
/// Rules live in one copy (`docs/lecons-des-parties.md` §2): when the
/// rule system already has the stat block, `from_rules` names it and
/// the numbers stay there. Fields written next to it override it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatBlock {
    /// An adversary id of the campaign's rule system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_rules: Option<Id>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub abilities: BTreeMap<String, i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_class: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_points: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attacks: Vec<Attack>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attack {
    pub name: String,
    pub damage: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Holding {
    pub item: Id,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShopEntry {
    pub item: Id,
    pub price: u32,
}

/// A stat-block creature with no NPC sheet (« marins de Gueule-Rouge »).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adversary {
    pub id: Id,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// The sprite, described for pixel art.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub art: String,
    pub stats: StatBlock,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gm_notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub id: Id,
    pub name: String,
    /// What the players see there.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// The place containing this one (a district, a ship).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Id>,
    /// The place, described for pixel art.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub art: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gm_notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: Id,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// What it does in play, in words; its rule effects belong to the
    /// rule system.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub effect: String,
    /// The rule system's item carrying its rule effects, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_rules: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<u32>,
    #[serde(default, skip_serializing_if = "Rarity::is_common")]
    pub rarity: Rarity,
    /// The 12 × 12 sprite, described for pixel art.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub art: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gm_notes: String,
}

/// The six tiers of `MEMORY.md` §2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rarity {
    #[default]
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Divine,
}

impl Rarity {
    fn is_common(&self) -> bool {
        *self == Self::Common
    }
}

/// A faction and where the party stands with it. How affinity moves
/// (favour with one lowers its rivals) is `campaign/track-factions-and-goals`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Faction {
    pub id: Id,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// How to deal with them (respect, honesty, the deal and the debt).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub diplomacy: String,
    #[serde(default)]
    pub affinity: Affinity,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rivals: Vec<Id>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub art: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gm_notes: String,
}

/// The affinity gauge's bounds and starting point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Affinity {
    pub start: i32,
    pub min: i32,
    pub max: i32,
}

impl Default for Affinity {
    fn default() -> Self {
        Self {
            start: 0,
            min: -5,
            max: 5,
        }
    }
}

/// A campaign goal to tick off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    pub id: Id,
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// The faction holding it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_by: Option<Id>,
    /// The item that fulfils it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<Id>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gm_notes: String,
}
