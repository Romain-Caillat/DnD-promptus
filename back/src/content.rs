//! Content compiled into the binary: the rule systems, the maps and the
//! sprite packs of the two witness worlds (`content/`).
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
const RULE_FILES: [&str; 2] = [
    include_str!("../../content/rules/corsaires/v1.yaml"),
    include_str!("../../content/rules/brasier/v1.yaml"),
];

/// The maps of the two worlds, by rule system id
/// (`content/maps/<rules>/<map>.yaml`).
const MAP_FILES: [(&str, &str); 2] = [
    (
        "corsaires",
        include_str!("../../content/maps/corsaires/quai-port-louis.yaml"),
    ),
    (
        "brasier",
        include_str!("../../content/maps/brasier/cure-dent-coursive.yaml"),
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
const SCENARIO_FILES: [&str; 2] = [
    include_str!("../../content/scenarios/corsaires/bagarre-du-quai.yaml"),
    include_str!("../../content/scenarios/brasier/abordage-coursive.yaml"),
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

/// How NPC or adversary `id` of `campaign` looks on the map
/// (characters/walk-in-four-directions): its look in the world's book —
/// a numbered copy of a fight, `who-2`, reads as `who` — or else a look
/// picked from its id in the campaign's pack (a generated campaign's
/// smuggler), the same on every screen and at every fight.
pub fn npc_look(campaign: &Campaign, id: &str) -> CharacterLook {
    let base = id
        .rsplit_once('-')
        .filter(|(_, n)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .map_or(id, |(who, _)| who);
    let book = look_book(campaign);
    let known = book.and_then(|b| {
        b.foes
            .iter()
            .chain(&b.party)
            .find(|l| l.id == id || l.id == base)
    });
    if let Some(l) = known {
        return l.look.clone();
    }
    picked_look(pack_for(campaign), base)
}

/// A look chosen piece by piece from a hash of `seed`: the body, the skin,
/// the hair and its colour, the outfit and its dye.
fn picked_look(pack: &Pack, seed: &str) -> CharacterLook {
    use promptus_shared::sprite::{Slot, Worn};
    // FNV-1a: stable across builds and platforms, unlike the std hasher.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in seed.bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    let mut pick = |n: usize| -> usize {
        if n == 0 {
            return 0;
        }
        let i = usize::try_from(h % (n as u64)).unwrap_or(0);
        h = h.rotate_right(13).wrapping_mul(0x0100_0000_01b3);
        i
    };
    let mut look = plain_look(pack);
    let id_of = |slot: Slot, i: usize| pack.pieces(slot).get(i).map(|p| p.id.clone());
    if let Some(body) = id_of(Slot::Body, pick(pack.pieces(Slot::Body).len())) {
        look.body = body;
    }
    let p = &pack.palettes;
    if let Some(s) = p.skin.get(pick(p.skin.len())) {
        look.skin = s.id.clone();
    }
    look.hair.style = id_of(Slot::Hair, pick(pack.pieces(Slot::Hair).len()));
    if let Some(c) = p.hair.get(pick(p.hair.len())) {
        look.hair.colour = c.id.clone();
    }
    if let Some(outfit) = id_of(Slot::Outfit, pick(pack.pieces(Slot::Outfit).len())) {
        let dye = p.cloth.get(pick(p.cloth.len())).map(|c| c.id.clone());
        look.outfit = Some(Worn {
            piece: outfit,
            dye,
            accent: None,
        });
    }
    look
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
            // An NPC no look book knows still gets one, every time the same.
            let smuggler = npc_look(&campaign, "contrebandier-3");
            assert_eq!(smuggler, npc_look(&campaign, "contrebandier-1"));
            for d in [Direction::South, Direction::North, Direction::West] {
                render(packs(), &smuggler, d).unwrap_or_else(|e| panic!("{rules} {d:?}: {e}"));
            }
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
