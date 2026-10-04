//! The rules view of a map: one record per cell, built from the grid,
//! the doors and the props of every *real* layer (all and GM; never the
//! players-only illusions). Movement and sight read only this.

use super::grid::Cell;
use super::model::{Cover, DoorState, Map, Water};

/// What stops a creature from entering a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Obstacle {
    OutOfMap,
    Wall,
    Void,
    /// A closed or locked door, by id.
    Door {
        id: String,
        state: DoorState,
    },
    /// A prop that blocks movement, by id.
    Prop(String),
    DeepWater,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hard {
    Wall,
    Void,
    Door(usize),
    Prop(usize),
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Info {
    pub hard: Option<Hard>,
    pub opaque: bool,
    pub difficult: bool,
    pub deep: bool,
    pub elevation: i8,
    pub climbable: bool,
    /// Best cover a prop in this cell gives (below total).
    pub cover: Cover,
}

pub(crate) struct Board<'a> {
    pub map: &'a Map,
    info: Vec<Info>,
}

impl<'a> Board<'a> {
    pub fn new(map: &'a Map) -> Self {
        let grid = &map.grid;
        let mut info: Vec<Info> = grid
            .cells()
            .map(|c| {
                let k = grid.kind(c).expect("cell from the grid");
                Info {
                    hard: if k.void {
                        Some(Hard::Void)
                    } else if k.wall {
                        Some(Hard::Wall)
                    } else {
                        None
                    },
                    opaque: k.wall,
                    difficult: k.difficult || k.water == Water::Shallow,
                    deep: k.water == Water::Deep,
                    elevation: k.elevation,
                    climbable: false,
                    cover: Cover::None,
                }
            })
            .collect();

        for (i, door) in map.doors.iter().enumerate() {
            if !map.visibility(&door.layer).is_real() {
                continue;
            }
            if let Some(idx) = grid.index(door.at) {
                let cell = &mut info[idx];
                if door.state == DoorState::Open {
                    cell.hard = None;
                    cell.opaque = false;
                } else {
                    cell.hard = Some(Hard::Door(i));
                    cell.opaque = true;
                }
            }
        }

        for (i, prop) in map.props.iter().enumerate() {
            if !map.visibility(&prop.layer).is_real() {
                continue;
            }
            for c in prop.cells() {
                let Some(idx) = grid.index(c) else { continue };
                let cell = &mut info[idx];
                if prop.blocks_movement && cell.hard.is_none() {
                    cell.hard = Some(Hard::Prop(i));
                }
                if prop.cover == Cover::Total {
                    cell.opaque = true;
                } else {
                    cell.cover = cell.cover.max(prop.cover);
                }
                cell.difficult |= prop.difficult;
                cell.climbable |= prop.climbable;
            }
        }
        Self { map, info }
    }

    pub fn get(&self, c: Cell) -> Option<&Info> {
        self.map.grid.index(c).map(|i| &self.info[i])
    }

    pub fn obstacle(&self, hard: Hard) -> Obstacle {
        match hard {
            Hard::Wall => Obstacle::Wall,
            Hard::Void => Obstacle::Void,
            Hard::Door(i) => {
                let d = &self.map.doors[i];
                Obstacle::Door {
                    id: d.id.clone(),
                    state: d.state,
                }
            }
            Hard::Prop(i) => Obstacle::Prop(self.map.props[i].id.clone()),
        }
    }
}
