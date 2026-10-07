//! What the engine reports happened, in order — the source of the GM
//! journal line, the player toast and the TV moment.

use serde::{Deserialize, Serialize};

use super::check::RollBreakdown;
use super::model::ConditionKind;
use super::progression::ProgressEvent;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollPurpose {
    Attack,
    Contest,
    /// Resisting a condition.
    Save,
}

/// Damage, term by term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageBreakdown {
    /// The amount as written (`3`, `1d6+2`).
    pub amount: String,
    pub faces: Vec<u32>,
    pub rolled: i32,
    /// Situations and conditions of the attacker.
    pub bonus: i32,
    pub multiplier: i32,
    /// Conditions of the target (negative reduces).
    pub taken_modifier: i32,
    pub ignored: bool,
    pub total: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Roll {
        roller: String,
        against: Option<String>,
        purpose: RollPurpose,
        breakdown: RollBreakdown,
    },
    Missed {
        target: String,
    },
    Damaged {
        target: String,
        breakdown: DamageBreakdown,
        hp_before: i32,
        hp_after: i32,
    },
    Healed {
        target: String,
        amount: i32,
        hp_before: i32,
        hp_after: i32,
    },
    ConditionApplied {
        target: String,
        name: String,
        kind: ConditionKind,
        turns: Option<u32>,
    },
    ConditionResisted {
        target: String,
        name: String,
    },
    ConditionEnded {
        target: String,
        name: String,
    },
    KnockedOut {
        target: String,
    },
    Revived {
        target: String,
    },
    OutOfScene {
        target: String,
    },
    TurnLost {
        who: String,
        because: String,
    },
    CooldownStarted {
        who: String,
        action: String,
        counter: u32,
    },
    Progress {
        who: String,
        change: ProgressEvent,
    },
    ItemUsed {
        who: String,
        item: String,
        left: u32,
    },
    /// Something the engine does not compute: the GM decides.
    ForTheGm {
        text: String,
    },
    /// A formalised house rule fired on `target`; its effects follow.
    /// `shown`: players may read the rule (`house::PlayersSee::Rule`).
    HouseRule {
        rule: String,
        name: String,
        target: String,
        shown: bool,
    },
}
