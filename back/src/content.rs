//! Content compiled into the binary: the rule systems, the maps and the
//! sprite packs of the two witness worlds (`content/`), and the D&D 5e
//! SRD preset (`engine/add-srd-preset`), which has rules, a map and a
//! scenario but no theme or sprite pack of its own yet: its campaigns
//! fall back to the first pack and the plain map tiles.
//!
//! The rule systems here are the presets a campaign starts from; a
//! campaign's own versions are stored in the database and resolved by
//! [`crate::rules`]. Its sprite pack is the one its world's looks are
//! drawn from (`content/sprites/looks/<world>.yaml`, matched on the rule
//! system's id), the first pack otherwise.
//!
//! Every file is checked by the tests of `shared` and by
//! `tests/sprites_test.rs` / `tests/creator_test.rs`: a broken one never
//! reaches a build, so loading them here may panic.

use std::sync::{Arc, LazyLock};

use promptus_shared::combat::scenario::Scenario;
use promptus_shared::maps::Map;
use promptus_shared::rules::RuleSystem;
use promptus_shared::sprite::{CharacterLook, LookBook, Pack, Packs};
use promptus_shared::story::{Campaign, RuleSystemRef};
use promptus_shared::theme::Theme;

/// The pack files, in a fixed order: their hash versions the renders.
pub const PACK_FILES: [&str; 2] = [
    include_str!("../../content/sprites/marins-1718/pack.yaml"),
    include_str!("../../content/sprites/equipage-spatial/pack.yaml"),
];
const LOOK_FILES: [&str; 2] = [
    include_str!("../../content/sprites/looks/corsaires.yaml"),
    include_str!("../../content/sprites/looks/brasier.yaml"),
];
const RULE_FILES: [&str; 3] = [
    include_str!("../../content/rules/corsaires/v1.yaml"),
    include_str!("../../content/rules/brasier/v1.yaml"),
    include_str!("../../content/rules/srd/v1.yaml"),
];

/// The maps of the two worlds and the SRD, by rule system id
/// (`content/maps/<rules>/<map>.yaml`).
const MAP_FILES: [(&str, &str); 3] = [
    (
        "corsaires",
        include_str!("../../content/maps/corsaires/quai-port-louis.yaml"),
    ),
    (
        "brasier",
        include_str!("../../content/maps/brasier/cure-dent-coursive.yaml"),
    ),
    (
        "srd",
        include_str!("../../content/maps/srd/route-des-gobelins.yaml"),
    ),
];

static MAPS: LazyLock<Vec<(&'static str, Map)>> = LazyLock::new(|| {
    MAP_FILES
        .iter()
        .map(|(rules, t)| (*rules, Map::from_yaml(t).expect("the embedded maps load")))
        .collect()
});

/// Map `id` of the world `campaign` plays in, if this server has it.
pub fn map(campaign: &Campaign, id: &str) -> Option<&'static Map> {
    MAPS.iter()
        .find(|(rules, m)| *rules == campaign.rules.id && m.id == id)
        .map(|(_, m)| m)
}

/// Every map of the world `campaign` plays in.
pub fn maps(campaign: &Campaign) -> impl Iterator<Item = &'static Map> {
    MAPS.iter()
        .filter(move |(rules, _)| *rules == campaign.rules.id)
        .map(|(_, m)| m)
}

/// The theme packs of the two worlds (`content/themes/<world>.yaml`),
/// matched on the rule system's id.
const THEME_FILES: [&str; 2] = [
    include_str!("../../content/themes/corsaires.yaml"),
    include_str!("../../content/themes/brasier.yaml"),
];

static THEMES: LazyLock<Vec<Theme>> = LazyLock::new(|| {
    THEME_FILES
        .iter()
        .map(|t| Theme::from_yaml(t).expect("the embedded themes load"))
        .collect()
});

/// The theme of the world `campaign` plays in, if this server has it.
pub fn theme(campaign: &Campaign) -> Option<&'static Theme> {
    THEMES.iter().find(|t| t.rules == campaign.rules.id)
}

static PACKS: LazyLock<Packs> =
    LazyLock::new(|| Packs::from_yaml(PACK_FILES).expect("the embedded sprite packs load"));

static LOOKS: LazyLock<Vec<LookBook>> = LazyLock::new(|| {
    LOOK_FILES
        .iter()
        .map(|t| LookBook::from_yaml(t).expect("the embedded looks parse"))
        .collect()
});

static RULES: LazyLock<Vec<(&'static str, Arc<RuleSystem>)>> = LazyLock::new(|| {
    RULE_FILES
        .iter()
        .map(|t| {
            let system = RuleSystem::from_yaml(t).expect("the embedded rule systems load");
            (*t, Arc::new(system))
        })
        .collect()
});

/// The fight scenarios of the two worlds (`content/scenarios/`): the
/// rule system editor plays them on every saved draft.
const SCENARIO_FILES: [&str; 3] = [
    include_str!("../../content/scenarios/corsaires/bagarre-du-quai.yaml"),
    include_str!("../../content/scenarios/brasier/abordage-coursive.yaml"),
    include_str!("../../content/scenarios/srd/embuscade-des-gobelins.yaml"),
];

static SCENARIOS: LazyLock<Vec<Scenario>> = LazyLock::new(|| {
    SCENARIO_FILES
        .iter()
        .map(|t| Scenario::from_yaml(t).expect("the embedded scenarios load"))
        .collect()
});

/// The scenarios written for rule system `rules` (its id).
pub fn scenarios(rules: &str) -> impl Iterator<Item = &'static Scenario> {
    SCENARIOS.iter().filter(move |s| s.rules == rules)
}

/// An embedded map of the world `rules` (a rule system id).
pub fn world_map(rules: &str, id: &str) -> Option<&'static Map> {
    MAPS.iter()
        .find(|(r, m)| *r == rules && m.id == id)
        .map(|(_, m)| m)
}

pub fn packs() -> &'static Packs {
    &PACKS
}

pub fn look_books() -> &'static [LookBook] {
    &LOOKS
}

/// Every preset, in the order a GM is offered them.
pub fn presets() -> impl Iterator<Item = &'static Arc<RuleSystem>> {
    RULES.iter().map(|(_, s)| s)
}

/// The preset `r` points at, if this server has it.
pub fn preset(r: &RuleSystemRef) -> Option<&'static Arc<RuleSystem>> {
    RULES
        .iter()
        .find(|(_, s)| s.id == r.id && s.version == r.version)
        .map(|(_, s)| s)
}

/// The YAML the preset `r` was written in: where a campaign's first
/// edited version starts from.
pub fn preset_yaml(r: &RuleSystemRef) -> Option<&'static str> {
    RULES
        .iter()
        .find(|(_, s)| s.id == r.id && s.version == r.version)
        .map(|(t, _)| *t)
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
        for rules in ["corsaires", "brasier", "srd", "unknown-system"] {
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
