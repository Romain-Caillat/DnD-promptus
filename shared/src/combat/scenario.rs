//! An encounter written as data, so it can be played again and again:
//! which map, who fights on each side (class or stat block, level,
//! start cell), how each side plays, the doors' state, and the rule
//! changes to try on it. Scenarios live in
//! `content/scenarios/<world>/<id>.yaml`; the fight simulator
//! ([`super::simulate`]) and the engine's tests both start fights from
//! them.
//!
//! ```yaml
//! id: bagarre-du-quai
//! name: La bagarre du quai
//! rules: corsaires            # content/rules/<rules>/v<version>.yaml
//! version: 1
//! map: quai-port-louis        # content/maps/<rules>/<map>.yaml
//! context: sol                # the rule system's turn context
//! doors: { porte-chantier: closed }
//! party:
//!   policies: [brawler, focus]
//!   fighters:
//!     - { id: bretteur, class: bretteur }             # level 1, next free start
//! opposition:
//!   policies: [brawler]
//!   morale: { leader: gueule_rouge, flee_when_fallen: 3 }
//!   fighters:
//!     - { id: gueule_rouge, adversary: gueule_rouge, name: Gueule-Rouge }
//! adversaries: []             # stand-in stat blocks the rules lack
//! simulation: { fights: 200, seed: 1 }
//! variants: []                # rules::variant::Variant
//! ```

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;

use crate::maps::{Cell, DoorState, Map, Side as MapSide};
use crate::rules::RuleSystem;
use crate::rules::dice::SeededDice;
use crate::rules::sheet::{Combatant, Side};
use crate::rules::variant::{BuildError, Variant, build_system};

use super::fight::{Fight, SetupError, Standing, Step};
use super::run::{Brawler, Decision, Focus, Policy};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    /// French, shown in the report.
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Where the encounter comes from (a scene of the source).
    #[serde(default)]
    pub source: String,
    /// The rule system's id, and the version the scenario is written for.
    pub rules: String,
    pub version: u32,
    /// The map's id, in the rule system's world.
    pub map: String,
    /// The turn context the fight runs in.
    pub context: String,
    /// Door states set before the fight.
    #[serde(default)]
    pub doors: BTreeMap<String, DoorState>,
    pub party: SideSpec,
    pub opposition: SideSpec,
    /// Stat blocks the rule system does not have yet, added to it for
    /// this scenario only (a block the rules already have wins).
    #[serde(default)]
    pub adversaries: Vec<Value>,
    #[serde(default)]
    pub simulation: SimulationSpec,
    #[serde(default)]
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SideSpec {
    /// The ways this side may play; the simulator tries each.
    #[serde(default = "default_policies")]
    pub policies: Vec<PolicyKind>,
    #[serde(default)]
    pub morale: Option<Morale>,
    pub fighters: Vec<FighterSpec>,
}

fn default_policies() -> Vec<PolicyKind> {
    vec![PolicyKind::Brawler]
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FighterSpec {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// A player character of this class…
    #[serde(default)]
    pub class: Option<String>,
    /// …or one adversary from this stat block.
    #[serde(default)]
    pub adversary: Option<String>,
    /// Player characters only; the XP threshold of that level.
    #[serde(default = "one")]
    pub level: u32,
    /// Absent: the map's next start cell for this side, in file order.
    #[serde(default)]
    pub at: Option<Cell>,
}

fn one() -> u32 {
    1
}

/// When the members of a side run, as a stat block's tactics say ("if
/// three sailors fall, the other two flee; if Gueule-Rouge falls,
/// everyone flees"). Checked at the start of each member's turn; the
/// leader never runs.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Morale {
    #[serde(default)]
    pub leader: Option<String>,
    /// How many of the others must be out of the fight.
    #[serde(default)]
    pub flee_when_fallen: Option<usize>,
    /// When `flee_when_fallen` is reached, the leader sounds the retreat
    /// and runs too.
    #[serde(default)]
    pub retreat_all: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyKind {
    /// [`Brawler`]: nearest enemy, walks straight at them.
    Brawler,
    /// [`Focus`]: weakest enemy in reach, no stopping in doorways.
    Focus,
}

impl PolicyKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Brawler => "brawler",
            Self::Focus => "focus",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SimulationSpec {
    #[serde(default = "default_fights")]
    pub fights: u32,
    #[serde(default = "one_u64")]
    pub seed: u64,
}

fn default_fights() -> u32 {
    100
}

fn one_u64() -> u64 {
    1
}

impl Default for SimulationSpec {
    fn default() -> Self {
        Self {
            fights: default_fights(),
            seed: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioError {
    Yaml(String),
    /// A fighter is neither a class nor a stat block, or both.
    FighterKind(String),
    UnknownLevel {
        fighter: String,
        level: u32,
    },
    UnknownDoor(String),
    /// More fighters on a side than the map has start cells for it.
    NoStartLeft(String),
    Setup(SetupError),
}

impl fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml(e) => write!(f, "YAML: {e}"),
            Self::FighterKind(id) => {
                write!(
                    f,
                    "fighter `{id}` needs exactly one of `class`, `adversary`"
                )
            }
            Self::UnknownLevel { fighter, level } => {
                write!(f, "fighter `{fighter}`: level {level} is not in the rules")
            }
            Self::UnknownDoor(id) => write!(f, "no door `{id}` on the map"),
            Self::NoStartLeft(id) => write!(f, "fighter `{id}`: no start cell left on the map"),
            Self::Setup(e) => write!(f, "the fight cannot start: {e:?}"),
        }
    }
}

impl Scenario {
    pub fn from_yaml(text: &str) -> Result<Self, ScenarioError> {
        serde_yaml_ng::from_str(text).map_err(|e| ScenarioError::Yaml(e.to_string()))
    }

    /// The rule system this scenario fights under: the draft's text,
    /// plus the scenario's stand-in stat blocks, plus `variant` if any.
    pub fn system(
        &self,
        rules_text: &str,
        variant: Option<&Variant>,
    ) -> Result<RuleSystem, BuildError> {
        build_system(rules_text, &self.adversaries, variant)
    }

    pub fn variant(&self, id: &str) -> Option<&Variant> {
        self.variants.iter().find(|v| v.id == id)
    }

    /// The map with this scenario's door states.
    pub fn prepare_map(&self, mut map: Map) -> Result<Map, ScenarioError> {
        for (id, state) in &self.doors {
            if !map.set_door_state(id, *state) {
                return Err(ScenarioError::UnknownDoor(id.clone()));
            }
        }
        Ok(map)
    }

    fn side(&self, side: Side) -> &SideSpec {
        match side {
            Side::Party => &self.party,
            Side::Opposition => &self.opposition,
        }
    }

    /// Who stands where: the party, then the opposition, each in file
    /// order (the order breaks initiative ties).
    pub fn placements(
        &self,
        system: &RuleSystem,
        map: &Map,
    ) -> Result<Vec<(Combatant, Cell)>, ScenarioError> {
        let mut out = Vec::new();
        for (side, map_side) in [
            (Side::Party, MapSide::Party),
            (Side::Opposition, MapSide::Foes),
        ] {
            let mut starts = map
                .starts
                .iter()
                .filter(|s| s.side == Some(map_side))
                .map(|s| s.at);
            for f in &self.side(side).fighters {
                let name = f.name.as_deref().unwrap_or(&f.id);
                let combatant = match (&f.class, &f.adversary) {
                    (Some(class), None) => {
                        let mut c = Combatant::from_class(system, &f.id, name, class)
                            .map_err(|e| ScenarioError::Setup(SetupError::Sheet(e)))?;
                        let xp = system
                            .progression
                            .levels
                            .iter()
                            .find(|l| l.level == f.level)
                            .ok_or_else(|| ScenarioError::UnknownLevel {
                                fighter: f.id.clone(),
                                level: f.level,
                            })?
                            .xp;
                        if let Some(p) = c.progress.as_mut() {
                            p.total_xp = xp;
                        }
                        c
                    }
                    (None, Some(block)) => Combatant::from_adversary(system, &f.id, name, block)
                        .map_err(|e| ScenarioError::Setup(SetupError::Sheet(e)))?,
                    _ => return Err(ScenarioError::FighterKind(f.id.clone())),
                };
                let at = match f.at {
                    Some(at) => at,
                    None => starts
                        .next()
                        .ok_or_else(|| ScenarioError::NoStartLeft(f.id.clone()))?,
                };
                out.push((combatant, at));
            }
        }
        Ok(out)
    }

    /// Opens the fight with dice seeded by `seed`; the same dice go on
    /// to play it.
    pub fn start(
        &self,
        system: &RuleSystem,
        map: &Map,
        seed: u64,
    ) -> Result<(Step, SeededDice), ScenarioError> {
        let map = self.prepare_map(map.clone())?;
        let placements = self.placements(system, &map)?;
        let mut dice = SeededDice::new(seed);
        let step = Fight::start(system, &self.context, map, placements, &mut dice)
            .map_err(ScenarioError::Setup)?;
        Ok((step, dice))
    }

    /// How `side` plays with `kind`, under its morale.
    pub fn policy(&self, side: Side, kind: PolicyKind) -> Box<dyn Policy> {
        let base: Box<dyn Policy> = match kind {
            PolicyKind::Brawler => Box::new(Brawler::default()),
            PolicyKind::Focus => Box::new(Focus::default()),
        };
        let spec = self.side(side);
        match &spec.morale {
            Some(m) => Box::new(WithMorale {
                morale: m.clone(),
                members: spec.fighters.iter().map(|f| f.id.clone()).collect(),
                context: self.context.clone(),
                base,
            }),
            None => base,
        }
    }
}

/// A policy that runs when the side's morale breaks, and otherwise
/// plays the base policy.
struct WithMorale {
    morale: Morale,
    members: Vec<String>,
    context: String,
    base: Box<dyn Policy>,
}

impl Policy for WithMorale {
    fn decide(&mut self, system: &RuleSystem, fight: &Fight, who: &str) -> Decision {
        let leader = self.morale.leader.as_deref();
        let down = |id: &str| fight.standing.get(id) != Some(&Standing::InFight);
        let fallen = self
            .members
            .iter()
            .filter(|id| Some(id.as_str()) != leader && down(id))
            .count();
        let full = system
            .turn_context(&self.context)
            .map_or(0, |c| c.actions_per_turn);
        let fresh = fight
            .combatant(who)
            .is_some_and(|c| c.turn.actions_left == full);
        let routed = self.morale.flee_when_fallen.is_some_and(|n| fallen >= n);
        let broken = if Some(who) == leader {
            self.morale.retreat_all && routed
        } else {
            leader.is_some_and(down) || routed
        };
        if fresh && broken {
            return Decision::Flee { difficulty: None };
        }
        self.base.decide(system, fight, who)
    }
}
