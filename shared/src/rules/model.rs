//! The rule system as data: what a `content/rules/<id>/v<n>.yaml` file
//! deserializes into. Nothing here decides anything; `check`, `action`,
//! `conditions` and `progression` read these values. See
//! `docs/rules-format.md` for the format as the GM edits it.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::dice::DiceExpr;
use super::formula::Formula;

/// A complete, versioned rule system. A campaign points at one
/// `(id, version)`; a change ships as a new version.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSystem {
    pub id: String,
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Documents this version was transcribed from.
    #[serde(default)]
    pub sources: Vec<String>,
    pub abilities: Vec<AbilityDef>,
    /// Ability score → modifier, in terms of `score`.
    pub modifier: Formula,
    pub stats: Stats,
    pub check: CheckRule,
    pub difficulties: Vec<Difficulty>,
    pub outcomes: Outcomes,
    #[serde(default)]
    pub group_check: Option<GroupCheckRule>,
    pub attack: AttackRule,
    pub initiative: InitiativeRule,
    pub action_kinds: Vec<ActionKind>,
    pub turn_contexts: Vec<TurnContext>,
    pub cooldowns: CooldownRule,
    pub durations: DurationRule,
    pub progression: Progression,
    pub zero_hp: ZeroHpRule,
    pub creation: CreationRule,
    #[serde(default)]
    pub movement: Vec<MovementRule>,
    /// How a fight on a grid reads the rules: which action a move and a
    /// flight spend, what cover and long range do to an attack, how big
    /// a zone is. Every field has a default (`docs/rules-format.md`).
    #[serde(default)]
    pub combat: CombatRule,
    #[serde(default)]
    pub situations: Vec<Situation>,
    /// Labels a class or an adversary carries (`mort_vivant`, `chef`):
    /// what a house rule's trigger or exception can name.
    #[serde(default)]
    pub traits: Vec<TraitDef>,
    /// Kinds of damage an action's `damage` tag may name (`feu`).
    #[serde(default)]
    pub damage_types: Vec<DamageTypeDef>,
    #[serde(default)]
    pub resources: Vec<ResourceDef>,
    pub conditions: Vec<ConditionDef>,
    #[serde(default)]
    pub peoples: Vec<PeopleDef>,
    pub classes: Vec<ClassDef>,
    #[serde(default)]
    pub items: Vec<ItemDef>,
    #[serde(default)]
    pub adversary_tiers: Vec<AdversaryTier>,
    #[serde(default)]
    pub adversaries: Vec<AdversaryDef>,
    /// The GM's own rules, written in French (`campaign/edit-rule-system`).
    /// The co-GM reads them. One with a `formal` form is applied by the
    /// engine (`super::house`); one without is the GM's to apply at the
    /// table.
    #[serde(default)]
    pub house_rules: Vec<HouseRule>,
}

/// A house rule as the GM wrote it, and as the server applies it once
/// formalised (`engine/formalise-house-rules`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseRule {
    pub id: String,
    /// A short title (« Les morts-vivants craignent le feu »).
    pub name: String,
    /// The rule, as the GM would say it at the table.
    pub text: String,
    /// The rule as the engine judges it, once the GM validated it.
    #[serde(default)]
    pub formal: Option<super::house::FormalRule>,
}

impl HouseRule {
    /// Whether players may read this rule (its name and text). A
    /// formalised rule can show only its effect.
    pub fn shown_to_players(&self) -> bool {
        self.formal
            .as_ref()
            .is_none_or(|f| f.players == super::house::PlayersSee::Rule)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraitDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageTypeDef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbilityDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

/// Stats the engine itself relies on, each a formula over abilities and
/// `level`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub armor_class: StatDef,
    pub hit_points: StatDef,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatDef {
    pub name: String,
    pub abbr: String,
    pub formula: Formula,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckRule {
    /// A single die, as `1d20`: its natural face decides the critical bands.
    pub dice: DiceExpr,
    /// Whether the system knows advantage/disadvantage (roll twice, keep
    /// the best/worst). When false, nothing may grant it.
    pub advantage: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Difficulty {
    pub id: String,
    pub name: String,
    pub value: i32,
    #[serde(default)]
    pub description: String,
}

/// The four outcome bands of a roll.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcomes {
    pub critical_failure: NaturalBand,
    pub failure: Band,
    pub success: Band,
    pub critical_success: NaturalBand,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub grants: Grants,
}

/// A band reached by the natural face alone, whatever the target.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaturalBand {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Natural faces that land in this band (`[20]`, `[19, 20]`).
    pub natural: Vec<u32>,
    /// The rolls the natural faces decide (`all` when absent). D&D 5e
    /// gives a natural 20 or 1 its weight on attacks only: a check or a
    /// save is then judged by its total.
    #[serde(default = "every_roll")]
    pub rolls: RollScope,
    #[serde(default)]
    pub grants: Grants,
    /// Damage multiplier when this band is an attack (critical hit).
    #[serde(default)]
    pub damage_multiplier: Option<i32>,
}

fn every_roll() -> RollScope {
    RollScope::All
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grants {
    #[serde(default)]
    pub xp: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupCheckRule {
    pub succeeds_when: GroupThreshold,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupThreshold {
    AtLeastHalf,
    Majority,
    All,
    Any,
}

impl GroupThreshold {
    /// Successes a group of `n` needs.
    #[must_use]
    pub fn needed(self, n: usize) -> usize {
        match self {
            Self::AtLeastHalf => n.div_ceil(2),
            Self::Majority => n / 2 + 1,
            Self::All => n,
            Self::Any => 1,
        }
    }
}

/// How an attack roll is built.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttackRule {
    /// Which ability a class action adds when it does not name one.
    pub ability: AttackAbility,
    /// Whether an action's or condition's "precision" adds to the roll.
    pub precision: PrecisionRule,
    /// A bonus added to every attack roll, growing with `level` (D&D's
    /// proficiency bonus). Adversaries count as level 1.
    #[serde(default)]
    pub bonus: Option<AttackBonus>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttackBonus {
    /// What the roll breakdown calls it (« Maîtrise »).
    pub name: String,
    /// Over `level`.
    pub formula: Formula,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttackAbility {
    /// The first of the class's primary abilities.
    FirstPrimary,
    /// The best modifier among the class's primary abilities.
    BestPrimary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrecisionRule {
    AddedToAttackRoll,
    /// Precision values exist in the data but change no roll.
    NotApplied,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitiativeRule {
    pub dice: DiceExpr,
    pub bonus: Formula,
    pub ties: InitiativeTies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InitiativeTies {
    PartyFirst,
    Reroll,
}

/// A family of actions a turn spends ("Attaquer", "Se déplacer"…).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionKind {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Actions of the turn budget it consumes.
    pub cost: u32,
}

/// The action economy in one setting (ground fight, aboard a ship…).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnContext {
    pub id: String,
    pub name: String,
    pub actions_per_turn: u32,
    #[serde(default)]
    pub limits: Vec<KindLimit>,
    /// Documents this context sends the reader to.
    #[serde(default)]
    pub references: Vec<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindLimit {
    pub kind: String,
    pub max_per_turn: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooldownRule {
    pub meaning: CooldownMeaning,
    #[serde(default)]
    pub note: String,
}

/// What "cooldown N turns" means. With 0 there is no cooldown at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CooldownMeaning {
    /// Unusable for the rest of the turn and the user's next N turns
    /// (cooldown 1 = every other turn).
    SkipNextTurns,
    /// The turn of use counts as one of the N (cooldown 1 = not twice in
    /// the same turn, usable again next turn).
    TurnOfUseCounts,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurationRule {
    /// Durations count down at the end of the bearer's turns. When false,
    /// a condition applied during the bearer's own turn does not count
    /// that turn (a 1-turn self buff lasts until the end of the next one).
    pub application_turn_counts: bool,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progression {
    /// Every this many XP, the XP bar empties and grants upgrade points.
    pub upgrade_every_xp: u32,
    /// Ability points each upgrade grants (+1 to an ability of choice).
    pub upgrade_points: u32,
    /// Level thresholds on total XP, from level 1 at 0 XP.
    pub levels: Vec<LevelThreshold>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelThreshold {
    pub level: u32,
    pub xp: u32,
}

/// What happens to a combatant brought to 0 hit points.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case", deny_unknown_fields)]
pub enum ZeroHpRule {
    /// Knocked out; a heal brings them back; without one for N of their
    /// turns they are out of the scene.
    KnockedOut {
        condition: String,
        out_after_turns: u32,
        out_condition: String,
        #[serde(default)]
        note: String,
    },
    /// Death saves (engine/save-against-death plays them; modelled here).
    DeathSaves {
        condition: String,
        difficulty: i32,
        successes: u32,
        failures: u32,
        #[serde(default)]
        note: String,
    },
}

impl ZeroHpRule {
    pub fn condition(&self) -> &str {
        match self {
            Self::KnockedOut { condition, .. } | Self::DeathSaves { condition, .. } => condition,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationRule {
    pub abilities: AbilityAssignment,
    /// Action slots learned in play, on top of the class actions.
    pub free_action_slots: u32,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbilityAssignment {
    /// Scores come from the class; the player does not choose them.
    FromClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapScale {
    World,
    Place,
    Encounter,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementRule {
    pub scale: MapScale,
    /// Cells one move action covers; `None` when the system does not say.
    pub cells_per_move: Option<u32>,
    /// Path costs on the grid (diagonals, difficult terrain, climbing,
    /// swimming), as `maps::check_path` reads them; `None` when the
    /// system does not say (the maps then fall back to their defaults).
    #[serde(default)]
    pub grid: Option<crate::maps::MovementRules>,
    #[serde(default)]
    pub note: String,
}

/// Fighting on a grid (`combat/`): what the system says about moving,
/// fleeing, cover, long range and areas.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatRule {
    /// Action kind one move spends. `None`: one free move per turn.
    #[serde(default)]
    pub move_kind: Option<String>,
    /// How a combatant leaves a fight. `None`: fleeing spends every
    /// action left, with no roll.
    #[serde(default)]
    pub flee: Option<FleeRule>,
    /// Attack roll modifier per cover level of the target.
    #[serde(default)]
    pub cover: CoverRule,
    /// What attacking beyond an action's `range`, up to its
    /// `long_range`, does to the roll.
    #[serde(default)]
    pub long_range: LongRangeRule,
    /// Radius, in cells, of a `zone` area around the aimed cell.
    #[serde(default = "default_zone_radius")]
    pub zone_radius: u32,
    #[serde(default)]
    pub note: String,
}

fn default_zone_radius() -> u32 {
    1
}

impl Default for CombatRule {
    fn default() -> Self {
        Self {
            move_kind: None,
            flee: None,
            cover: CoverRule::default(),
            long_range: LongRangeRule::default(),
            zone_radius: default_zone_radius(),
            note: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleeRule {
    /// Action kind a flight spends (its cost: usually the whole turn).
    pub kind: String,
    /// Ability rolled when the GM says the flight is hard; the GM gives
    /// the difficulty when it happens.
    #[serde(default)]
    pub ability: Option<String>,
}

/// Added to attack rolls against a target behind cover (negative
/// values make it harder to hit). Total cover blocks sight: no attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverRule {
    pub half: i32,
    pub three_quarters: i32,
}

impl Default for CoverRule {
    /// The D&D 5e values (+2 and +5 to AC), as roll modifiers.
    fn default() -> Self {
        Self {
            half: -2,
            three_quarters: -5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LongRangeRule {
    /// Needs a system with advantage.
    Disadvantage,
    /// Added to the attack roll.
    Modifier(i32),
}

impl Default for LongRangeRule {
    fn default() -> Self {
        Self::Modifier(-2)
    }
}

/// A circumstance the GM declares when an action is played ("furtif",
/// "en hauteur"), which some actions reward.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Situation {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDef {
    pub id: String,
    pub name: String,
    pub abbr: String,
    pub start: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionKind {
    Boon,
    Bane,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub kind: ConditionKind,
    pub effects: Vec<ConditionEffect>,
    /// The effect drawn on the character while the condition lasts;
    /// none shows only the badge.
    #[serde(default)]
    pub visual: Option<crate::sprite::ConditionVisual>,
}

/// Which rolls a modifier touches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RollScope {
    All,
    Checks,
    Attacks,
    Saves,
}

impl RollScope {
    pub fn covers(self, roll: RollScope) -> bool {
        self == RollScope::All || self == roll
    }
}

/// What a condition does while it lasts. Anything the engine cannot
/// judge is a `narrative` effect: shown to the GM, never computed.
#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionEffect {
    /// No action this turn (stunned, knocked down, frightened, KO).
    SkipsTurn,
    CannotMove,
    /// Movement scaled to this percentage.
    MovementPercent(u32),
    RollModifier {
        rolls: RollScope,
        value: i32,
    },
    Advantage {
        rolls: RollScope,
    },
    Disadvantage {
        rolls: RollScope,
    },
    /// Attack rolls against the bearer.
    AdvantageAgainst,
    DisadvantageAgainst,
    /// Precision of the bearer's attacks.
    Precision(i32),
    /// Precision of attacks against the bearer.
    PrecisionAgainst(i32),
    DamageDealt(i32),
    /// Added to damage the bearer takes (negative reduces it).
    DamageTaken(i32),
    IgnoreDamage,
    /// Added to one ability score, or to all when `ability` is absent.
    AbilityBonus {
        #[serde(default)]
        ability: Option<String>,
        value: i32,
    },
    /// Damage at the end of each of the bearer's turns.
    DamagePerTurn(DiceExpr),
    /// When the bearer hits, the target also receives this.
    OnHit(Box<ApplySpec>),
    /// Ends as soon as the bearer is targeted by an attack.
    EndsWhenAttacked,
    Narrative(String),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeopleDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub primary_abilities: Vec<String>,
    #[serde(default)]
    pub secondary_abilities: Vec<String>,
    pub abilities: BTreeMap<String, i32>,
    /// This class's own hit point formula, in place of
    /// `stats.hit_points` (a hit die per class).
    #[serde(default)]
    pub hit_points: Option<Formula>,
    /// This class's own armour class formula, in place of
    /// `stats.armor_class` (the armour it starts in).
    #[serde(default)]
    pub armor_class: Option<Formula>,
    /// Ids of `traits` this class's characters carry.
    #[serde(default)]
    pub traits: Vec<String>,
    #[serde(default)]
    pub items: Vec<StartingItem>,
    pub actions: Vec<ActionDef>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartingItem {
    pub item: String,
    pub qty: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub price: Option<u32>,
    /// One use spends one of the stack.
    #[serde(default)]
    pub consumable: bool,
    /// What using the item does, as an action.
    #[serde(default)]
    pub action: Option<ActionDef>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdversaryTier {
    pub id: String,
    pub name: String,
    pub armor_class: i32,
}

/// A reference stat block. Its armour class and hit points are stated,
/// not derived: that is how the source writes them.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdversaryDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tier: Option<String>,
    /// Ids of `traits` (`mort_vivant`, `chef`).
    #[serde(default)]
    pub traits: Vec<String>,
    pub abilities: BTreeMap<String, i32>,
    pub armor_class: i32,
    pub hit_points: i32,
    pub actions: Vec<ActionDef>,
    #[serde(default)]
    pub tactics: String,
    #[serde(default)]
    pub note: String,
    /// Why this stat block departs from its tier or the armour-class
    /// formula on purpose; the lint then leaves those values alone.
    #[serde(default)]
    pub exception: Option<String>,
}

/// An action a combatant can play — a class attack, an adversary's
/// weapon, an item's use. One model for all three.
#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Id of an `action_kinds` entry: what the turn spends.
    pub kind: String,
    /// Level the character must reach; absent = always available.
    #[serde(default)]
    pub level: Option<u32>,
    pub target: Targeting,
    #[serde(default)]
    pub area: Option<AreaShape>,
    /// Reach in cells: how far the target (or a zone's centre, or a
    /// line's end) may be. Absent = 1, adjacent cells only.
    #[serde(default)]
    pub range: Option<u32>,
    /// Beyond `range` and up to this, the attack still goes with the
    /// system's `combat.long_range` penalty.
    #[serde(default)]
    pub long_range: Option<u32>,
    pub roll: RollSpec,
    /// Ability added to the attack roll; absent = the system's rule.
    #[serde(default)]
    pub ability: Option<String>,
    pub tags: Vec<Tag>,
}

impl ActionDef {
    pub fn cooldown(&self) -> u32 {
        self.tags
            .iter()
            .find_map(|t| match t {
                Tag::Cooldown(n) => Some(*n),
                _ => None,
            })
            .unwrap_or(0)
    }

    /// Reach in cells (1 when the action does not say).
    pub fn reach(&self) -> u32 {
        self.range.unwrap_or(1)
    }

    pub fn precision(&self) -> i32 {
        self.tags
            .iter()
            .filter_map(|t| match t {
                Tag::Precision(n) => Some(*n),
                _ => None,
            })
            .sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Targeting {
    #[serde(rename = "self")]
    Myself,
    Ally,
    AllyOrSelf,
    Enemy,
    /// One or more enemies in an area.
    Enemies,
    /// Every ally, the actor included.
    AllAllies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AreaShape {
    MeleeBurst,
    Zone,
    Line,
}

/// What decides whether the action lands, per target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RollSpec {
    /// Lands without a roll (heals, buffs).
    None,
    /// Check die + ability (+ precision if the system says so) vs AC.
    Attack,
    /// Hits without rolling.
    AutoHit,
    /// Hits as a critical without rolling.
    AutoCritical,
    /// Both roll; the actor wins ties.
    Contest { actor: String, target: String },
}

/// One typed tag of an action, as the action card shows it.
#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Tag {
    Damage(DamageTag),
    Heal(HealTag),
    Buff(ApplySpec),
    Control(ApplySpec),
    Precision(i32),
    Cooldown(u32),
    /// A bonus that only holds in a declared situation.
    Situational(SituationalBonus),
    /// The player picks one of these when playing the action.
    Choice(Vec<Tag>),
    /// What the engine does not compute; the GM adjudicates.
    Note(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct DamageTag {
    pub amount: DiceExpr,
    /// Id of a `damage_types` entry (`feu`), when the system types damage.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub damage_type: Option<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct HealTag {
    pub amount: DiceExpr,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SituationalBonus {
    pub situation: String,
    #[serde(default)]
    pub precision: i32,
    #[serde(default)]
    pub damage: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Recipient {
    /// The action's targets.
    Targets,
    #[serde(rename = "self")]
    Myself,
}

/// A condition put on someone: a named one from the system, or inline
/// effects named after the action.
#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplySpec {
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub effects: Vec<ConditionEffect>,
    /// Inline only: boon or bane (badge colour).
    #[serde(default)]
    pub kind: Option<ConditionKind>,
    pub turns: u32,
    pub to: Recipient,
    /// The recipient may resist with this roll.
    #[serde(default)]
    pub save: Option<SaveSpec>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SaveSpec {
    pub ability: String,
    /// Id of a named difficulty; absent = the GM sets it when it happens.
    #[serde(default)]
    pub difficulty: Option<String>,
}

impl RuleSystem {
    pub fn ability(&self, id: &str) -> Option<&AbilityDef> {
        self.abilities.iter().find(|a| a.id == id)
    }
    pub fn difficulty(&self, id: &str) -> Option<&Difficulty> {
        self.difficulties.iter().find(|d| d.id == id)
    }
    pub fn class(&self, id: &str) -> Option<&ClassDef> {
        self.classes.iter().find(|c| c.id == id)
    }
    pub fn adversary(&self, id: &str) -> Option<&AdversaryDef> {
        self.adversaries.iter().find(|a| a.id == id)
    }
    pub fn item(&self, id: &str) -> Option<&ItemDef> {
        self.items.iter().find(|i| i.id == id)
    }
    pub fn condition(&self, id: &str) -> Option<&ConditionDef> {
        self.conditions.iter().find(|c| c.id == id)
    }
    pub fn action_kind(&self, id: &str) -> Option<&ActionKind> {
        self.action_kinds.iter().find(|k| k.id == id)
    }
    pub fn turn_context(&self, id: &str) -> Option<&TurnContext> {
        self.turn_contexts.iter().find(|c| c.id == id)
    }
    pub fn situation(&self, id: &str) -> Option<&Situation> {
        self.situations.iter().find(|s| s.id == id)
    }
    pub fn trait_def(&self, id: &str) -> Option<&TraitDef> {
        self.traits.iter().find(|t| t.id == id)
    }
    pub fn damage_type(&self, id: &str) -> Option<&DamageTypeDef> {
        self.damage_types.iter().find(|t| t.id == id)
    }
    pub fn house_rule(&self, id: &str) -> Option<&HouseRule> {
        self.house_rules.iter().find(|h| h.id == id)
    }
    /// The hit point formula of a character of `class`: the class's
    /// own, else the system's.
    pub fn hit_points_formula<'a>(&'a self, class: Option<&'a ClassDef>) -> &'a Formula {
        class
            .and_then(|c| c.hit_points.as_ref())
            .unwrap_or(&self.stats.hit_points.formula)
    }
    /// The armour class formula of a character of `class`: the class's
    /// own, else the system's.
    pub fn armor_class_formula<'a>(&'a self, class: Option<&'a ClassDef>) -> &'a Formula {
        class
            .and_then(|c| c.armor_class.as_ref())
            .unwrap_or(&self.stats.armor_class.formula)
    }
    /// The movement rule of one map scale, if the system has one.
    pub fn movement(&self, scale: MapScale) -> Option<&MovementRule> {
        self.movement.iter().find(|m| m.scale == scale)
    }
    pub fn max_level(&self) -> u32 {
        self.progression
            .levels
            .iter()
            .map(|l| l.level)
            .max()
            .unwrap_or(1)
    }
}
