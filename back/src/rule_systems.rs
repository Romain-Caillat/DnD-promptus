//! The rule systems a new campaign can start from (`content/rules/`).
//!
//! Rule systems are data (`MEMORY.md` §1). Until the GM can store and
//! edit their own (`engine/edit-rule-system`), the presets are the files
//! of the two witness worlds, compiled into the binary and checked by the
//! rules engine at first use — the same loader the `worlds` report runs,
//! so a file that does not load never reaches a build that passes tests.

use std::sync::LazyLock;

use promptus_shared::rules::RuleSystem;
use promptus_shared::story::RuleSystemRef;
use serde::Serialize;

const FILES: [&str; 2] = [
    include_str!("../../content/rules/corsaires/v1.yaml"),
    include_str!("../../content/rules/brasier/v1.yaml"),
];

static PRESETS: LazyLock<Vec<RuleSystem>> = LazyLock::new(|| {
    FILES
        .iter()
        .map(|text| RuleSystem::from_yaml(text).expect("an embedded rule system loads"))
        .collect()
});

/// Every preset, in the order a GM is offered them.
pub fn presets() -> &'static [RuleSystem] {
    &PRESETS
}

/// The preset `rules` points at, if there is one.
#[must_use]
pub fn find(rules: &RuleSystemRef) -> Option<&'static RuleSystem> {
    presets()
        .iter()
        .find(|s| s.id == rules.id && s.version == rules.version)
}

/// A preset as the GM's campaign screens show it: what it is called and
/// the stat names it brings. The rules themselves stay on the server.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetSummary {
    pub id: String,
    pub version: u32,
    pub name: String,
    pub description: String,
    /// The six abilities, in the system's order.
    pub abilities: Vec<NamedStat>,
    pub hit_points: NamedStat,
    pub armor_class: NamedStat,
}

#[derive(Debug, Clone, Serialize)]
pub struct NamedStat {
    /// The short name (`FOR`, `PV`).
    pub abbr: String,
    pub name: String,
}

impl PresetSummary {
    #[must_use]
    pub fn of(system: &RuleSystem) -> Self {
        Self {
            id: system.id.clone(),
            version: system.version,
            name: system.name.clone(),
            description: system.description.clone(),
            abilities: system
                .abilities
                .iter()
                .map(|a| NamedStat {
                    abbr: a.id.clone(),
                    name: a.name.clone(),
                })
                .collect(),
            hit_points: NamedStat {
                abbr: system.stats.hit_points.abbr.clone(),
                name: system.stats.hit_points.name.clone(),
            },
            armor_class: NamedStat {
                abbr: system.stats.armor_class.abbr.clone(),
                name: system.stats.armor_class.name.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_witness_systems_load_and_are_found_by_their_ref() {
        let ids: Vec<_> = presets()
            .iter()
            .map(|s| (s.id.as_str(), s.version))
            .collect();
        assert_eq!(ids, [("corsaires", 1), ("brasier", 1)]);
        let corsaires = RuleSystemRef {
            id: "corsaires".into(),
            version: 1,
        };
        assert_eq!(find(&corsaires).map(|s| s.abilities.len()), Some(6));
        let unknown = RuleSystemRef {
            id: "corsaires".into(),
            version: 9,
        };
        assert!(find(&unknown).is_none());
    }
}
