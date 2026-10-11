//! Who and what stands on the maps (`campaign/place-cast-and-clues`).
//!
//! Writing a scene is also placing who is there and what is found there.
//! The preparation keeps it in the story, next to the entity it places:
//! - an NPC's `spots` — a map, a cell, and the scene it is for (none: every
//!   scene played on that map);
//! - a clue's `anchor` — its scene is already the clue's `node`;
//! - a fight's `anchor` (`Encounter::anchor`) — where it breaks out on a
//!   place's map; it opens the scene's battle plan (`Node::map`), whose
//!   foe starts place the adversaries.
//!
//! The maps stay views: nothing here is stored on a map. The evening
//! copies the NPCs to hidden tokens when it shows the map (`board`), the
//! clues are drawn from the story and the table's found clues, and the
//! fights are the GM's alone.
//!
//! Pure: the GM's atlas, the GM's board and the players' projection all
//! read the marks here.

use serde::Serialize;

use super::model::{Campaign, Id, Node};
use crate::maps::{Cell, Map, Side};

/// What stands on one map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Cast {
    pub npcs: Vec<NpcMark>,
    pub clues: Vec<ClueMark>,
    pub fights: Vec<FightMark>,
}

/// One of an NPC's spots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NpcMark {
    pub npc: Id,
    pub name: String,
    /// The spot's place in the NPC's `spots`: what a move or a removal
    /// names.
    pub index: usize,
    pub at: Cell,
    /// The scene it is for; none for every scene on the map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene: Option<Id>,
}

/// A clue's spot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClueMark {
    pub clue: Id,
    /// What the players learn (the only part they ever see).
    pub text: String,
    pub at: Cell,
    /// Its scene (`Clue::node`).
    pub scene: Id,
}

/// Where a scene's fight breaks out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FightMark {
    pub scene: Id,
    pub title: String,
    /// 1-based, the scene's place in the campaign (as its pin).
    pub number: usize,
    pub at: Cell,
    /// The battle plan: the scene's own map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<String>,
    /// How many opponents the fight holds.
    pub opponents: u32,
    /// How many of them the battle plan already places, when it is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placed: Option<u32>,
}

/// Everything placed on `map`. `plans` are the maps a fight's battle
/// plan may be (the atlas passes every map, the evening none): each
/// fight then says how many opponents it already places.
#[must_use]
pub fn on_map(c: &Campaign, map: &str, plans: &[Map]) -> Cast {
    let npcs = c
        .npcs
        .iter()
        .flat_map(|n| {
            n.spots
                .iter()
                .enumerate()
                .filter(|(_, s)| s.map == map)
                .map(|(index, s)| NpcMark {
                    npc: n.id.clone(),
                    name: n.name.clone(),
                    index,
                    at: s.at,
                    scene: s.scene.clone(),
                })
        })
        .collect();
    let clues = c
        .clues
        .iter()
        .filter_map(|cl| {
            let a = cl.anchor.as_ref().filter(|a| a.map == map)?;
            Some(ClueMark {
                clue: cl.id.clone(),
                text: cl.text.clone(),
                at: a.at,
                scene: cl.node.clone(),
            })
        })
        .collect();
    let fights = c
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| {
            // A ship battle opens from its own panel, not from a mark.
            let e = n.encounter.as_ref().filter(|e| e.vehicles.is_none())?;
            let a = e.anchor.as_ref().filter(|a| a.map == map)?;
            let plan = n
                .map
                .as_deref()
                .and_then(|id| plans.iter().find(|m| m.id == id));
            Some(FightMark {
                scene: n.id.clone(),
                title: n.title.clone(),
                number: i + 1,
                at: a.at,
                map: n.map.clone(),
                opponents: opponents(n),
                placed: plan.map(|m| placed(n, m)),
            })
        })
        .collect();
    Cast {
        npcs,
        clues,
        fights,
    }
}

/// How many opponents `node`'s fight holds.
#[must_use]
pub fn opponents(node: &Node) -> u32 {
    node.encounter
        .as_ref()
        .map_or(0, |e| e.opponents.iter().map(|o| o.count.max(1)).sum())
}

/// How many of `node`'s opponents its battle plan `map` places: one per
/// foe start, as the fight sets them down (a start naming the opponent
/// first, then any). Fewer than [`opponents`] and the fight cannot open
/// (`NO_START_LEFT`).
#[must_use]
pub fn placed(node: &Node, map: &Map) -> u32 {
    let foes = map
        .starts
        .iter()
        .filter(|s| s.side == Some(Side::Foes))
        .count();
    opponents(node).min(u32::try_from(foes).unwrap_or(u32::MAX))
}

/// The NPC spots on `map` an evening sets down when it shows the map
/// while `scene` is the current one (or none): each NPC's spots for
/// every scene, and those for `scene`. As `(npc, spot index, cell)`.
#[must_use]
pub fn standing_on<'a>(
    c: &'a Campaign,
    map: &str,
    scene: Option<&str>,
) -> Vec<(&'a Id, usize, Cell)> {
    c.npcs
        .iter()
        .flat_map(|n| {
            n.spots.iter().enumerate().filter_map(move |(i, s)| {
                (s.map == map && (s.scene.is_none() || s.scene.as_deref() == scene))
                    .then_some((&n.id, i, s.at))
            })
        })
        .collect()
}

/// The NPC spots for `scene` alone, on `map`: what opening that scene
/// adds to the map already shown.
#[must_use]
pub fn arriving_with<'a>(c: &'a Campaign, map: &str, scene: &str) -> Vec<(&'a Id, usize, Cell)> {
    c.npcs
        .iter()
        .flat_map(|n| {
            n.spots.iter().enumerate().filter_map(move |(i, s)| {
                (s.map == map && s.scene.as_deref() == Some(scene)).then_some((&n.id, i, s.at))
            })
        })
        .collect()
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
  - id: sc_cimetiere
    act: a1
    title: Le cimetière
    map: crypte
    encounter:
      opponents: [{ who: adv_goule, count: 3 }]
      anchor: { map: cimetiere, at: [4, 1] }
  - { id: sc_veillee, act: a1, title: La veillée }
revelations: [{ id: rev_a, statement: A, importance: optional }]
clues:
  - { id: cl_tombe, revelation: rev_a, node: sc_cimetiere, text: Une tombe ouverte, anchor: { map: cimetiere, at: [2, 2] } }
  - { id: cl_cloche, revelation: rev_a, node: sc_veillee, text: La cloche }
npcs:
  - id: pnj_oswin
    name: Oswin
    disposition: neutral
    spots:
      - { map: cimetiere, at: [1, 1] }
      - { map: cimetiere, at: [3, 3], scene: sc_veillee }
      - { map: village, at: [0, 0] }
adversaries:
  - { id: adv_goule, name: Goule, stats: { hit_points: 9 } }
";

    #[test]
    fn a_map_holds_the_npcs_clues_and_fights_placed_on_it() {
        let c = from_yaml(STORY).unwrap();
        let cast = on_map(&c, "cimetiere", &[]);
        assert_eq!(
            cast.npcs
                .iter()
                .map(|m| (m.index, m.at, m.scene.as_deref()))
                .collect::<Vec<_>>(),
            [
                (0, Cell::new(1, 1), None),
                (1, Cell::new(3, 3), Some("sc_veillee"))
            ]
        );
        assert_eq!(cast.clues.len(), 1);
        assert_eq!(cast.clues[0].clue, "cl_tombe");
        assert_eq!(cast.clues[0].scene, "sc_cimetiere");
        assert_eq!(cast.fights.len(), 1);
        let f = &cast.fights[0];
        assert_eq!((f.number, f.opponents, f.placed), (1, 3, None));
        assert_eq!(f.map.as_deref(), Some("crypte"));
        assert_eq!(on_map(&c, "village", &[]).npcs.len(), 1);
    }

    #[test]
    fn the_evening_sets_down_the_spots_for_every_scene_and_the_current_one() {
        let c = from_yaml(STORY).unwrap();
        let cells = |v: Vec<(&Id, usize, Cell)>| v.into_iter().map(|x| x.1).collect::<Vec<_>>();
        assert_eq!(cells(standing_on(&c, "cimetiere", None)), [0]);
        assert_eq!(
            cells(standing_on(&c, "cimetiere", Some("sc_veillee"))),
            [0, 1]
        );
        assert_eq!(cells(arriving_with(&c, "cimetiere", "sc_veillee")), [1]);
        assert!(arriving_with(&c, "cimetiere", "sc_cimetiere").is_empty());
    }
}
