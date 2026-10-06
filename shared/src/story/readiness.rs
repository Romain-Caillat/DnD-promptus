//! Is an act ready to be played? (`campaign/check-act-readiness`,
//! `docs/lecons-des-parties.md` §3 chain 2.)
//!
//! An act is **ready** when every scene has its fields, every piece of
//! knowledge the act needs has at least three paths and not only
//! through optional scenes, every player has at least one hook in the
//! act, every planned fight has opponents with numbers and a tactic,
//! and every planned fight has been simulated. The GM sees one gauge
//! per act and what is missing; they decide (it never blocks).
//!
//! The Brasier's act 1 was not ready and nothing said so; the
//! Corsaires' act 1 hid the Greyhound's route in one optional scene.
//! Both are this module's tests.

use std::collections::BTreeSet;

use serde::Serialize;

use super::encounter::encounter_scenario;
use super::library::Library;
use super::model::{Campaign, Id, Importance, Node};
use super::validate::MIN_CRITICAL_CLUE_NODES;
use crate::combat::scenario::PolicyKind;
use crate::combat::simulate::{SimParams, simulate};

/// Fights played per planned encounter: enough to know it runs to an
/// end, cheap enough to recompute on every look.
pub const FIGHTS: u32 = 12;

/// The five things an act needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    SceneFields,
    KnowledgePaths,
    PlayerHooks,
    Encounters,
    Fights,
}

/// What one check finds missing, on one thing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Gap {
    /// The scene, revelation, party member or fight it is about.
    pub id: Id,
    pub name: String,
    /// `MISSING_FIELDS`, `FEW_PATHS`, `ONLY_OPTIONAL`, `NO_HOOK`,
    /// `NO_STATS`, `NO_TACTICS`, `NOT_STAGED`, `SIMULATION_FAILED`,
    /// `NO_RULES`.
    pub code: &'static str,
    /// The fields a scene lacks (`MISSING_FIELDS`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<&'static str>,
    /// Distinct scenes giving a clue (`FEW_PATHS`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<usize>,
    /// English, for developers and logs.
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Check {
    pub kind: CheckKind,
    /// Things that pass, out of `total`.
    pub done: usize,
    pub total: usize,
    pub gaps: Vec<Gap>,
}

/// One act's gauge.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActReadiness {
    pub act: Id,
    pub title: String,
    pub ready: bool,
    pub done: usize,
    pub total: usize,
    pub checks: Vec<Check>,
}

/// The fields every scene carries (Romain's prep format, chain 2).
fn missing_fields(n: &Node) -> Vec<&'static str> {
    let mut out = Vec::new();
    if n.summary.trim().is_empty() {
        out.push("summary");
    }
    if n.location.is_none() {
        out.push("location");
    }
    if n.ambience.mood.trim().is_empty() {
        out.push("ambience");
    }
    if n.read_aloud.trim().is_empty() {
        out.push("read_aloud");
    }
    if n.flow.trim().is_empty() {
        out.push("flow");
    }
    if !n.exits.is_empty() && n.transition.trim().is_empty() {
        out.push("transition");
    }
    out
}

fn check(kind: CheckKind, total: usize, gaps: Vec<Gap>) -> Check {
    // A thing with several gaps (a fight without stats nor tactic)
    // counts once.
    let failing: BTreeSet<&str> = gaps.iter().map(|g| g.id.as_str()).collect();
    Check {
        kind,
        done: total.saturating_sub(failing.len()),
        total,
        gaps,
    }
}

fn gap(id: &str, name: &str, code: &'static str, detail: String) -> Gap {
    Gap {
        id: id.to_string(),
        name: name.to_string(),
        code,
        fields: Vec::new(),
        paths: None,
        detail,
    }
}

/// The gauge of every act of `campaign`, in order. `library` gives the
/// rule system and the maps the fights are simulated with; without
/// them, the fights are not simulated, and say so.
#[must_use]
pub fn readiness(campaign: &Campaign, library: &Library<'_>) -> Vec<ActReadiness> {
    campaign
        .acts
        .iter()
        .map(|act| {
            let scenes: Vec<&Node> = campaign.nodes.iter().filter(|n| n.act == act.id).collect();
            let checks = vec![
                scene_fields(&scenes),
                knowledge_paths(campaign, &scenes),
                player_hooks(campaign, &scenes),
                encounters(campaign, &scenes),
                fights(campaign, &scenes, library),
            ];
            let done = checks.iter().map(|c| c.done).sum();
            let total = checks.iter().map(|c| c.total).sum();
            ActReadiness {
                act: act.id.clone(),
                title: act.title.clone(),
                // An act without scenes is not prepared yet.
                ready: !scenes.is_empty() && checks.iter().all(|c| c.gaps.is_empty()),
                done,
                total,
                checks,
            }
        })
        .collect()
}

fn scene_fields(scenes: &[&Node]) -> Check {
    let gaps = scenes
        .iter()
        .filter_map(|n| {
            let fields = missing_fields(n);
            (!fields.is_empty()).then(|| Gap {
                detail: format!("`{}` lacks {}", n.id, fields.join(", ")),
                fields,
                ..gap(&n.id, &n.title, "MISSING_FIELDS", String::new())
            })
        })
        .collect();
    check(CheckKind::SceneFields, scenes.len(), gaps)
}

/// What the act needs players to know: what its scenes require on
/// entry, and the critical revelations its scenes give clues to.
fn knowledge_paths(c: &Campaign, scenes: &[&Node]) -> Check {
    let here: BTreeSet<&str> = scenes.iter().map(|n| n.id.as_str()).collect();
    let mut needed: Vec<&str> = Vec::new();
    for n in scenes {
        for r in &n.requires {
            if !needed.contains(&r.as_str()) {
                needed.push(r);
            }
        }
    }
    for r in &c.revelations {
        if r.importance == Importance::Critical
            && c.clues_for(&r.id).any(|cl| here.contains(cl.node.as_str()))
            && !needed.contains(&r.id.as_str())
        {
            needed.push(&r.id);
        }
    }
    let mut gaps = Vec::new();
    let mut total = 0;
    for rev in needed {
        let Some(r) = c.revelations.iter().find(|r| r.id == rev) else {
            continue; // a dangling reference, the validator's to report
        };
        total += 1;
        let nodes: BTreeSet<&str> = c.clues_for(rev).map(|cl| cl.node.as_str()).collect();
        let required_by: Vec<&str> = scenes
            .iter()
            .filter(|n| n.requires.iter().any(|x| x == rev))
            .map(|n| n.id.as_str())
            .collect();
        // Where the players can learn it before entering a scene that
        // requires it: anywhere but that scene.
        let paths: Vec<&str> = nodes
            .iter()
            .copied()
            .filter(|n| !required_by.contains(n))
            .collect();
        if paths.len() < MIN_CRITICAL_CLUE_NODES {
            gaps.push(Gap {
                paths: Some(paths.len()),
                ..gap(
                    rev,
                    &r.statement,
                    "FEW_PATHS",
                    format!("`{rev}` is given in {} scene(s)", paths.len()),
                )
            });
        }
        if !paths.is_empty()
            && paths
                .iter()
                .all(|n| c.node(n).is_some_and(|node| node.optional))
        {
            gaps.push(gap(
                rev,
                &r.statement,
                "ONLY_OPTIONAL",
                format!(
                    "`{rev}` is only given in optional scenes: {}",
                    paths.join(", ")
                ),
            ));
        }
    }
    check(CheckKind::KnowledgePaths, total, gaps)
}

fn player_hooks(c: &Campaign, scenes: &[&Node]) -> Check {
    let gaps = c
        .party
        .iter()
        .filter(|p| {
            !scenes
                .iter()
                .any(|n| n.player_hooks.iter().any(|h| h.character == p.id))
        })
        .map(|p| {
            gap(
                &p.id,
                &p.name,
                "NO_HOOK",
                format!("`{}` has no hook in the act", p.id),
            )
        })
        .collect();
    check(CheckKind::PlayerHooks, c.party.len(), gaps)
}

/// Whether `who` fights with numbers: a stat block naming the rule
/// system's, or written out.
fn has_stats(c: &Campaign, who: &str) -> bool {
    let block = c
        .adversary(who)
        .map(|a| &a.stats)
        .or_else(|| c.npc(who).and_then(|n| n.stats.as_ref()));
    block.is_some_and(|s| {
        s.from_rules.is_some() || (s.hit_points.is_some() && s.armor_class.is_some())
    })
}

fn encounters(c: &Campaign, scenes: &[&Node]) -> Check {
    let mut gaps = Vec::new();
    let mut total = 0;
    for n in scenes {
        let Some(e) = &n.encounter else { continue };
        total += 1;
        let unnumbered: Vec<&str> = e
            .opponents
            .iter()
            .filter(|o| !has_stats(c, &o.who))
            .map(|o| o.who.as_str())
            .collect();
        if !unnumbered.is_empty() {
            gaps.push(gap(
                &n.id,
                &n.title,
                "NO_STATS",
                format!("no numbers for {}", unnumbered.join(", ")),
            ));
        }
        if e.tactics.is_empty() {
            gaps.push(gap(&n.id, &n.title, "NO_TACTICS", "no tactic".into()));
        }
    }
    check(CheckKind::Encounters, total, gaps)
}

fn fights(c: &Campaign, scenes: &[&Node], library: &Library<'_>) -> Check {
    let planned: Vec<&&Node> = scenes.iter().filter(|n| n.encounter.is_some()).collect();
    let mut gaps = Vec::new();
    for n in &planned {
        let Some(rules) = library.rules else {
            gaps.push(gap(
                &n.id,
                &n.title,
                "NO_RULES",
                "no rule system to simulate with".into(),
            ));
            continue;
        };
        let staged = encounter_scenario(c, n, rules).map_err(|e| e.to_string());
        let map = n
            .map
            .as_deref()
            .and_then(|id| library.maps.unwrap_or(&[]).iter().find(|m| m.id == id));
        let result = staged.and_then(|sc| {
            let map = map.ok_or_else(|| "the scene's map is not known".to_string())?;
            let params = SimParams::new(FIGHTS, 1);
            simulate(
                rules,
                &sc,
                map,
                PolicyKind::Brawler,
                PolicyKind::Brawler,
                &params,
            )
            .map(|_| ())
            .map_err(|e| format!("simulation: {e}"))
        });
        if let Err(detail) = result {
            let code = if detail.starts_with("simulation:") {
                "SIMULATION_FAILED"
            } else {
                "NOT_STAGED"
            };
            gaps.push(gap(&n.id, &n.title, code, detail));
        }
    }
    check(CheckKind::Fights, planned.len(), gaps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::Map;
    use crate::rules::RuleSystem;
    use crate::story::from_yaml;

    const PLAYED: &str = include_str!("../../../content/fixtures/corsaires-acte-1-joue.yaml");
    const CORSAIRES: &str = include_str!("../../../content/campaigns/corsaires/campagne.yaml");
    const CORSAIRES_RULES: &str = include_str!("../../../content/rules/corsaires/v1.yaml");
    const QUAY: &str = include_str!("../../../content/maps/corsaires/quai-port-louis.yaml");
    const BRASIER: &str = include_str!("../../../content/campaigns/brasier/campagne.yaml");
    const BRASIER_RULES: &str = include_str!("../../../content/rules/brasier/v1.yaml");
    const CORRIDOR: &str = include_str!("../../../content/maps/brasier/cure-dent-coursive.yaml");

    fn gauge(yaml: &str, rules: &str, map: &str) -> Vec<ActReadiness> {
        let rules = RuleSystem::from_yaml(rules).unwrap();
        let maps = [Map::from_yaml(map).unwrap()];
        readiness(
            &from_yaml(yaml).unwrap(),
            &Library {
                rules: Some(&rules),
                maps: Some(&maps),
            },
        )
    }

    fn gaps(act: &ActReadiness, kind: CheckKind) -> Vec<(&'static str, &str)> {
        act.checks
            .iter()
            .find(|c| c.kind == kind)
            .unwrap()
            .gaps
            .iter()
            .map(|g| (g.code, g.id.as_str()))
            .collect()
    }

    #[test]
    fn the_corsaires_act_as_played_hides_the_route_in_an_optional_scene() {
        let acts = gauge(PLAYED, CORSAIRES_RULES, QUAY);
        let act = &acts[0];
        assert!(!act.ready);
        let knowledge = gaps(act, CheckKind::KnowledgePaths);
        assert!(
            knowledge.contains(&("ONLY_OPTIONAL", "rev_route")),
            "{knowledge:?}"
        );
        // Nobody had a scene written for them, and the quay fight had
        // no tactic and no map to be tried on.
        assert_eq!(gaps(act, CheckKind::PlayerHooks).len(), 6);
        assert_eq!(
            gaps(act, CheckKind::Encounters),
            [("NO_TACTICS", "sc_quai")]
        );
        assert_eq!(gaps(act, CheckKind::Fights), [("NOT_STAGED", "sc_quai")]);
    }

    #[test]
    fn the_corsaires_act_as_rewritten_is_ready() {
        let acts = gauge(CORSAIRES, CORSAIRES_RULES, QUAY);
        assert!(acts[0].ready, "{:#?}", acts[0]);
        assert_eq!(acts[0].done, acts[0].total);
    }

    #[test]
    fn the_brasier_act_one_is_not_ready_and_says_why() {
        let acts = gauge(BRASIER, BRASIER_RULES, CORRIDOR);
        let act = &acts[0];
        assert!(!act.ready);
        assert!(act.done < act.total);
        // A scene left half-written, the engine's secret given once, and
        // the boarding fight against a stat block the rules do not have.
        assert_eq!(
            gaps(act, CheckKind::SceneFields),
            [("MISSING_FIELDS", "sc_carapace_sereth")]
        );
        assert!(gaps(act, CheckKind::KnowledgePaths).contains(&("FEW_PATHS", "rev_moteur")));
        assert_eq!(
            gaps(act, CheckKind::Fights),
            [("NOT_STAGED", "sc_toboggan")]
        );
        // Acts with no scene yet are not ready either.
        assert!(acts[1..].iter().all(|a| !a.ready));
    }
}
