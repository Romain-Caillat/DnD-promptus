//! Checks of a campaign against what it points at outside itself: its
//! rule system and the grid maps its scenes are played on. Like
//! [`validate`](super::validate::validate), they report and never block.
//!
//! `validate` only sees the campaign. A campaign names rule-system data
//! (classes, adversary stat blocks, items, abilities, difficulties) and
//! map files; whoever has those at hand — the `worlds` report, the
//! server once rule systems are stored — passes them in a [`Library`].
//! Each part is optional: what is not supplied is not checked.

use std::collections::BTreeSet;

use super::model::{Campaign, ClueCheck, PlannedCheck, StatBlock};
use super::validate::{Issue, Severity, validate};
use crate::maps::Map;
use crate::rules::RuleSystem;

/// What a campaign may be checked against.
#[derive(Debug, Clone, Copy, Default)]
pub struct Library<'a> {
    /// The rule system the campaign names (`campaign.rules`).
    pub rules: Option<&'a RuleSystem>,
    /// Every map a scene may name.
    pub maps: Option<&'a [Map]>,
}

/// [`validate`], plus the checks against `library`.
#[must_use]
pub fn validate_with(campaign: &Campaign, library: &Library<'_>) -> Vec<Issue> {
    let mut issues = validate(campaign);
    if let Some(rules) = library.rules {
        check_rules(&mut issues, campaign, rules);
    }
    if let Some(maps) = library.maps {
        check_maps(&mut issues, campaign, maps);
    }
    issues
}

fn push(issues: &mut Vec<Issue>, severity: Severity, code: &'static str, path: String, detail: String) {
    issues.push(Issue {
        severity,
        code,
        path,
        detail,
    });
}

struct RuleIds<'a> {
    abilities: BTreeSet<&'a str>,
    difficulties: Vec<i32>,
}

impl RuleIds<'_> {
    fn ability(&self, issues: &mut Vec<Issue>, stat: &str, path: String) {
        if !self.abilities.contains(stat) {
            let known: Vec<_> = self.abilities.iter().copied().collect();
            push(
                issues,
                Severity::Error,
                "RULES_UNKNOWN_ABILITY",
                path,
                format!(
                    "`{stat}` is not an ability of the rule system ({})",
                    known.join(", ")
                ),
            );
        }
    }

    fn difficulty(&self, issues: &mut Vec<Issue>, value: u32, path: String) {
        let named = i32::try_from(value).is_ok_and(|v| self.difficulties.contains(&v));
        if !named {
            let known: Vec<_> = self.difficulties.iter().map(i32::to_string).collect();
            push(
                issues,
                Severity::Warning,
                "DIFFICULTY_OFF_SCALE",
                path,
                format!(
                    "difficulty {value} is none of the rule system's named difficulties ({})",
                    known.join(", ")
                ),
            );
        }
    }

    fn planned(&self, issues: &mut Vec<Issue>, k: &PlannedCheck, path: &str) {
        self.ability(issues, &k.stat, format!("{path}.stat"));
        self.difficulty(issues, k.difficulty, format!("{path}.difficulty"));
    }

    fn clue(&self, issues: &mut Vec<Issue>, k: &ClueCheck, path: &str) {
        self.ability(issues, &k.stat, format!("{path}.stat"));
        self.difficulty(issues, k.difficulty, format!("{path}.difficulty"));
    }
}

fn check_rules(issues: &mut Vec<Issue>, c: &Campaign, rules: &RuleSystem) {
    if c.rules.id != rules.id || c.rules.version != rules.version {
        push(
            issues,
            Severity::Error,
            "RULES_MISMATCH",
            "rules".into(),
            format!(
                "the campaign plays `{}` v{}, the rule system supplied is `{}` v{}",
                c.rules.id, c.rules.version, rules.id, rules.version
            ),
        );
        return;
    }
    let ids = RuleIds {
        abilities: rules.abilities.iter().map(|a| a.id.as_str()).collect(),
        difficulties: rules.difficulties.iter().map(|d| d.value).collect(),
    };
    let classes: BTreeSet<&str> = rules.classes.iter().map(|x| x.id.as_str()).collect();
    let adversaries: BTreeSet<&str> = rules.adversaries.iter().map(|x| x.id.as_str()).collect();
    let items: BTreeSet<&str> = rules.items.iter().map(|x| x.id.as_str()).collect();

    let unknown = |issues: &mut Vec<Issue>, code, what: &str, id: &str, path: String| {
        push(
            issues,
            Severity::Error,
            code,
            path,
            format!("`{id}` is not {what} of rule system `{}`", rules.id),
        );
    };
    let stats = |issues: &mut Vec<Issue>, s: &StatBlock, path: &str| {
        if let Some(id) = &s.from_rules {
            if !adversaries.contains(id.as_str()) {
                unknown(
                    issues,
                    "RULES_UNKNOWN_ADVERSARY",
                    "an adversary",
                    id,
                    format!("{path}.from_rules"),
                );
            }
        }
        for stat in s.abilities.keys() {
            ids.ability(issues, stat, format!("{path}.abilities.{stat}"));
        }
    };

    for (i, p) in c.party.iter().enumerate() {
        if let Some(class) = &p.class {
            if !classes.contains(class.as_str()) {
                unknown(
                    issues,
                    "RULES_UNKNOWN_CLASS",
                    "a class",
                    class,
                    format!("party[{i}].class"),
                );
            }
        }
    }
    for (i, n) in c.nodes.iter().enumerate() {
        for (j, k) in n.checks.iter().enumerate() {
            ids.planned(issues, k, &format!("nodes[{i}].checks[{j}]"));
        }
    }
    for (i, cl) in c.clues.iter().enumerate() {
        if let Some(k) = &cl.check {
            ids.clue(issues, k, &format!("clues[{i}].check"));
        }
    }
    for (i, n) in c.npcs.iter().enumerate() {
        if let Some(s) = &n.stats {
            stats(issues, s, &format!("npcs[{i}].stats"));
        }
    }
    for (i, a) in c.adversaries.iter().enumerate() {
        stats(issues, &a.stats, &format!("adversaries[{i}].stats"));
    }
    for (i, it) in c.items.iter().enumerate() {
        if let Some(id) = &it.from_rules {
            if !items.contains(id.as_str()) {
                unknown(
                    issues,
                    "RULES_UNKNOWN_ITEM",
                    "an item",
                    id,
                    format!("items[{i}].from_rules"),
                );
            }
        }
    }
}

fn check_maps(issues: &mut Vec<Issue>, c: &Campaign, maps: &[Map]) {
    for (i, n) in c.nodes.iter().enumerate() {
        let Some(id) = &n.map else { continue };
        let path = format!("nodes[{i}].map");
        let Some(map) = maps.iter().find(|m| m.id == *id) else {
            push(
                issues,
                Severity::Error,
                "MAP_UNKNOWN",
                path,
                format!("`{id}` is not a known map"),
            );
            continue;
        };
        // A token placed on the map stands for someone the campaign
        // knows: the fight's opponents come from its sheets.
        for s in &map.starts {
            let Some(entity) = &s.entity else { continue };
            if c.npc(entity).is_none() && c.adversary(entity).is_none() {
                push(
                    issues,
                    Severity::Warning,
                    "MAP_ENTITY_UNKNOWN",
                    path.clone(),
                    format!(
                        "map `{id}` places `{entity}` (start `{}`), which is no NPC or adversary of the campaign",
                        s.id
                    ),
                );
            }
        }
    }
}
