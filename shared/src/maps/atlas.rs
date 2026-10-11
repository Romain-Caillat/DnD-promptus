//! The atlas (maps/browse-atlas): how a campaign's maps nest — a region
//! in hexes, the places on it, the encounters in a place — read from the
//! maps' exits.
//!
//! An exit from map A to map B makes A the parent of B only when B's
//! scale is strictly finer (world › place › encounter; a world map may
//! hold an encounter directly). Every edge goes down one scale or more,
//! so the tree has no cycle whatever the exits say: an exit between two
//! maps of the same scale, or back up, is a side link. An exit to a map
//! neither the world nor the campaign has is dropped.
//!
//! A map several maps lead down to keeps one parent: the one of the
//! nearest coarser scale (a place before the region for an encounter),
//! then a campaign map before a world map, then the smallest id. A map
//! no exit leads down to is a root.
//!
//! The story's places will anchor maps too (`campaign/pin-scenes-on-maps`);
//! until then exits are the only source.

use std::collections::{BTreeMap, BTreeSet};

use super::{Map, Scale};

/// Where a map comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// Compiled into the server with the world (`content/maps/`).
    World,
    /// The campaign's own (`campaign_maps`).
    Campaign,
}

/// One map handed to [`tree`].
#[derive(Clone, Copy, Debug)]
pub struct Sheet<'a> {
    pub map: &'a Map,
    pub origin: Origin,
}

/// Where a child map hangs on its parent: the exit leading to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub map: String,
    pub exit: String,
}

/// An exit to a map that is not a child: same scale, or back up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub to: String,
    pub exit: String,
}

/// One map's place in the atlas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub origin: Origin,
    pub parent: Option<Anchor>,
    /// In the order of the parent's exits.
    pub children: Vec<String>,
    pub links: Vec<Link>,
}

/// Every map once, roots first then depth first; `roots` in that order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Atlas {
    pub roots: Vec<String>,
    pub nodes: Vec<Node>,
}

impl Atlas {
    #[must_use]
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
}

const fn rank(scale: Scale) -> u8 {
    match scale {
        Scale::World => 0,
        Scale::Place => 1,
        Scale::Encounter => 2,
    }
}

/// The atlas of `sheets`. A campaign map shadows a world map of the same
/// id (as at the table, `campaign_maps::playable`).
#[must_use]
pub fn tree(sheets: &[Sheet<'_>]) -> Atlas {
    let mut by_id: BTreeMap<&str, Sheet<'_>> = BTreeMap::new();
    for s in sheets {
        match by_id.get(s.map.id.as_str()) {
            Some(had) if had.origin == Origin::Campaign && s.origin == Origin::World => {}
            _ => {
                by_id.insert(&s.map.id, *s);
            }
        }
    }

    // Each map's chosen parent: by (parent scale rank, origin, reversed id),
    // the largest rank wins, then the campaign, then the smallest id.
    let mut parent: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    for (&id, s) in &by_id {
        for e in &s.map.exits {
            let Some(child) = by_id.get(e.to.as_str()) else {
                continue;
            };
            if rank(child.map.scale) <= rank(s.map.scale) {
                continue;
            }
            let better = match parent.get(e.to.as_str()) {
                None => true,
                Some(&(had, _)) => {
                    let h = &by_id[had];
                    let key = |x: &Sheet<'_>| (rank(x.map.scale), x.origin);
                    (key(s), std::cmp::Reverse(id)) > (key(h), std::cmp::Reverse(had))
                }
            };
            if better {
                parent.insert(&e.to, (id, &e.id));
            }
        }
    }

    let children = |id: &str| -> Vec<String> {
        let mut seen = BTreeSet::new();
        by_id[id]
            .map
            .exits
            .iter()
            .filter(|e| {
                parent
                    .get(e.to.as_str())
                    .is_some_and(|&(p, x)| p == id && x == e.id)
            })
            .filter(|e| seen.insert(e.to.as_str()))
            .map(|e| e.to.clone())
            .collect()
    };

    let mut roots: Vec<&str> = by_id
        .keys()
        .copied()
        .filter(|id| !parent.contains_key(id))
        .collect();
    roots.sort_by_key(|id| {
        let s = &by_id[id];
        (rank(s.map.scale), s.origin, s.map.name.as_str(), *id)
    });

    let mut nodes = Vec::with_capacity(by_id.len());
    let mut stack: Vec<String> = roots.iter().rev().map(|r| (*r).to_owned()).collect();
    while let Some(id) = stack.pop() {
        let s = &by_id[id.as_str()];
        let kids = children(&id);
        stack.extend(kids.iter().rev().cloned());
        let links = s
            .map
            .exits
            .iter()
            .filter(|e| by_id.contains_key(e.to.as_str()))
            .filter(|e| !kids.contains(&e.to))
            .map(|e| Link {
                to: e.to.clone(),
                exit: e.id.clone(),
            })
            .collect();
        nodes.push(Node {
            parent: parent.get(id.as_str()).map(|&(p, x)| Anchor {
                map: p.to_owned(),
                exit: x.to_owned(),
            }),
            origin: s.origin,
            children: kids,
            links,
            id,
        });
    }
    Atlas {
        roots: roots.into_iter().map(str::to_owned).collect(),
        nodes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{Cell, Exit};

    fn map(id: &str, scale: Scale, exits: &[(&str, &str)]) -> Map {
        let mut m = Map::from_yaml(&format!(
            "version: 1\nid: {id}\nname: {id}\nscale: {}\ntheme: t\nlayers:\n  - {{ id: base, name: Base, visibility: all }}\ngrid:\n  legend:\n    '.': {{ terrain: sol }}\n  rows:\n    - '...'\n    - '...'\n",
            match scale {
                Scale::World => "world",
                Scale::Place => "place",
                Scale::Encounter => "encounter",
            }
        ))
        .expect("a test map");
        m.exits = exits
            .iter()
            .map(|(exit, to)| Exit {
                id: (*exit).into(),
                cells: vec![Cell { x: 1, y: 1 }],
                to: (*to).into(),
                arrive: None,
                label: None,
                layer: "base".into(),
            })
            .collect();
        m
    }

    fn atlas(maps: &[(Map, Origin)]) -> Atlas {
        let sheets: Vec<Sheet<'_>> = maps
            .iter()
            .map(|(m, o)| Sheet { map: m, origin: *o })
            .collect();
        tree(&sheets)
    }

    fn parent_of(a: &Atlas, id: &str) -> Option<String> {
        a.node(id)
            .and_then(|n| n.parent.as_ref())
            .map(|p| p.map.clone())
    }

    #[test]
    fn a_region_holds_its_places_and_a_place_its_encounters() {
        let a = atlas(&[
            (
                map("region", Scale::World, &[("vers-quai", "quai")]),
                Origin::World,
            ),
            (
                map("quai", Scale::Place, &[("vers-cale", "cale")]),
                Origin::World,
            ),
            (map("cale", Scale::Encounter, &[]), Origin::Campaign),
            (map("seule", Scale::Encounter, &[]), Origin::Campaign),
        ]);
        assert_eq!(a.roots, ["region", "seule"]);
        assert_eq!(parent_of(&a, "quai").as_deref(), Some("region"));
        assert_eq!(parent_of(&a, "cale").as_deref(), Some("quai"));
        assert_eq!(a.node("quai").unwrap().children, ["cale"]);
        let order: Vec<&str> = a.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(order, ["region", "quai", "cale", "seule"]);
    }

    #[test]
    fn exits_between_equals_or_back_up_never_make_a_cycle() {
        let a = atlas(&[
            (
                map("region", Scale::World, &[("vers-quai", "quai")]),
                Origin::World,
            ),
            (
                map(
                    "quai",
                    Scale::Place,
                    &[("vers-region", "region"), ("vers-rues", "rues")],
                ),
                Origin::World,
            ),
            (
                map("rues", Scale::Place, &[("vers-quai", "quai")]),
                Origin::World,
            ),
        ]);
        assert_eq!(a.roots, ["region", "rues"]);
        assert_eq!(parent_of(&a, "region"), None);
        assert_eq!(parent_of(&a, "rues"), None);
        assert_eq!(a.nodes.len(), 3, "each map once");
        let links: Vec<&str> = a
            .node("quai")
            .unwrap()
            .links
            .iter()
            .map(|l| l.to.as_str())
            .collect();
        assert_eq!(links, ["region", "rues"]);
    }

    #[test]
    fn an_exit_to_a_missing_map_is_dropped() {
        let a = atlas(&[(
            map("quai", Scale::Place, &[("ecoutille", "nulle-part")]),
            Origin::World,
        )]);
        let quai = a.node("quai").unwrap();
        assert!(quai.children.is_empty() && quai.links.is_empty());
    }

    #[test]
    fn a_map_led_to_twice_keeps_the_nearest_scale_then_the_campaign() {
        let a = atlas(&[
            (
                map("region", Scale::World, &[("a", "duel"), ("b", "quai")]),
                Origin::World,
            ),
            (map("quai", Scale::Place, &[("c", "duel")]), Origin::World),
            (
                map("quai-copie", Scale::Place, &[("c", "duel")]),
                Origin::Campaign,
            ),
            (map("duel", Scale::Encounter, &[]), Origin::Campaign),
        ]);
        assert_eq!(parent_of(&a, "duel").as_deref(), Some("quai-copie"));
        assert!(
            a.node("region")
                .unwrap()
                .links
                .iter()
                .any(|l| l.to == "duel"),
            "the region's own exit stays a link"
        );
    }

    #[test]
    fn a_campaign_map_shadows_the_world_map_of_the_same_id() {
        let a = atlas(&[
            (map("quai", Scale::Place, &[]), Origin::World),
            (map("quai", Scale::Place, &[]), Origin::Campaign),
        ]);
        assert_eq!(a.nodes.len(), 1);
        assert_eq!(a.nodes[0].origin, Origin::Campaign);
    }
}
