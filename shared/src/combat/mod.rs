//! Fights on a grid map, on top of the rules engine (`rules`) and the
//! maps (`maps`): initiative and turn order, the active combatant's turn
//! (move, act, flee, end), attacks gated by range and line of sight with
//! cover, areas resolved from the grid, who leaves the fight, the end of
//! it and the XP gained. Pure: a fight in, the next fight and its events
//! out; dice injected.
//!
//! - [`fight`] — the fight state and its commands
//! - [`reach`] — range, sight, cover and lines on the grid
//! - [`run`] — playing a whole fight with one policy per side
//! - [`scenario`] — an encounter as data: map, rosters, tactics, variants
//! - [`simulate`] — N seeded fights summed up, and two rule versions compared

pub mod fight;
pub mod reach;
pub mod run;
pub mod scenario;
pub mod simulate;

pub use fight::{
    Aim, CombatRefusal, EndReason, Fight, FightEnd, FightEvent, InitiativeRoll, Play, SetupError,
    Standing, Step,
};
pub use run::{Brawler, Decision, FightLog, Focus, Limits, LogEntry, Policy, run_fight};
pub use scenario::{PolicyKind, Scenario, ScenarioError};
pub use simulate::{SimParams, SimReport, TimeModel, compare, simulate};
