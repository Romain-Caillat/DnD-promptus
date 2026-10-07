//! What a draft of the rule system touches, recomputed on every save
//! (`campaign/edit-rule-system`): the GM sees the effect of a change
//! before any player meets it (`docs/lecons-des-parties.md` §3, chain 1).
//!
//! - the **lint** of the draft, and which findings the change added or
//!   cleared against the version the campaign plays;
//! - **what players will read** changed (`rule_changes`), the list the
//!   rules page shows them before the next session;
//! - the **campaign references** the draft breaks: a scene check on an
//!   ability that is gone, an adversary or a class that no longer
//!   exists, a difficulty off the new scale;
//! - the **fights**: every scenario of the world, and every planned
//!   encounter of the campaign that can be staged
//!   (`story::encounter_scenario`), played on the current version and on
//!   the draft with the same seeds;
//! - the **house rules** the server judges: every case of each formal
//!   rule replayed on the draft (`engine/formalise-house-rules`).

use std::collections::BTreeSet;

use promptus_shared::combat::scenario::{PolicyKind, Scenario};
use promptus_shared::combat::simulate::{SimParams, SimReport, simulate};
use promptus_shared::issue::Issue;
use promptus_shared::maps::Map;
use promptus_shared::rules::changes::{RuleChange, rule_changes};
use promptus_shared::rules::house::{CaseResult, run_cases};
use promptus_shared::rules::{RuleSystem, lint};
use promptus_shared::story::{Campaign, Library, encounter_scenario, validate_with};
use serde::Serialize;

use crate::content;

/// Fights played per scenario and version: enough to see a swing of a
/// few points, short enough to answer a save at once.
pub const FIGHTS: u32 = 40;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub lint: Vec<Issue>,
    /// Findings of the draft the current version does not have.
    pub lint_added: Vec<Issue>,
    /// Findings of the current version the draft no longer has.
    pub lint_removed: Vec<Issue>,
    /// What the players' rules page will list as changed.
    pub changes: Vec<RuleChange>,
    /// The campaign's references the draft no longer satisfies.
    pub story: Vec<Issue>,
    pub fights: Vec<FightCheck>,
    /// The draft's formalised house rules and their cases.
    pub house_rules: Vec<HouseRuleCheck>,
}

/// One formalised house rule, its cases replayed on the draft.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HouseRuleCheck {
    pub id: String,
    pub name: String,
    pub cases: Vec<CaseResult>,
}

/// One fight, on both versions.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightCheck {
    pub id: String,
    pub name: String,
    /// `scenario` for the world's test fights, `encounter` for a scene
    /// of the campaign.
    pub source: &'static str,
    pub current: Option<FightSummary>,
    pub draft: Option<FightSummary>,
    /// Why the draft could not play it (a stat block it removed…).
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightSummary {
    pub fights: u32,
    pub party_win_rate: f64,
    pub rounds: f64,
    pub minutes: f64,
}

impl FightSummary {
    fn of(r: &SimReport) -> Self {
        Self {
            fights: r.params.fights,
            party_win_rate: r.party_win_rate,
            rounds: r.rounds.mean,
            minutes: r.minutes.mean,
        }
    }
}

/// The text and parsed form of one version.
pub struct Version<'a> {
    pub text: &'a str,
    pub system: &'a RuleSystem,
}

/// The report of `draft` against `current`, for `campaign` and the maps
/// it may play on. CPU work: callers run it off the async runtime.
#[must_use]
pub fn report(
    campaign: &Campaign,
    maps: &[Map],
    current: &Version<'_>,
    draft: &Version<'_>,
) -> Report {
    let key = |i: &Issue| (i.code, i.path.clone());
    let lint_draft = lint(draft.system);
    let lint_current = lint(current.system);
    let before: BTreeSet<_> = lint_current.iter().map(key).collect();
    let after: BTreeSet<_> = lint_draft.iter().map(key).collect();
    let lint_added = lint_draft
        .iter()
        .filter(|i| !before.contains(&key(i)))
        .cloned()
        .collect();
    let lint_removed = lint_current
        .iter()
        .filter(|i| !after.contains(&key(i)))
        .cloned()
        .collect();

    let mut staged = campaign.clone();
    staged.rules.version = draft.system.version;
    fn library<'a>(s: &'a RuleSystem, maps: &'a [Map]) -> Library<'a> {
        Library {
            rules: Some(s),
            maps: Some(maps),
        }
    }
    let rule_codes = |issues: Vec<Issue>| -> BTreeSet<(&'static str, String)> {
        issues
            .into_iter()
            .filter(|i| i.code.starts_with("RULES_") || i.code == "DIFFICULTY_OFF_SCALE")
            .map(|i| (i.code, i.path))
            .collect()
    };
    let already = rule_codes(validate_with(campaign, &library(current.system, maps)));
    let story = validate_with(&staged, &library(draft.system, maps))
        .into_iter()
        .filter(|i| i.code.starts_with("RULES_") || i.code == "DIFFICULTY_OFF_SCALE")
        .filter(|i| !already.contains(&(i.code, i.path.clone())))
        .collect();

    Report {
        lint: lint_draft,
        lint_added,
        lint_removed,
        changes: rule_changes(current.system, draft.system),
        story,
        fights: fights(campaign, maps, current, draft),
        house_rules: draft
            .system
            .house_rules
            .iter()
            .filter(|h| h.formal.is_some())
            .map(|h| HouseRuleCheck {
                id: h.id.clone(),
                name: h.name.clone(),
                cases: run_cases(draft.system, h),
            })
            .collect(),
    }
}

fn fights(
    campaign: &Campaign,
    maps: &[Map],
    current: &Version<'_>,
    draft: &Version<'_>,
) -> Vec<FightCheck> {
    let mut out = Vec::new();
    for sc in content::scenarios(&draft.system.id) {
        let Some(map) = maps
            .iter()
            .find(|m| m.id == sc.map)
            .or_else(|| content::world_map(&sc.rules, &sc.map))
        else {
            continue;
        };
        out.push(play_both(sc, "scenario", map, current, draft));
    }
    for node in &campaign.nodes {
        let Some(map_id) = node.map.as_deref() else {
            continue;
        };
        let Some(map) = maps.iter().find(|m| m.id == map_id) else {
            continue;
        };
        if let Ok(sc) = encounter_scenario(campaign, node, draft.system) {
            out.push(play_both(&sc, "encounter", map, current, draft));
        }
    }
    out
}

fn play_both(
    sc: &Scenario,
    source: &'static str,
    map: &Map,
    current: &Version<'_>,
    draft: &Version<'_>,
) -> FightCheck {
    let run = |text: &str| -> Result<FightSummary, String> {
        let system = sc.system(text, None).map_err(|e| e.to_string())?;
        let party = sc
            .party
            .policies
            .first()
            .copied()
            .unwrap_or(PolicyKind::Brawler);
        let opposition = sc
            .opposition
            .policies
            .first()
            .copied()
            .unwrap_or(PolicyKind::Brawler);
        let params = SimParams::new(FIGHTS, sc.simulation.seed);
        simulate(&system, sc, map, party, opposition, &params)
            .map(|r| FightSummary::of(&r))
            .map_err(|e| e.to_string())
    };
    let draft_run = run(draft.text);
    FightCheck {
        id: sc.id.clone(),
        name: sc.name.clone(),
        source,
        current: run(current.text).ok(),
        error: draft_run.as_ref().err().cloned(),
        draft: draft_run.ok(),
    }
}
