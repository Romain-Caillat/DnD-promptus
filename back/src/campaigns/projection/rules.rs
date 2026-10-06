//! The campaign's rules as one page for players (`player/read-the-rules`),
//! part of the single projection point (`MEMORY.md` §3).
//!
//! Everything is read from the rule system, nothing is written by hand:
//! a rule that changes in the data changes on the page. Allow-list as in
//! the rest of the projection — never the adversaries, their tiers, nor
//! any GM note (on a rule, a class, the creation rule or an action tag).
//!
//! With a character whose sheet names a class, the page also carries
//! the player's own numbers, computed by the engine: what each ability
//! needs on the die against each difficulty, each card's cost, cooldown
//! and attack roll, and worked rolls, one per outcome band, made by the
//! engine itself with the die faces set (`RollBreakdown`, the type a
//! live roll will carry).

use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::action::{action_cards, attack_modifiers};
use promptus_shared::rules::changes::RuleChange;
use promptus_shared::rules::check::{
    Advantage, Modifier, OutcomeBand, RollBreakdown, ability_check, ability_modifiers, band_for,
    difficulty,
};
use promptus_shared::rules::dice::ScriptedDice;
use promptus_shared::rules::model::{
    AttackAbility, ConditionKind, CooldownMeaning, GroupThreshold, LongRangeRule, PrecisionRule,
    RollScope, ZeroHpRule,
};
use promptus_shared::rules::sheet::Combatant;
use serde::Serialize;

use super::{ActionCardView, card_view, level_one};
use crate::players::CharacterSheet;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesView {
    pub name: String,
    pub version: u32,
    pub check: CheckView,
    pub abilities: Vec<AbilityView>,
    pub difficulties: Vec<DifficultyView>,
    /// The four bands, from natural 1 to natural 20.
    pub outcomes: Vec<OutcomeView>,
    pub attack: AttackView,
    pub hit_points: StatView,
    /// The action economy, one entry per setting (ground, ship…).
    pub turns: Vec<TurnView>,
    pub action_kinds: Vec<KindView>,
    pub cooldown: &'static str,
    /// Whether a condition's turns count the turn it was put on.
    pub application_turn_counts: bool,
    pub conditions: Vec<ConditionView>,
    pub zero_hp: ZeroHpView,
    pub progression: ProgressionView,
    pub combat: CombatView,
    /// How a group check succeeds, when the system has one.
    pub group_check: Option<&'static str>,
    /// The player's own numbers; `None` for a spectator or a sheet
    /// with no class yet.
    pub mine: Option<MineView>,
    /// What changed since the version the player last read.
    pub changes: Option<ChangesView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckView {
    /// `1d20`.
    pub die: String,
    pub faces: u32,
    pub advantage: bool,
    /// Score → modifier, as the system writes it.
    pub modifier: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbilityView {
    pub id: String,
    pub name: String,
    pub description: String,
    /// The player's score and modifier, when they have a sheet.
    pub score: Option<i32>,
    pub modifier: Option<i32>,
    /// Per difficulty, in the system's order: the smallest face that
    /// succeeds with this modifier.
    pub needs: Vec<NeedView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeedView {
    pub difficulty: String,
    /// `None`: no face succeeds.
    pub from_face: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifficultyView {
    pub id: String,
    pub name: String,
    pub value: i32,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeView {
    pub band: OutcomeBand,
    pub name: String,
    pub description: String,
    /// Faces that land here whatever the target (critical bands).
    pub natural: Vec<u32>,
    pub xp: u32,
    pub damage_multiplier: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackView {
    /// Which ability an attack adds: `first_primary`, `best_primary`.
    pub ability: &'static str,
    pub precision_applies: bool,
    pub armor_class: StatView,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatView {
    pub name: String,
    pub abbr: String,
    pub formula: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnView {
    pub name: String,
    pub actions_per_turn: u32,
    pub limits: Vec<LimitView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitView {
    pub kind: String,
    pub max_per_turn: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KindView {
    pub name: String,
    pub description: String,
    pub cost: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionView {
    pub name: String,
    pub description: String,
    pub kind: ConditionKind,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "rule", rename_all = "camelCase")]
pub enum ZeroHpView {
    #[serde(rename_all = "camelCase")]
    KnockedOut {
        condition: String,
        out_after_turns: u32,
        out_condition: String,
    },
    #[serde(rename_all = "camelCase")]
    DeathSaves {
        difficulty: i32,
        successes: u32,
        failures: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionView {
    pub upgrade_every_xp: u32,
    pub upgrade_points: u32,
    pub levels: Vec<LevelView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LevelView {
    pub level: u32,
    pub xp: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatView {
    /// The action kind a move spends; `None`: one free move per turn.
    pub move_kind: Option<KindView>,
    /// The action kind a flight spends, and the ability rolled when the
    /// GM says it is hard.
    pub flee: Option<FleeView>,
    /// Added to an attack roll against a target behind cover.
    pub cover_half: i32,
    pub cover_three_quarters: i32,
    /// Beyond an action's range: disadvantage, or this modifier.
    pub long_range_disadvantage: bool,
    pub long_range_modifier: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FleeView {
    pub kind: KindView,
    pub ability: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MineView {
    pub class_name: String,
    pub cards: Vec<RuleCardView>,
    /// The ability and difficulty of the worked rolls.
    pub example_ability: String,
    pub example_difficulty: String,
    /// One roll per outcome band the player can land in, as the engine
    /// makes it with the die face set.
    pub examples: Vec<RollBreakdown>,
}

/// A card as the rules page explains it: what it is, what it costs, its
/// cooldown, and what its attack roll adds.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleCardView {
    #[serde(flatten)]
    pub card: ActionCardView,
    /// Actions of the turn it spends.
    pub cost: u32,
    /// For a card that rolls to hit: every modifier and where it comes
    /// from. The target's armour class decides; the player does not
    /// know it before the roll.
    pub attack_modifiers: Option<Vec<Modifier>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangesView {
    pub from_version: u32,
    pub to_version: u32,
    /// The campaign moved to another rule system, or the server no
    /// longer has the version read: the whole page is to read again.
    pub replaced: bool,
    pub items: Vec<RuleChange>,
}

const fn attack_ability_code(a: AttackAbility) -> &'static str {
    match a {
        AttackAbility::FirstPrimary => "first_primary",
        AttackAbility::BestPrimary => "best_primary",
    }
}

const fn cooldown_code(m: CooldownMeaning) -> &'static str {
    match m {
        CooldownMeaning::SkipNextTurns => "skip_next_turns",
        CooldownMeaning::TurnOfUseCounts => "turn_of_use_counts",
    }
}

const fn group_code(t: GroupThreshold) -> &'static str {
    match t {
        GroupThreshold::AtLeastHalf => "at_least_half",
        GroupThreshold::Majority => "majority",
        GroupThreshold::All => "all",
        GroupThreshold::Any => "any",
    }
}

fn kind_view(system: &RuleSystem, id: &str) -> Option<KindView> {
    system.action_kind(id).map(|k| KindView {
        name: k.name.clone(),
        description: k.description.clone(),
        cost: k.cost,
    })
}

fn condition_name(system: &RuleSystem, id: &str) -> String {
    system
        .condition(id)
        .map_or_else(|| id.to_string(), |c| c.name.clone())
}

/// The smallest natural face that succeeds against `target` with
/// `bonus`, as the engine bands it.
fn from_face(system: &RuleSystem, bonus: i32, target: i32) -> Option<u32> {
    (1..=system.check.dice.faces).find(|&f| {
        band_for(system, f, f as i32 + bonus, Some(target)).is_some_and(OutcomeBand::is_success)
    })
}

fn sum(mods: &[Modifier]) -> i32 {
    mods.iter().map(|m| m.value).sum()
}

/// The difficulty the worked rolls are made against: the middle one.
fn example_difficulty(system: &RuleSystem) -> Option<&str> {
    let mut d: Vec<_> = system.difficulties.iter().collect();
    d.sort_by_key(|d| d.value);
    d.get(d.len().saturating_sub(1) / 2).map(|d| d.id.as_str())
}

/// One roll per band `sheet` can land in with `ability` against the
/// middle difficulty, each made by the engine with its face set: the
/// critical faces, the best failing face and the first succeeding one.
fn examples(system: &RuleSystem, sheet: &Combatant, ability: &str) -> Option<Vec<RollBreakdown>> {
    let target = difficulty(system, example_difficulty(system)?).ok()?;
    let (mods, _, _) = ability_modifiers(system, sheet, ability, RollScope::Checks).ok()?;
    let bonus = sum(&mods);
    let faces = system.check.dice.faces;
    let band = |f: u32| band_for(system, f, f as i32 + bonus, Some(target.value()));
    let pick = [
        (1..=faces).find(|&f| band(f) == Some(OutcomeBand::CriticalFailure)),
        (1..=faces)
            .rev()
            .find(|&f| band(f) == Some(OutcomeBand::Failure)),
        (1..=faces).find(|&f| band(f) == Some(OutcomeBand::Success)),
        (1..=faces)
            .rev()
            .find(|&f| band(f) == Some(OutcomeBand::CriticalSuccess)),
    ];
    pick.into_iter()
        .flatten()
        .map(|face| {
            ability_check(
                system,
                sheet,
                ability,
                RollScope::Checks,
                Some(target.clone()),
                Advantage::Normal,
                &mut ScriptedDice::new([face]),
            )
            .ok()
        })
        .collect()
}

fn mine(system: &RuleSystem, sheet: &Combatant) -> Option<MineView> {
    let class = sheet.class(system)?;
    let cards = action_cards(system, sheet)
        .iter()
        .map(|card| RuleCardView {
            card: card_view(system, card),
            cost: system.action_kind(&card.action.kind).map_or(0, |k| k.cost),
            attack_modifiers: card
                .attack_bonus
                .and_then(|_| attack_modifiers(system, sheet, card.action).ok())
                .map(|(mods, _, _)| mods),
        })
        .collect();
    let ability = class.primary_abilities.first()?;
    Some(MineView {
        class_name: class.name.clone(),
        cards,
        example_ability: system.ability(ability)?.name.clone(),
        example_difficulty: system.difficulty(example_difficulty(system)?)?.name.clone(),
        examples: examples(system, sheet, ability)?,
    })
}

/// The rules page of `system` for a player whose sheet is `sheet` (none
/// for a spectator), with what changed since they last read it.
#[must_use]
pub fn project_rules(
    system: &RuleSystem,
    sheet: Option<&CharacterSheet>,
    changes: Option<ChangesView>,
) -> RulesView {
    let me = sheet.and_then(|s| level_one(system, s.class_id.as_deref()?, &s.abilities));
    let stat = |s: &promptus_shared::rules::model::StatDef| StatView {
        name: s.name.clone(),
        abbr: s.abbr.clone(),
        formula: s.formula.to_string(),
    };
    let o = &system.outcomes;
    let outcome = |band: OutcomeBand, desc: &str, natural: &[u32], mult: Option<i32>| OutcomeView {
        band,
        name: band.name(system).to_string(),
        description: desc.to_string(),
        natural: natural.to_vec(),
        xp: band.xp(system),
        damage_multiplier: mult,
    };
    RulesView {
        name: system.name.clone(),
        version: system.version,
        check: CheckView {
            die: system.check.dice.to_string(),
            faces: system.check.dice.faces,
            advantage: system.check.advantage,
            modifier: system.modifier.to_string(),
        },
        abilities: system
            .abilities
            .iter()
            .map(|a| {
                let mods = me.as_ref().and_then(|c| {
                    ability_modifiers(system, c, &a.id, RollScope::Checks)
                        .ok()
                        .map(|(m, _, _)| sum(&m))
                });
                AbilityView {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    description: a.description.clone(),
                    score: me.as_ref().and_then(|c| c.score(&a.id)),
                    modifier: mods,
                    needs: mods
                        .map(|bonus| {
                            system
                                .difficulties
                                .iter()
                                .map(|d| NeedView {
                                    difficulty: d.id.clone(),
                                    from_face: from_face(system, bonus, d.value),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            })
            .collect(),
        difficulties: system
            .difficulties
            .iter()
            .map(|d| DifficultyView {
                id: d.id.clone(),
                name: d.name.clone(),
                value: d.value,
                description: d.description.clone(),
            })
            .collect(),
        outcomes: vec![
            outcome(
                OutcomeBand::CriticalFailure,
                &o.critical_failure.description,
                &o.critical_failure.natural,
                None,
            ),
            outcome(OutcomeBand::Failure, &o.failure.description, &[], None),
            outcome(OutcomeBand::Success, &o.success.description, &[], None),
            outcome(
                OutcomeBand::CriticalSuccess,
                &o.critical_success.description,
                &o.critical_success.natural,
                o.critical_success.damage_multiplier,
            ),
        ],
        attack: AttackView {
            ability: attack_ability_code(system.attack.ability),
            precision_applies: system.attack.precision == PrecisionRule::AddedToAttackRoll,
            armor_class: stat(&system.stats.armor_class),
        },
        hit_points: stat(&system.stats.hit_points),
        turns: system
            .turn_contexts
            .iter()
            .map(|c| TurnView {
                name: c.name.clone(),
                actions_per_turn: c.actions_per_turn,
                limits: c
                    .limits
                    .iter()
                    .map(|l| LimitView {
                        kind: kind_view(system, &l.kind).map_or_else(|| l.kind.clone(), |k| k.name),
                        max_per_turn: l.max_per_turn,
                    })
                    .collect(),
            })
            .collect(),
        action_kinds: system
            .action_kinds
            .iter()
            .filter_map(|k| kind_view(system, &k.id))
            .collect(),
        cooldown: cooldown_code(system.cooldowns.meaning),
        application_turn_counts: system.durations.application_turn_counts,
        conditions: system
            .conditions
            .iter()
            .map(|c| ConditionView {
                name: c.name.clone(),
                description: c.description.clone(),
                kind: c.kind,
            })
            .collect(),
        zero_hp: match &system.zero_hp {
            ZeroHpRule::KnockedOut {
                condition,
                out_after_turns,
                out_condition,
                ..
            } => ZeroHpView::KnockedOut {
                condition: condition_name(system, condition),
                out_after_turns: *out_after_turns,
                out_condition: condition_name(system, out_condition),
            },
            ZeroHpRule::DeathSaves {
                difficulty,
                successes,
                failures,
                ..
            } => ZeroHpView::DeathSaves {
                difficulty: *difficulty,
                successes: *successes,
                failures: *failures,
            },
        },
        progression: ProgressionView {
            upgrade_every_xp: system.progression.upgrade_every_xp,
            upgrade_points: system.progression.upgrade_points,
            levels: system
                .progression
                .levels
                .iter()
                .map(|l| LevelView {
                    level: l.level,
                    xp: l.xp,
                })
                .collect(),
        },
        combat: CombatView {
            move_kind: system
                .combat
                .move_kind
                .as_deref()
                .and_then(|k| kind_view(system, k)),
            flee: system.combat.flee.as_ref().and_then(|f| {
                Some(FleeView {
                    kind: kind_view(system, &f.kind)?,
                    ability: f
                        .ability
                        .as_deref()
                        .and_then(|a| system.ability(a))
                        .map(|a| a.name.clone()),
                })
            }),
            cover_half: system.combat.cover.half,
            cover_three_quarters: system.combat.cover.three_quarters,
            long_range_disadvantage: system.combat.long_range == LongRangeRule::Disadvantage,
            long_range_modifier: match system.combat.long_range {
                LongRangeRule::Modifier(n) => Some(n),
                LongRangeRule::Disadvantage => None,
            },
        },
        group_check: system
            .group_check
            .as_ref()
            .map(|g| group_code(g.succeeds_when)),
        mine: me.as_ref().and_then(|c| mine(system, c)),
        changes,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use promptus_shared::rules::check::ModifierSource;
    use promptus_shared::story::RuleSystemRef;

    use super::*;

    fn corsaires() -> &'static RuleSystem {
        crate::content::rule_system(&RuleSystemRef {
            id: "corsaires".into(),
            version: 1,
        })
        .unwrap()
    }

    fn bretteur(abilities: &[(&str, i32)]) -> CharacterSheet {
        CharacterSheet {
            class_id: Some("bretteur".into()),
            abilities: abilities
                .iter()
                .map(|(k, v)| ((*k).to_string(), *v))
                .collect::<BTreeMap<_, _>>(),
            ..CharacterSheet::default()
        }
    }

    #[test]
    fn what_a_player_needs_on_the_die_follows_their_own_scores() {
        let s = corsaires();
        // FOR 13 → +1: against Moyen (10) a 9 succeeds, an 8 does not.
        let page = project_rules(s, Some(&bretteur(&[("FOR", 13)])), None);
        let force = page.abilities.iter().find(|a| a.id == "FOR").unwrap();
        assert_eq!(force.modifier, Some(1));
        let need = |a: &AbilityView, d: &str| {
            a.needs
                .iter()
                .find(|n| n.difficulty == d)
                .unwrap()
                .from_face
        };
        assert_eq!(need(force, "moyen"), Some(9));
        // Très difficile (20) with +1: 19 + 1 = 20 is enough.
        assert_eq!(need(force, "tres_difficile"), Some(19));
        // INT 8 → -1: Très difficile only on a natural 20.
        let int = page.abilities.iter().find(|a| a.id == "INT").unwrap();
        assert_eq!(need(int, "tres_difficile"), Some(20));

        let page = project_rules(s, Some(&bretteur(&[("FOR", 17)])), None);
        let force = page.abilities.iter().find(|a| a.id == "FOR").unwrap();
        // +3 against Facile (5): everything but the natural 1.
        assert_eq!(need(force, "facile"), Some(2));
    }

    #[test]
    fn worked_rolls_land_in_each_band_with_the_engine_breakdown() {
        let page = project_rules(corsaires(), Some(&bretteur(&[])), None);
        let mine = page.mine.unwrap();
        assert_eq!(mine.example_ability, "Force");
        assert_eq!(mine.example_difficulty, "Moyen");
        let bands: Vec<_> = mine.examples.iter().map(|r| r.band).collect();
        assert_eq!(
            bands,
            [
                Some(OutcomeBand::CriticalFailure),
                Some(OutcomeBand::Failure),
                Some(OutcomeBand::Success),
                Some(OutcomeBand::CriticalSuccess),
            ]
        );
        // FOR 13 (+1) against 10: 8 + 1 = 9 fails, 9 + 1 = 10 succeeds.
        assert_eq!((mine.examples[1].natural, mine.examples[1].total), (8, 9));
        assert_eq!((mine.examples[2].natural, mine.examples[2].total), (9, 10));
        assert_eq!(
            mine.examples[2].modifiers[0].source,
            ModifierSource::Ability("FOR".into())
        );
    }

    #[test]
    fn each_card_says_its_cost_cooldown_and_what_its_attack_adds() {
        let page = project_rules(corsaires(), Some(&bretteur(&[])), None);
        let cards = page.mine.unwrap().cards;
        let estocade = cards.iter().find(|c| c.card.id == "estocade").unwrap();
        assert_eq!(estocade.cost, 1);
        assert_eq!(estocade.card.cooldown, 0);
        // The Corsaires add the first primary ability and no precision.
        let mods = estocade.attack_modifiers.as_ref().unwrap();
        assert_eq!(mods.len(), 1, "{mods:?}");
        assert_eq!(mods[0].source, ModifierSource::Ability("FOR".into()));
        assert_eq!(mods[0].value, 1);
        assert!(!page.attack.precision_applies);
        assert!(cards.iter().any(|c| c.card.cooldown > 0));
    }

    #[test]
    fn a_spectator_reads_the_rules_without_numbers_or_stat_blocks() {
        let page = project_rules(corsaires(), None, None);
        assert!(page.mine.is_none());
        assert!(page.abilities.iter().all(|a| a.needs.is_empty()));
        let json = serde_json::to_string(&page).unwrap();
        for a in &corsaires().adversaries {
            assert!(!json.contains(&a.name), "{} leaked", a.name);
        }
        for t in &corsaires().adversary_tiers {
            assert!(!json.contains(&t.name), "{} leaked", t.name);
        }
    }
}
