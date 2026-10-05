//! Whether a character a player built follows the campaign's rule
//! system (`session/validate-characters`). Like the lint, it **reports,
//! never blocks**: the GM reads the issues, then validates anyway or
//! returns the sheet with a word (`MEMORY.md` §1, "the GM has the last
//! word").
//!
//! The input is the part of a sheet the rules speak about; the server
//! maps its stored sheet onto it. Codes:
//!
//! | Code | Severity | What |
//! | --- | --- | --- |
//! | `NAME_MISSING` | warning | the character has no name |
//! | `CLASS_MISSING` | error | no class chosen |
//! | `CLASS_UNKNOWN` | error | the class is not one of the system's |
//! | `ABILITY_UNKNOWN` | error | a score for an ability the system does not have |
//! | `ABILITY_OFF_CLASS` | warning | scores come from the class, and this one differs |
//! | `ABILITY_BUDGET_EXCEEDED` | warning | the scores add up to more than the class gives |

use std::collections::BTreeMap;

use super::model::{AbilityAssignment, RuleSystem};
use crate::issue::{Issue, Severity};

/// What the rules check on a character.
#[derive(Debug, Clone, Copy)]
pub struct CharacterInput<'a> {
    pub name: &'a str,
    pub class_id: Option<&'a str>,
    /// Scores by ability id, as the player set them. Empty means "the
    /// class's", for a system where scores come from the class.
    pub abilities: &'a BTreeMap<String, i32>,
}

fn issue(
    severity: Severity,
    code: &'static str,
    path: String,
    detail: String,
    message: String,
) -> Issue {
    Issue {
        severity,
        code,
        path,
        detail,
        message: Some(message),
    }
}

/// Every rule `character` breaks in `system`, in sheet order.
#[must_use]
pub fn check_character(system: &RuleSystem, character: &CharacterInput<'_>) -> Vec<Issue> {
    let mut out = Vec::new();
    if character.name.trim().is_empty() {
        out.push(issue(
            Severity::Warning,
            "NAME_MISSING",
            "name".into(),
            "the character has no name".into(),
            "Le personnage n'a pas de nom.".into(),
        ));
    }

    let class = match character.class_id {
        None => {
            out.push(issue(
                Severity::Error,
                "CLASS_MISSING",
                "classId".into(),
                "no class chosen".into(),
                "Aucune classe choisie.".into(),
            ));
            None
        }
        Some(id) => {
            let class = system.class(id);
            if class.is_none() {
                out.push(issue(
                    Severity::Error,
                    "CLASS_UNKNOWN",
                    "classId".into(),
                    format!("class {id} is not in {} v{}", system.id, system.version),
                    format!(
                        "La classe « {id} » n'existe pas dans les règles {}.",
                        system.name
                    ),
                ));
            }
            class
        }
    };

    for (id, score) in character.abilities {
        if system.ability(id).is_none() {
            out.push(issue(
                Severity::Error,
                "ABILITY_UNKNOWN",
                format!("abilities.{id}"),
                format!("ability {id} is not in the system"),
                format!("La caractéristique « {id} » ({score}) n'existe pas dans ces règles."),
            ));
        }
    }

    let Some(class) = class else { return out };
    match system.creation.abilities {
        AbilityAssignment::FromClass => {
            if character.abilities.is_empty() {
                return out;
            }
            // In the system's order, so the GM reads them as on the sheet.
            for def in &system.abilities {
                let (Some(&given), Some(&set)) = (
                    class.abilities.get(&def.id),
                    character.abilities.get(&def.id),
                ) else {
                    continue;
                };
                if given != set {
                    out.push(issue(
                        Severity::Warning,
                        "ABILITY_OFF_CLASS",
                        format!("abilities.{}", def.id),
                        format!("{} is {set}, class {} gives {given}", def.id, class.id),
                        format!(
                            "{} à {set} : la classe {} donne {given}.",
                            def.name, class.name
                        ),
                    ));
                }
            }
            let budget: i32 = class.abilities.values().sum();
            let spent: i32 = system
                .abilities
                .iter()
                .map(|a| {
                    character
                        .abilities
                        .get(&a.id)
                        .or_else(|| class.abilities.get(&a.id))
                        .copied()
                        .unwrap_or(0)
                })
                .sum();
            if spent > budget {
                out.push(issue(
                    Severity::Warning,
                    "ABILITY_BUDGET_EXCEEDED",
                    "abilities".into(),
                    format!("scores add up to {spent}, class {} gives {budget}", class.id),
                    format!(
                        "Les caractéristiques totalisent {spent}, {} de plus que les {budget} de la classe {}.",
                        spent - budget,
                        class.name
                    ),
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corsaires() -> RuleSystem {
        RuleSystem::from_yaml(include_str!("../../../content/rules/corsaires/v1.yaml")).unwrap()
    }

    fn codes(issues: &[Issue]) -> Vec<&'static str> {
        issues.iter().map(|i| i.code).collect()
    }

    fn scores(pairs: &[(&str, i32)]) -> BTreeMap<String, i32> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    #[test]
    fn a_class_character_with_the_class_scores_passes() {
        let s = corsaires();
        let class = s.class("bretteur").unwrap();
        for abilities in [BTreeMap::new(), class.abilities.clone()] {
            let c = CharacterInput {
                name: "Borin",
                class_id: Some("bretteur"),
                abilities: &abilities,
            };
            assert!(check_character(&s, &c).is_empty(), "{abilities:?}");
        }
    }

    #[test]
    fn a_raised_score_is_flagged_with_what_the_class_gives() {
        let s = corsaires();
        let mut abilities = s.class("bretteur").unwrap().abilities.clone();
        abilities.insert("FOR".into(), 18);
        let c = CharacterInput {
            name: "Borin",
            class_id: Some("bretteur"),
            abilities: &abilities,
        };
        let issues = check_character(&s, &c);
        assert_eq!(
            codes(&issues),
            ["ABILITY_OFF_CLASS", "ABILITY_BUDGET_EXCEEDED"]
        );
        assert_eq!(issues[0].path, "abilities.FOR");
        let msg = issues[0].message.as_deref().unwrap();
        assert!(msg.contains("18") && msg.contains("13"), "{msg}");
        assert!(issues[1].message.as_deref().unwrap().contains("5 de plus"));
    }

    #[test]
    fn a_moved_point_is_off_class_but_within_budget() {
        let s = corsaires();
        let mut abilities = s.class("bretteur").unwrap().abilities.clone();
        *abilities.get_mut("FOR").unwrap() -= 1;
        *abilities.get_mut("SAG").unwrap() += 1;
        let c = CharacterInput {
            name: "Borin",
            class_id: Some("bretteur"),
            abilities: &abilities,
        };
        assert_eq!(
            codes(&check_character(&s, &c)),
            ["ABILITY_OFF_CLASS", "ABILITY_OFF_CLASS"]
        );
    }

    #[test]
    fn missing_and_unknown_choices_are_errors() {
        let s = corsaires();
        let abilities = scores(&[("MANA", 3)]);
        let c = CharacterInput {
            name: " ",
            class_id: None,
            abilities: &abilities,
        };
        assert_eq!(
            codes(&check_character(&s, &c)),
            ["NAME_MISSING", "CLASS_MISSING", "ABILITY_UNKNOWN"]
        );
        let empty = BTreeMap::new();
        let c = CharacterInput {
            name: "Borin",
            class_id: Some("nain-guerrier"),
            abilities: &empty,
        };
        assert_eq!(codes(&check_character(&s, &c)), ["CLASS_UNKNOWN"]);
    }
}
