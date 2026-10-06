//! Content compiled into the binary: the rule systems and the sprite
//! packs of the two witness worlds (`content/`).
//!
//! A campaign names its rule system by `(id, version)` and finds it
//! here until rule systems are stored per campaign
//! (`campaign/edit-rule-system`). Its sprite pack is the one its world's
//! looks are drawn from (`content/sprites/looks/<world>.yaml`, matched
//! on the rule system's id), the first pack otherwise.
//!
//! Every file is checked by the tests of `shared` and by
//! `tests/sprites_test.rs` / `tests/creator_test.rs`: a broken one never
//! reaches a build, so loading them here may panic.

use std::sync::LazyLock;

use promptus_shared::rules::RuleSystem;
use promptus_shared::sprite::{CharacterLook, LookBook, Pack, Packs};
use promptus_shared::story::{Campaign, RuleSystemRef};

/// The pack files, in a fixed order: their hash versions the renders.
pub const PACK_FILES: [&str; 2] = [
    include_str!("../../content/sprites/marins-1718/pack.yaml"),
    include_str!("../../content/sprites/equipage-spatial/pack.yaml"),
];
const LOOK_FILES: [&str; 2] = [
    include_str!("../../content/sprites/looks/corsaires.yaml"),
    include_str!("../../content/sprites/looks/brasier.yaml"),
];
const RULE_FILES: [&str; 2] = [
    include_str!("../../content/rules/corsaires/v1.yaml"),
    include_str!("../../content/rules/brasier/v1.yaml"),
];

static PACKS: LazyLock<Packs> =
    LazyLock::new(|| Packs::from_yaml(PACK_FILES).expect("the embedded sprite packs load"));

static LOOKS: LazyLock<Vec<LookBook>> = LazyLock::new(|| {
    LOOK_FILES
        .iter()
        .map(|t| LookBook::from_yaml(t).expect("the embedded looks parse"))
        .collect()
});

static RULES: LazyLock<Vec<RuleSystem>> = LazyLock::new(|| {
    RULE_FILES
        .iter()
        .map(|t| RuleSystem::from_yaml(t).expect("the embedded rule systems load"))
        .collect()
});

pub fn packs() -> &'static Packs {
    &PACKS
}

pub fn look_books() -> &'static [LookBook] {
    &LOOKS
}

/// The rule system a campaign plays, if this server has it.
pub fn rule_system(r: &RuleSystemRef) -> Option<&'static RuleSystem> {
    RULES
        .iter()
        .find(|s| s.id == r.id && s.version == r.version)
}

fn look_book(campaign: &Campaign) -> Option<&'static LookBook> {
    LOOKS.iter().find(|b| b.world == campaign.rules.id)
}

/// The sprite pack a campaign's characters are made from.
pub fn pack_for(campaign: &Campaign) -> &'static Pack {
    look_book(campaign)
        .and_then(|b| b.party.first())
        .and_then(|l| PACKS.get(&l.look.pack))
        .or_else(|| PACKS.iter().next())
        .expect("at least one embedded sprite pack")
}

/// The look a new character starts from: the first party look of the
/// campaign's world, or the plainest look of its pack.
pub fn start_look(campaign: &Campaign) -> CharacterLook {
    let pack = pack_for(campaign);
    look_book(campaign)
        .and_then(|b| b.party.iter().find(|l| l.look.pack == pack.id))
        .map(|l| l.look.clone())
        .unwrap_or_else(|| plain_look(pack))
}

fn plain_look(pack: &Pack) -> CharacterLook {
    use promptus_shared::sprite::{Hair, Slot};
    let first = |slot| pack.pieces(slot).first().map(|p| p.id.clone());
    CharacterLook {
        pack: pack.id.clone(),
        body: first(Slot::Body).unwrap_or_default(),
        skin: pack
            .palettes
            .skin
            .first()
            .map(|s| s.id.clone())
            .unwrap_or_default(),
        hair: Hair {
            style: first(Slot::Hair),
            colour: pack
                .palettes
                .hair
                .first()
                .map(|s| s.id.clone())
                .unwrap_or_default(),
        },
        beard: None,
        headwear: None,
        outfit: first(Slot::Outfit).map(|p| promptus_shared::sprite::Worn::plain(&p)),
        armour: None,
        weapon: None,
        accessories: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use promptus_shared::sprite::{Direction, render};

    #[test]
    fn every_campaign_world_gets_a_pack_and_a_start_look_that_draws() {
        for rules in ["corsaires", "brasier", "unknown-system"] {
            let campaign = Campaign::empty(
                "c",
                "C",
                "w",
                RuleSystemRef {
                    id: rules.into(),
                    version: 1,
                },
            );
            let look = start_look(&campaign);
            assert_eq!(look.pack, pack_for(&campaign).id, "{rules}");
            render(packs(), &look, Direction::East).unwrap_or_else(|e| panic!("{rules}: {e}"));
        }
        let brasier = Campaign::empty(
            "c",
            "C",
            "w",
            RuleSystemRef {
                id: "brasier".into(),
                version: 1,
            },
        );
        assert_eq!(pack_for(&brasier).id, "equipage-spatial");
    }
}
