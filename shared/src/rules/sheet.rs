//! The explicit state the engine reads and returns: combatants (player
//! characters and adversaries) gathered in a scene. Pure data — the
//! server persists it, the engine never holds any of it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::formula::{Formula, FormulaEnv};
use super::model::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Party,
    Opposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Class(String),
    Adversary(String),
}

/// XP state of a player character.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    pub total_xp: u32,
    /// The bar that empties every `upgrade_every_xp`.
    pub bar: u32,
    pub upgrade_points: u32,
}

/// A condition on a combatant, with its effects resolved so the badge
/// and the engine read the same thing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveCondition {
    /// The system's condition id, or `None` for an action's inline effect.
    pub id: Option<String>,
    pub name: String,
    pub kind: ConditionKind,
    pub effects: Vec<ConditionEffect>,
    /// Bearer's turns left; `None` lasts until removed (KO).
    pub remaining: Option<u32>,
    /// Who put it there.
    pub source: String,
    /// Applied during the bearer's own turn and that turn does not count.
    pub skip_next_tick: bool,
    /// Bearer's turns spent under it (KO → out of the scene).
    pub turns_elapsed: u32,
}

impl ActiveCondition {
    pub fn has(&self, effect: &ConditionEffect) -> bool {
        self.effects.contains(effect)
    }
}

/// What the current turn has spent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnBudget {
    pub actions_left: u32,
    pub spent_by_kind: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Combatant {
    pub id: String,
    pub name: String,
    pub side: Side,
    pub origin: Origin,
    pub abilities: BTreeMap<String, i32>,
    pub hit_points: i32,
    /// Player characters only.
    pub progress: Option<Progress>,
    /// Actions learned in play, in the free slots.
    pub learned_actions: Vec<ActionDef>,
    pub inventory: BTreeMap<String, u32>,
    pub resources: BTreeMap<String, i32>,
    pub conditions: Vec<ActiveCondition>,
    /// Action id → cooldown counter (0 or absent = ready).
    pub cooldowns: BTreeMap<String, u32>,
    pub turn: TurnBudget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetError {
    UnknownClass(String),
    UnknownAdversary(String),
    UnknownAbility(String),
    NoUpgradePoint,
    NoFreeSlot { slots: u32 },
    DuplicateAction(String),
    Formula(String),
}

impl Combatant {
    /// A level-1 character of a class, at full hit points.
    pub fn from_class(
        system: &RuleSystem,
        id: &str,
        name: &str,
        class_id: &str,
    ) -> Result<Self, SheetError> {
        let class = system
            .class(class_id)
            .ok_or_else(|| SheetError::UnknownClass(class_id.into()))?;
        let mut c = Self {
            id: id.into(),
            name: name.into(),
            side: Side::Party,
            origin: Origin::Class(class_id.into()),
            abilities: class.abilities.clone(),
            hit_points: 0,
            progress: Some(Progress::default()),
            learned_actions: Vec::new(),
            inventory: class
                .items
                .iter()
                .map(|i| (i.item.clone(), i.qty))
                .collect(),
            resources: system
                .resources
                .iter()
                .map(|r| (r.id.clone(), r.start))
                .collect(),
            conditions: Vec::new(),
            cooldowns: BTreeMap::new(),
            turn: TurnBudget::default(),
        };
        c.hit_points = c.max_hit_points(system)?;
        Ok(c)
    }

    /// One adversary from a reference stat block.
    pub fn from_adversary(
        system: &RuleSystem,
        id: &str,
        name: &str,
        adversary_id: &str,
    ) -> Result<Self, SheetError> {
        let a = system
            .adversary(adversary_id)
            .ok_or_else(|| SheetError::UnknownAdversary(adversary_id.into()))?;
        Ok(Self {
            id: id.into(),
            name: name.into(),
            side: Side::Opposition,
            origin: Origin::Adversary(adversary_id.into()),
            abilities: a.abilities.clone(),
            hit_points: a.hit_points,
            progress: None,
            learned_actions: Vec::new(),
            inventory: BTreeMap::new(),
            resources: BTreeMap::new(),
            conditions: Vec::new(),
            cooldowns: BTreeMap::new(),
            turn: TurnBudget::default(),
        })
    }

    pub fn class<'s>(&self, system: &'s RuleSystem) -> Option<&'s ClassDef> {
        match &self.origin {
            Origin::Class(id) => system.class(id),
            Origin::Adversary(_) => None,
        }
    }

    /// Level from total XP; adversaries have none (`None`).
    pub fn level(&self, system: &RuleSystem) -> Option<u32> {
        self.progress
            .map(|p| super::progression::level_for(system, p.total_xp))
    }

    /// Score with the bonuses of current conditions.
    pub fn score(&self, ability: &str) -> Option<i32> {
        let base = *self.abilities.get(ability)?;
        let bonus: i32 = self
            .conditions
            .iter()
            .flat_map(|c| &c.effects)
            .filter_map(|e| match e {
                ConditionEffect::AbilityBonus {
                    ability: None,
                    value,
                } => Some(*value),
                ConditionEffect::AbilityBonus {
                    ability: Some(a),
                    value,
                } if a == ability => Some(*value),
                _ => None,
            })
            .sum();
        Some(base + bonus)
    }

    pub fn modifier(&self, system: &RuleSystem, ability: &str) -> Result<i32, SheetError> {
        let score = self
            .score(ability)
            .ok_or_else(|| SheetError::UnknownAbility(ability.into()))?;
        modifier_for(system, score)
    }

    fn eval(&self, system: &RuleSystem, f: &Formula) -> Result<i32, SheetError> {
        let env = SheetEnv {
            system,
            sheet: self,
        };
        f.eval(&env)
            .map(|v| v as i32)
            .map_err(|e| SheetError::Formula(e.0))
    }

    /// Armour class: an adversary's stated value, else the formula.
    pub fn armor_class(&self, system: &RuleSystem) -> Result<i32, SheetError> {
        match &self.origin {
            Origin::Adversary(id) => system
                .adversary(id)
                .map(|a| a.armor_class)
                .ok_or_else(|| SheetError::UnknownAdversary(id.clone())),
            Origin::Class(_) => self.eval(system, system.armor_class_formula(self.class(system))),
        }
    }

    pub fn max_hit_points(&self, system: &RuleSystem) -> Result<i32, SheetError> {
        match &self.origin {
            Origin::Adversary(id) => system
                .adversary(id)
                .map(|a| a.hit_points)
                .ok_or_else(|| SheetError::UnknownAdversary(id.clone())),
            Origin::Class(_) => self.eval(system, system.hit_points_formula(self.class(system))),
        }
    }

    pub fn initiative_bonus(&self, system: &RuleSystem) -> Result<i32, SheetError> {
        self.eval(system, &system.initiative.bonus)
    }

    /// The system's attack bonus at this combatant's level (adversaries
    /// count as level 1), with its name; `None` when the system has none.
    pub fn attack_bonus(&self, system: &RuleSystem) -> Result<Option<(String, i32)>, SheetError> {
        system
            .attack
            .bonus
            .as_ref()
            .map(|b| Ok((b.name.clone(), self.eval(system, &b.formula)?)))
            .transpose()
    }

    /// The traits this combatant carries: its class's or its stat
    /// block's (read through its origin, never stored on the sheet).
    pub fn traits<'s>(&self, system: &'s RuleSystem) -> &'s [String] {
        match &self.origin {
            Origin::Class(id) => system.class(id).map(|c| c.traits.as_slice()),
            Origin::Adversary(id) => system.adversary(id).map(|a| a.traits.as_slice()),
        }
        .unwrap_or(&[])
    }

    /// The class or stat block id this combatant comes from.
    pub fn origin_id(&self) -> &str {
        match &self.origin {
            Origin::Class(id) | Origin::Adversary(id) => id,
        }
    }

    /// Every action this combatant owns: class actions (locked or not),
    /// learned ones, or an adversary's.
    pub fn actions<'a>(&'a self, system: &'a RuleSystem) -> Vec<&'a ActionDef> {
        let base: Vec<&ActionDef> = match &self.origin {
            Origin::Class(id) => system
                .class(id)
                .map(|c| c.actions.iter().collect())
                .unwrap_or_default(),
            Origin::Adversary(id) => system
                .adversary(id)
                .map(|a| a.actions.iter().collect())
                .unwrap_or_default(),
        };
        base.into_iter().chain(&self.learned_actions).collect()
    }

    /// Fills a free slot with an action learned in play.
    pub fn learn_action(
        &mut self,
        system: &RuleSystem,
        action: ActionDef,
    ) -> Result<(), SheetError> {
        let slots = system.creation.free_action_slots;
        if self.learned_actions.len() as u32 >= slots {
            return Err(SheetError::NoFreeSlot { slots });
        }
        if self.actions(system).iter().any(|a| a.id == action.id) {
            return Err(SheetError::DuplicateAction(action.id));
        }
        self.learned_actions.push(action);
        Ok(())
    }

    pub fn condition_named(&self, id: &str) -> Option<&ActiveCondition> {
        self.conditions.iter().find(|c| c.id.as_deref() == Some(id))
    }

    pub fn effects(&self) -> impl Iterator<Item = (&ActiveCondition, &ConditionEffect)> {
        self.conditions
            .iter()
            .flat_map(|c| c.effects.iter().map(move |e| (c, e)))
    }

    /// The condition that keeps this combatant from acting, if any.
    pub fn incapacitated_by(&self) -> Option<&ActiveCondition> {
        self.conditions
            .iter()
            .find(|c| c.has(&ConditionEffect::SkipsTurn))
    }

    pub fn can_move(&self) -> bool {
        !self
            .conditions
            .iter()
            .any(|c| c.has(&ConditionEffect::CannotMove) || c.has(&ConditionEffect::SkipsTurn))
    }

    /// Percentage of normal movement left by conditions.
    pub fn movement_percent(&self) -> u32 {
        if !self.can_move() {
            return 0;
        }
        self.effects()
            .filter_map(|(_, e)| match e {
                ConditionEffect::MovementPercent(p) => Some(*p),
                _ => None,
            })
            .fold(100, |acc, p| acc * p / 100)
    }
}

/// The system's modifier formula applied to a score.
pub fn modifier_for(system: &RuleSystem, score: i32) -> Result<i32, SheetError> {
    struct ScoreEnv(i64);
    impl FormulaEnv for ScoreEnv {
        fn var(&self, name: &str) -> Option<i64> {
            (name == "score").then_some(self.0)
        }
        fn ability_mod(&self, _: &str) -> Option<i64> {
            None
        }
        fn ability_score(&self, _: &str) -> Option<i64> {
            None
        }
    }
    system
        .modifier
        .eval(&ScoreEnv(score as i64))
        .map(|v| v as i32)
        .map_err(|e| SheetError::Formula(e.0))
}

struct SheetEnv<'a> {
    system: &'a RuleSystem,
    sheet: &'a Combatant,
}

impl FormulaEnv for SheetEnv<'_> {
    fn var(&self, name: &str) -> Option<i64> {
        match name {
            "level" => Some(self.sheet.level(self.system).unwrap_or(1) as i64),
            _ => None,
        }
    }
    fn ability_mod(&self, ability: &str) -> Option<i64> {
        self.sheet
            .modifier(self.system, ability)
            .ok()
            .map(|v| v as i64)
    }
    fn ability_score(&self, ability: &str) -> Option<i64> {
        self.sheet.score(ability).map(|v| v as i64)
    }
}

/// Everyone in play, plus whose turn it is and which action economy
/// applies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub context: String,
    pub combatants: BTreeMap<String, Combatant>,
    /// Whose turn it is; actions are refused outside one's own turn.
    pub active: Option<String>,
}

impl Scene {
    pub fn new(context: &str, combatants: impl IntoIterator<Item = Combatant>) -> Self {
        Self {
            context: context.into(),
            combatants: combatants.into_iter().map(|c| (c.id.clone(), c)).collect(),
            active: None,
        }
    }

    pub fn get(&self, id: &str) -> Option<&Combatant> {
        self.combatants.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Combatant> {
        self.combatants.get_mut(id)
    }
}
