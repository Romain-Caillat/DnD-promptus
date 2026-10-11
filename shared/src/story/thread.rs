//! The campaign's thread on a map (`campaign/draw-story-thread`): where
//! it starts, where it goes, where it can end, read from the scenes'
//! exits and drawn between their pins.
//!
//! Each exit from a scene to another is a step. On a map, a step joins
//! the two scenes' pins there (`pins::place_on`, so a scene in a tavern
//! inside a town stands on the town on the region's map). Steps between
//! the same two pins, in the same direction, make one segment: a pin
//! holding many scenes is one end, not a knot of lines. A step whose two
//! scenes share a pin stays inside it, and a step one end of which has
//! no pin on the map (another region, a dream) is not drawn there. A
//! scene leaving by two exits to two places is two segments — the thread
//! branches, it is never one line.
//!
//! Within an act, a scene no exit of its act leads to is a start, and a
//! scene with no exit staying in its act is an end. The thread is
//! computed, never stored: the GM's atlas reads it, players never do.
//!
//! [`path_issues`] warns when two scenes an exit links stand where no
//! path joins them (`EXIT_WITHOUT_PATH`).

use std::collections::{BTreeSet, HashMap};

use serde::Serialize;

use super::model::{Campaign, Id, Location, Node};
use super::pins::place_on;
use crate::issue::{Issue, Severity};
use crate::maps::{Cell, Map, neighbours};

/// The thread on one map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Thread {
    /// In the order of their first step in the campaign.
    pub segments: Vec<Segment>,
    /// The scenes pinned on the map, in campaign order.
    pub stops: Vec<Stop>,
}

/// The steps from one pin to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Segment {
    pub from: End,
    pub to: End,
    pub steps: Vec<Step>,
}

/// A segment's end: a pin's place and its cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct End {
    pub place: Id,
    pub at: Cell,
}

/// One exit from a scene to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Step {
    pub from: Id,
    pub to: Id,
    /// The act of the scene it leaves.
    pub act: Id,
}

/// A scene on the map, and its part in its act's thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stop {
    pub scene: Id,
    /// The pin it stands on.
    pub place: Id,
    pub act: Id,
    /// No exit of its act leads here.
    pub start: bool,
    /// No exit leaves it for a scene of its act.
    pub end: bool,
}

/// Whether some scene of `node`'s act leads to it.
fn entered(c: &Campaign, node: &Node) -> bool {
    c.nodes
        .iter()
        .any(|n| n.id != node.id && n.act == node.act && n.exits.iter().any(|e| e.to == node.id))
}

/// Whether one of `node`'s exits stays in its act.
fn continues(c: &Campaign, node: &Node) -> bool {
    node.exits
        .iter()
        .filter(|e| e.to != node.id)
        .any(|e| c.node(&e.to).is_some_and(|m| m.act == node.act))
}

/// The thread of `c` on `map`.
#[must_use]
pub fn thread_on(c: &Campaign, map: &str) -> Thread {
    let end = |node: &Node| {
        let place = place_on(c, node, map)?;
        Some(End {
            place: place.id.clone(),
            at: place.anchor.as_ref()?.at,
        })
    };
    let mut segments: Vec<Segment> = Vec::new();
    let mut stops = Vec::new();
    for n in &c.nodes {
        let Some(from) = end(n) else { continue };
        stops.push(Stop {
            scene: n.id.clone(),
            place: from.place.clone(),
            act: n.act.clone(),
            start: !entered(c, n),
            end: !continues(c, n),
        });
        for e in &n.exits {
            let Some(to) = c.node(&e.to).and_then(end) else {
                continue;
            };
            if to.place == from.place {
                continue;
            }
            let step = Step {
                from: n.id.clone(),
                to: e.to.clone(),
                act: n.act.clone(),
            };
            match segments
                .iter_mut()
                .find(|s| s.from.place == from.place && s.to.place == to.place)
            {
                Some(s) => s.steps.push(step),
                None => segments.push(Segment {
                    from: from.clone(),
                    to,
                    steps: vec![step],
                }),
            }
        }
    }
    Thread { segments, stops }
}

/// Which cells of a map can be walked to from which: a component id per
/// cell, `None` for a wall without a door or the void. On the region's
/// hexes as on the squares of a place, by the map's own neighbours.
fn components(map: &Map) -> Vec<Option<u32>> {
    let w = map.grid.width();
    let h = map.grid.height();
    let doors: BTreeSet<Cell> = map.doors.iter().map(|d| d.at).collect();
    let open = |c: Cell| {
        map.grid
            .kind(c)
            .is_some_and(|k| !k.void && (!k.wall || doors.contains(&c)))
    };
    let index = |c: Cell| (c.y as u32 * w + c.x as u32) as usize;
    let mut out = vec![None; (w * h) as usize];
    let mut next = 0;
    for y in 0..h {
        for x in 0..w {
            let start = Cell::new(x as i32, y as i32);
            if out[index(start)].is_some() || !open(start) {
                continue;
            }
            out[index(start)] = Some(next);
            let mut stack = vec![start];
            while let Some(c) = stack.pop() {
                for n in neighbours(map.geometry(), c) {
                    if map.grid.contains(n) && out[index(n)].is_none() && open(n) {
                        out[index(n)] = Some(next);
                        stack.push(n);
                    }
                }
            }
            next += 1;
        }
    }
    out
}

/// The maps linked by exits, followed either way: a group id per map.
fn linked(maps: &[Map]) -> HashMap<&str, usize> {
    let mut group: HashMap<&str, usize> = maps
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.as_str(), i))
        .collect();
    // Few maps: merge until nothing moves.
    loop {
        let mut moved = false;
        for m in maps {
            for e in &m.exits {
                let (Some(&a), Some(&b)) = (group.get(m.id.as_str()), group.get(e.to.as_str()))
                else {
                    continue;
                };
                if a != b {
                    let (keep, drop) = (a.min(b), a.max(b));
                    for g in group.values_mut() {
                        if *g == drop {
                            *g = keep;
                        }
                    }
                    moved = true;
                }
            }
        }
        if !moved {
            return group;
        }
    }
}

/// The places up `node`'s chain, from its own.
fn chain<'a>(c: &'a Campaign, node: &Node) -> Vec<&'a Location> {
    node.location
        .as_deref()
        .map_or_else(Vec::new, |l| super::pins::chain(c, l))
}

/// The anchors of `places` on maps `maps` has.
fn anchors<'a>(places: &[&'a Location], maps: &[Map]) -> Vec<(&'a str, Cell)> {
    places
        .iter()
        .filter_map(|l| l.anchor.as_ref())
        .filter(|a| maps.iter().any(|m| m.id == a.map))
        .map(|a| (a.map.as_str(), a.at))
        .collect()
}

/// Exits linking two scenes no path joins (`campaign/draw-story-thread`),
/// as warnings on `nodes[i].exits[j]`: the table could not walk from one
/// to the other. Two scenes are joined when their chains of places share
/// a place (a tavern and the port of one town), or when one place of
/// each chain is anchored on the same map, on cells walls do not part
/// (a door opens), or on two maps joined by exits, followed either way.
/// A scene with no anchored place has nothing to check
/// (`SCENE_WITHOUT_PLACE` says it), nor does an anchor on a map `maps`
/// lacks or in a wall. One walk over each map's cells, made when first
/// needed, then reused: a large region costs one pass.
#[must_use]
pub fn path_issues(c: &Campaign, maps: &[Map]) -> Vec<Issue> {
    let groups = linked(maps);
    let mut walks: HashMap<&str, Vec<Option<u32>>> = HashMap::new();
    let mut issues = Vec::new();
    for (i, n) in c.nodes.iter().enumerate() {
        let here = chain(c, n);
        for (j, e) in n.exits.iter().enumerate() {
            let Some(m) = c.node(&e.to).filter(|m| m.id != n.id) else {
                continue;
            };
            let there = chain(c, m);
            if here.iter().any(|a| there.iter().any(|b| a.id == b.id)) {
                continue;
            }
            let (from, to) = (anchors(&here, maps), anchors(&there, maps));
            if from.is_empty() || to.is_empty() {
                continue;
            }
            let mut cell = |map: &str, at: Cell| -> Option<u32> {
                let m = maps.iter().find(|m| m.id == map)?;
                if !m.grid.contains(at) {
                    return None;
                }
                let walk = walks.entry(m.id.as_str()).or_insert_with(|| components(m));
                walk[(at.y as u32 * m.grid.width() + at.x as u32) as usize]
            };
            let mut joined = false;
            'pairs: for &(ma, a) in &from {
                for &(mb, b) in &to {
                    joined = if ma == mb {
                        match (cell(ma, a), cell(mb, b)) {
                            (Some(x), Some(y)) => x == y,
                            // An anchor in a wall or off the grid: said elsewhere.
                            _ => true,
                        }
                    } else {
                        groups.get(ma) == groups.get(mb)
                    };
                    if joined {
                        break 'pairs;
                    }
                }
            }
            if !joined {
                issues.push(Issue {
                    severity: Severity::Warning,
                    code: "EXIT_WITHOUT_PATH",
                    path: format!("nodes[{i}].exits[{j}]"),
                    detail: format!(
                        "`{}` leads to `{}`, but no path joins their places on the maps",
                        n.id, m.id
                    ),
                    message: None,
                });
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{Door, DoorState, Exit};
    use crate::story::edit::{Edit, apply};
    use crate::story::from_yaml;

    const STORY: &str = r"
id: t
title: T
rules: { id: r, version: 1 }
bible: {}
acts: [{ id: a1, title: Un }, { id: a2, title: Deux }]
nodes:
  - { id: sc_a, act: a1, title: A, location: l_taverne, exits: [{ to: sc_b, label: B }, { to: sc_c, label: C }] }
  - { id: sc_b, act: a1, title: B, location: l_port, exits: [{ to: sc_d, label: D }] }
  - { id: sc_c, act: a1, title: C, location: l_phare, exits: [{ to: sc_d, label: D }, { to: sc_e, label: E }] }
  - { id: sc_d, act: a1, title: D, location: l_taverne, exits: [{ to: sc_f, label: F }] }
  - { id: sc_e, act: a1, title: E }
  - { id: sc_f, act: a2, title: F, location: l_phare }
  - { id: sc_g, act: a2, title: G, location: l_ile }
locations:
  - { id: l_ville, name: Ville, anchor: { map: region, at: [1, 1] } }
  - { id: l_taverne, name: Taverne, parent: l_ville, anchor: { map: ville, at: [1, 1] } }
  - { id: l_port, name: Port, parent: l_ville, anchor: { map: ville, at: [4, 1] } }
  - { id: l_phare, name: Phare, anchor: { map: region, at: [3, 0] } }
  - { id: l_ile, name: Île, anchor: { map: region, at: [0, 2] } }
";

    fn story() -> Campaign {
        from_yaml(STORY).unwrap()
    }

    /// (from place, to place, ["scene>scene"]) of each segment.
    fn lines(t: &Thread) -> Vec<(&str, &str, Vec<String>)> {
        t.segments
            .iter()
            .map(|s| {
                (
                    s.from.place.as_str(),
                    s.to.place.as_str(),
                    s.steps
                        .iter()
                        .map(|x| format!("{}>{}", x.from, x.to))
                        .collect(),
                )
            })
            .collect()
    }

    fn steps(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_branch_stays_two_segments_and_steps_inside_a_pin_draw_nothing() {
        let c = story();
        // On the town: the tavern and the port. sc_a's other branch, to
        // the lighthouse, leaves the town's map.
        let town = thread_on(&c, "ville");
        assert_eq!(
            lines(&town),
            [
                ("l_taverne", "l_port", steps(&["sc_a>sc_b"])),
                ("l_port", "l_taverne", steps(&["sc_b>sc_d"])),
            ]
        );
        assert_eq!(town.segments[0].from.at, Cell::new(1, 1));
        assert_eq!(town.segments[0].to.at, Cell::new(4, 1));
        // On the region the town holds the tavern's and the port's scenes:
        // A > B and B > D stay inside its pin; A branches to the
        // lighthouse, and two steps share the town > lighthouse segment.
        let region = thread_on(&c, "region");
        assert_eq!(
            lines(&region),
            [
                ("l_ville", "l_phare", steps(&["sc_a>sc_c", "sc_d>sc_f"])),
                ("l_phare", "l_ville", steps(&["sc_c>sc_d"])),
            ]
        );
        // A scene with no place (sc_e) breaks nothing: no stop, no segment.
        assert!(region.stops.iter().all(|s| s.scene != "sc_e"));
        assert_eq!(region.stops.len(), 6);
    }

    #[test]
    fn an_exit_added_adds_a_segment_and_starts_and_ends_follow_the_act() {
        let c = story();
        let region = thread_on(&c, "region");
        let stop = |t: &Thread, id: &str| t.stops.iter().find(|s| s.scene == id).cloned().unwrap();
        // Act 1 starts at sc_a; sc_d only leaves for act 2: an end of act
        // 1, its step still on the thread. sc_f opens act 2 and ends it.
        assert!(stop(&region, "sc_a").start && !stop(&region, "sc_a").end);
        assert!(!stop(&region, "sc_b").start);
        assert!(stop(&region, "sc_d").end);
        assert_eq!(stop(&region, "sc_d").place, "l_ville");
        assert!(stop(&region, "sc_f").start && stop(&region, "sc_f").end);
        assert_eq!(region.segments[0].steps[1].act, "a1");

        // The scene editor saves the whole list of exits.
        let (c, _) = apply(
            &c,
            &[Edit::Set {
                target: "sc_f".into(),
                field: "exits".into(),
                value: serde_json::json!([{ "to": "sc_b", "label": "Retour" }]),
            }],
        )
        .unwrap();
        let after = thread_on(&c, "region");
        assert_eq!(after.segments.len(), region.segments.len());
        // On the region, sc_f > sc_b joins the existing lighthouse > town
        // segment; on the town's map it is outside.
        assert_eq!(
            lines(&after)[1],
            ("l_phare", "l_ville", steps(&["sc_c>sc_d", "sc_f>sc_b"]))
        );
        // An exit from the lighthouse to the island is a new segment.
        let (c, _) = apply(
            &c,
            &[Edit::Set {
                target: "sc_f".into(),
                field: "exits".into(),
                value: serde_json::json!([{ "to": "sc_b", "label": "Retour" }, { "to": "sc_g", "label": "Île" }]),
            }],
        )
        .unwrap();
        let added = thread_on(&c, "region");
        assert_eq!(added.segments.len(), after.segments.len() + 1);
        assert_eq!(
            lines(&added)[2],
            ("l_phare", "l_ile", steps(&["sc_f>sc_g"]))
        );
        assert!(!stop(&added, "sc_f").end && stop(&added, "sc_g").end);
        // In the town, a second exit between the same two pins joins
        // their segment, and sc_a is no start once sc_b leads back to it.
        let (c, _) = apply(
            &c,
            &[Edit::Set {
                target: "sc_b".into(),
                field: "exits".into(),
                value: serde_json::json!([{ "to": "sc_d", "label": "D" }, { "to": "sc_a", "label": "A" }]),
            }],
        )
        .unwrap();
        let town = thread_on(&c, "ville");
        assert_eq!(
            lines(&town)[1],
            ("l_port", "l_taverne", steps(&["sc_b>sc_d", "sc_b>sc_a"]))
        );
        assert!(!stop(&town, "sc_a").start, "sc_b now leads back to sc_a");
    }

    fn map(id: &str, rows: &[&str], exits: &[&str]) -> Map {
        let mut m = Map::from_yaml(&format!(
            "version: 1\nid: {id}\nname: {id}\nscale: place\ntheme: t\nlayers:\n  - {{ id: base, name: Base, visibility: all }}\ngrid:\n  legend:\n    '.': {{ terrain: sol }}\n    '#': {{ terrain: mur, wall: true }}\n  rows:\n{}\n",
            rows.iter()
                .map(|r| format!("    - '{r}'"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
        .expect("a test map");
        m.exits = exits
            .iter()
            .map(|to| Exit {
                id: format!("vers-{to}"),
                cells: vec![Cell::new(0, 0)],
                to: (*to).into(),
                arrive: None,
                label: None,
                layer: "base".into(),
            })
            .collect();
        m
    }

    #[test]
    fn two_linked_scenes_with_no_path_between_their_places_are_warned() {
        let mut c = story();
        // A wall splits the town, but the tavern and the port are both in
        // it: their chains share the town.
        let walled = map("ville", &["..#..", "..#..", "..#.."], &[]);
        let region = map("region", &["....", "....", "...."], &[]);
        let codes = |c: &Campaign, maps: &[Map]| -> Vec<String> {
            path_issues(c, maps)
                .iter()
                .map(|i| format!("{} {}", i.code, i.path))
                .collect()
        };
        // The lighthouse stands on the region, the tavern on the town (in
        // the town, itself on the region): joined through the chain's
        // anchor on the region.
        assert!(codes(&c, &[walled.clone(), region.clone()]).is_empty());

        // Without the town as parent, the wall parts the tavern and the
        // port, and no exit joins the town's map to the region.
        for l in &mut c.locations {
            if l.id == "l_taverne" || l.id == "l_port" {
                l.parent = None;
            }
        }
        let found = codes(&c, &[walled.clone(), region.clone()]);
        assert_eq!(
            found,
            [
                "EXIT_WITHOUT_PATH nodes[0].exits[0]",
                "EXIT_WITHOUT_PATH nodes[0].exits[1]",
                "EXIT_WITHOUT_PATH nodes[1].exits[0]",
                "EXIT_WITHOUT_PATH nodes[2].exits[0]",
                "EXIT_WITHOUT_PATH nodes[3].exits[0]",
            ]
        );
        assert!(
            path_issues(&c, &[walled.clone(), region.clone()])
                .iter()
                .all(|i| i.severity == Severity::Warning)
        );

        // A door in the wall, and an exit from the town to the region
        // (followed back up too): every path opens.
        let mut doored = map("ville", &["..#..", "..#..", "..#.."], &["region"]);
        doored.doors.push(Door {
            id: "porte-1".into(),
            at: Cell::new(2, 1),
            state: DoorState::Closed,
            label: None,
            layer: "base".into(),
        });
        assert!(codes(&c, &[doored, region]).is_empty());
    }

    #[test]
    fn hexes_are_walked_by_their_own_neighbours() {
        // Odd rows shift right: (1, 1) touches (1, 0) and (2, 0) on hexes,
        // not (0, 0), which squares reach across the corner.
        let mut m = map("region", &[".#", "#."], &[]);
        m.scale = crate::maps::Scale::World;
        let w = components(&m);
        assert_eq!(w[1], None, "a wall is walked by no one");
        assert_ne!(w[0], w[3], "(0, 0) and (1, 1) are no neighbours on hexes");
        m.scale = crate::maps::Scale::Place;
        let w = components(&m);
        assert_eq!(w[0], w[3], "squares meet at the corner");
    }
}
