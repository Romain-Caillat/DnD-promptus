//! Grid maps: the model every other part reads (rules, rendering, fog,
//! generation), and the rules the server applies on it — movement,
//! range, line of sight and cover.
//!
//! The grid carries the rules; drawing is only a projection of it.
//! File format: `docs/map-format.md`. Fixtures: `content/maps/`.

mod board;
mod format;
mod grid;
mod hex;
mod model;
mod movement;
mod projection;
mod sight;
mod v1;

pub use board::Obstacle;
pub use format::{Issue, MapError};
pub use grid::{Cell, Diagonal, Direction, Geometry, adjacent, distance, neighbours};
pub use hex::Hex;
pub use model::*;
pub use movement::{
    MovementRules, Occupancy, PathError, check_path, reachable, shortest_path, standable,
};
pub use sight::{Sight, illumination, line_of_sight, visible_cells};
pub use v1::{V1Map, load_v1_story_maps};
