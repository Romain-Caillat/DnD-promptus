//! The story frieze (`campaign/order-maps-by-story`): the maps an act's
//! scenes cross, in the order the story walks them, one act per evening.
//!
//! **A scene's stop.** The grid map it is played on (`Node::map`) when the
//! atlas has it; otherwise the finest map carrying its place — the place
//! itself or the first of its parents anchored there (`pins::chain`),
//! the finest scale winning, the nearest place breaking a tie. A scene
//! whose own place has no map of its own gets a **map to create** named
//! after that place: when no place up its chain is anchored on a map of
//! the atlas, or when the only carrying map is a region (`world` scale)
//! and what is anchored there is a parent, not the scene's own place —
//! a tavern inside Port-Louis is not drawn by Port-Louis' hex on the
//! region. A scene with neither map nor place (a dream) has no stop.
//!
//! **The walk.** Each act starts at the campaign's `start_node` when it
//! is one of its scenes, else at its scenes no other scene of the act
//! leads to (none: its first scene). From a scene with one way on, the
//! next scene follows. From several ways on — a fork — the scenes up to
//! where every way meets again (the nearest scene every path from the
//! fork to an end of the act goes through) form a **branch**: their
//! stops share one slot of the frieze, then the walk goes on from that
//! meeting scene. Ways that never meet put everything left in the
//! branch. Exits to another act, to the scene itself or back to a scene
//! already walked are not followed, so a cycle never loops. Scenes the
//! walk never reaches start again from the first of them.
//!
//! Consecutive slots holding the same single stop make one: two scenes
//! in the same tavern are one stop. An act with no stop has an empty
//! frieze — the GM writes it. The maps no stop names are "outside the
//! story", in the atlas's order.
//!
//! Pure, and computed at every atlas read: the GM's Atlas and Claude's
//! `get_atlas` read the same order.

use std::collections::{BTreeSet, VecDeque};

use serde::Serialize;

use super::model::{Campaign, Id, Node};
use super::pins::{PinScene, chain};
use crate::maps::Scale;

/// A map the atlas holds, as the frieze needs it.
#[derive(Debug, Clone, Copy)]
pub struct Known<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub scale: Scale,
}

/// The whole frieze: one per act, then the maps no scene crosses.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Frieze {
    pub acts: Vec<ActFrieze>,
    /// Atlas maps no stop names, in the atlas's order.
    pub outside: Vec<String>,
}

/// One act, one evening.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActFrieze {
    pub act: Id,
    pub title: String,
    /// 1-based: the act's place among the acts, its evening.
    pub evening: usize,
    pub slots: Vec<Slot>,
}

/// One step of the frieze: a single stop, or a branch's stops side by side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Slot {
    pub stops: Vec<Stop>,
}

/// A map the scenes of a slot stand on — or the map to create for a place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stop {
    /// The atlas map; `None` when it is still to create.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<String>,
    /// The place whose map is to create (only then).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<Id>,
    /// The map's name, or the place's.
    pub name: String,
    /// In walk order.
    pub scenes: Vec<PinScene>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Key {
    Map(String),
    Missing(Id),
}

const fn rank(scale: Scale) -> u8 {
    match scale {
        Scale::World => 0,
        Scale::Place => 1,
        Scale::Encounter => 2,
    }
}

/// Where `node` stands in the frieze, if anywhere.
fn stop_of(c: &Campaign, maps: &[Known<'_>], node: &Node) -> Option<Key> {
    let known = |id: &str| maps.iter().find(|m| m.id == id);
    if let Some(m) = node.map.as_deref().and_then(known) {
        return Some(Key::Map(m.id.to_owned()));
    }
    let own = node.location.as_deref()?;
    let mut best: Option<(&str, Known<'_>)> = None;
    for place in chain(c, own) {
        let Some(m) = place.anchor.as_ref().and_then(|a| known(&a.map)) else {
            continue;
        };
        if best.is_none_or(|(_, b)| rank(m.scale) > rank(b.scale)) {
            best = Some((place.id.as_str(), *m));
        }
    }
    match best {
        Some((place, m)) if m.scale != Scale::World || place == own => {
            Some(Key::Map(m.id.to_owned()))
        }
        _ => Some(Key::Missing(own.to_owned())),
    }
}

/// The scenes of one act, by their index in the campaign, and its walk.
struct Walk<'a> {
    nodes: &'a [Node],
    /// Indices into `nodes` of the act's scenes, in campaign order.
    scenes: Vec<usize>,
    walked: BTreeSet<usize>,
}

impl Walk<'_> {
    /// The ways on from `i`: other scenes of the act, not walked yet, once each.
    fn next(&self, i: usize) -> Vec<usize> {
        let mut out = Vec::new();
        for e in &self.nodes[i].exits {
            let Some(j) = self
                .scenes
                .iter()
                .copied()
                .find(|&j| self.nodes[j].id == e.to)
            else {
                continue;
            };
            if j != i && !self.walked.contains(&j) && !out.contains(&j) {
                out.push(j);
            }
        }
        out
    }

    /// The scenes reachable from `from` (included), never through `avoid`, breadth first.
    fn reach(&self, from: &[usize], avoid: &BTreeSet<usize>) -> Vec<usize> {
        let mut seen: Vec<usize> = Vec::new();
        let mut queue: VecDeque<usize> = from
            .iter()
            .copied()
            .filter(|j| !avoid.contains(j))
            .collect();
        while let Some(i) = queue.pop_front() {
            if seen.contains(&i) {
                continue;
            }
            seen.push(i);
            queue.extend(self.next(i).into_iter().filter(|j| !avoid.contains(j)));
        }
        seen
    }

    /// Where the ways `from` meet again: the nearest scene every path from
    /// them to an end goes through, if any.
    fn meeting(&self, from: &[usize]) -> Option<usize> {
        let all = self.reach(from, &BTreeSet::new());
        let ends: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&i| self.next(i).is_empty())
            .collect();
        if ends.is_empty() {
            return None;
        }
        let meets: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&j| {
                let without = self.reach(from, &BTreeSet::from([j]));
                !ends.iter().any(|e| *e != j && without.contains(e))
            })
            .collect();
        meets.iter().copied().find(|&j| {
            let others: BTreeSet<usize> = meets.iter().copied().filter(|&k| k != j).collect();
            self.reach(from, &others).contains(&j)
        })
    }

    /// Walk from the ways `from`: each step one scene, or a branch.
    fn steps(&mut self, mut from: Vec<usize>, out: &mut Vec<Vec<usize>>) {
        loop {
            from.retain(|j| !self.walked.contains(j));
            match from.as_slice() {
                [] => return,
                [one] => {
                    let one = *one;
                    self.walked.insert(one);
                    out.push(vec![one]);
                    from = self.next(one);
                }
                _ => {
                    let meet = self.meeting(&from);
                    let avoid: BTreeSet<usize> = meet.into_iter().collect();
                    let branch = self.reach(&from, &avoid);
                    self.walked.extend(branch.iter().copied());
                    out.push(branch);
                    let Some(m) = meet else { return };
                    from = vec![m];
                }
            }
        }
    }
}

/// The frieze of `c` over the atlas's `maps` (in the atlas's order).
#[must_use]
pub fn frieze(c: &Campaign, maps: &[Known<'_>]) -> Frieze {
    let mut named: BTreeSet<String> = BTreeSet::new();
    let acts = c
        .acts
        .iter()
        .enumerate()
        .map(|(a, act)| {
            let scenes: Vec<usize> = (0..c.nodes.len())
                .filter(|&i| c.nodes[i].act == act.id)
                .collect();
            let mut walk = Walk {
                nodes: &c.nodes,
                scenes: scenes.clone(),
                walked: BTreeSet::new(),
            };
            let start = c
                .bible
                .start_node
                .as_deref()
                .and_then(|s| scenes.iter().copied().find(|&i| c.nodes[i].id == s));
            let starts: Vec<usize> = match start {
                Some(s) => vec![s],
                None => scenes
                    .iter()
                    .copied()
                    .filter(|&i| {
                        !scenes.iter().any(|&j| {
                            j != i && c.nodes[j].exits.iter().any(|e| e.to == c.nodes[i].id)
                        })
                    })
                    .collect(),
            };
            let mut steps = Vec::new();
            walk.steps(starts, &mut steps);
            while let Some(&rest) = scenes.iter().find(|i| !walk.walked.contains(i)) {
                walk.steps(vec![rest], &mut steps);
            }

            let mut slots: Vec<(Vec<Key>, Slot)> = Vec::new();
            for step in steps {
                let mut keys: Vec<Key> = Vec::new();
                let mut stops: Vec<Stop> = Vec::new();
                for i in step {
                    let node = &c.nodes[i];
                    let Some(key) = stop_of(c, maps, node) else {
                        continue;
                    };
                    let scene = PinScene::of(i, node);
                    if let Some(k) = keys.iter().position(|k| *k == key) {
                        stops[k].scenes.push(scene);
                        continue;
                    }
                    stops.push(match &key {
                        Key::Map(id) => Stop {
                            name: maps
                                .iter()
                                .find(|m| m.id == id)
                                .map_or_else(|| id.clone(), |m| m.name.to_owned()),
                            map: Some(id.clone()),
                            place: None,
                            scenes: vec![scene],
                        },
                        Key::Missing(place) => Stop {
                            name: c
                                .location(place)
                                .map_or_else(|| place.clone(), |l| l.name.clone()),
                            map: None,
                            place: Some(place.clone()),
                            scenes: vec![scene],
                        },
                    });
                    keys.push(key);
                }
                if stops.is_empty() {
                    continue;
                }
                match slots.last_mut() {
                    Some((had, slot)) if keys.len() == 1 && *had == keys => {
                        let more = stops.remove(0).scenes;
                        slot.stops[0].scenes.extend(more);
                    }
                    _ => slots.push((keys, Slot { stops })),
                }
            }
            for (keys, _) in &slots {
                for k in keys {
                    if let Key::Map(id) = k {
                        named.insert(id.clone());
                    }
                }
            }
            ActFrieze {
                act: act.id.clone(),
                title: act.title.clone(),
                evening: a + 1,
                slots: slots.into_iter().map(|(_, s)| s).collect(),
            }
        })
        .collect();
    Frieze {
        acts,
        outside: maps
            .iter()
            .filter(|m| !named.contains(m.id))
            .map(|m| m.id.to_owned())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::from_yaml;

    const CORSAIRES: &str = include_str!("../../../content/campaigns/corsaires/campagne.yaml");

    /// The Corsaires' world maps, as the atlas lists them.
    const ATLAS: [Known<'static>; 6] = [
        Known {
            id: "cotes-bretagne-sud",
            name: "Les côtes de Bretagne sud",
            scale: Scale::World,
        },
        Known {
            id: "port-louis-rues",
            name: "Port-Louis, le quartier bas",
            scale: Scale::Place,
        },
        Known {
            id: "quai-port-louis",
            name: "Le quai de Port-Louis",
            scale: Scale::Encounter,
        },
        Known {
            id: "le-palais",
            name: "Le port du Palais",
            scale: Scale::Place,
        },
        Known {
            id: "large-de-belle-ile",
            name: "Au large de Belle-Île",
            scale: Scale::Place,
        },
        Known {
            id: "pont-du-greyhound",
            name: "L'abordage du Greyhound",
            scale: Scale::Encounter,
        },
    ];

    /// Each slot as text: its stops' names joined by « · », a stop to
    /// create marked « ! », and the scenes of each in brackets.
    fn read(a: &ActFrieze) -> Vec<String> {
        a.slots
            .iter()
            .map(|s| {
                s.stops
                    .iter()
                    .map(|st| {
                        let scenes: Vec<&str> = st.scenes.iter().map(|x| x.id.as_str()).collect();
                        format!(
                            "{}{} [{}]",
                            if st.map.is_none() { "!" } else { "" },
                            st.name,
                            scenes.join(" ")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" · ")
            })
            .collect()
    }

    #[test]
    fn the_corsaires_as_played_in_production_cross_their_places_before_their_maps() {
        // In production only Port-Louis is pinned, on the region: every
        // place inside it is a map to create, the quai's fight is on its own
        // map, and the market, the shipyard, the office and Anaïs' errand
        // are the branch between the night port and the quai.
        let mut c = from_yaml(CORSAIRES).unwrap();
        for l in &mut c.locations {
            if l.id != "lieu_port_louis" {
                l.anchor = None;
            }
        }
        let f = frieze(&c, &ATLAS);
        assert_eq!(f.acts.len(), 2);
        assert_eq!(f.acts[0].evening, 1);
        assert_eq!(f.acts[0].title, "L'Offre qu'on ne refuse pas");
        assert_eq!(
            read(&f.acts[0]),
            [
                "!Le Perroquet Noyé [sc_perroquet sc_proposition]",
                "!Le port de nuit [sc_port_nuit]",
                "!Le marché noir de Kerjean [sc_marche_noir] · !Le chantier naval [sc_chantier] · \
                 !Le bureau du capitaine de port [sc_bureau_morel] · !Le Perroquet Noyé [sc_anais]",
                "Le quai de Port-Louis [sc_quai]",
                "!La Mâchoire [sc_depart]",
            ]
        );
        assert_eq!(
            f.acts[0].slots[0].stops[0].place.as_deref(),
            Some("lieu_perroquet")
        );
        assert_eq!(
            read(&f.acts[1]),
            ["L'abordage du Greyhound [sc_interception_greyhound]"]
        );
        assert_eq!(
            f.outside,
            [
                "cotes-bretagne-sud",
                "port-louis-rues",
                "le-palais",
                "large-de-belle-ile"
            ]
        );
    }

    #[test]
    fn the_corsaires_file_pins_its_places_on_the_streets_so_the_streets_hold_them() {
        let c = from_yaml(CORSAIRES).unwrap();
        let f = frieze(&c, &ATLAS);
        assert_eq!(
            read(&f.acts[0]),
            [
                "Port-Louis, le quartier bas [sc_perroquet sc_proposition sc_port_nuit \
                 sc_marche_noir sc_chantier sc_bureau_morel sc_anais]",
                "Le quai de Port-Louis [sc_quai]",
                "Port-Louis, le quartier bas [sc_depart]",
            ]
        );
    }

    const STORY: &str = r"
id: t
title: T
rules: { id: r, version: 1 }
bible: { start_node: sc_b }
acts: [{ id: a1, title: Un }, { id: a2, title: Deux }, { id: a3, title: Trois }]
nodes:
  - { id: sc_a, act: a1, title: A, location: l_cave, exits: [{ to: sc_a, label: x }] }
  - { id: sc_b, act: a1, title: B, location: l_ville, exits: [{ to: sc_c, label: x }, { to: sc_d, label: x }] }
  - { id: sc_c, act: a1, title: C, location: l_taverne, exits: [{ to: sc_d, label: x }, { to: sc_b, label: x }] }
  - { id: sc_d, act: a1, title: D, location: l_tour, exits: [{ to: sc_c, label: x }, { to: sc_e, label: x }] }
  - { id: sc_e, act: a2, title: E, map: crypte, exits: [{ to: sc_f, label: x }] }
  - { id: sc_f, act: a2, title: F, exits: [{ to: sc_e, label: x }] }
  - { id: sc_g, act: a2, title: G, location: l_nulle_part }
locations:
  - { id: l_ville, name: Ville, anchor: { map: region, at: [1, 1] } }
  - { id: l_taverne, name: Taverne, parent: l_ville }
  - { id: l_tour, name: Tour, parent: l_ville, anchor: { map: ville, at: [2, 2] } }
  - { id: l_cave, name: Cave, parent: l_tour }
  - { id: l_nulle_part, name: Nulle part }
";

    const MAPS: [Known<'static>; 4] = [
        Known {
            id: "region",
            name: "Région",
            scale: Scale::World,
        },
        Known {
            id: "ville",
            name: "La ville",
            scale: Scale::Place,
        },
        Known {
            id: "crypte",
            name: "La crypte",
            scale: Scale::Encounter,
        },
        Known {
            id: "oubliee",
            name: "L'oubliée",
            scale: Scale::Place,
        },
    ];

    #[test]
    fn a_fork_whose_ways_never_meet_is_one_branch_and_cycles_end() {
        let c = from_yaml(STORY).unwrap();
        let f = frieze(&c, &MAPS);
        // B on the region (its own place is pinned there), then C and D
        // lead to each other and never meet: one branch. A, which nothing
        // leads to once the walk starts at B, comes after; its cave sits in
        // the tower, pinned on the town.
        assert_eq!(
            read(&f.acts[0]),
            [
                "Région [sc_b]",
                "!Taverne [sc_c] · La ville [sc_d]",
                "La ville [sc_a]"
            ]
        );
        // E and F lead to each other, so only G opens the act: its place is
        // pinned nowhere. Then E on its own map; F has neither place nor map.
        assert_eq!(read(&f.acts[1]), ["!Nulle part [sc_g]", "La crypte [sc_e]"]);
        assert!(f.acts[2].slots.is_empty());
        assert_eq!(f.acts[2].evening, 3);
        assert_eq!(f.outside, ["oubliee"]);
    }

    #[test]
    fn without_a_start_node_an_act_opens_on_the_scenes_nothing_leads_to() {
        let mut c = from_yaml(STORY).unwrap();
        c.bible.start_node = None;
        let f = frieze(&c, &MAPS);
        // A loops on itself only: nothing else leads to it, so it opens the act.
        assert_eq!(f.acts[0].slots[0].stops[0].scenes[0].id, "sc_a");
    }

    #[test]
    fn ways_that_meet_again_form_a_branch_then_go_on_from_where_they_meet() {
        let c = from_yaml(
            r"
id: t
title: T
rules: { id: r, version: 1 }
bible: {}
acts: [{ id: a1, title: Un }]
nodes:
  - { id: s1, act: a1, title: '1', map: m1, exits: [{ to: s2, label: x }, { to: s3, label: x }, { to: s4, label: x }] }
  - { id: s2, act: a1, title: '2', map: m2, exits: [{ to: s4, label: x }] }
  - { id: s3, act: a1, title: '3', map: m2, exits: [{ to: s4, label: x }] }
  - { id: s4, act: a1, title: '4', map: m3, exits: [{ to: s5, label: x }] }
  - { id: s5, act: a1, title: '5', map: m3 }
",
        )
        .unwrap();
        let maps = [
            Known {
                id: "m1",
                name: "M1",
                scale: Scale::Place,
            },
            Known {
                id: "m2",
                name: "M2",
                scale: Scale::Place,
            },
            Known {
                id: "m3",
                name: "M3",
                scale: Scale::Place,
            },
        ];
        let f = frieze(&c, &maps);
        // The branch's two scenes share one map: one stop; the two scenes
        // on M3 one after the other make one slot.
        assert_eq!(read(&f.acts[0]), ["M1 [s1]", "M2 [s2 s3]", "M3 [s4 s5]"]);
    }
}
