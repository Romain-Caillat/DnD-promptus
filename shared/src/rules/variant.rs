//! "What if" versions of a rule system: a named set of edits applied to
//! the YAML text before it is loaded, so a GM can try a rule change
//! (precision added to the roll, fixed NPC damage, another reading of
//! "CD 1") without touching the file. The draft stays the reference; a
//! variant lives next to it, in a scenario.
//!
//! An edit is `path: value`. The path uses the lint's notation:
//! dot-separated keys, and after a key, `[id]` picks the list element
//! whose `id` is that, `[n]` the n-th one (from 0):
//!
//! ```yaml
//! set:
//!   attack.precision: added_to_attack_roll
//!   cooldowns.meaning: turn_of_use_counts
//!   adversaries[gueule_rouge].actions[gueule_rouge_sabre].tags[0].damage.amount: 5
//! ```
//!
//! Every step must exist, except the last key of a mapping, which is
//! added when missing. The edited document then goes through
//! [`RuleSystem::from_yaml`] like any version, so a variant that breaks
//! the rules is refused the same way.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value};

use super::load::LoadError;
use super::model::RuleSystem;

/// A named rule change: edits on the rule system's YAML.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub id: String,
    /// French, shown in the report.
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// `path: value`, applied in order.
    pub set: Mapping,
}

/// Why an edit could not be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantError {
    /// The edit's whole path.
    pub path: String,
    pub detail: String,
}

impl fmt::Display for VariantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.detail)
    }
}

/// A rule system that could not be built from a draft plus edits.
#[derive(Debug, Clone)]
pub enum BuildError {
    Yaml(String),
    Edit(VariantError),
    Load(LoadError),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml(e) => write!(f, "YAML: {e}"),
            Self::Edit(e) => write!(f, "edit {e}"),
            Self::Load(e) => write!(f, "{e}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Key(String),
    Index(usize),
    Id(String),
}

fn parse_path(path: &str) -> Result<Vec<Step>, String> {
    let mut steps = Vec::new();
    for part in path.split('.') {
        let (key, mut rest) = match part.find('[') {
            Some(i) => (&part[..i], &part[i..]),
            None => (part, ""),
        };
        if key.is_empty() {
            return Err(format!("empty key in `{part}`"));
        }
        steps.push(Step::Key(key.to_string()));
        while !rest.is_empty() {
            let close = rest
                .find(']')
                .filter(|_| rest.starts_with('['))
                .ok_or_else(|| format!("unbalanced brackets in `{part}`"))?;
            let inner = &rest[1..close];
            if inner.is_empty() {
                return Err(format!("empty selector in `{part}`"));
            }
            steps.push(match inner.parse::<usize>() {
                Ok(n) => Step::Index(n),
                Err(_) => Step::Id(inner.to_string()),
            });
            rest = &rest[close + 1..];
        }
    }
    Ok(steps)
}

fn describe(steps: &[Step]) -> String {
    let mut s = String::new();
    for step in steps {
        match step {
            Step::Key(k) if s.is_empty() => s.push_str(k),
            Step::Key(k) => {
                s.push('.');
                s.push_str(k);
            }
            Step::Index(n) => s.push_str(&format!("[{n}]")),
            Step::Id(id) => s.push_str(&format!("[{id}]")),
        }
    }
    s
}

/// Sets `value` at `path` in `doc`.
pub fn set_path(doc: &mut Value, path: &str, value: Value) -> Result<(), VariantError> {
    let err = |detail: String| VariantError {
        path: path.to_string(),
        detail,
    };
    let steps = parse_path(path).map_err(err)?;
    let mut node = doc;
    for (i, step) in steps.iter().enumerate() {
        let last = i + 1 == steps.len();
        let here = || describe(&steps[..=i]);
        node = match (step, node) {
            (Step::Key(k), Value::Mapping(m)) => {
                let key = Value::String(k.clone());
                if last {
                    m.insert(key, value);
                    return Ok(());
                }
                m.get_mut(&key)
                    .ok_or_else(|| err(format!("`{}` does not exist", here())))?
            }
            (Step::Index(n), Value::Sequence(items)) => {
                let len = items.len();
                let slot = items
                    .get_mut(*n)
                    .ok_or_else(|| err(format!("`{}`: the list has {len} elements", here())))?;
                if last {
                    *slot = value;
                    return Ok(());
                }
                slot
            }
            (Step::Id(id), Value::Sequence(items)) => {
                let slot = items
                    .iter_mut()
                    .find(|v| v.get("id").and_then(Value::as_str) == Some(id.as_str()))
                    .ok_or_else(|| err(format!("`{}`: no element has this id", here())))?;
                if last {
                    *slot = value;
                    return Ok(());
                }
                slot
            }
            (Step::Key(_), _) => return Err(err(format!("`{}`: not a mapping", here()))),
            (_, _) => return Err(err(format!("`{}`: not a list", here()))),
        };
    }
    unreachable!("a parsed path has at least one step")
}

impl Variant {
    /// Applies every edit to a parsed rule system.
    pub fn apply(&self, doc: &mut Value) -> Result<(), VariantError> {
        for (path, value) in &self.set {
            let path = path.as_str().ok_or_else(|| VariantError {
                path: format!("{path:?}"),
                detail: "a path must be a string".into(),
            })?;
            set_path(doc, path, value.clone())?;
        }
        Ok(())
    }
}

/// Builds a rule system from a draft's text, with extra adversary stat
/// blocks (added only when the draft has no block with that id) and an
/// optional variant applied on top.
pub fn build_system(
    text: &str,
    extra_adversaries: &[Value],
    variant: Option<&Variant>,
) -> Result<RuleSystem, BuildError> {
    let mut doc: Value =
        serde_yaml_ng::from_str(text).map_err(|e| BuildError::Yaml(e.to_string()))?;
    if !extra_adversaries.is_empty() {
        let list = doc
            .get_mut("adversaries")
            .and_then(Value::as_sequence_mut)
            .ok_or_else(|| {
                BuildError::Edit(VariantError {
                    path: "adversaries".into(),
                    detail: "the rule system has no adversary list".into(),
                })
            })?;
        for block in extra_adversaries {
            let id = block.get("id").and_then(Value::as_str);
            let present = list
                .iter()
                .any(|a| a.get("id").and_then(Value::as_str) == id);
            if !present {
                list.push(block.clone());
            }
        }
    }
    if let Some(v) = variant {
        v.apply(&mut doc).map_err(BuildError::Edit)?;
    }
    let edited = serde_yaml_ng::to_string(&doc).map_err(|e| BuildError::Yaml(e.to_string()))?;
    RuleSystem::from_yaml(&edited).map_err(BuildError::Load)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Value {
        serde_yaml_ng::from_str(
            "a: { b: 1 }\nlist:\n  - { id: x, tags: [{ d: 1 }, { d: 2 }] }\n  - { id: y }\n",
        )
        .unwrap()
    }

    #[test]
    fn paths_reach_keys_ids_and_indexes() {
        let mut d = doc();
        set_path(&mut d, "a.b", Value::from(5)).unwrap();
        set_path(&mut d, "list[x].tags[1].d", Value::from(9)).unwrap();
        set_path(&mut d, "a.new", Value::from("v")).unwrap();
        assert_eq!(d["a"]["b"], Value::from(5));
        assert_eq!(d["list"][0]["tags"][1]["d"], Value::from(9));
        assert_eq!(d["list"][0]["tags"][0]["d"], Value::from(1));
        assert_eq!(d["a"]["new"], Value::from("v"));
    }

    #[test]
    fn a_missing_step_is_refused_and_names_where() {
        let mut d = doc();
        let e = set_path(&mut d, "list[z].tags", Value::from(1)).unwrap_err();
        assert!(e.detail.contains("list[z]"), "{e}");
        let e = set_path(&mut d, "nope.b", Value::from(1)).unwrap_err();
        assert!(e.detail.contains("nope"), "{e}");
        let e = set_path(&mut d, "list[x].tags[7]", Value::from(1)).unwrap_err();
        assert!(e.detail.contains("2 elements"), "{e}");
        assert_eq!(d, doc(), "nothing changed on the way");
    }
}
