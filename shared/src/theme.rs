//! Theme packs (maps/package-theme-packs, maps/build-tileset-packs): what
//! makes a world look like itself without changing a line of the
//! interface, the grid or the rules engine.
//!
//! A theme groups the world's tilesets (one per map `theme` id), its
//! character sprite pack, its title font and its accent colours. The
//! names of the six statistics and the world's own resources stay data
//! of the rule system (`rules::RuleSystem`), read from there.
//!
//! A tileset gives every material (`terrain` of a map's legend) its
//! palette and pattern, the walls their cap and face, and every prop
//! `kind` its shape and colours. The renderer draws the sixteen autotile
//! variants of each material from this (`front/src/features/map`); a
//! material may also carry an atlas generated once by the image model
//! and validated by the GM (`media_assets`, kind `tileset`), which then
//! replaces the drawn tiles.
//!
//! Files: `content/themes/<world>.yaml`. A theme is checked against the
//! maps of its world: every material and prop kind they use is there.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::maps::Map;

/// `#rrggbb`.
pub type Color = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub id: String,
    /// French, shown to the GM.
    pub name: String,
    /// The rule system this world plays (`content/rules/<rules>`).
    pub rules: String,
    /// The character sprite pack (`content/sprites/<pack>`).
    pub sprite_pack: String,
    /// The titles' font, one the front ships (`cinzel`,
    /// `chakra-petch`, `cormorant-garamond`).
    pub title_font: String,
    /// Accent colours of the screens: `accent`, `accent_soft`.
    pub colors: BTreeMap<String, Color>,
    pub tilesets: Vec<Tileset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tileset {
    /// The id maps name in `theme`.
    pub id: String,
    pub name: String,
    /// Floor under the fog and outside the map.
    pub void: Color,
    pub walls: WallLook,
    pub materials: BTreeMap<String, Material>,
    pub props: BTreeMap<String, PropLook>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WallLook {
    /// The top of a wall, drawn one tile up.
    pub cap: Color,
    /// The face seen when the cell below is open.
    pub face: Color,
    /// Joints between stones or plates.
    pub joint: Color,
    pub pattern: WallPattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallPattern {
    Bricks,
    Stones,
    Planks,
    Plates,
}

/// How a material's tile is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    Plain,
    Cobbles,
    Planks,
    Flagstones,
    Grating,
    Plates,
    Water,
    Grass,
    Sand,
    Dirt,
    Rock,
    Asphalt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Material {
    pub name: String,
    pub base: Color,
    pub dark: Color,
    pub light: Color,
    pub pattern: Pattern,
    /// Outdoor ground: its edges melt into the neighbours by noise
    /// (maps/blend-outdoor-terrain) instead of a hard tile border.
    #[serde(default)]
    pub blend: bool,
}

/// How a prop is drawn: the shape and its two colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Crate,
    Crates,
    Barrel,
    Post,
    Rail,
    Mast,
    Rope,
    Hatch,
    Ladder,
    Console,
    Lockers,
    Pipes,
    Reactor,
    Beacon,
    Tree,
    Rock,
    Car,
    Barricade,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropLook {
    pub name: String,
    pub shape: Shape,
    pub color: Color,
    pub accent: Color,
}

/// What a theme lacks for a map, as `(what, id)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Missing {
    Tileset(String),
    Material(String),
    Prop(String),
}

impl Theme {
    /// # Errors
    ///
    /// The YAML is not a theme.
    pub fn from_yaml(text: &str) -> Result<Self, serde_yaml_ng::Error> {
        serde_yaml_ng::from_str(text)
    }

    #[must_use]
    pub fn tileset(&self, id: &str) -> Option<&Tileset> {
        self.tilesets.iter().find(|t| t.id == id)
    }

    /// Everything `map` uses that this theme does not draw.
    #[must_use]
    pub fn missing_for(&self, map: &Map) -> Vec<Missing> {
        let Some(t) = self.tileset(&map.theme) else {
            return vec![Missing::Tileset(map.theme.clone())];
        };
        let mut out = Vec::new();
        for (_, kind) in map.grid.legend() {
            if !kind.wall && !kind.void && !t.materials.contains_key(&kind.terrain) {
                out.push(Missing::Material(kind.terrain.clone()));
            }
        }
        for p in &map.props {
            if !t.props.contains_key(&p.kind) {
                out.push(Missing::Prop(p.kind.clone()));
            }
        }
        for o in &map.objects {
            if !t.props.contains_key(&o.kind) {
                out.push(Missing::Prop(o.kind.clone()));
            }
        }
        out.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        out.dedup();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEMES: [&str; 2] = [
        include_str!("../../content/themes/corsaires.yaml"),
        include_str!("../../content/themes/brasier.yaml"),
    ];
    const MAPS: [&str; 6] = [
        include_str!("../../content/maps/corsaires/quai-port-louis.yaml"),
        include_str!("../../content/maps/brasier/cure-dent-coursive.yaml"),
        include_str!("../../content/maps/corsaires/cotes-bretagne-sud.yaml"),
        include_str!("../../content/maps/corsaires/le-palais.yaml"),
        include_str!("../../content/maps/brasier/systeme-brasier.yaml"),
        include_str!("../../content/maps/brasier/reliquaire-sereth.yaml"),
    ];

    #[test]
    fn each_world_draws_every_material_and_prop_of_its_maps() {
        let themes: Vec<Theme> = THEMES
            .iter()
            .map(|t| Theme::from_yaml(t).unwrap())
            .collect();
        for text in MAPS {
            let map = Map::from_yaml(text).unwrap();
            let theme = themes
                .iter()
                .find(|t| t.tileset(&map.theme).is_some())
                .unwrap_or_else(|| panic!("no theme draws {}", map.theme));
            assert_eq!(theme.missing_for(&map), [], "{}", map.id);
        }
    }

    #[test]
    fn a_missing_material_is_named() {
        let theme = Theme::from_yaml(THEMES[0]).unwrap();
        let mut map = Map::from_yaml(MAPS[1]).unwrap();
        map.theme = theme.tilesets[0].id.clone();
        assert!(
            theme
                .missing_for(&map)
                .contains(&Missing::Material("métal".into()))
        );
    }
}
