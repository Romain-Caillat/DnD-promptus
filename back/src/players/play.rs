//! gm/adjust-sheets-fast — a validated character's live play state
//! (migration `008_character_play.sql`), and the history of what changed
//! it.
//!
//! The sheet (`characters.sheet`) is what the player wrote and the GM
//! reviewed. What moves at the table is held here, by the server: XP
//! earned, hit points lost, the rules' resources (the Corsaires' gold),
//! the bag. The GM changes it in one gesture ([`adjust`]); the player
//! reads it (`campaigns::projection::project_play`) and only chooses
//! what they carry ([`equip`]). Each change is one row of the history
//! ([`history`]), and calls [`super::touch_character`] in its
//! transaction so both screens follow live.
//!
//! Nothing the rules derive is stored: maximum hit points, level, the
//! XP bar and upgrade points, armour class and the unlocked cards come
//! from the sheet and this state through the rules engine
//! ([`combatant`]).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::dice::{DiceSource, SeededDice};
use promptus_shared::rules::progression::{
    HitPointChoice, LevelHitPointsTaken, choose_level_hit_points, gain_xp,
};
use promptus_shared::rules::sheet::{Combatant, Progress, SheetError};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{CharacterSheet, CharacterStatus, Player, touch_character};
use crate::campaigns;
use crate::error::AppError;

/// Largest change one gesture may make, in points of anything.
pub const DELTA_MAX: i32 = 1000;
/// Most items given or taken in one gesture.
pub const QTY_MAX: u32 = 99;
/// Longest name and description of an item the GM names.
pub const ITEM_NAME_MAX: usize = 80;
pub const ITEM_TEXT_MAX: usize = 500;
/// How many history lines the GM's board shows.
pub const HISTORY_LIMIT: i64 = 60;

/// One line of the bag. Shown to its player whole: it holds nothing
/// GM-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryEntry {
    /// Stable within the bag: the item id for what the class starts
    /// with, a fresh UUID for anything given later.
    pub key: String,
    /// A rule system item, by id. `None` for an item the GM named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// The name of an item the GM named; empty for a rule system item,
    /// whose name comes from the rules.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub qty: u32,
    /// On the character rather than in the bag; the player's choice.
    #[serde(default)]
    pub equipped: bool,
}

impl InventoryEntry {
    /// The item's name as the player reads it.
    #[must_use]
    pub fn display_name(&self, rules: &RuleSystem) -> String {
        match &self.item {
            Some(id) => rules
                .item(id)
                .map_or_else(|| id.clone(), |i| i.name.clone()),
            None => self.name.clone(),
        }
    }
}

/// What the server holds about a character in play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayState {
    pub total_xp: u32,
    /// Hit points lost (`0..=max`).
    pub damage: i32,
    /// Amount by resource id; a missing id holds the rules' start.
    pub resources: BTreeMap<String, i32>,
    pub inventory: Vec<InventoryEntry>,
    /// Level → the hit die the player kept for it (`engine/level-up`).
    pub level_hit_dice: BTreeMap<u32, i32>,
    /// Ability id → upgrade points the player spent on it.
    pub upgrades: BTreeMap<String, i32>,
}

impl PlayState {
    /// Where a freshly validated character starts: unhurt, no XP, the
    /// rules' starting resources, the class's starting items.
    #[must_use]
    pub fn start(rules: &RuleSystem, sheet: &CharacterSheet) -> Self {
        let class = sheet.class_id.as_deref().and_then(|id| rules.class(id));
        Self {
            total_xp: 0,
            damage: 0,
            resources: BTreeMap::new(),
            inventory: class
                .map(|c| {
                    c.items
                        .iter()
                        .map(|i| InventoryEntry {
                            key: i.item.clone(),
                            item: Some(i.item.clone()),
                            name: String::new(),
                            description: String::new(),
                            qty: i.qty,
                            equipped: false,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            level_hit_dice: BTreeMap::new(),
            upgrades: BTreeMap::new(),
        }
    }

    /// The amount of resource `id`.
    #[must_use]
    pub fn resource(&self, rules: &RuleSystem, id: &str) -> i32 {
        self.resources.get(id).copied().unwrap_or_else(|| {
            rules
                .resources
                .iter()
                .find(|r| r.id == id)
                .map_or(0, |r| r.start)
        })
    }
}

/// The character as the rules engine sees it in play: the class with
/// the sheet's scores, the XP earned, the hit points left. `None` when
/// the sheet names no class of `rules`.
#[must_use]
pub fn combatant(
    rules: &RuleSystem,
    sheet: &CharacterSheet,
    state: &PlayState,
) -> Option<Combatant> {
    let mut c =
        Combatant::from_class(rules, "sheet", &sheet.name, sheet.class_id.as_deref()?).ok()?;
    for (id, score) in &sheet.abilities {
        if c.abilities.contains_key(id) {
            c.abilities.insert(id.clone(), *score);
        }
    }
    let mut spent = 0;
    for (id, points) in &state.upgrades {
        if let Some(score) = c.abilities.get_mut(id) {
            *score += points;
            spent += points;
        }
    }
    let mut progress = Progress::default();
    gain_xp(rules, &mut progress, state.total_xp);
    // XP taken back may leave points spent beyond what is earned: none
    // left, and the scores keep what was chosen.
    progress.upgrade_points = progress
        .upgrade_points
        .saturating_sub(u32::try_from(spent).unwrap_or(0));
    c.progress = Some(progress);
    c.level_hit_dice.clone_from(&state.level_hit_dice);
    let max = c.max_hit_points(rules).ok()?;
    c.hit_points = (max - state.damage).clamp(0, max.max(0));
    Some(c)
}

/// One gesture of the GM on a character.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Adjustment {
    /// XP earned (or taken back, to undo a mistake).
    Xp { delta: i32 },
    /// Hit points: negative hurts, positive heals, within `0..=max`.
    HitPoints { delta: i32 },
    /// A resource of the rules, by id (`or`).
    Resource { resource: String, delta: i32 },
    /// A rule system item by id, or one the GM names.
    #[serde(rename_all = "camelCase")]
    GiveItem {
        #[serde(default)]
        item: Option<String>,
        #[serde(default)]
        name: String,
        #[serde(default)]
        description: String,
        #[serde(default = "one")]
        qty: u32,
    },
    /// Take some or all of a bag line back.
    TakeItem {
        entry: String,
        #[serde(default = "one")]
        qty: u32,
    },
}

fn one() -> u32 {
    1
}

/// What a change did, as the history keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Xp,
    HitPoints,
    Resource,
    Item,
    Equip,
    /// A level's hit points taken: the maximum before and after.
    Level,
    /// An upgrade point spent: the ability's score before and after.
    Upgrade,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Xp => "xp",
            Self::HitPoints => "hit_points",
            Self::Resource => "resource",
            Self::Item => "item",
            Self::Equip => "equip",
            Self::Level => "level",
            Self::Upgrade => "upgrade",
        }
    }

    fn parse(s: &str) -> Result<Self, AppError> {
        Ok(match s {
            "xp" => Self::Xp,
            "hit_points" => Self::HitPoints,
            "resource" => Self::Resource,
            "item" => Self::Item,
            "equip" => Self::Equip,
            "level" => Self::Level,
            "upgrade" => Self::Upgrade,
            other => return Err(AppError::Internal(format!("unknown adjustment {other}"))),
        })
    }
}

/// One change: total XP, hit points left, an amount, a quantity, or
/// equipped (1) / in the bag (0), before and after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub kind: Kind,
    /// The resource's or the item's name.
    pub label: Option<String>,
    pub before: i32,
    pub after: i32,
}

fn check_delta(delta: i32) -> Result<(), AppError> {
    if delta == 0 || delta.abs() > DELTA_MAX {
        return Err(AppError::BadRequest("INVALID_DELTA"));
    }
    Ok(())
}

fn check_qty(qty: u32) -> Result<(), AppError> {
    if qty == 0 || qty > QTY_MAX {
        return Err(AppError::BadRequest("INVALID_QUANTITY"));
    }
    Ok(())
}

fn to_i32(n: u32) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// Apply `adjustment` to `state`. Returns what changed, or `None` when
/// nothing did (healing a character already whole).
///
/// # Errors
///
/// 400 `INVALID_DELTA`, `INVALID_QUANTITY`, `UNKNOWN_RESOURCE`,
/// `UNKNOWN_ITEM`, `INVALID_ITEM_NAME`, `TEXT_TOO_LONG`; 404
/// `NO_SUCH_ENTRY`; 409 `NOT_ENOUGH` when taking more than there is;
/// 409 `NO_PLAY_SHEET` when the sheet names no class of the rules.
pub fn apply(
    rules: &RuleSystem,
    sheet: &CharacterSheet,
    state: &mut PlayState,
    adjustment: Adjustment,
) -> Result<Option<Record>, AppError> {
    let record = match adjustment {
        Adjustment::Xp { delta } => {
            check_delta(delta)?;
            let before = state.total_xp;
            state.total_xp = before.saturating_add_signed(delta);
            Record {
                kind: Kind::Xp,
                label: None,
                before: to_i32(before),
                after: to_i32(state.total_xp),
            }
        }
        Adjustment::HitPoints { delta } => {
            check_delta(delta)?;
            let c = combatant(rules, sheet, state).ok_or(AppError::Conflict("NO_PLAY_SHEET"))?;
            let max = c
                .max_hit_points(rules)
                .map_err(|e| AppError::Internal(format!("max hit points: {e:?}")))?
                .max(0);
            let before = c.hit_points;
            let after = (before + delta).clamp(0, max);
            state.damage = max - after;
            Record {
                kind: Kind::HitPoints,
                label: None,
                before,
                after,
            }
        }
        Adjustment::Resource { resource, delta } => {
            check_delta(delta)?;
            let def = rules
                .resources
                .iter()
                .find(|r| r.id == resource)
                .ok_or(AppError::BadRequest("UNKNOWN_RESOURCE"))?;
            let before = state.resource(rules, &def.id);
            let after = before + delta;
            if after < 0 {
                return Err(AppError::Conflict("NOT_ENOUGH"));
            }
            state.resources.insert(def.id.clone(), after);
            Record {
                kind: Kind::Resource,
                label: Some(def.name.clone()),
                before,
                after,
            }
        }
        Adjustment::GiveItem {
            item,
            name,
            description,
            qty,
        } => {
            check_qty(qty)?;
            let entry = match item {
                Some(id) => {
                    let def = rules
                        .item(&id)
                        .ok_or(AppError::BadRequest("UNKNOWN_ITEM"))?;
                    InventoryEntry {
                        key: Uuid::new_v4().to_string(),
                        item: Some(def.id.clone()),
                        name: String::new(),
                        description: String::new(),
                        qty,
                        equipped: false,
                    }
                }
                None => {
                    let name = name.trim().to_string();
                    let description = description.trim().to_string();
                    if name.is_empty()
                        || name.chars().count() > ITEM_NAME_MAX
                        || name.chars().any(char::is_control)
                    {
                        return Err(AppError::BadRequest("INVALID_ITEM_NAME"));
                    }
                    if description.chars().count() > ITEM_TEXT_MAX {
                        return Err(AppError::BadRequest("TEXT_TOO_LONG"));
                    }
                    InventoryEntry {
                        key: Uuid::new_v4().to_string(),
                        item: None,
                        name,
                        description,
                        qty,
                        equipped: false,
                    }
                }
            };
            let label = entry.display_name(rules);
            // The same item stacks on its line.
            let existing = state.inventory.iter_mut().find(|e| {
                e.item == entry.item && e.name == entry.name && e.description == entry.description
            });
            let (before, after) = match existing {
                Some(e) => {
                    let before = e.qty;
                    e.qty = e.qty.saturating_add(qty);
                    (before, e.qty)
                }
                None => {
                    state.inventory.push(entry);
                    (0, qty)
                }
            };
            Record {
                kind: Kind::Item,
                label: Some(label),
                before: to_i32(before),
                after: to_i32(after),
            }
        }
        Adjustment::TakeItem { entry, qty } => {
            check_qty(qty)?;
            let at = state
                .inventory
                .iter()
                .position(|e| e.key == entry)
                .ok_or(AppError::NotFound("NO_SUCH_ENTRY"))?;
            let line = &mut state.inventory[at];
            if qty > line.qty {
                return Err(AppError::Conflict("NOT_ENOUGH"));
            }
            let label = line.display_name(rules);
            let before = line.qty;
            line.qty -= qty;
            let after = line.qty;
            if after == 0 {
                state.inventory.remove(at);
            }
            Record {
                kind: Kind::Item,
                label: Some(label),
                before: to_i32(before),
                after: to_i32(after),
            }
        }
    };
    Ok((record.before != record.after).then_some(record))
}

// --- Storage -------------------------------------------------------------------

type StateRow = (
    i32,
    i32,
    Json<BTreeMap<String, i32>>,
    Json<Vec<InventoryEntry>>,
    Json<BTreeMap<u32, i32>>,
    Json<BTreeMap<String, i32>>,
);

const STATE_COLUMNS: &str = "total_xp, damage, resources, inventory, level_hit_dice, upgrades";

fn state_from_row(
    (total_xp, damage, resources, inventory, level_hit_dice, upgrades): StateRow,
) -> PlayState {
    PlayState {
        total_xp: u32::try_from(total_xp).unwrap_or(0),
        damage,
        resources: resources.0,
        inventory: inventory.0,
        level_hit_dice: level_hit_dice.0,
        upgrades: upgrades.0,
    }
}

/// The stored play state of character `id`, if it was ever changed.
///
/// # Errors
///
/// Fails on a database error.
pub async fn stored(pool: &PgPool, id: Uuid) -> Result<Option<PlayState>, AppError> {
    let row: Option<StateRow> = sqlx::query_as(&format!(
        "SELECT {STATE_COLUMNS} FROM character_play WHERE character_id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(state_from_row))
}

async fn save(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    character: Uuid,
    state: &PlayState,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO character_play
           (character_id, campaign_id, total_xp, damage, resources, inventory, level_hit_dice, upgrades)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (character_id) DO UPDATE
           SET total_xp = EXCLUDED.total_xp, damage = EXCLUDED.damage,
               resources = EXCLUDED.resources, inventory = EXCLUDED.inventory,
               level_hit_dice = EXCLUDED.level_hit_dice, upgrades = EXCLUDED.upgrades",
    )
    .bind(character)
    .bind(campaign)
    .bind(to_i32(state.total_xp))
    .bind(state.damage)
    .bind(Json(&state.resources))
    .bind(Json(&state.inventory))
    .bind(Json(&state.level_hit_dice))
    .bind(Json(&state.upgrades))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Who made a change: the GM's gesture, the player's own choice, or the
/// rules applying a result the server resolved (XP of a check, damage
/// in a fight, loot the GM validated).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    Gm,
    Player,
    Rules,
}

async fn log(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    character: Uuid,
    actor: Actor,
    record: &Record,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO play_adjustments
           (campaign_id, character_id, actor, kind, label, before_value, after_value)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(campaign)
    .bind(character)
    .bind(match actor {
        Actor::Gm => "gm",
        Actor::Player => "player",
        Actor::Rules => "rules",
    })
    .bind(record.kind.as_str())
    .bind(&record.label)
    .bind(record.before)
    .bind(record.after)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Under the campaign lock (`MEMORY.md` §3: combatants are world
/// writes), the validated character `id` of `campaign` with its play
/// state, its row locked too.
async fn lock_in_play(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    id: Uuid,
    rules: &RuleSystem,
) -> Result<(CharacterSheet, PlayState), AppError> {
    campaigns::lock(tx, campaign)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    let row: Option<(String, Json<CharacterSheet>)> = sqlx::query_as(
        "SELECT status, sheet FROM characters WHERE id = $1 AND campaign_id = $2 FOR UPDATE",
    )
    .bind(id)
    .bind(campaign)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((status, sheet)) = row else {
        return Err(AppError::NotFound("NOT_FOUND"));
    };
    if CharacterStatus::parse(&status)? != CharacterStatus::Validated {
        return Err(AppError::Conflict("CHARACTER_NOT_VALIDATED"));
    }
    let sheet = sheet.0;
    let state: Option<StateRow> = sqlx::query_as(&format!(
        "SELECT {STATE_COLUMNS} FROM character_play WHERE character_id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    let state = state.map_or_else(|| PlayState::start(rules, &sheet), state_from_row);
    Ok((sheet, state))
}

/// The GM's gesture on character `id` of `campaign`: applied, stored,
/// logged, and both screens told. Returns the sheet and the new state.
/// The caller has checked the GM owns the campaign.
///
/// # Errors
///
/// 404 `NOT_FOUND` when no such character sits at this table; 409
/// `CHARACTER_NOT_VALIDATED` before the GM validated it; the errors of
/// [`apply`].
pub async fn adjust(
    pool: &PgPool,
    campaign: Uuid,
    rules: &RuleSystem,
    id: Uuid,
    adjustment: Adjustment,
) -> Result<(CharacterSheet, PlayState), AppError> {
    let mut tx = pool.begin().await?;
    let out = adjust_in(&mut tx, campaign, rules, id, adjustment, Actor::Gm).await?;
    tx.commit().await?;
    Ok(out)
}

/// [`adjust`] inside the caller's transaction, for changes the server
/// applies as part of a larger write (a check's XP, a fight's wounds,
/// validated loot). Takes the campaign lock if the caller has not yet.
///
/// # Errors
///
/// As [`adjust`].
pub async fn adjust_in(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    rules: &RuleSystem,
    id: Uuid,
    adjustment: Adjustment,
    actor: Actor,
) -> Result<(CharacterSheet, PlayState), AppError> {
    let (sheet, mut state) = lock_in_play(tx, campaign, id, rules).await?;
    if combatant(rules, &sheet, &state).is_none() {
        return Err(AppError::Conflict("NO_PLAY_SHEET"));
    }
    if let Some(record) = apply(rules, &sheet, &mut state, adjustment)? {
        save(tx, campaign, id, &state).await?;
        log(tx, campaign, id, actor, &record).await?;
        touch_character(tx, campaign, id).await?;
    }
    Ok((sheet, state))
}

/// The validated character `id` of `campaign` and its play state, read
/// under the campaign lock the caller holds.
///
/// # Errors
///
/// 404 `NOT_FOUND`; 409 `CHARACTER_NOT_VALIDATED`.
pub async fn in_play_locked(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    id: Uuid,
    rules: &RuleSystem,
) -> Result<(CharacterSheet, PlayState), AppError> {
    lock_in_play(tx, campaign, id, rules).await
}

/// The player puts bag line `entry` on their character, or back in the
/// bag. Logged like the GM's changes, the GM's board follows.
///
/// # Errors
///
/// 404 `NO_CHARACTER` for a spectator, `NO_SUCH_ENTRY`; 409
/// `CHARACTER_NOT_VALIDATED`.
pub async fn equip(
    pool: &PgPool,
    player: &Player,
    rules: &RuleSystem,
    entry: &str,
    equipped: bool,
) -> Result<(), AppError> {
    let id = own_character(pool, player).await?;
    let mut tx = pool.begin().await?;
    let (_, mut state) = lock_in_play(&mut tx, player.campaign_id, id, rules).await?;
    let line = state
        .inventory
        .iter_mut()
        .find(|e| e.key == entry)
        .ok_or(AppError::NotFound("NO_SUCH_ENTRY"))?;
    if line.equipped == equipped {
        return Ok(());
    }
    line.equipped = equipped;
    let record = Record {
        kind: Kind::Equip,
        label: Some(line.display_name(rules)),
        before: i32::from(!equipped),
        after: i32::from(equipped),
    };
    save(&mut tx, player.campaign_id, id, &state).await?;
    log(&mut tx, player.campaign_id, id, Actor::Player, &record).await?;
    touch_character(&mut tx, player.campaign_id, id).await?;
    tx.commit().await?;
    Ok(())
}

/// The character a player plays, by their player row.
async fn own_character(pool: &PgPool, player: &Player) -> Result<Uuid, AppError> {
    let id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM characters WHERE player_id = $1")
        .bind(player.id)
        .fetch_optional(pool)
        .await?;
    id.ok_or(AppError::NotFound("NO_CHARACTER"))
}

/// Takes the hit points of reached `level` (pure; the server rolls).
///
/// # Errors
///
/// 409 `NO_PLAY_SHEET`, `NO_LEVEL_HIT_POINTS` when the rules add none,
/// `LEVEL_NOT_REACHED`, `LEVEL_ALREADY_TAKEN`.
pub fn take_level(
    rules: &RuleSystem,
    sheet: &CharacterSheet,
    state: &mut PlayState,
    level: u32,
    choice: HitPointChoice,
    dice: &mut dyn DiceSource,
) -> Result<(Record, LevelHitPointsTaken), AppError> {
    let mut c = combatant(rules, sheet, state).ok_or(AppError::Conflict("NO_PLAY_SHEET"))?;
    let took =
        choose_level_hit_points(rules, &mut c, level, choice, dice).map_err(|e| match e {
            SheetError::NoLevelHitPoints => AppError::Conflict("NO_LEVEL_HIT_POINTS"),
            SheetError::LevelNotReached(_) => AppError::Conflict("LEVEL_NOT_REACHED"),
            SheetError::LevelHitPointsChosen(_) => AppError::Conflict("LEVEL_ALREADY_TAKEN"),
            other => AppError::Internal(format!("level hit points: {other:?}")),
        })?;
    state.level_hit_dice.insert(level, took.die);
    let record = Record {
        kind: Kind::Level,
        label: Some(level.to_string()),
        before: took.max_before,
        after: took.max_after,
    };
    Ok((record, took))
}

/// Spends one upgrade point on `ability` (pure).
///
/// # Errors
///
/// 400 `UNKNOWN_ABILITY`; 409 `NO_PLAY_SHEET`, `NO_UPGRADE_POINT`.
pub fn spend_upgrade(
    rules: &RuleSystem,
    sheet: &CharacterSheet,
    state: &mut PlayState,
    ability: &str,
) -> Result<Record, AppError> {
    let def = rules
        .ability(ability)
        .ok_or(AppError::BadRequest("UNKNOWN_ABILITY"))?;
    let c = combatant(rules, sheet, state).ok_or(AppError::Conflict("NO_PLAY_SHEET"))?;
    if c.progress.is_none_or(|p| p.upgrade_points == 0) {
        return Err(AppError::Conflict("NO_UPGRADE_POINT"));
    }
    let before = c.abilities.get(&def.id).copied().unwrap_or(0);
    *state.upgrades.entry(def.id.clone()).or_insert(0) += 1;
    Ok(Record {
        kind: Kind::Upgrade,
        label: Some(def.name.clone()),
        before,
        after: before + 1,
    })
}

/// The player takes a reached level's hit points on their own
/// character: rolled by the server, or the average. Logged like the
/// GM's changes; the GM's board follows.
///
/// # Errors
///
/// 404 `NO_CHARACTER`; 409 `CHARACTER_NOT_VALIDATED`; those of
/// [`take_level`].
pub async fn level_up(
    pool: &PgPool,
    player: &Player,
    rules: &RuleSystem,
    level: u32,
    choice: HitPointChoice,
) -> Result<LevelHitPointsTaken, AppError> {
    let id = own_character(pool, player).await?;
    let mut tx = pool.begin().await?;
    let (sheet, mut state) = lock_in_play(&mut tx, player.campaign_id, id, rules).await?;
    let (record, took) = take_level(
        rules,
        &sheet,
        &mut state,
        level,
        choice,
        &mut SeededDice::from_os(),
    )?;
    save(&mut tx, player.campaign_id, id, &state).await?;
    log(&mut tx, player.campaign_id, id, Actor::Player, &record).await?;
    touch_character(&mut tx, player.campaign_id, id).await?;
    tx.commit().await?;
    Ok(took)
}

/// The player spends an upgrade point on `ability`.
///
/// # Errors
///
/// 404 `NO_CHARACTER`; 409 `CHARACTER_NOT_VALIDATED`; those of
/// [`spend_upgrade`].
pub async fn upgrade(
    pool: &PgPool,
    player: &Player,
    rules: &RuleSystem,
    ability: &str,
) -> Result<(), AppError> {
    let id = own_character(pool, player).await?;
    let mut tx = pool.begin().await?;
    let (sheet, mut state) = lock_in_play(&mut tx, player.campaign_id, id, rules).await?;
    let record = spend_upgrade(rules, &sheet, &mut state, ability)?;
    save(&mut tx, player.campaign_id, id, &state).await?;
    log(&mut tx, player.campaign_id, id, Actor::Player, &record).await?;
    touch_character(&mut tx, player.campaign_id, id).await?;
    tx.commit().await?;
    Ok(())
}

/// One line of the history, as the GM reads it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: Uuid,
    pub character_id: Uuid,
    /// `gm`, `player` or `rules`.
    pub actor: String,
    pub kind: Kind,
    pub label: Option<String>,
    pub before: i32,
    pub after: i32,
    pub created_at: DateTime<Utc>,
}

/// The latest changes of `campaign`, newest first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn history(pool: &PgPool, campaign: Uuid) -> Result<Vec<HistoryEntry>, AppError> {
    type Row = (
        Uuid,
        Uuid,
        String,
        String,
        Option<String>,
        i32,
        i32,
        DateTime<Utc>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, character_id, actor, kind, label, before_value, after_value, created_at
         FROM play_adjustments WHERE campaign_id = $1
         ORDER BY created_at DESC, id LIMIT $2",
    )
    .bind(campaign)
    .bind(HISTORY_LIMIT)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(
            |(id, character_id, actor, kind, label, before, after, created_at)| {
                Ok(HistoryEntry {
                    id,
                    character_id,
                    actor,
                    kind: Kind::parse(&kind)?,
                    label,
                    before,
                    after,
                    created_at,
                })
            },
        )
        .collect()
}

/// A validated character of the table, with what it plays from.
#[derive(Debug, Clone)]
pub struct InPlay {
    pub id: Uuid,
    pub player_id: Uuid,
    pub nickname: String,
    pub sheet: CharacterSheet,
    /// `None` until first changed: the starting state.
    pub state: Option<PlayState>,
}

/// Every validated character of `campaign`, in order of arrival.
///
/// # Errors
///
/// Fails on a database error.
pub async fn in_play(pool: &PgPool, campaign: Uuid) -> Result<Vec<InPlay>, AppError> {
    type Row = (
        Uuid,
        Uuid,
        String,
        Json<CharacterSheet>,
        Option<i32>,
        Option<i32>,
        Option<Json<BTreeMap<String, i32>>>,
        Option<Json<Vec<InventoryEntry>>>,
        Option<Json<BTreeMap<u32, i32>>>,
        Option<Json<BTreeMap<String, i32>>>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT c.id, p.id, p.nickname, c.sheet,
                cp.total_xp, cp.damage, cp.resources, cp.inventory, cp.level_hit_dice, cp.upgrades
         FROM characters c
         JOIN players p ON p.id = c.player_id
         LEFT JOIN character_play cp ON cp.character_id = c.id
         WHERE c.campaign_id = $1 AND c.status = 'validated'
         ORDER BY p.created_at, p.id",
    )
    .bind(campaign)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                player_id,
                nickname,
                sheet,
                xp,
                damage,
                resources,
                inventory,
                hit_dice,
                upgrades,
            )| {
                let state = match (xp, damage, resources, inventory, hit_dice, upgrades) {
                    (
                        Some(xp),
                        Some(damage),
                        Some(resources),
                        Some(inventory),
                        Some(hit_dice),
                        Some(upgrades),
                    ) => Some(state_from_row((
                        xp, damage, resources, inventory, hit_dice, upgrades,
                    ))),
                    _ => None,
                };
                InPlay {
                    id,
                    player_id,
                    nickname,
                    sheet: sheet.0,
                    state,
                }
            },
        )
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use promptus_shared::story::RuleSystemRef;

    fn corsaires() -> &'static RuleSystem {
        crate::content::preset(&RuleSystemRef {
            id: "corsaires".into(),
            version: 1,
        })
        .unwrap()
    }

    fn bretteur() -> CharacterSheet {
        CharacterSheet {
            name: "Lyra".into(),
            class_id: Some("bretteur".into()),
            ..CharacterSheet::default()
        }
    }

    #[test]
    fn a_new_character_starts_whole_with_the_class_kit_and_the_rules_purse() {
        let r = corsaires();
        let s = PlayState::start(r, &bretteur());
        let c = combatant(r, &bretteur(), &s).unwrap();
        assert_eq!(c.hit_points, 10);
        assert_eq!(s.resource(r, "or"), 10);
        let kit: Vec<&str> = s.inventory.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(kit, ["sabre_d_abordage", "dague_de_ceinture"]);
    }

    #[test]
    fn hit_points_stay_between_zero_and_the_rules_maximum() {
        let (r, sheet) = (corsaires(), bretteur());
        let mut s = PlayState::start(r, &sheet);
        let hurt = apply(r, &sheet, &mut s, Adjustment::HitPoints { delta: -4 }).unwrap();
        assert_eq!(hurt.map(|h| (h.before, h.after)), Some((10, 6)));
        let down = apply(r, &sheet, &mut s, Adjustment::HitPoints { delta: -50 }).unwrap();
        assert_eq!(down.map(|h| h.after), Some(0));
        apply(r, &sheet, &mut s, Adjustment::HitPoints { delta: 3 }).unwrap();
        assert_eq!(combatant(r, &sheet, &s).unwrap().hit_points, 3);
        apply(r, &sheet, &mut s, Adjustment::HitPoints { delta: 99 }).unwrap();
        // Healing someone whole changes nothing: nothing to log.
        assert_eq!(
            apply(r, &sheet, &mut s, Adjustment::HitPoints { delta: 1 }).unwrap(),
            None
        );
        assert_eq!(s.damage, 0);
    }

    #[test]
    fn xp_makes_the_level_through_the_rules() {
        let (r, sheet) = (corsaires(), bretteur());
        let mut s = PlayState::start(r, &sheet);
        for _ in 0..5 {
            apply(r, &sheet, &mut s, Adjustment::Xp { delta: 1 }).unwrap();
        }
        let c = combatant(r, &sheet, &s).unwrap();
        assert_eq!(c.level(r), Some(2));
        assert_eq!(c.progress.unwrap().upgrade_points, 1);
        apply(r, &sheet, &mut s, Adjustment::Xp { delta: -9 }).unwrap();
        assert_eq!(s.total_xp, 0);
    }

    #[test]
    fn the_purse_and_the_bag_refuse_to_go_below_nothing() {
        let (r, sheet) = (corsaires(), bretteur());
        let mut s = PlayState::start(r, &sheet);
        let gold = |delta| Adjustment::Resource {
            resource: "or".into(),
            delta,
        };
        apply(r, &sheet, &mut s, gold(5)).unwrap();
        assert_eq!(s.resource(r, "or"), 15);
        assert!(matches!(
            apply(r, &sheet, &mut s, gold(-16)),
            Err(AppError::Conflict("NOT_ENOUGH"))
        ));
        let rum = || Adjustment::GiveItem {
            item: Some("fiole_de_rhum_fortifiant".into()),
            name: String::new(),
            description: String::new(),
            qty: 1,
        };
        apply(r, &sheet, &mut s, rum()).unwrap();
        let stacked = apply(r, &sheet, &mut s, rum()).unwrap().unwrap();
        assert_eq!((stacked.before, stacked.after), (1, 2));
        assert_eq!(stacked.label.as_deref(), Some("Fiole de rhum fortifiant"));
        let key = s.inventory.last().unwrap().key.clone();
        let take = |qty| Adjustment::TakeItem {
            entry: key.clone(),
            qty,
        };
        assert!(matches!(
            apply(r, &sheet, &mut s, take(3)),
            Err(AppError::Conflict("NOT_ENOUGH"))
        ));
        apply(r, &sheet, &mut s, take(2)).unwrap();
        assert!(s.inventory.iter().all(|e| e.key != key));
        assert!(matches!(
            apply(r, &sheet, &mut s, gold(0)),
            Err(AppError::BadRequest("INVALID_DELTA"))
        ));
    }

    #[test]
    fn the_gm_may_name_an_item_the_rules_do_not_list() {
        let (r, sheet) = (corsaires(), bretteur());
        let mut s = PlayState::start(r, &sheet);
        let given = apply(
            r,
            &sheet,
            &mut s,
            Adjustment::GiveItem {
                item: None,
                name: "  Lanterne du phare ".into(),
                description: "Elle ne s'éteint pas.".into(),
                qty: 1,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(given.label.as_deref(), Some("Lanterne du phare"));
        let unknown = Adjustment::GiveItem {
            item: Some("epee_laser".into()),
            name: String::new(),
            description: String::new(),
            qty: 1,
        };
        assert!(apply(r, &sheet, &mut s, unknown).is_err());
    }
}
