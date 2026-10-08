//! The rule systems a new campaign can start from, as the GM's campaign
//! screens list them. The presets themselves are the files of the two
//! witness worlds and the D&D 5e SRD (`content`); a campaign's own
//! versions are `crate::rules`.

use promptus_shared::rules::RuleSystem;
use serde::Serialize;

use crate::content;

/// Every preset, in the order a GM is offered them.
pub fn summaries() -> Vec<PresetSummary> {
    content::presets().map(|s| PresetSummary::of(s)).collect()
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
    fn both_witness_systems_and_the_srd_are_offered_with_their_stat_names() {
        let ids: Vec<_> = summaries()
            .iter()
            .map(|s| (s.id.clone(), s.version, s.abilities.len()))
            .collect();
        assert_eq!(
            ids,
            [
                ("corsaires".to_string(), 1, 6),
                ("brasier".to_string(), 1, 6),
                ("srd".to_string(), 1, 6)
            ]
        );
    }
}
