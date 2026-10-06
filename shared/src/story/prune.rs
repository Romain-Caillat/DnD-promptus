//! Dropping what a model invented (`ai/generate-campaign`).
//!
//! A generated campaign may point at ids the model never declared — a
//! scene in a place nobody wrote, a clue for a revelation that does not
//! exist — or declare one id twice. [`prune`] removes each such
//! reference, the smallest way the format allows, and says what it
//! removed so the GM can see it:
//! - an optional reference (`location`, `faction`, `source`…) is
//!   emptied;
//! - a scene's act that does not exist becomes the first act;
//! - an element that cannot stand without the reference (a clue without
//!   its scene, an exit to nowhere, an NPC presence of nobody) is
//!   removed from its list;
//! - of two elements with the same id, the later one is removed;
//! - a rule-system id the rule system does not have (an adversary's
//!   `from_rules`, a check's stat) is emptied, or its element removed,
//!   the same way.
//!
//! Nothing else is touched: the validator's other findings (three-clue
//! rule, unreachable scenes…) are for the repair step and the GM.

use serde::Serialize;
use serde_json::Value;

use super::library::{Library, validate_with};
use super::model::Campaign;
use super::validate::Severity;

/// One thing [`prune`] removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pruned {
    /// The validator's code: `REF_DANGLING`, `REF_WRONG_KIND`,
    /// `ID_DUPLICATE`, `ID_INVALID`.
    pub code: &'static str,
    /// Where it was, in the document as it stood when it was removed.
    pub path: String,
    /// The validator's detail (English).
    pub detail: String,
}

const CODES: [&str; 8] = [
    "REF_DANGLING",
    "REF_WRONG_KIND",
    "ID_DUPLICATE",
    "ID_INVALID",
    "RULES_UNKNOWN_ADVERSARY",
    "RULES_UNKNOWN_ITEM",
    "RULES_UNKNOWN_CLASS",
    "RULES_UNKNOWN_ABILITY",
];

/// A safety bound: each round removes at least one thing.
const ROUNDS_MAX: usize = 2_000;

/// `campaign` without its invented ids, and what was removed. Rule-system
/// ids are checked when `library` has the rule system.
#[must_use]
pub fn prune(campaign: &Campaign, library: &Library<'_>) -> (Campaign, Vec<Pruned>) {
    let mut current = campaign.clone();
    let mut pruned = Vec::new();
    for _ in 0..ROUNDS_MAX {
        let Some(issue) = validate_with(&current, library)
            .into_iter()
            .find(|i| i.severity == Severity::Error && CODES.contains(&i.code))
        else {
            break;
        };
        let Some(next) = remove(&current, &issue.path) else {
            break;
        };
        pruned.push(Pruned {
            code: issue.code,
            path: issue.path,
            detail: issue.detail,
        });
        current = next;
    }
    (current, pruned)
}

/// One step of a path: a field or an index.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Field(String),
    Index(usize),
}

/// `nodes[3].npcs[0].npc` → `nodes`, `3`, `npcs`, `0`, `npc`.
fn steps(path: &str) -> Option<Vec<Step>> {
    let mut out = Vec::new();
    for part in path.split('.') {
        let (name, mut rest) = part.split_once('[').map_or((part, ""), |(n, r)| (n, r));
        if !name.is_empty() {
            out.push(Step::Field(name.to_string()));
        }
        while !rest.is_empty() {
            let (index, after) = rest.split_once(']')?;
            out.push(Step::Index(index.parse().ok()?));
            rest = after.strip_prefix('[').unwrap_or(after);
        }
    }
    Some(out)
}

fn at<'a>(doc: &'a mut Value, path: &[Step]) -> Option<&'a mut Value> {
    path.iter().try_fold(doc, |v, s| match s {
        Step::Field(f) => v.get_mut(f.as_str()),
        Step::Index(i) => v.get_mut(*i),
    })
}

/// The campaign with the value at `path` gone, the smallest way that
/// still reads as a campaign.
fn remove(campaign: &Campaign, path: &str) -> Option<Campaign> {
    let mut path = steps(path)?;
    let doc = serde_json::to_value(campaign).ok()?;
    // A declared id: the element goes (`npcs[3].id` → `npcs[3]`).
    if path.last() == Some(&Step::Field("id".into())) {
        path.pop();
    }
    // A scene in an act that does not exist: the first act.
    if let [Step::Field(nodes), Step::Index(_), Step::Field(act)] = path.as_slice()
        && nodes == "nodes"
        && act == "act"
        && let Some(first) = campaign.acts.first()
    {
        let mut doc = doc;
        *at(&mut doc, &path)? = Value::String(first.id.clone());
        return serde_json::from_value(doc).ok();
    }
    // An optional field: emptied.
    if let Some(Step::Field(_)) = path.last() {
        let mut emptied = doc.clone();
        let (field, parent) = path.split_last()?;
        if let (Step::Field(f), Some(Value::Object(map))) = (field, at(&mut emptied, parent)) {
            map.remove(f);
            if let Ok(c) = serde_json::from_value::<Campaign>(emptied) {
                return Some(c);
            }
        }
    }
    // Otherwise the innermost list element that holds it.
    let cut = path.iter().rposition(|s| matches!(s, Step::Index(_)))?;
    let Step::Index(index) = path[cut] else {
        return None;
    };
    let mut doc = doc;
    let list = at(&mut doc, &path[..cut])?.as_array_mut()?;
    if index >= list.len() {
        return None;
    }
    list.remove(index);
    serde_json::from_value(doc).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::from_yaml;

    const KERBRUME: &str = include_str!("../../../content/fixtures/phare-de-kerbrume.yaml");

    fn kerbrume() -> Campaign {
        from_yaml(KERBRUME).unwrap()
    }

    #[test]
    fn a_valid_campaign_is_left_as_it_is() {
        let c = kerbrume();
        let (out, pruned) = prune(&c, &Library::default());
        assert!(pruned.is_empty(), "{pruned:?}");
        assert_eq!(out, c);
    }

    #[test]
    fn invented_references_are_emptied_or_removed() {
        let mut c = kerbrume();
        let node = c.nodes[0].id.clone();
        c.nodes[0].location = Some("lieu_invente".into());
        c.nodes[0].act = "acte_invente".into();
        c.nodes[0].exits.push(super::super::model::Exit {
            to: "sc_nulle_part".into(),
            label: "Vers nulle part".into(),
        });
        let mut ghost = c.clues[0].clone();
        ghost.id = "cl_fantome".into();
        ghost.node = "sc_fantome".into();
        c.clues.push(ghost);
        let mut twin = c.npcs[0].clone();
        twin.name = "Le jumeau".into();
        c.npcs.push(twin);

        let (out, pruned) = prune(&c, &Library::default());
        let codes: Vec<&str> = pruned.iter().map(|p| p.code).collect();
        assert_eq!(pruned.len(), 5, "{pruned:?}");
        assert!(codes.contains(&"ID_DUPLICATE"));
        assert!(
            crate::story::validate(&out)
                .iter()
                .all(|i| !CODES.contains(&i.code))
        );
        let first = out.node(&node).unwrap();
        assert_eq!(first.location, None);
        assert_eq!(first.act, out.acts[0].id);
        assert!(first.exits.iter().all(|e| e.to != "sc_nulle_part"));
        assert!(out.clue("cl_fantome").is_none());
        assert_eq!(out.npcs.len(), c.npcs.len() - 1);
        assert_ne!(out.npcs.last().unwrap().name, "Le jumeau");
    }

    #[test]
    fn paths_read_fields_and_indexes() {
        assert_eq!(
            steps("nodes[3].npcs[0].npc").unwrap(),
            vec![
                Step::Field("nodes".into()),
                Step::Index(3),
                Step::Field("npcs".into()),
                Step::Index(0),
                Step::Field("npc".into()),
            ]
        );
        assert_eq!(
            steps("nodes[1].requires[2]").unwrap().last(),
            Some(&Step::Index(2))
        );
    }
}
