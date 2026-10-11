//! Where the scenes stand on the maps (`campaign/pin-scenes-on-maps`).
//!
//! A scene names its place (`Node::location`); a place may be anchored on
//! a map (`Location::anchor`), and sits inside its parent. On a given
//! map, a scene's pin is the **first place up its chain anchored on that
//! map**: the tavern's scenes stand on the tavern on the town's map, and
//! on Port-Louis on the region's, when the tavern is anchored on the town
//! and Port-Louis on the region. The chain of parents is région › lieu ›
//! rencontre; the maps stay views of the story, which holds every pin.
//!
//! Several scenes sharing a place share one pin. Scenes are numbered by
//! their order in the campaign (1, 2, …), the same number on every map.
//! A scene no place of its chain anchors anywhere has no pin: allowed (a
//! dream, a memory), reported by the validator and the act gauge.
//!
//! Pure: the GM's atlas, the GM's board at the table and the players'
//! projection all read pins here.

use std::collections::BTreeSet;

use serde::Serialize;

use super::model::{Campaign, Id, Location, Node};
use crate::maps::Cell;

/// One pin on a map: a place and the scenes standing there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pin {
    /// The anchored place.
    pub location: Id,
    pub name: String,
    pub at: Cell,
    /// In campaign order.
    pub scenes: Vec<PinScene>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PinScene {
    pub id: Id,
    pub title: String,
    /// 1-based, the scene's place in the campaign.
    pub number: usize,
    /// The grid map it is played on (`Node::map`), if it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<String>,
}

impl PinScene {
    /// `node`, the `index`-th scene of its campaign.
    #[must_use]
    pub fn of(index: usize, node: &Node) -> Self {
        Self {
            id: node.id.clone(),
            title: node.title.clone(),
            number: index + 1,
            map: node.map.clone(),
        }
    }
}

/// The places from `location` up its parents, each once: a parent loop
/// (which nothing forbids in a hand-written file) ends the walk.
pub(crate) fn chain<'a>(c: &'a Campaign, location: &str) -> Vec<&'a Location> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut next = Some(location);
    while let Some(id) = next {
        if !seen.insert(id) {
            break;
        }
        let Some(l) = c.location(id) else { break };
        out.push(l);
        next = l.parent.as_deref();
    }
    out
}

/// The place whose anchor on `map` holds `node`'s pin there.
#[must_use]
pub fn place_on<'a>(c: &'a Campaign, node: &Node, map: &str) -> Option<&'a Location> {
    let start = node.location.as_deref()?;
    chain(c, start)
        .into_iter()
        .find(|l| l.anchor.as_ref().is_some_and(|a| a.map == map))
}

/// Whether some place up `node`'s chain is anchored on a map.
#[must_use]
pub fn is_pinned(c: &Campaign, node: &Node) -> bool {
    node.location
        .as_deref()
        .is_some_and(|l| chain(c, l).iter().any(|x| x.anchor.is_some()))
}

/// Every pin on `map`, in the order of their first scene.
#[must_use]
pub fn pins_on(c: &Campaign, map: &str) -> Vec<Pin> {
    let mut pins: Vec<Pin> = Vec::new();
    for (i, n) in c.nodes.iter().enumerate() {
        let Some(place) = place_on(c, n, map) else {
            continue;
        };
        let Some(anchor) = &place.anchor else {
            continue;
        };
        let scene = PinScene::of(i, n);
        match pins.iter_mut().find(|p| p.location == place.id) {
            Some(p) => p.scenes.push(scene),
            None => pins.push(Pin {
                location: place.id.clone(),
                name: place.name.clone(),
                at: anchor.at,
                scenes: vec![scene],
            }),
        }
    }
    pins
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::from_yaml;

    const STORY: &str = r"
id: t
title: T
rules: { id: r, version: 1 }
bible: {}
acts: [{ id: a1, title: Un }]
nodes:
  - { id: sc_a, act: a1, title: A, location: l_taverne }
  - { id: sc_b, act: a1, title: B, location: l_port }
  - { id: sc_c, act: a1, title: C, location: l_taverne }
  - { id: sc_d, act: a1, title: D }
  - { id: sc_e, act: a1, title: E, location: l_boucle }
locations:
  - { id: l_ville, name: Ville, anchor: { map: region, at: [5, 0] } }
  - { id: l_taverne, name: Taverne, parent: l_ville, anchor: { map: ville, at: [2, 3] } }
  - { id: l_port, name: Port, parent: l_ville }
  - { id: l_boucle, name: Boucle, parent: l_boucle2 }
  - { id: l_boucle2, name: Boucle 2, parent: l_boucle }
";

    fn ids(p: &Pin) -> Vec<(&str, usize)> {
        p.scenes.iter().map(|s| (s.id.as_str(), s.number)).collect()
    }

    #[test]
    fn a_scene_stands_on_the_first_place_up_its_chain_anchored_on_each_map() {
        let c = from_yaml(STORY).unwrap();
        // On the region, the town holds all its scenes, the tavern's too.
        let region = pins_on(&c, "region");
        assert_eq!(region.len(), 1);
        assert_eq!(region[0].location, "l_ville");
        assert_eq!(region[0].at, Cell::new(5, 0));
        assert_eq!(ids(&region[0]), [("sc_a", 1), ("sc_b", 2), ("sc_c", 3)]);
        // On the town, only the tavern is anchored: two scenes share it,
        // numbered as in the campaign.
        let town = pins_on(&c, "ville");
        assert_eq!(town.len(), 1);
        assert_eq!(ids(&town[0]), [("sc_a", 1), ("sc_c", 3)]);
    }

    #[test]
    fn a_scene_without_an_anchored_place_has_no_pin_and_a_parent_loop_ends() {
        let c = from_yaml(STORY).unwrap();
        let node = |id: &str| c.node(id).unwrap();
        assert!(is_pinned(&c, node("sc_b")));
        assert!(!is_pinned(&c, node("sc_d")));
        assert!(!is_pinned(&c, node("sc_e")));
        assert!(place_on(&c, node("sc_e"), "region").is_none());
    }
}
