//! What one preparation write changed, and how to take it back
//! (`platform/stream-prep-edits`).
//!
//! Every write of a campaign's story — the GM's edits, an import, the
//! co-GM's workshop or generation, the settings — is compared entity by
//! entity: each item of an id'd list (`nodes`, `clues`, `npcs`…), the
//! bible, and the campaign's own fields (title, universe, rule system).
//! The comparison gives, whoever wrote:
//! - [`Touch`]es: which entities moved, by id, for the screen to light up;
//! - [`Restore`]s: each moved entity as it was before, the inverse that
//!   [`revert`] applies to undo the write.
//!
//! An undo restores only the entities that write touched, so it keeps
//! every later change made elsewhere; whether a later write touched the
//! same entities (and so covers the one to undo) is the caller's check,
//! on the [`Touch`] keys.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::edit::{Kind, LISTS, name_of, place_of};
use super::model::{Campaign, Id};

/// The campaign's own fields: everything that is neither an id'd list
/// nor the bible.
const OWN_FIELDS: [&str; 5] = ["format", "id", "title", "world", "rules"];

/// The id of the bible and of the campaign's own fields in a [`Touch`].
pub const BIBLE: &str = "bible";
pub const CAMPAIGN: &str = "campaign";

/// How an entity moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Add,
    Set,
    Remove,
}

/// One entity a write touched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Touch {
    pub op: Op,
    pub kind: Kind,
    /// The entity's id, or [`BIBLE`] / [`CAMPAIGN`].
    pub id: Id,
    /// Its name, title or statement (after the write, or before a removal).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Where it lands: a clue's scene, a scene's act (as it was, for a
    /// removal).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<Id>,
}

impl Touch {
    /// Whether `self` and `other` touch the same entity.
    #[must_use]
    pub fn same_entity(&self, other: &Self) -> bool {
        self.kind == other.kind && self.id == other.id
    }
}

/// One entity as it was before a write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Restore {
    pub kind: Kind,
    pub id: Id,
    /// The entity before; `None` when the write added it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    /// Where it stood in its list, to put a removed entity back in place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
}

/// What a write changed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Diff {
    pub touched: Vec<Touch>,
    pub inverse: Vec<Restore>,
}

impl Diff {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.touched.is_empty()
    }
}

fn doc(campaign: &Campaign) -> Map<String, Value> {
    match serde_json::to_value(campaign) {
        Ok(Value::Object(map)) => map,
        // A campaign always serializes to an object.
        _ => Map::new(),
    }
}

fn own_fields(doc: &Map<String, Value>) -> Value {
    Value::Object(
        OWN_FIELDS
            .iter()
            .filter_map(|k| doc.get(*k).map(|v| ((*k).to_string(), v.clone())))
            .collect(),
    )
}

fn items<'a>(doc: &'a Map<String, Value>, list: &str) -> &'a [Value] {
    doc.get(list)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn id_of(entity: &Value) -> Option<&str> {
    entity.get("id").and_then(Value::as_str)
}

/// Compare two versions of a campaign entity by entity.
#[must_use]
pub fn diff(before: &Campaign, after: &Campaign) -> Diff {
    let (old, new) = (doc(before), doc(after));
    let mut out = Diff::default();
    let (own_before, own_after) = (own_fields(&old), own_fields(&new));
    if own_before != own_after {
        out.touched.push(Touch {
            op: Op::Set,
            kind: Kind::Campaign,
            id: CAMPAIGN.to_string(),
            name: after.title.clone(),
            place: None,
        });
        out.inverse.push(Restore {
            kind: Kind::Campaign,
            id: CAMPAIGN.to_string(),
            before: Some(own_before),
            index: None,
        });
    }
    if old.get(BIBLE) != new.get(BIBLE) {
        out.touched.push(Touch {
            op: Op::Set,
            kind: Kind::Bible,
            id: BIBLE.to_string(),
            name: String::new(),
            place: None,
        });
        out.inverse.push(Restore {
            kind: Kind::Bible,
            id: BIBLE.to_string(),
            before: Some(old.get(BIBLE).cloned().unwrap_or(Value::Null)),
            index: None,
        });
    }
    for (list, kind) in LISTS {
        let (was, is) = (items(&old, list), items(&new, list));
        for (index, entity) in was.iter().enumerate() {
            let Some(id) = id_of(entity) else { continue };
            let now = is.iter().find(|e| id_of(e) == Some(id));
            if now == Some(entity) {
                continue;
            }
            out.touched.push(Touch {
                op: if now.is_some() { Op::Set } else { Op::Remove },
                kind,
                id: id.to_string(),
                name: name_of(now.unwrap_or(entity)),
                place: place_of(kind, now.unwrap_or(entity)),
            });
            out.inverse.push(Restore {
                kind,
                id: id.to_string(),
                before: Some(entity.clone()),
                index: Some(index),
            });
        }
        for entity in is {
            let Some(id) = id_of(entity) else { continue };
            if was.iter().any(|e| id_of(e) == Some(id)) {
                continue;
            }
            out.touched.push(Touch {
                op: Op::Add,
                kind,
                id: id.to_string(),
                name: name_of(entity),
                place: place_of(kind, entity),
            });
            out.inverse.push(Restore {
                kind,
                id: id.to_string(),
                before: None,
                index: None,
            });
        }
    }
    out
}

/// Put back each entity of `inverse` as it was, on top of `current`:
/// what else changed since is kept.
///
/// # Errors
///
/// The result is no campaign (a restored entity no longer fits the
/// format): the message of the reader.
pub fn revert(current: &Campaign, inverse: &[Restore]) -> Result<Campaign, String> {
    let mut doc = doc(current);
    for r in inverse {
        match r.kind {
            Kind::Campaign => {
                for k in OWN_FIELDS {
                    doc.remove(k);
                }
                if let Some(Value::Object(fields)) = &r.before {
                    doc.extend(fields.clone());
                }
            }
            Kind::Bible => {
                doc.insert(
                    BIBLE.to_string(),
                    r.before.clone().unwrap_or(Value::Object(Map::new())),
                );
            }
            kind => {
                let Some((list, _)) = LISTS.iter().find(|(_, k)| *k == kind) else {
                    continue;
                };
                let entry = doc
                    .entry((*list).to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if !entry.is_array() {
                    *entry = Value::Array(Vec::new());
                }
                let Value::Array(entities) = entry else {
                    continue;
                };
                let at = entities.iter().position(|e| id_of(e) == Some(&r.id));
                match (&r.before, at) {
                    (None, Some(i)) => {
                        entities.remove(i);
                    }
                    (None, None) => {}
                    (Some(was), Some(i)) => entities[i] = was.clone(),
                    (Some(was), None) => {
                        let i = r.index.unwrap_or(entities.len()).min(entities.len());
                        entities.insert(i, was.clone());
                    }
                }
            }
        }
    }
    serde_json::from_value(Value::Object(doc)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::edit::{Edit, apply};
    use crate::story::from_yaml;
    use serde_json::json;

    fn witness() -> Campaign {
        from_yaml(include_str!(
            "../../../content/campaigns/corsaires/campagne.yaml"
        ))
        .expect("corsaires")
    }

    fn edited(c: &Campaign, edits: &[Edit]) -> Campaign {
        apply(c, edits).expect("edits apply").0
    }

    #[test]
    fn undoing_a_write_restores_every_kind_of_change_in_place() {
        let before = witness();
        let second_node = before.nodes[1].id.clone();
        let first_clue = before.clues[0].id.clone();
        let after = edited(
            &before,
            &[
                Edit::Set {
                    target: first_clue.clone(),
                    field: "text".into(),
                    value: json!("Une empreinte de botte"),
                },
                Edit::Remove {
                    target: second_node.clone(),
                },
                Edit::Add {
                    kind: Kind::Location,
                    value: json!({ "id": "lieu-cimetiere", "name": "Cimetière" }),
                },
                Edit::Set {
                    target: "bible".into(),
                    field: "pitch".into(),
                    value: json!("Autre chose"),
                },
                Edit::Set {
                    target: "campaign".into(),
                    field: "title".into(),
                    value: json!("Titre neuf"),
                },
            ],
        );
        let d = diff(&before, &after);
        let ids: Vec<(Op, &str)> = d.touched.iter().map(|t| (t.op, t.id.as_str())).collect();
        assert!(ids.contains(&(Op::Set, first_clue.as_str())), "{ids:?}");
        assert!(ids.contains(&(Op::Remove, second_node.as_str())), "{ids:?}");
        assert!(ids.contains(&(Op::Add, "lieu-cimetiere")), "{ids:?}");
        assert!(ids.contains(&(Op::Set, BIBLE)), "{ids:?}");
        assert!(ids.contains(&(Op::Set, CAMPAIGN)), "{ids:?}");
        assert_eq!(ids.len(), 5, "only what moved: {ids:?}");
        // The removed scene goes back where it was, not at the end.
        assert_eq!(revert(&after, &d.inverse).expect("reverts"), before);
    }

    #[test]
    fn undoing_a_write_keeps_what_changed_elsewhere_since() {
        let before = witness();
        let (first, last) = (
            before.npcs[0].id.clone(),
            before.npcs[before.npcs.len() - 1].id.clone(),
        );
        assert_ne!(first, last);
        let claude = edited(
            &before,
            &[Edit::Set {
                target: first.clone(),
                field: "name".into(),
                value: json!("Écrit par Claude"),
            }],
        );
        let undo = diff(&before, &claude).inverse;
        let gm = edited(
            &claude,
            &[Edit::Set {
                target: last.clone(),
                field: "name".into(),
                value: json!("Écrit par le MJ"),
            }],
        );
        let back = revert(&gm, &undo).expect("reverts");
        assert_eq!(back.npcs[0], before.npcs[0]);
        assert_eq!(back.npcs.last().unwrap().name, "Écrit par le MJ");
    }

    #[test]
    fn a_clue_says_which_scene_it_lands_in() {
        let before = witness();
        let clue = before.clues[0].clone();
        let after = edited(
            &before,
            &[Edit::Remove {
                target: clue.id.clone(),
            }],
        );
        let touched = diff(&before, &after).touched;
        assert_eq!(touched.len(), 1);
        assert_eq!(touched[0].place.as_deref(), Some(clue.node.as_str()));
    }

    #[test]
    fn an_unchanged_story_touches_nothing() {
        let c = witness();
        assert!(diff(&c, &c.clone()).is_empty());
    }
}
