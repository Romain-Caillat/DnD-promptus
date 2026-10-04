//! The campaign: story graph, entities, living world state
//! (`campaign/model-story-graph`).
//!
//! - [`model`] — the prepared campaign, one document with stable ids;
//! - [`validate`] — checks that report and never block;
//! - [`world`] — what changed in play, and the pure operations on it;
//! - [`yaml`] — import and export of a whole campaign.

pub mod model;
pub mod validate;
pub mod world;
pub mod yaml;

pub use model::*;
pub use validate::{Issue, Severity, validate};
pub use world::{ClueReveal, FlagValue, FrontAdvance, NodeStatus, WorldError, WorldState};
pub use yaml::{YamlError, from_yaml, to_yaml};
