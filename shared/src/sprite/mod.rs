//! Layered pixel-art characters (`characters/render-layered-sprite`).
//!
//! A character is a description — [`CharacterLook`]: a body, a skin, hair,
//! worn pieces and their colours, by stable ids into a [`Pack`] — never an
//! image. [`render`] draws it to RGBA, the same way for every caller: the
//! server renders the PNG that the phone, the GM screen and the TV all
//! show (`GET /api/sprites/render.png`), so there is one implementation.
//!
//! Style ported from `docs/design/sprite-prototype.py` and `avatar.py`:
//! a 20 x 26 grid in right profile, three-tone shading (lit top edges,
//! dark back and bottom edges), a dark 1-px outline around the whole
//! silhouette — never over the face — and a drop shadow. Heroes face
//! east, enemies west (the mirror).
//!
//! Conditions show on the character as an effect drawn over the sprite
//! ([`ConditionVisual`]); which condition shows which effect is data in
//! the rule system (`conditions[].visual`).

pub mod colour;
pub mod look;
pub mod pack;
pub mod render;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use look::{CharacterLook, Hair, LookBook, NamedLook, Worn};
pub use pack::{Catalogue, CataloguePiece, Direction, Pack, Slot};
pub use render::{Composite, Image, compose, render};

/// The packs a server knows, by id.
#[derive(Debug, Clone, Default)]
pub struct Packs(BTreeMap<String, Pack>);

impl Packs {
    /// Read every pack file given.
    ///
    /// # Errors
    ///
    /// The first pack that does not load, or two packs with one id.
    pub fn from_yaml<'a>(files: impl IntoIterator<Item = &'a str>) -> Result<Self, SpriteError> {
        let mut packs = BTreeMap::new();
        for text in files {
            let pack = Pack::from_yaml(text)?;
            if packs.contains_key(&pack.id) {
                return Err(SpriteError::Pack(format!("pack {} defined twice", pack.id)));
            }
            packs.insert(pack.id.clone(), pack);
        }
        Ok(Self(packs))
    }

    pub fn get(&self, id: &str) -> Option<&Pack> {
        self.0.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Pack> {
        self.0.values()
    }
}

/// Why a pack or a look cannot be drawn.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpriteError {
    /// A pack file that does not parse or does not hold together.
    #[error("{0}")]
    Pack(String),
    #[error("unknown sprite pack {0}")]
    UnknownPack(String),
    #[error("pack {pack} has no {slot} piece {piece}")]
    UnknownPiece {
        pack: String,
        slot: &'static str,
        piece: String,
    },
    #[error("{value} is neither a colour of the {palette} palette nor #RRGGBB")]
    UnknownColour {
        palette: &'static str,
        value: String,
    },
    #[error("{slot} piece {piece} has no frame facing {direction}")]
    MissingFrame {
        slot: &'static str,
        piece: String,
        direction: &'static str,
    },
}

impl SpriteError {
    /// A stable machine-readable code for the API.
    pub fn code(&self) -> &'static str {
        match self {
            SpriteError::Pack(_) => "SPRITE_PACK_INVALID",
            SpriteError::UnknownPack(_) => "SPRITE_UNKNOWN_PACK",
            SpriteError::UnknownPiece { .. } => "SPRITE_UNKNOWN_PIECE",
            SpriteError::UnknownColour { .. } => "SPRITE_UNKNOWN_COLOUR",
            SpriteError::MissingFrame { .. } => "SPRITE_MISSING_FRAME",
        }
    }
}

/// The effect a condition shows on the character (`MEMORY.md` §2, board
/// « États des personnages »). The effect is drawn over the sprite by the
/// client, animated; a condition with no visual shows only its badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionVisual {
    /// Sickly tint that pulses, bubbles rising.
    Poison,
    /// Stars circling the head, the body sways.
    Etourdi,
    /// Slow breath, Z drifting up.
    Endormi,
    /// Grey dithered ghost, dashed outline.
    Invisible,
    /// Flames licking up, the body flickers.
    Feu,
    /// Chains across, a small struggle.
    Entrave,
    /// Fast tremble, sweat drop.
    Effraye,
    /// Halo, soft white glow, sparkles.
    Beni,
    /// Out of the fight: greyed and lying down.
    Ko,
    /// Picked out as a target: a mark above the head.
    Cible,
}
