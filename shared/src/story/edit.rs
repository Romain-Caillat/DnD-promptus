//! Edits of a campaign, by id (`campaign/review-story-graph`).
//!
//! The GM's review screen and the co-GM's workshop change a campaign
//! the same way: a list of [`Edit`]s, each naming what it touches by its
//! stable id. Applying them yields the new campaign and the
//! [`Change`]s to show (« + Indice … → nœud … », « ~ PNJ Aldric ·
//! motivation … → … »). A list applies whole or not at all: the first
//! edit that does not fit stops it, with its index.
//!
//! An edit works on the serialized document, so every field of the
//! format can be edited without a variant per field; the result is read
//! back through the model, which refuses unknown fields and wrong
//! types exactly as an import does. Whether the result is *good* is the
//! validator's concern (`validate`), and it reports, never blocks —
//! except for the parts of a scene the scene sheet edits
//! (`campaign/edit-scenes-in-one-place`): a `set` reaching a scene's
//! fight, ambience, checks, exits, loot or XP is refused when it leaves
//! that scene a problem it did not have (an opponent who names nothing,
//! an exit to nowhere, a stat the rules lack, a link that is not
//! YouTube), `EDIT_SCENE_INVALID`.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::library::{Library, validate_with};
use super::model::{Campaign, Id};
use super::validate::{Severity, validate};

/// The lists of a campaign whose items carry an id, and the kind each
/// holds.
const LISTS: [(&str, Kind); 12] = [
    ("party", Kind::PartyMember),
    ("acts", Kind::Act),
    ("fronts", Kind::Front),
    ("nodes", Kind::Node),
    ("revelations", Kind::Revelation),
    ("clues", Kind::Clue),
    ("npcs", Kind::Npc),
    ("adversaries", Kind::Adversary),
    ("locations", Kind::Location),
    ("items", Kind::Item),
    ("factions", Kind::Faction),
    ("goals", Kind::Goal),
];

/// What an edit adds, or what an id names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    PartyMember,
    Act,
    Front,
    Node,
    Revelation,
    Clue,
    Npc,
    Adversary,
    Location,
    Item,
    Faction,
    Goal,
    /// The bible (target `bible`).
    Bible,
    /// The campaign's title and universe (target `campaign`).
    Campaign,
}

impl Kind {
    fn list(self) -> Option<&'static str> {
        LISTS.iter().find(|(_, k)| *k == self).map(|(l, _)| *l)
    }
}

/// One change, by id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    /// Set `field` of `target` — an id, `bible` or `campaign`. A dotted
    /// field reaches inside an object (`stats.hit_points`); `null`
    /// clears it back to its default.
    Set {
        target: Id,
        field: String,
        value: Value,
    },
    /// Add an entity of `kind`; `value` carries its new id.
    Add { kind: Kind, value: Value },
    /// Remove the entity `target`. What still points at it is the
    /// validator's to report.
    Remove { target: Id },
}

/// What one edit changed, for the screen.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    /// `add`, `set` or `remove`.
    pub op: &'static str,
    pub kind: Kind,
    pub id: Id,
    /// The entity's name, title or statement.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<Value>,
    /// Where it lands: a clue's node, a node's act.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<Id>,
}

/// Why an edit does not apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditError {
    /// The edit's index in its list.
    pub index: usize,
    /// `EDIT_UNKNOWN_TARGET`, `EDIT_BAD_FIELD`, `EDIT_ID_TAKEN`,
    /// `EDIT_NO_ID`, `EDIT_INVALID`.
    pub code: &'static str,
    pub detail: String,
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "edit {}: {} ({})", self.index, self.code, self.detail)
    }
}

impl std::error::Error for EditError {}

/// Fields an edit never sets: an id is stable, references to it would
/// break silently.
const FROZEN: [&str; 2] = ["id", "format"];

/// The parts of a scene a `set` may only leave sound: what the scene
/// sheet edits, where a wrong id would reach the table (a fight against
/// nobody, an exit to nowhere).
const SCENE_PARTS: [&str; 6] = ["encounter", "ambience", "checks", "exits", "loot", "xp"];

/// The warnings a scene part may not gain either, on top of every error:
/// the structural slips of those parts.
const SCENE_SLIPS: [&str; 4] = [
    "ENCOUNTER_EMPTY",
    "EXIT_TO_SELF",
    "LOOT_EMPTY",
    "MUSIC_NOT_YOUTUBE",
];

/// Apply `edits` to `campaign`, all or nothing.
///
/// # Errors
///
/// The first edit that does not fit.
pub fn apply(campaign: &Campaign, edits: &[Edit]) -> Result<(Campaign, Vec<Change>), EditError> {
    apply_with(campaign, edits, &Library::default())
}

/// [`apply`], with the scene parts also checked against `library` (a
/// check's `stat` against the rule system's abilities).
///
/// # Errors
///
/// The first edit that does not fit.
pub fn apply_with(
    campaign: &Campaign,
    edits: &[Edit],
    library: &Library<'_>,
) -> Result<(Campaign, Vec<Change>), EditError> {
    let mut doc = serde_json::to_value(campaign).map_err(|e| EditError {
        index: 0,
        code: "EDIT_INVALID",
        detail: e.to_string(),
    })?;
    let mut changes = Vec::with_capacity(edits.len());
    let mut current = campaign.clone();
    for (index, edit) in edits.iter().enumerate() {
        let err = |code: &'static str, detail: String| EditError {
            index,
            code,
            detail,
        };
        let change = apply_one(&mut doc, edit).map_err(|(code, detail)| err(code, detail))?;
        let next: Campaign =
            serde_json::from_value(doc.clone()).map_err(|e| err("EDIT_INVALID", e.to_string()))?;
        if let Some(node) = scene_part(edit) {
            let before = scene_problems(&current, library, node);
            if let Some(new) = scene_problems(&next, library, node)
                .into_iter()
                .find(|p| !before.contains(p))
            {
                let (code, path, detail) = new;
                return Err(err(
                    "EDIT_SCENE_INVALID",
                    format!("{code} {path}: {detail}"),
                ));
            }
        }
        current = next;
        changes.push(change);
    }
    Ok((current, changes))
}

/// What a proposal keeps: each edit that applies on top of the ones
/// kept before it and adds no error to the campaign (an invented id, a
/// reference to nothing). Returns the edits kept and how many were
/// dropped.
#[must_use]
pub fn sanitize(campaign: &Campaign, edits: Vec<Edit>) -> (Vec<Edit>, usize) {
    let errors = |c: &Campaign| -> BTreeSet<(&'static str, String)> {
        validate(c)
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| (i.code, i.detail))
            .collect()
    };
    let mut kept: Vec<Edit> = Vec::new();
    let mut base = campaign.clone();
    let mut known = errors(&base);
    let mut dropped = 0;
    for edit in edits {
        match apply(&base, std::slice::from_ref(&edit)) {
            Ok((next, _)) => {
                let now = errors(&next);
                if now.is_subset(&known) {
                    known = now;
                    base = next;
                    kept.push(edit);
                } else {
                    dropped += 1;
                }
            }
            Err(_) => dropped += 1,
        }
    }
    (kept, dropped)
}

/// The scene whose sheet part `edit` sets, if it does.
fn scene_part(edit: &Edit) -> Option<&str> {
    let Edit::Set { target, field, .. } = edit else {
        return None;
    };
    let first = field.split('.').next()?;
    SCENE_PARTS.contains(&first).then_some(target.as_str())
}

/// What is wrong in scene `node` that a sheet edit may not add: its
/// errors and its structural slips, as `(code, path, detail)`. Empty
/// when `node` is no scene (the edit then targeted something else).
fn scene_problems(
    c: &Campaign,
    library: &Library<'_>,
    node: &str,
) -> BTreeSet<(&'static str, String, String)> {
    let Some(i) = c.nodes.iter().position(|n| n.id == node) else {
        return BTreeSet::new();
    };
    let prefix = format!("nodes[{i}].");
    validate_with(c, library)
        .into_iter()
        .filter(|x| x.path.starts_with(&prefix))
        .filter(|x| x.severity == Severity::Error || SCENE_SLIPS.contains(&x.code))
        .map(|x| (x.code, x.path, x.detail))
        .collect()
}

type Failure = (&'static str, String);

fn apply_one(doc: &mut Value, edit: &Edit) -> Result<Change, Failure> {
    match edit {
        Edit::Set {
            target,
            field,
            value,
        } => {
            if field
                .split('.')
                .any(|f| FROZEN.contains(&f) || f.is_empty())
            {
                return Err(("EDIT_BAD_FIELD", format!("`{field}` cannot be set")));
            }
            let (kind, entity) = find(doc, target)?;
            let before = set_path(entity, field, value.clone())?;
            let name = name_of(entity);
            let place = place_of(kind, entity);
            Ok(Change {
                op: "set",
                kind,
                id: target.clone(),
                name,
                field: Some(field.clone()),
                before: Some(before),
                after: Some(value.clone()),
                place,
            })
        }
        Edit::Add { kind, value } => {
            let Some(list) = kind.list() else {
                return Err(("EDIT_BAD_FIELD", format!("{kind:?} cannot be added")));
            };
            let Some(id) = value.get("id").and_then(Value::as_str).map(str::to_string) else {
                return Err(("EDIT_NO_ID", "an added entity needs an `id`".into()));
            };
            if find(doc, &id).is_ok() || id == "bible" || id == "campaign" {
                return Err(("EDIT_ID_TAKEN", format!("`{id}` is already used")));
            }
            let root = doc
                .as_object_mut()
                .ok_or(("EDIT_INVALID", "not a campaign".into()))?;
            let items = root
                .entry(list)
                .or_insert_with(|| Value::Array(Vec::new()))
                .as_array_mut()
                .ok_or(("EDIT_INVALID", format!("`{list}` is not a list")))?;
            items.push(value.clone());
            Ok(Change {
                op: "add",
                kind: *kind,
                id,
                name: name_of(value),
                field: None,
                before: None,
                after: Some(value.clone()),
                place: place_of(*kind, value),
            })
        }
        Edit::Remove { target } => {
            for (list, kind) in LISTS {
                let Some(items) = doc.get_mut(list).and_then(Value::as_array_mut) else {
                    continue;
                };
                if let Some(at) = items
                    .iter()
                    .position(|v| v.get("id").and_then(Value::as_str) == Some(target))
                {
                    let gone = items.remove(at);
                    return Ok(Change {
                        op: "remove",
                        kind,
                        id: target.clone(),
                        name: name_of(&gone),
                        field: None,
                        before: Some(gone.clone()),
                        after: None,
                        place: place_of(kind, &gone),
                    });
                }
            }
            Err(("EDIT_UNKNOWN_TARGET", format!("no `{target}` to remove")))
        }
    }
}

/// The entity `target` names, and its kind.
fn find<'a>(doc: &'a mut Value, target: &str) -> Result<(Kind, &'a mut Value), Failure> {
    let unknown = || {
        (
            "EDIT_UNKNOWN_TARGET",
            format!("no `{target}` in the campaign"),
        )
    };
    match target {
        "campaign" => return Ok((Kind::Campaign, doc)),
        "bible" => {
            let root = doc.as_object_mut().ok_or_else(unknown)?;
            let bible = root
                .entry("bible")
                .or_insert_with(|| Value::Object(Map::new()));
            return Ok((Kind::Bible, bible));
        }
        _ => {}
    }
    let at = LISTS.iter().find_map(|(list, kind)| {
        let items = doc.get(list)?.as_array()?;
        let i = items
            .iter()
            .position(|v| v.get("id").and_then(Value::as_str) == Some(target))?;
        Some((*list, *kind, i))
    });
    let (list, kind, i) = at.ok_or_else(unknown)?;
    Ok((kind, &mut doc[list][i]))
}

/// Set the dotted `path` of `entity` to `value` (`null` removes it);
/// returns what was there.
fn set_path(entity: &mut Value, path: &str, value: Value) -> Result<Value, Failure> {
    let bad = || ("EDIT_BAD_FIELD", format!("`{path}` is not a field here"));
    let mut node = entity;
    let mut keys = path.split('.').peekable();
    while let Some(key) = keys.next() {
        let obj = node.as_object_mut().ok_or_else(bad)?;
        if keys.peek().is_none() {
            let before = if value.is_null() {
                obj.remove(key)
            } else {
                obj.insert(key.to_string(), value)
            };
            return Ok(before.unwrap_or(Value::Null));
        }
        node = obj.entry(key).or_insert_with(|| Value::Object(Map::new()));
    }
    Err(bad())
}

fn name_of(entity: &Value) -> String {
    ["name", "title", "statement", "label"]
        .iter()
        .find_map(|k| entity.get(*k).and_then(Value::as_str))
        .or_else(|| entity.get("text").and_then(Value::as_str))
        .unwrap_or("")
        .to_string()
}

fn place_of(kind: Kind, entity: &Value) -> Option<Id> {
    let key = match kind {
        Kind::Clue => "node",
        Kind::Node => "act",
        _ => return None,
    };
    entity.get(key).and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::from_yaml;
    use serde_json::json;

    const KERBRUME: &str = include_str!("../../../content/fixtures/phare-de-kerbrume.yaml");

    fn edits(v: Value) -> Vec<Edit> {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn sets_adds_and_removes_by_id() {
        let c = from_yaml(KERBRUME).unwrap();
        let npc = c.npcs[0].id.clone();
        let node = c.nodes[1].id.clone();
        let rev = c.revelations[0].id.clone();
        let (next, changes) = apply(
            &c,
            &edits(json!([
                { "op": "set", "target": npc, "field": "motivation", "value": "Acheter le silence." },
                { "op": "set", "target": "bible", "field": "tone", "value": "Sombre." },
                { "op": "add", "kind": "clue", "value": {
                    "id": "cl_nouveau_registre", "revelation": rev, "node": node, "text": "Un registre."
                }},
                { "op": "remove", "target": "cl_nouveau_registre" },
            ])),
        )
        .unwrap();
        assert_eq!(next.npc(&npc).unwrap().motivation, "Acheter le silence.");
        assert_eq!(next.bible.tone, "Sombre.");
        assert!(next.clue("cl_nouveau_registre").is_none());
        assert_eq!(changes[0].op, "set");
        assert_eq!(changes[0].name, c.npcs[0].name);
        assert_eq!(changes[2].place.as_deref(), Some(node.as_str()));
        assert_eq!(changes[3].op, "remove");
    }

    #[test]
    fn refuses_what_the_format_refuses_with_the_edit_index() {
        let c = from_yaml(KERBRUME).unwrap();
        let npc = c.npcs[0].id.clone();
        let fail = |v: Value| apply(&c, &edits(v)).unwrap_err();
        let e = fail(json!([
            { "op": "set", "target": "bible", "field": "tone", "value": "Ok." },
            { "op": "set", "target": npc, "field": "humeur", "value": "x" },
        ]));
        assert_eq!((e.index, e.code), (1, "EDIT_INVALID"));
        assert_eq!(
            fail(json!([{ "op": "set", "target": "pnj_fantome", "field": "name", "value": "x" }]))
                .code,
            "EDIT_UNKNOWN_TARGET"
        );
        assert_eq!(
            fail(json!([{ "op": "set", "target": npc, "field": "id", "value": "x" }])).code,
            "EDIT_BAD_FIELD"
        );
        assert_eq!(
            fail(json!([{ "op": "add", "kind": "npc", "value": { "id": npc, "name": "x", "disposition": "neutral" } }]))
                .code,
            "EDIT_ID_TAKEN"
        );
    }

    #[test]
    fn the_scene_sheet_sets_a_fight_its_sound_and_its_exits() {
        let c = from_yaml(KERBRUME).unwrap();
        let (next, changes) = apply(
            &c,
            &edits(json!([
                { "op": "set", "target": "sc_taverne", "field": "encounter", "value": {
                    "opponents": [{ "who": "adv_contrebandier", "count": 3 }, { "who": "pnj_corentin" }],
                    "tactics": ["Ils bloquent la porte."],
                    "morale": [{ "when": "Deux tombent", "then": "Les autres fuient." }]
                }},
                { "op": "set", "target": "sc_taverne", "field": "xp", "value": [{ "amount": 2, "reason": "Bagarre gagnée" }] },
                { "op": "set", "target": "sc_taverne", "field": "loot", "value": [{ "item": "obj_sabre", "found": "Sous le comptoir" }] },
                { "op": "set", "target": "sc_taverne", "field": "ambience", "value": {
                    "mood": "Enfumée", "sounds": "Chopes, rires",
                    "music": [{ "mood": "tension", "title": "Taverne", "url": "https://www.youtube.com/watch?v=abc", "search": "tavern music" }]
                }},
                { "op": "set", "target": "sc_taverne", "field": "exits", "value": [{ "to": "sc_port", "label": "Vers le port" }] },
                { "op": "set", "target": "sc_crique", "field": "encounter", "value": null },
            ])),
        )
        .unwrap();
        let tavern = next.node("sc_taverne").unwrap();
        let fight = tavern.encounter.as_ref().unwrap();
        assert_eq!(fight.opponents[0].count, 3);
        assert_eq!(fight.morale[0].then, "Les autres fuient.");
        assert_eq!(tavern.ambience.music[0].search, "tavern music");
        assert_eq!(tavern.exits[0].to, "sc_port");
        assert!(next.node("sc_crique").unwrap().encounter.is_none());
        assert_eq!(changes[0].field.as_deref(), Some("encounter"));
    }

    #[test]
    fn a_scene_part_naming_nothing_is_refused_whole() {
        let c = from_yaml(KERBRUME).unwrap();
        let fail = |v: Value| apply(&c, &edits(v)).unwrap_err();
        // An opponent who is no adversary nor NPC: nothing is applied.
        let e = fail(json!([
            { "op": "set", "target": "sc_taverne", "field": "summary", "value": "Autre." },
            { "op": "set", "target": "sc_taverne", "field": "encounter", "value": {
                "opponents": [{ "who": "adv_kraken" }]
            }},
        ]));
        assert_eq!((e.index, e.code), (1, "EDIT_SCENE_INVALID"));
        assert!(e.detail.contains("adv_kraken"), "{}", e.detail);
        // The wrong kind, a dotted path, nobody to fight, an exit to
        // nowhere or to itself, loot of nothing, a link off YouTube.
        for bad in [
            json!({ "op": "set", "target": "sc_taverne", "field": "encounter", "value": { "opponents": [{ "who": "obj_sabre" }] } }),
            json!({ "op": "set", "target": "sc_crique", "field": "encounter.opponents", "value": [{ "who": "pnj_inconnu" }] }),
            json!({ "op": "set", "target": "sc_crique", "field": "encounter.opponents", "value": [] }),
            json!({ "op": "set", "target": "sc_taverne", "field": "exits", "value": [{ "to": "sc_ailleurs", "label": "x" }] }),
            json!({ "op": "set", "target": "sc_taverne", "field": "exits", "value": [{ "to": "sc_taverne", "label": "x" }] }),
            json!({ "op": "set", "target": "sc_taverne", "field": "loot", "value": [{ "found": "Rien" }] }),
            json!({ "op": "set", "target": "sc_taverne", "field": "ambience.music", "value": [
                { "mood": "calm", "title": "x", "url": "https://example.com/x" }
            ] }),
        ] {
            assert_eq!(
                fail(json!([bad.clone()])).code,
                "EDIT_SCENE_INVALID",
                "{bad}"
            );
        }
        // Outside the sheet's parts, a dangling reference stays the
        // validator's to report.
        assert!(
            apply(
                &c,
                &edits(json!([{ "op": "set", "target": "sc_taverne", "field": "location", "value": "lieu_disparu" }]))
            )
            .is_ok()
        );
    }

    #[test]
    fn a_problem_the_scene_already_had_does_not_block_the_sheet() {
        let c = from_yaml(KERBRUME).unwrap();
        // An opponent removed elsewhere leaves a reference to nothing…
        let (broken, _) = apply(
            &c,
            &edits(json!([{ "op": "remove", "target": "adv_contrebandier" }])),
        )
        .unwrap();
        // …and the GM may still set the fight's tactics.
        assert!(
            apply(
                &broken,
                &edits(json!([{ "op": "set", "target": "sc_crique", "field": "encounter.tactics", "value": ["Ils fuient."] }]))
            )
            .is_ok()
        );
    }

    #[test]
    fn a_planned_check_names_an_ability_of_the_rules() {
        let c = from_yaml(KERBRUME).unwrap();
        let rules = crate::rules::RuleSystem::from_yaml(include_str!(
            "../../../content/rules/corsaires/v1.yaml"
        ))
        .unwrap();
        let library = Library {
            rules: Some(&rules),
            maps: None,
        };
        let check = |stat: &str| {
            apply_with(
                &c,
                &edits(
                    json!([{ "op": "set", "target": "sc_taverne", "field": "checks", "value": [
                        { "action": "Calmer la salle", "stat": stat, "difficulty": 10, "natural_1": "Une chope vole." }
                    ]}]),
                ),
                &library,
            )
        };
        assert!(check("CHA").is_ok());
        assert_eq!(check("CHARME").unwrap_err().code, "EDIT_SCENE_INVALID");
    }

    #[test]
    fn a_proposal_keeps_what_fits_and_drops_invented_ids() {
        let c = from_yaml(KERBRUME).unwrap();
        let rev = c.revelations[0].id.clone();
        let node = c.nodes[0].id.clone();
        let (kept, dropped) = sanitize(
            &c,
            edits(json!([
                { "op": "add", "kind": "clue", "value": {
                    "id": "cl_ok", "revelation": rev, "node": node, "text": "Vrai."
                }},
                { "op": "add", "kind": "clue", "value": {
                    "id": "cl_faux", "revelation": rev, "node": "sc_inventee", "text": "Faux."
                }},
                { "op": "set", "target": "pnj_invente", "field": "name", "value": "x" },
            ])),
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(dropped, 2);
    }
}
