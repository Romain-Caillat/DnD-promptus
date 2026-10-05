//! A character's look: what it is made of and in which colours, never
//! an image. The map, the TV, the sheet and the creator all draw it
//! from this description.

use serde::{Deserialize, Serialize};

/// The description of a character, by stable ids into one pack.
///
/// Colours are a swatch id of the pack's palette (`skin` for the skin,
/// `hair` for hair and beard, `cloth` for the dye and accent of worn
/// pieces) or `#RRGGBB`. A worn piece without colours keeps its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterLook {
    pub pack: String,
    pub body: String,
    pub skin: String,
    pub hair: Hair,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beard: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headwear: Option<Worn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outfit: Option<Worn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armour: Option<Worn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weapon: Option<Worn>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accessories: Vec<Worn>,
}

/// Hair and beard share one colour. No style is a shaved head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hair {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    pub colour: String,
}

/// A piece the character wears or holds, optionally recoloured.
/// Written either as its id alone (`weapon: sabre`) or in full.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "WornFile")]
pub struct Worn {
    pub piece: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dye: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
}

impl Worn {
    pub fn plain(piece: &str) -> Self {
        Self {
            piece: piece.to_string(),
            dye: None,
            accent: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum WornFile {
    Id(String),
    Full(WornFull),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WornFull {
    piece: String,
    #[serde(default)]
    dye: Option<String>,
    #[serde(default)]
    accent: Option<String>,
}

impl From<WornFile> for Worn {
    fn from(f: WornFile) -> Self {
        match f {
            WornFile::Id(piece) => Self::plain(&piece),
            WornFile::Full(WornFull { piece, dye, accent }) => Self { piece, dye, accent },
        }
    }
}

/// The looks of one world (`content/sprites/looks/<world>.yaml`): its
/// party slots, drawn facing right, and its foes, drawn facing left.
/// Ids are the campaign's party slot, NPC and adversary ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LookBook {
    pub format: u32,
    pub world: String,
    pub campaign: String,
    pub party: Vec<NamedLook>,
    pub foes: Vec<NamedLook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedLook {
    pub id: String,
    pub look: CharacterLook,
}

impl LookBook {
    /// # Errors
    ///
    /// The YAML error when the file does not parse.
    pub fn from_yaml(text: &str) -> Result<Self, serde_yaml_ng::Error> {
        serde_yaml_ng::from_str(text)
    }
}
