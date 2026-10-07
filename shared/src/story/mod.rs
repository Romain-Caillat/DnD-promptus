//! The campaign: story graph, entities, living world state
//! (`campaign/model-story-graph`).
//!
//! - [`model`] — the prepared campaign, one document with stable ids;
//! - [`validate`] — checks that report and never block;
//! - [`world`] — what changed in play, and the pure operations on it;
//! - [`yaml`] — import and export of a whole campaign;
//! - [`library`] — checks against the rule system and the maps;
//! - [`encounter`] — a scene's planned fight, staged for the simulator;
//! - [`edit`] — changes by id, from the review screen and the co-GM;
//! - [`readiness`] — one gauge per act: is it ready to be played;
//! - [`prune`] — dropping the ids a model invented;
//! - [`recap`] — what a session changed, and the recaps written from it;
//! - [`v1`] — importer for V1 campaigns (entities YAML + story JSON).

pub mod edit;
pub mod encounter;
pub mod library;
pub mod model;
pub mod prune;
pub mod readiness;
pub mod recap;
pub mod v1;
pub mod validate;
pub mod world;
pub mod yaml;

pub use encounter::{StagingError, encounter_scenario};
pub use library::{Library, validate_with};
pub use model::*;
pub use readiness::{ActReadiness, readiness};
pub use validate::{Issue, Severity, validate};
pub use world::{
    AffinityShift, ClueReveal, FlagValue, FrontAdvance, NodeStatus, WorldError, WorldState,
};
pub use yaml::{YamlError, from_yaml, to_yaml};
