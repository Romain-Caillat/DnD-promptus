//! A ship battle written as data, played again and again with the
//! reference tactics ([`super::policy`]) before a table plays it — the
//! « un tour de vaisseau simulé avant d'en jouer un » of
//! `docs/lecons-des-parties.md`. Scenarios live in
//! `content/scenarios/<world>/vaisseau/<id>.yaml`:
//!
//! ```yaml
//! id: essaim-vorr
//! name: L'essaim vorr
//! rules: brasier                 # content/rules/<rules>/v<version>.yaml
//! version: 1
//! map: abords-du-toboggan        # content/maps/<rules>/<map>.yaml
//! ship: { kind: cure_dent, facing: e }      # on the map's first party start
//! crew:
//!   - { id: pilote, class: pilote }          # seated by the stations' `suits`
//!   - { id: mecano, class: mecano, station: ingenieur }
//! opponents:
//!   - { id: chasseur_1, kind: chasseur_vorr, facing: w }   # foes starts, in order
//! current: w                     # where the wind / pull comes from (default: the map's)
//! simulation: { battles: 200, seed: 1 }
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::combat::simulate::Spread;
use crate::maps::{Direction, Map, Side as MapSide};
use crate::rules::RuleSystem;
use crate::rules::dice::SeededDice;
use crate::rules::sheet::{Combatant, Side};

use super::battle::{Battle, BattleEndReason, BattleEvent, Crew, Placed, Setup, ShipStanding};
use super::geometry::Facing;
use super::policy::{run_battle, seat_by_class};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleScenario {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
    pub rules: String,
    pub version: u32,
    pub map: String,
    pub ship: ShipSpec,
    pub crew: Vec<CrewSpec>,
    pub opponents: Vec<ShipSpec>,
    #[serde(default)]
    pub current: Option<Direction>,
    #[serde(default)]
    pub simulation: BattleSpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShipSpec {
    #[serde(default)]
    pub id: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub name: Option<String>,
    pub facing: Facing,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrewSpec {
    pub id: String,
    pub class: String,
    /// Absent: the first free station that suits the class.
    #[serde(default)]
    pub station: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleSpec {
    #[serde(default = "default_battles")]
    pub battles: u32,
    #[serde(default = "one")]
    pub seed: u64,
}

fn default_battles() -> u32 {
    100
}

fn one() -> u64 {
    1
}

impl Default for BattleSpec {
    fn default() -> Self {
        Self {
            battles: default_battles(),
            seed: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VehicleScenarioError {
    Yaml(String),
    NoVehicleRules,
    UnknownClass(String),
    NoStartLeft(String),
    Setup(String),
}

impl fmt::Display for VehicleScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml(e) => write!(f, "YAML: {e}"),
            Self::NoVehicleRules => write!(f, "the rule system has no `vehicles` block"),
            Self::UnknownClass(c) => write!(f, "no class `{c}`"),
            Self::NoStartLeft(id) => write!(f, "`{id}`: no start cell left on the map"),
            Self::Setup(e) => write!(f, "the battle cannot start: {e}"),
        }
    }
}

/// Minutes at the table. INTERPRETATION until a battle is timed: the
/// crew acts at the same time, so their turn is one bounded block, not
/// a sum over six players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleTime {
    pub setup_seconds: u32,
    pub crew_turn_seconds: u32,
    pub enemy_turn_seconds: u32,
}

impl Default for BattleTime {
    fn default() -> Self {
        Self {
            setup_seconds: 180,
            crew_turn_seconds: 120,
            enemy_turn_seconds: 30,
        }
    }
}

/// N battles of one scenario, summed up.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BattleReport {
    pub scenario: String,
    pub name: String,
    pub battles: u32,
    pub seed: u64,
    pub time: BattleTime,
    pub victories: u32,
    pub defeats: u32,
    pub disengaged: u32,
    pub stalemates: u32,
    pub party_win_rate: f64,
    /// Enemy ships taken out by hull, by morale, over all battles.
    pub destroyed: u32,
    pub struck: u32,
    pub rounds: Spread,
    pub minutes: Spread,
    /// The party ship's hull at the end.
    pub hull_left: Spread,
    /// Damages rolled aboard the party ship, per battle.
    pub damages_aboard: f64,
    /// Orders the engine refused (always 0 unless a tactic is wrong).
    pub refusals: usize,
}

impl VehicleScenario {
    /// The battle a scene of a campaign holds, as a scenario: the
    /// campaign's party (its members with a class) as the crew, the
    /// scene's ships as opponents.
    pub fn from_story(
        campaign: &crate::story::Campaign,
        node: &str,
        v: &crate::story::model::VehicleEncounter,
    ) -> Self {
        let mut opponents = Vec::new();
        for g in &v.ships {
            for n in 1..=g.count.max(1) {
                opponents.push(ShipSpec {
                    id: Some(if g.count > 1 {
                        format!("{}-{n}", g.ship)
                    } else {
                        g.ship.clone()
                    }),
                    kind: g.ship.clone(),
                    name: g.name.as_ref().map(|name| {
                        if g.count > 1 {
                            format!("{name} {n}")
                        } else {
                            name.clone()
                        }
                    }),
                    facing: g.facing,
                });
            }
        }
        Self {
            id: node.into(),
            name: node.into(),
            description: String::new(),
            source: String::new(),
            rules: campaign.rules.id.clone(),
            version: campaign.rules.version,
            map: v.map.clone(),
            ship: ShipSpec {
                id: Some(v.ship.clone()),
                kind: v.ship.clone(),
                name: v.name.clone(),
                facing: v.facing,
            },
            crew: campaign
                .party
                .iter()
                .filter_map(|p| {
                    p.class.as_ref().map(|class| CrewSpec {
                        id: p.id.clone(),
                        class: class.clone(),
                        station: None,
                    })
                })
                .collect(),
            opponents,
            current: None,
            simulation: BattleSpec::default(),
        }
    }

    pub fn from_yaml(text: &str) -> Result<Self, VehicleScenarioError> {
        serde_yaml_ng::from_str(text).map_err(|e| VehicleScenarioError::Yaml(e.to_string()))
    }

    /// The battle at its start (dice rolled for initiative).
    pub fn battle(
        &self,
        system: &RuleSystem,
        map: &Map,
        seed: u64,
    ) -> Result<Battle, VehicleScenarioError> {
        let v = system
            .vehicles
            .as_ref()
            .ok_or(VehicleScenarioError::NoVehicleRules)?;
        let starts = |side: MapSide| {
            map.starts
                .iter()
                .filter(move |s| s.side == Some(side))
                .map(|s| s.at)
        };
        let party_at = starts(MapSide::Party)
            .next()
            .ok_or_else(|| VehicleScenarioError::NoStartLeft(self.ship.kind.clone()))?;
        let mut foes = starts(MapSide::Foes);
        let mut enemies = Vec::new();
        for (i, o) in self.opponents.iter().enumerate() {
            let id =
                o.id.clone()
                    .unwrap_or_else(|| format!("{}-{}", o.kind, i + 1));
            let at = foes
                .next()
                .ok_or_else(|| VehicleScenarioError::NoStartLeft(id.clone()))?;
            enemies.push(Placed {
                id,
                kind: o.kind.clone(),
                name: o.name.clone(),
                at,
                facing: o.facing,
            });
        }
        let mut crew = Vec::new();
        for c in &self.crew {
            let sheet = Combatant::from_class(system, &c.id, &c.id, &c.class)
                .map_err(|_| VehicleScenarioError::UnknownClass(c.class.clone()))?;
            crew.push(Crew::from_combatant(system, &sheet, Some(c.class.clone())));
        }
        let seats = seat_by_class(
            v,
            &self
                .crew
                .iter()
                .filter(|c| c.station.is_none())
                .map(|c| (c.id.clone(), Some(c.class.clone())))
                .collect::<Vec<_>>(),
        );
        for (member, spec) in crew.iter_mut().zip(&self.crew) {
            member.station = spec
                .station
                .clone()
                .or_else(|| seats.get(&spec.id).cloned());
        }
        let mut map = map.clone();
        if let Some(from) = self.current {
            map.ambience.wind = Some(crate::maps::Wind {
                from,
                strength: map.ambience.wind.map_or(3, |w| w.strength),
            });
        }
        let setup = Setup {
            party_ship: Placed {
                id: self
                    .ship
                    .id
                    .clone()
                    .unwrap_or_else(|| self.ship.kind.clone()),
                kind: self.ship.kind.clone(),
                name: self.ship.name.clone(),
                at: party_at,
                facing: self.ship.facing,
            },
            crew,
            enemies,
        };
        let mut dice = SeededDice::new(seed);
        Battle::start(system, map, setup, &mut dice)
            .map(|s| s.battle)
            .map_err(|e| VehicleScenarioError::Setup(format!("{e:?}")))
    }

    /// Plays the scenario's battles (`spec` overrides the file's count
    /// and seed): battle `i` uses seed `seed + i`.
    pub fn simulate(
        &self,
        system: &RuleSystem,
        map: &Map,
        spec: Option<BattleSpec>,
        time: BattleTime,
    ) -> Result<BattleReport, VehicleScenarioError> {
        let spec = spec.unwrap_or(self.simulation);
        let mut r = BattleReport {
            scenario: self.id.clone(),
            name: self.name.clone(),
            battles: spec.battles,
            seed: spec.seed,
            time,
            victories: 0,
            defeats: 0,
            disengaged: 0,
            stalemates: 0,
            party_win_rate: 0.0,
            destroyed: 0,
            struck: 0,
            rounds: Spread::of(Vec::new()),
            minutes: Spread::of(Vec::new()),
            hull_left: Spread::of(Vec::new()),
            damages_aboard: 0.0,
            refusals: 0,
        };
        let (mut rounds, mut minutes, mut hull) = (Vec::new(), Vec::new(), Vec::new());
        let mut damages = 0u32;
        for i in 0..spec.battles {
            let seed = spec.seed + u64::from(i);
            let start = self.battle(system, map, seed)?;
            // The battle's own dice continue from another stream.
            let mut dice = SeededDice::new(seed.wrapping_mul(7919).wrapping_add(17));
            let log = run_battle(system, start, &mut dice);
            r.refusals += log.refusals.len();
            let b = &log.battle;
            match b.end.as_ref().map(|e| e.reason) {
                Some(BattleEndReason::Victory) => r.victories += 1,
                Some(BattleEndReason::Defeat) => r.defeats += 1,
                Some(BattleEndReason::Disengaged) => r.disengaged += 1,
                _ => r.stalemates += 1,
            }
            for s in b.ships.iter().filter(|s| s.side == Side::Opposition) {
                match s.standing {
                    ShipStanding::Destroyed => r.destroyed += 1,
                    ShipStanding::Struck => r.struck += 1,
                    _ => {}
                }
            }
            let (mut crew_turns, mut enemy_turns) = (0u32, 0u32);
            for e in &log.events {
                match e {
                    BattleEvent::TurnStarted { unit, .. } => {
                        let party = b
                            .units
                            .iter()
                            .any(|u| &u.id == unit && u.side == Side::Party);
                        if party {
                            crew_turns += 1;
                        } else {
                            enemy_turns += 1;
                        }
                    }
                    BattleEvent::DamageRolled { .. } => damages += 1,
                    _ => {}
                }
            }
            rounds.push(f64::from(b.end.as_ref().map_or(b.round, |e| e.rounds)));
            let seconds = time.setup_seconds
                + crew_turns * time.crew_turn_seconds
                + enemy_turns * time.enemy_turn_seconds;
            minutes.push(f64::from(seconds) / 60.0);
            hull.push(f64::from(b.party_ship().hull));
        }
        let n = f64::from(spec.battles.max(1));
        r.party_win_rate = f64::from(r.victories) / n;
        r.rounds = Spread::of(rounds);
        r.minutes = Spread::of(minutes);
        r.hull_left = Spread::of(hull);
        r.damages_aboard = f64::from(damages) / n;
        Ok(r)
    }
}
