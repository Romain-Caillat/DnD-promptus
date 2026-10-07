//! The rules engine: a rule system is data (`content/rules/<id>/v<n>.yaml`)
//! and these modules apply it. Pure functions over explicit state; dice
//! come from an injected [`dice::DiceSource`].
//!
//! - [`model`] / [`load`] — the rule system and its validation
//! - [`dice`], [`formula`] — dice expressions, the formula language
//! - [`check`] — checks, outcome bands, group checks
//! - [`sheet`] — combatants and the scene they are in
//! - [`action`] — resolving an action (to-hit, damage, effects, cooldown, XP)
//! - [`conditions`] — conditions and the turn boundaries
//! - [`progression`] — XP, upgrade points (earned and spent), levels
//! - [`trade`] — a shop's prices after a haggle
//! - [`lint`] — whether the rules are good: coherence and balance checks
//!   that report and never block (`lint::lint`, `lint::balance_report`)
//! - [`character`] — whether a player's character follows the rules
//!   (reports, never blocks)
//! - [`changes`] — what changed between two versions, as players read it
//! - [`variant`] — "what if" edits applied to a draft before it loads

pub mod action;
pub mod changes;
pub mod character;
pub mod check;
pub mod conditions;
pub mod dice;
pub mod events;
pub mod formula;
pub mod lint;
pub mod load;
pub mod model;
pub mod progression;
pub mod sheet;
pub mod trade;
pub mod variant;

pub use character::{CharacterInput, check_character};
pub use lint::{BalanceParams, BalanceReport, balance_report, lint, lint_with};
pub use load::{ErrorCode, LoadError, RuleError};
pub use model::RuleSystem;
