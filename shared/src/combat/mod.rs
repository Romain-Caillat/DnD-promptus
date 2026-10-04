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

pub mod fight;
pub mod reach;
pub mod run;

pub use fight::{
    Aim, CombatRefusal, EndReason, Fight, FightEnd, FightEvent, InitiativeRoll, Play, SetupError,
    Standing, Step,
};
pub use run::{Brawler, Decision, FightLog, Limits, LogEntry, Policy, run_fight};
