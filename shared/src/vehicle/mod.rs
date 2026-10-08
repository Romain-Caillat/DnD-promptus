//! Vehicle combat (engine/support-vehicle-combat): ships on a grid with
//! a bow and firing arcs, a crew at their stations, power split between
//! channels, damage aboard, morale as a second way to win, boarding that
//! hands over to a fight on the deck. One system, two skins — the
//! Corsaires' brig and the Brasier's Cure-Dent — written as data in the
//! rule system's `vehicles:` block.
//!
//! - [`model`] — the `vehicles:` block; [`load`] validates it
//! - [`geometry`] — facing, arcs, distance
//! - [`battle`] — the battle state and its commands
//! - [`policy`] — how a crew and an enemy ship play, for the simulator
//!   and the co-GM's proposals
//! - [`scenario`] — a ship battle written as data, simulated N times

pub mod battle;
pub mod geometry;
pub mod load;
pub mod model;
pub mod policy;
pub mod scenario;

pub use battle::{
    Aim, Battle, BattleEnd, BattleEndReason, BattleEvent, BattleRefusal, Crew, Placed, Setup, Ship,
    ShipStanding, Step,
};
pub use geometry::{Arc, Facing};
pub use model::VehicleRules;
