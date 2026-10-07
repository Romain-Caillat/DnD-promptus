//! engine/formalise-house-rules — the co-GM turns a house rule the GM
//! wrote in French into a rule the server judges, and the server replays
//! its cases.
//!
//! - [`formalise`]: the co-GM's proposal (template `house-rule.v1`, a
//!   counted call). Ids the model invented — a trait, a damage type, a
//!   condition, a class or adversary the rules do not have — are taken
//!   out and listed in `dropped`, never shown as if they existed
//!   (`MEMORY.md` §3). Nothing is stored: the GM edits the proposal and
//!   adds it to the draft through the editor's ordinary save.
//! - [`try_rule`]: the same check on a form the GM edited, without the
//!   model: does it load, and what do its cases give.
//!
//! Both work on the campaign's draft: the rule is put into the draft's
//! text in memory (replacing the rule with the same id) and the whole
//! system goes through the loader, so a formal rule obeys exactly what
//! a saved one does.

use std::collections::BTreeMap;

use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::house::{CaseFighter, CaseResult, FormalRule, HouseEffect, Who};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::versions;
use crate::ai::{self, Ai, AiError, LlmRequest, ledger, templates};
use crate::auth::guard::CurrentGm;
use crate::error::AppError;

const NAME_MAX: usize = 200;
const TEXT_MAX: usize = 4_000;

/// A house rule as the editor holds it, formal form or not yet.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleInput {
    pub id: String,
    pub name: String,
    pub text: String,
    /// The formal form to check ([`try_rule`] only), in the file's shape.
    #[serde(default)]
    pub formal: Option<serde_json::Value>,
}

/// Something the model named that these rules do not have.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dropped {
    /// `trait`, `damage_type`, `condition`, `origin`, `case`.
    pub kind: &'static str,
    pub id: String,
}

/// Why the formal form does not load, as the loader says it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub code: &'static str,
    pub path: String,
    pub detail: String,
}

/// A formal form, checked against the draft.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub id: String,
    /// The formal form, in the file's shape (what the editor puts under
    /// `house_rules[].formal`).
    pub formal: serde_json::Value,
    /// Empty when it loads.
    pub problems: Vec<Problem>,
    /// Its cases replayed (none while it does not load).
    pub cases: Vec<CaseResult>,
    pub dropped: Vec<Dropped>,
    /// The co-GM's word to the GM (a doubt, a question); empty for
    /// [`try_rule`].
    pub remark: String,
}

fn check_input(input: &RuleInput) -> Result<(), AppError> {
    if input.id.trim().is_empty() || input.name.trim().is_empty() || input.text.trim().is_empty() {
        return Err(AppError::BadRequest("EMPTY_TEXT"));
    }
    if input.name.chars().count() > NAME_MAX || input.text.chars().count() > TEXT_MAX {
        return Err(AppError::BadRequest("TEXT_TOO_LONG"));
    }
    Ok(())
}

/// The draft's text with `rule` (and its formal form) in place of the
/// house rule with the same id, or added at the end.
fn with_rule(draft: &str, input: &RuleInput, formal: &FormalRule) -> Result<String, AppError> {
    use serde_yaml_ng::{Mapping, Value};
    let mut doc: Value =
        serde_yaml_ng::from_str(draft).map_err(|e| AppError::internal("draft yaml", e))?;
    // JSON's enums are one-key maps, the file's shape.
    let formal_json =
        serde_json::to_value(formal).map_err(|e| AppError::internal("formal json", e))?;
    let formal_yaml: Value =
        serde_yaml_ng::to_value(&formal_json).map_err(|e| AppError::internal("formal yaml", e))?;
    let mut rule = Mapping::new();
    rule.insert("id".into(), input.id.trim().into());
    rule.insert("name".into(), input.name.trim().into());
    rule.insert("text".into(), input.text.trim().into());
    rule.insert("formal".into(), formal_yaml);
    let root = doc
        .as_mapping_mut()
        .ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let list = root
        .entry("house_rules".into())
        .or_insert_with(|| Value::Sequence(Vec::new()));
    if !list.is_sequence() {
        *list = Value::Sequence(Vec::new());
    }
    let seq = list.as_sequence_mut().expect("a sequence");
    let id = Value::from(input.id.trim());
    match seq
        .iter_mut()
        .find(|r| r.get("id").is_some_and(|v| *v == id))
    {
        Some(existing) => *existing = Value::Mapping(rule),
        None => seq.push(Value::Mapping(rule)),
    }
    serde_yaml_ng::to_string(&doc).map_err(|e| AppError::internal("draft yaml", e))
}

/// Takes out of `formal` what `system` does not have, and says what.
fn sanitise(system: &RuleSystem, formal: &mut FormalRule) -> Vec<Dropped> {
    let mut dropped = Vec::new();
    let mut drop = |kind: &'static str, id: &str| {
        let d = Dropped {
            kind,
            id: id.to_string(),
        };
        if !dropped.contains(&d) {
            dropped.push(d);
        }
    };
    if let Some(t) = &formal.damage_type
        && system.damage_type(t).is_none()
    {
        drop("damage_type", t);
        formal.damage_type = None;
    }
    let mut who = |w: &mut Who| {
        for list in [&mut w.traits, &mut w.except_traits] {
            list.retain(|t| {
                let known = system.trait_def(t).is_some();
                if !known {
                    drop("trait", t);
                }
                known
            });
        }
        w.except.retain(|id| {
            let known = system.class(id).is_some() || system.adversary(id).is_some();
            if !known {
                drop("origin", id);
            }
            known
        });
    };
    who(&mut formal.actor);
    who(&mut formal.target);
    formal.effects.retain(|e| match e {
        HouseEffect::Apply(spec) => match &spec.condition {
            Some(c) if system.condition(c).is_none() => {
                drop("condition", c);
                false
            }
            _ => true,
        },
        _ => true,
    });
    let fighter_known = |f: &CaseFighter| match f {
        CaseFighter::Class(id) => system.class(id).is_some(),
        CaseFighter::Adversary(id) => system.adversary(id).is_some(),
    };
    formal.cases.retain(|c| {
        let ok = fighter_known(&c.actor) && fighter_known(&c.target);
        if !ok {
            drop("case", &c.name);
        }
        ok
    });
    dropped
}

/// Loads the draft with the rule in it: the cases, or why it does not
/// load (only this rule's errors; the rest of the draft already loads).
fn check(
    draft: &str,
    input: &RuleInput,
    formal: &FormalRule,
) -> Result<(Vec<Problem>, Vec<CaseResult>), AppError> {
    let text = with_rule(draft, input, formal)?;
    match RuleSystem::from_yaml(&text) {
        Ok(system) => {
            let rule = system
                .house_rule(input.id.trim())
                .ok_or_else(|| AppError::internal("house rule", "missing after insert"))?;
            Ok((
                Vec::new(),
                promptus_shared::rules::house::run_cases(&system, rule),
            ))
        }
        Err(e) => Ok((
            e.0.into_iter()
                .map(|r| Problem {
                    code: r.code.as_str(),
                    path: r.path,
                    detail: r.detail,
                })
                .collect(),
            Vec::new(),
        )),
    }
}

fn proposal(
    draft: &str,
    input: &RuleInput,
    formal: FormalRule,
    dropped: Vec<Dropped>,
    remark: String,
) -> Result<Proposal, AppError> {
    let (problems, cases) = check(draft, input, &formal)?;
    Ok(Proposal {
        id: input.id.trim().to_string(),
        formal: serde_json::to_value(&formal).map_err(|e| AppError::internal("formal json", e))?,
        problems,
        cases,
        dropped,
        remark,
    })
}

/// What these rules contain, for the model: every id it may use.
fn vocabulary(s: &RuleSystem) -> String {
    let mut out = String::new();
    let line = |out: &mut String, id: &str, name: &str, more: &str| {
        out.push_str(&format!("- `{id}` — {name}"));
        if !more.is_empty() {
            out.push_str(&format!(" : {more}"));
        }
        out.push('\n');
    };
    out.push_str("## États\n");
    for c in &s.conditions {
        line(&mut out, &c.id, &c.name, &c.description);
    }
    out.push_str("## Étiquettes\n");
    if s.traits.is_empty() {
        out.push_str("(aucune)\n");
    }
    for t in &s.traits {
        line(&mut out, &t.id, &t.name, &t.description);
    }
    out.push_str("## Types de dégâts\n");
    if s.damage_types.is_empty() {
        out.push_str("(aucun : ces règles ne typent pas les dégâts)\n");
    }
    for t in &s.damage_types {
        line(&mut out, &t.id, &t.name, "");
    }
    out.push_str("## Caractéristiques\n");
    for a in &s.abilities {
        line(&mut out, &a.id, &a.name, "");
    }
    out.push_str("## Difficultés\n");
    for d in &s.difficulties {
        line(&mut out, &d.id, &d.name, &d.value.to_string());
    }
    let actions = |list: &[promptus_shared::rules::model::ActionDef]| {
        list.iter()
            .map(|a| format!("`{}` ({})", a.id, a.name))
            .collect::<Vec<_>>()
            .join(", ")
    };
    out.push_str("## Classes (camp `party`)\n");
    for c in &s.classes {
        let traits = if c.traits.is_empty() {
            String::new()
        } else {
            format!(" ; étiquettes {}", c.traits.join(", "))
        };
        line(
            &mut out,
            &c.id,
            &c.name,
            &format!("actions {}{traits}", actions(&c.actions)),
        );
    }
    out.push_str("## Adversaires (camp `opposition`)\n");
    if s.adversaries.is_empty() {
        out.push_str("(aucun dans ces règles : pour un cas, une classe peut servir de cible)\n");
    }
    for a in &s.adversaries {
        let traits = if a.traits.is_empty() {
            String::new()
        } else {
            format!(" ; étiquettes {}", a.traits.join(", "))
        };
        line(
            &mut out,
            &a.id,
            &a.name,
            &format!("actions {}{traits}", actions(&a.actions)),
        );
    }
    out.push_str("## Objets utilisables\n");
    for i in s.items.iter().filter(|i| i.action.is_some()) {
        line(&mut out, &i.id, &i.name, &i.description);
    }
    out
}

#[derive(Deserialize)]
struct RawProposal {
    formal: FormalRule,
    #[serde(default)]
    remark: String,
}

/// The co-GM's formal form of house rule `input`, checked against the
/// campaign's draft. Not stored.
///
/// # Errors
///
/// 404 as `versions::get`; 409 `NO_DRAFT`, `AI_BUDGET_EXCEEDED`; 400
/// `EMPTY_TEXT`, `TEXT_TOO_LONG`, `RULES_INVALID` (the draft itself no
/// longer loads); 503 `AI_NOT_CONFIGURED`; 502 `AI_UNAVAILABLE`,
/// `AI_OUTPUT_INVALID`.
pub async fn formalise(
    pool: &PgPool,
    ai: &Ai,
    gm: &CurrentGm,
    campaign: Uuid,
    input: &RuleInput,
) -> Result<Proposal, AppError> {
    check_input(input)?;
    let (row, draft) = versions::draft_of(pool, gm, campaign).await?;
    let system = versions::load(&draft)?;
    let vars: BTreeMap<&str, String> = [
        ("system", system.name.clone()),
        ("name", input.name.trim().to_string()),
        ("text", input.text.trim().to_string()),
        ("vocabulary", vocabulary(&system)),
    ]
    .into_iter()
    .collect();
    let messages = templates::HOUSE_RULE
        .render(&vars)
        .map_err(|e| AppError::internal("house-rule template", e))?;
    let mut req = LlmRequest::json(messages);
    req.temperature = 0.2;
    req.max_tokens = 1_500;
    let reply = ai
        .complete(
            pool,
            row.id,
            "rules.house_rule",
            &templates::HOUSE_RULE,
            &req,
        )
        .await?;
    let raw: RawProposal = ai::parse_json(&reply.text).map_err(|e| ledger::app_error(&e))?;
    let mut formal = raw.formal;
    let dropped = sanitise(&system, &mut formal);
    if formal.effects.is_empty() {
        return Err(ledger::app_error(&AiError::Schema(
            "la règle proposée ne fait rien que ces règles connaissent".into(),
        )));
    }
    proposal(
        &draft,
        input,
        formal,
        dropped,
        raw.remark.trim().to_string(),
    )
}

/// Checks a formal form the GM edited, against the campaign's draft.
///
/// # Errors
///
/// As [`formalise`] without the AI ones; 400 `FORMAL_INVALID` when the
/// form does not have the format's shape.
pub async fn try_rule(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    input: &RuleInput,
) -> Result<Proposal, AppError> {
    check_input(input)?;
    let raw = input
        .formal
        .clone()
        .ok_or(AppError::BadRequest("FORMAL_INVALID"))?;
    let formal: FormalRule = serde_json::from_value(raw).map_err(|e| AppError::Invalid {
        code: "FORMAL_INVALID",
        detail: e.to_string(),
    })?;
    let (_, draft) = versions::draft_of(pool, gm, campaign).await?;
    versions::load(&draft)?;
    proposal(&draft, input, formal, Vec::new(), String::new())
}

#[cfg(test)]
mod tests {
    use promptus_shared::story::RuleSystemRef;

    use super::*;

    fn preset(id: &str) -> (&'static str, RuleSystem) {
        let r = RuleSystemRef {
            id: id.into(),
            version: 1,
        };
        let text = crate::content::preset_yaml(&r).unwrap();
        (text, RuleSystem::from_yaml(text).unwrap())
    }

    fn input(id: &str) -> RuleInput {
        RuleInput {
            id: id.into(),
            name: "Le pied qui glisse".into(),
            text: "Sur un 1 naturel, le corsaire glisse.".into(),
            formal: None,
        }
    }

    fn fumble(
        condition: &str,
        actor: CaseFighter,
        action: &str,
        target: CaseFighter,
    ) -> FormalRule {
        serde_json::from_value(serde_json::json!({
            "when": "miss",
            "critical": true,
            "actor": { "side": "party", "except_traits": ["trait_invente"] },
            "effects": [{ "apply": { "condition": condition, "turns": 1, "to": "self" } }],
            "players": "rule",
            "cases": [
                { "name": "Sur un 1", "actor": serde_json::to_value(&actor).unwrap(), "action": action,
                  "target": serde_json::to_value(&target).unwrap(), "roll": "fumble", "expect": "applies" },
                { "name": "Raté", "actor": serde_json::to_value(&actor).unwrap(), "action": action,
                  "target": serde_json::to_value(&target).unwrap(), "roll": "miss", "expect": "nothing" },
                { "name": "Inventé", "actor": { "class": "classe_inventee" }, "action": action,
                  "target": serde_json::to_value(&target).unwrap(), "roll": "miss", "expect": "nothing" }
            ]
        }))
        .unwrap()
    }

    #[test]
    fn invented_ids_are_dropped_and_the_rest_checks_on_both_worlds() {
        for (world, condition, actor, action, target) in [
            (
                "corsaires",
                "renverse",
                CaseFighter::Class("bretteur".into()),
                "estocade",
                CaseFighter::Adversary("marin_de_gueule_rouge".into()),
            ),
            (
                "brasier",
                "etourdi",
                CaseFighter::Class("pilote".into()),
                "tir_reflexe",
                CaseFighter::Class("mecano".into()),
            ),
        ] {
            let (text, system) = preset(world);
            let mut formal = fumble(condition, actor, action, target);
            let dropped = sanitise(&system, &mut formal);
            assert_eq!(
                dropped,
                [
                    Dropped {
                        kind: "trait",
                        id: "trait_invente".into()
                    },
                    Dropped {
                        kind: "case",
                        id: "Inventé".into()
                    }
                ],
                "{world}"
            );
            let p = proposal(text, &input("pied"), formal, dropped, String::new()).unwrap();
            assert!(p.problems.is_empty(), "{world}: {:?}", p.problems);
            assert_eq!(p.cases.len(), 2, "{world}");
            assert!(p.cases.iter().all(|c| c.passed), "{world}: {:?}", p.cases);
            // The form comes back in the file's shape.
            assert_eq!(p.formal["effects"][0]["apply"]["to"], "self");
        }
    }

    #[test]
    fn a_form_naming_a_missing_condition_comes_back_with_the_loader_s_problem() {
        let (text, _) = preset("corsaires");
        let formal = fumble(
            "effraye",
            CaseFighter::Class("bretteur".into()),
            "estocade",
            CaseFighter::Adversary("gueule_rouge".into()),
        );
        let p = proposal(text, &input("pied"), formal, Vec::new(), String::new()).unwrap();
        assert!(p.cases.is_empty());
        assert!(
            p.problems
                .iter()
                .any(|x| x.code == "unknown_condition" && x.path.starts_with("house_rules[pied]")),
            "{:?}",
            p.problems
        );
    }

    #[test]
    fn the_rule_replaces_the_one_with_its_id_and_keeps_the_others() {
        let (text, system) = preset("srd");
        let mut formal = fumble(
            "a_terre",
            CaseFighter::Class("guerrier".into()),
            "epee_longue",
            CaseFighter::Adversary("gobelin".into()),
        );
        sanitise(&system, &mut formal);
        let mut i = input("morts_vivants_feu");
        i.name = "Remplacée".into();
        let out = RuleSystem::from_yaml(&with_rule(text, &i, &formal).unwrap()).unwrap();
        assert_eq!(out.house_rules.len(), 1);
        assert_eq!(out.house_rules[0].name, "Remplacée");
        let out =
            RuleSystem::from_yaml(&with_rule(text, &input("autre"), &formal).unwrap()).unwrap();
        assert_eq!(out.house_rules.len(), 2);
    }
}
