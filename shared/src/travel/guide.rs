//! A world map's travel guide: how fast the party crosses each terrain,
//! how a day is cut, what it eats, who keeps watch, and the events each
//! terrain can bring.
//!
//! The guide sits next to the map, not in it: the map stays the grid the
//! rules read (`maps/model-grid-maps`), and the same hexes can be sailed
//! with one guide and walked with another. A world map without a guide
//! travels by [`Guide::default_for`]: one step per open hex, two through
//! difficult terrain or shallow water, nothing through walls, voids or
//! deep water.
//!
//! The event tables are GM material: the server draws from them and the
//! GM keeps one or none; players only ever receive the event kept.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::maps::{Cell, Map, Scale, Water};

/// Steps a party covers in one portion of a day, unless the guide says.
pub const DEFAULT_STEPS: u32 = 2;

/// The travel rules of one world map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guide {
    /// The world map it applies to.
    pub map: String,
    /// The portions of a travelling day, in order; night follows the last.
    #[serde(default = "default_portions")]
    pub portions: Vec<String>,
    /// Steps covered in one portion. Entering a hex costs its terrain's
    /// steps; what is left over carries to the next portion, so a hex
    /// dearer than one portion is crossed over several.
    #[serde(default = "default_steps")]
    pub steps_per_portion: u32,
    #[serde(default)]
    pub supplies: Supplies,
    /// The night's watches, in order.
    #[serde(default = "default_watches")]
    pub watches: Vec<String>,
    /// By terrain id (the map legend's `terrain`).
    #[serde(default)]
    pub terrains: BTreeMap<String, Terrain>,
    /// Events that can happen on any terrain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<TravelEvent>,
}

/// What the party lives on while travelling: rations, food and water,
/// the ship's energy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supplies {
    pub name: String,
    /// Eaten at each dawn, per party member.
    pub per_person_per_day: u32,
}

impl Default for Supplies {
    fn default() -> Self {
        Self {
            name: "Rations".into(),
            per_person_per_day: 1,
        }
    }
}

/// How one terrain is crossed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terrain {
    /// Shown to people (« la forêt », « la haute mer »).
    pub name: String,
    /// Steps to enter one hex; the map's cell flags decide when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<u32>,
    /// Nobody crosses it this way (land for a ship, the star itself).
    #[serde(default, skip_serializing_if = "is_false")]
    pub impassable: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<TravelEvent>,
}

/// One line of an event table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TravelEvent {
    pub id: String,
    pub title: String,
    /// What the table hears when the GM keeps it.
    pub text: String,
    /// For the GM only: where it leads, what it hides.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gm_notes: String,
}

/// What is wrong with a guide, for logs and tests.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GuideIssue {
    #[error("guide for {guide:?} read against map {map:?}")]
    WrongMap { guide: String, map: String },
    #[error("map {0:?} is not a world map")]
    NotWorld(String),
    #[error("the day has no portion")]
    NoPortion,
    #[error("a portion covers no step")]
    NoStep,
    #[error("the night has no watch")]
    NoWatch,
    #[error("terrain {0:?} is not on the map")]
    UnknownTerrain(String),
    #[error("terrain {0:?} costs no step")]
    FreeTerrain(String),
    #[error("event id {0:?} is used twice")]
    DuplicateEvent(String),
}

fn default_portions() -> Vec<String> {
    vec!["Matin".into(), "Après-midi".into(), "Soir".into()]
}

fn default_steps() -> u32 {
    DEFAULT_STEPS
}

fn default_watches() -> Vec<String> {
    vec![
        "Début de nuit".into(),
        "Milieu de nuit".into(),
        "Fin de nuit".into(),
    ]
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Guide {
    /// # Errors
    ///
    /// The YAML is not a guide.
    pub fn from_yaml(text: &str) -> Result<Self, serde_yaml_ng::Error> {
        serde_yaml_ng::from_str(text)
    }

    /// The guide of a world map that has none written.
    #[must_use]
    pub fn default_for(map: &Map) -> Self {
        Self {
            map: map.id.clone(),
            portions: default_portions(),
            steps_per_portion: DEFAULT_STEPS,
            supplies: Supplies::default(),
            watches: default_watches(),
            terrains: BTreeMap::new(),
            events: Vec::new(),
        }
    }

    /// Everything that does not hold against `map`.
    #[must_use]
    pub fn issues(&self, map: &Map) -> Vec<GuideIssue> {
        let mut out = Vec::new();
        if self.map != map.id {
            out.push(GuideIssue::WrongMap {
                guide: self.map.clone(),
                map: map.id.clone(),
            });
        }
        if map.scale != Scale::World {
            out.push(GuideIssue::NotWorld(map.id.clone()));
        }
        if self.portions.is_empty() {
            out.push(GuideIssue::NoPortion);
        }
        if self.steps_per_portion == 0 {
            out.push(GuideIssue::NoStep);
        }
        if self.watches.is_empty() {
            out.push(GuideIssue::NoWatch);
        }
        for (id, t) in &self.terrains {
            if !map.grid.legend().any(|(_, k)| &k.terrain == id) {
                out.push(GuideIssue::UnknownTerrain(id.clone()));
            }
            if t.cost == Some(0) && !t.impassable {
                out.push(GuideIssue::FreeTerrain(id.clone()));
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for e in self.all_events() {
            if !seen.insert(e.id.as_str()) {
                out.push(GuideIssue::DuplicateEvent(e.id.clone()));
            }
        }
        out
    }

    fn all_events(&self) -> impl Iterator<Item = &TravelEvent> {
        self.terrains
            .values()
            .flat_map(|t| t.events.iter())
            .chain(self.events.iter())
    }

    /// Steps to enter `cell`, or `None` when it cannot be entered.
    #[must_use]
    pub fn cost(&self, map: &Map, cell: Cell) -> Option<u32> {
        let kind = map.grid.kind(cell)?;
        match self.terrains.get(&kind.terrain) {
            Some(t) if t.impassable => None,
            Some(Terrain { cost: Some(c), .. }) => Some((*c).max(1)),
            _ => {
                if kind.wall || kind.void || kind.water == Water::Deep {
                    None
                } else if kind.difficult || kind.water == Water::Shallow {
                    Some(2)
                } else {
                    Some(1)
                }
            }
        }
    }

    /// The terrain's name for people: the guide's, else the map's id.
    #[must_use]
    pub fn terrain_name(&self, map: &Map, cell: Cell) -> String {
        let Some(kind) = map.grid.kind(cell) else {
            return String::new();
        };
        self.terrains
            .get(&kind.terrain)
            .map_or_else(|| kind.terrain.clone(), |t| t.name.clone())
    }

    /// The events that can happen on `cell`: its terrain's table, then
    /// the guide's general one.
    #[must_use]
    pub fn events_at(&self, map: &Map, cell: Cell) -> Vec<&TravelEvent> {
        let own = map
            .grid
            .kind(cell)
            .and_then(|k| self.terrains.get(&k.terrain))
            .map(|t| t.events.iter())
            .into_iter()
            .flatten();
        own.chain(self.events.iter()).collect()
    }

    /// An event of any table, by id.
    #[must_use]
    pub fn event(&self, id: &str) -> Option<&TravelEvent> {
        self.all_events().find(|e| e.id == id)
    }

    /// Portions in one day.
    #[must_use]
    pub fn portions_per_day(&self) -> u32 {
        u32::try_from(self.portions.len())
            .unwrap_or(u32::MAX)
            .max(1)
    }
}
