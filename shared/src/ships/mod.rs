//! engine/support-vehicle-combat — ships fighting on a grid: the
//! rule system's `ship_combat` section ([`model`]), the fight and its
//! orders ([`fight`]), and its rehearsal with seeded dice ([`simulate`]).
//!
//! A ship is a character the whole crew shares (`Combat_Vaisseau.md`
//! §1): hull and shields, a reactor's points shared between navigation,
//! weapons and shields, stations the players hold, weapons with an arc
//! and a range around the bow, damage that sets fires and knocks
//! stations out; morale as the second way to win.

pub mod fight;
pub mod model;
pub mod simulate;

pub use fight::{
    CrewOrders, Facing, Order, Ship, ShipEnd, ShipError, ShipEvent, ShipFight, ShipSide,
};
pub use model::ShipCombat;
pub use simulate::{ShipReport, ShipScenario, run_once, simulate};
