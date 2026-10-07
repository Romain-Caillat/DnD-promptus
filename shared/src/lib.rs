//! Pure Rust code shared by the server and the Tauri shell: the rules
//! engine and the domain models. No I/O lives here — callers read files
//! and hand the text over; the server stays authoritative on every rule,
//! and this crate is what it applies.
//!
//! `rules` is the rules engine, `story` the campaign graph, `maps` the
//! grid maps and their geometry, `combat` fights played on those maps,
//! `sprite` the layered pixel characters, `travel` the party's journeys
//! on a world map; `issue` what their checks
//! report.

pub mod combat;
pub mod issue;
pub mod maps;
pub mod rules;
pub mod sprite;
pub mod story;
pub mod theme;
pub mod travel;
