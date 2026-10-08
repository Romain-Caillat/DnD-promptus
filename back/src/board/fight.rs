//! gm/run-combat, player/fight-turn, copilot/propose-adversary-turns —
//! a fight at the table (`encounters`).
//!
//! The GM opens the encounter of a scene: the scene's map is shown, the
//! party stands where their tokens were (or on the party starts), the
//! opponents on their starts, and the engine rolls initiative
//! (`combat::fight::Fight`). Each command — move, act, use an item,
//! flee, end the turn — is checked and resolved by the engine, under the
//! campaign lock, and refused with the engine's reason otherwise. A
//! player commands their own character on its turn; the GM commands the
//! opposition, or asks the co-GM to propose a whole adversary turn
//! (`combat::run::Focus`, rolls already resolved) and validates it.
//!
//! The party's hit points follow the fight into their sheet in play
//! (`players::play`, logged by the rules); at the end the XP of their
//! rolls is granted and the scene's loot waits for the GM
//! ([`super::rewards`]).
//!
//! engine/save-against-death: under the `death_saves` rule a dying
//! character's turn is their save (`Command::DeathSave`), an ally next
//! to them may stabilise them (`Command::Stabilize`); when the dice
//! propose a death the GM confirms it (`GmCommand::ConfirmDeath`, the
//! character then falls: `players::fate::fall`) or decides another
//! outcome (`GmCommand::Spare`).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use promptus_shared::combat::fight::{Fight, FightEvent, Play, Standing};
use promptus_shared::combat::run::{Decision, Focus, Policy};
use promptus_shared::maps::{Cell, Map, Side as MapSide};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::conditions;
use promptus_shared::rules::dice::SeededDice;
use promptus_shared::rules::sheet::{Combatant, Side};
use promptus_shared::story::{Campaign, Node};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{Board, Token, TokenKind, character_token};
use crate::auth::guard::CurrentGm;
use crate::campaigns::CampaignRow;
use crate::error::AppError;
use crate::evening::knowledge::{self, JournalKind};
use crate::evening::session::{self, Status};
use crate::live::{self, Topic};
use crate::players::play::{self, Actor, Adjustment};
use crate::players::{Player, Role};

/// A line of the scene's loot, waiting for the GM to hand it out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LootLine {
    pub index: usize,
    pub item: Option<String>,
    /// The item's name, or the coins' (« 15 pièces d'or »).
    pub name: String,
    pub coins: Option<u32>,
    pub found: String,
    /// Found only by looking (the GM decides).
    pub hidden: bool,
    /// The character it went to.
    pub given_to: Option<Uuid>,
}

/// One command the co-GM proposes, as the GM reads it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProposedStep {
    Move {
        path: Vec<Cell>,
    },
    Act {
        action: String,
        targets: Vec<String>,
    },
    Flee,
    EndTurn,
}

/// A whole adversary turn, already played on a copy of the fight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub who: String,
    /// The fight version it was played from: accepting a stale one is
    /// refused.
    pub version: i32,
    pub steps: Vec<ProposedStep>,
    pub events: Vec<FightEvent>,
    pub fight: Fight,
}

/// An encounter as the server holds it.
#[derive(Debug, Clone)]
pub struct Encounter {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub node: String,
    pub live: bool,
    pub version: i32,
    pub fight: Fight,
    pub proposal: Option<Proposal>,
    pub loot: Vec<LootLine>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

type Row = (
    Uuid,
    Option<Uuid>,
    String,
    String,
    i32,
    Json<Fight>,
    Option<Json<Proposal>>,
    Json<Vec<LootLine>>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);

const COLUMNS: &str =
    "id, session_id, node, status, version, fight, proposal, loot, started_at, ended_at";

fn encounter(r: Row) -> Encounter {
    Encounter {
        id: r.0,
        session_id: r.1,
        node: r.2,
        live: r.3 == "live",
        version: r.4,
        fight: r.5.0,
        proposal: r.6.map(|p| p.0),
        loot: r.7.0,
        started_at: r.8,
        ended_at: r.9,
    }
}

/// The id of the live encounter of `campaign`, if one is being fought.
///
/// # Errors
///
/// A database error.
pub async fn live_id(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Option<Uuid>, AppError> {
    Ok(
        sqlx::query_scalar("SELECT id FROM encounters WHERE campaign_id = $1 AND status = 'live'")
            .bind(campaign)
            .fetch_optional(db)
            .await?,
    )
}

/// The live encounter of `campaign`, or the last one when none is live
/// (its loot may still wait for the GM).
///
/// # Errors
///
/// A database error.
pub async fn latest(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
) -> Result<Option<Encounter>, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM encounters WHERE campaign_id = $1
         ORDER BY (status = 'live') DESC, started_at DESC LIMIT 1"
    ))
    .bind(campaign)
    .fetch_optional(db)
    .await?;
    Ok(row.map(encounter))
}

/// The events of `encounter`, oldest first (the last `limit`).
///
/// # Errors
///
/// A database error.
pub async fn events(
    db: impl PgExecutor<'_>,
    encounter: Uuid,
    limit: i64,
) -> Result<Vec<FightEvent>, AppError> {
    let rows: Vec<Json<FightEvent>> = sqlx::query_scalar(
        "SELECT event FROM (SELECT seq, event FROM encounter_events WHERE encounter_id = $1
                            ORDER BY seq DESC LIMIT $2) e ORDER BY seq",
    )
    .bind(encounter)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|j| j.0).collect())
}

async fn locked_live(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
) -> Result<Encounter, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM encounters WHERE campaign_id = $1 AND status = 'live' FOR UPDATE"
    ))
    .bind(campaign)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(encounter).ok_or(AppError::Conflict("NO_FIGHT"))
}

/// The id a story's own stat block takes among the rules' adversaries.
fn stand_in_id(who: &str) -> String {
    format!("story:{who}")
}

/// The campaign's rules, plus a stand-in adversary for every NPC or
/// adversary of the story whose stat block is written inline rather
/// than taken from the rules (`stats.from_rules`): its abilities, armour
/// class, hit points, and one attack action per attack it lists.
///
/// # Errors
///
/// 409 `RULES_UNKNOWN`.
pub fn rules_of(row: &CampaignRow) -> Result<RuleSystem, AppError> {
    let base = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let mut rules = base.clone();
    let attack_kind = base
        .adversaries
        .iter()
        .flat_map(|a| a.actions.first())
        .map(|a| a.kind.clone())
        .next()
        .or_else(|| base.action_kinds.first().map(|k| k.id.clone()))
        .unwrap_or_default();
    let blocks = row
        .story
        .npcs
        .iter()
        .filter_map(|n| n.stats.as_ref().map(|s| (&n.id, &n.name, s)))
        .chain(
            row.story
                .adversaries
                .iter()
                .map(|a| (&a.id, &a.name, &a.stats)),
        );
    for (id, name, block) in blocks {
        let Some(hp) = block.hit_points.filter(|_| block.from_rules.is_none()) else {
            continue;
        };
        let actions: Vec<serde_json::Value> = block
            .attacks
            .iter()
            .enumerate()
            .map(|(i, a)| {
                serde_json::json!({
                    "id": format!("{id}-attaque-{}", i + 1),
                    "name": a.name,
                    "kind": attack_kind,
                    "target": "enemy",
                    "roll": "attack",
                    "tags": [{ "damage": { "amount": a.damage } }],
                })
            })
            .collect();
        let def = serde_json::json!({
            "id": stand_in_id(id),
            "name": name,
            "abilities": block.abilities,
            "armor_class": block.armor_class.unwrap_or(10),
            "hit_points": hp,
            "actions": actions,
        });
        let def = serde_json::from_value(def).map_err(|e| AppError::Invalid {
            code: "OPPONENT_WITHOUT_STATS",
            detail: format!("{id}: {e}"),
        })?;
        rules.adversaries.push(def);
    }
    Ok(rules)
}

/// The turn context fights are played in: the first the rules declare.
fn context(rules: &RuleSystem) -> Result<String, AppError> {
    rules
        .turn_contexts
        .first()
        .map(|c| c.id.clone())
        .ok_or(AppError::Conflict("RULES_HAVE_NO_FIGHT"))
}

fn loot_of(story: &Campaign, node: &Node, rules: &RuleSystem) -> Vec<LootLine> {
    let coin_name = rules
        .resources
        .first()
        .map_or_else(String::new, |r| r.name.to_lowercase());
    node.loot
        .iter()
        .enumerate()
        .map(|(index, l)| LootLine {
            index,
            item: l.item.clone(),
            name: match (&l.item, l.coins) {
                (Some(id), _) => story
                    .items
                    .iter()
                    .find(|i| &i.id == id)
                    .map_or_else(|| id.clone(), |i| i.name.clone()),
                (None, Some(n)) => format!("{n} {coin_name}"),
                (None, None) => l.found.clone(),
            },
            coins: l.coins,
            found: l.found.clone(),
            hidden: l.hidden,
            given_to: None,
        })
        .collect()
}

/// The opponents of `node`, built from the rules' stat blocks, each
/// with the cell it starts on.
fn opposition(
    story: &Campaign,
    node: &Node,
    rules: &RuleSystem,
    map: &Map,
    taken: &mut Vec<Cell>,
) -> Result<Vec<(Combatant, Cell)>, AppError> {
    let encounter = node
        .encounter
        .as_ref()
        .ok_or(AppError::BadRequest("NO_ENCOUNTER"))?;
    let foes: Vec<_> = map
        .starts
        .iter()
        .filter(|s| s.side == Some(MapSide::Foes))
        .collect();
    let mut out = Vec::new();
    for group in &encounter.opponents {
        let stand_in = || Some(stand_in_id(&group.who)).filter(|id| rules.adversary(id).is_some());
        let (name, block) = if let Some(n) = story.npc(&group.who) {
            (
                n.name.clone(),
                n.stats
                    .as_ref()
                    .and_then(|s| s.from_rules.clone())
                    .or_else(stand_in),
            )
        } else if let Some(a) = story.adversary(&group.who) {
            (a.name.clone(), a.stats.from_rules.clone().or_else(stand_in))
        } else {
            return Err(AppError::Invalid {
                code: "UNKNOWN_OPPONENT",
                detail: group.who.clone(),
            });
        };
        let block = block.ok_or_else(|| AppError::Invalid {
            code: "OPPONENT_WITHOUT_STATS",
            detail: group.who.clone(),
        })?;
        let count = group.count.max(1);
        for n in 1..=count {
            let (id, shown) = if count == 1 {
                (group.who.clone(), name.clone())
            } else {
                (format!("{}-{n}", group.who), format!("{name} {n}"))
            };
            let c = Combatant::from_adversary(rules, &id, &shown, &block).map_err(|e| {
                AppError::Invalid {
                    code: "OPPONENT_WITHOUT_STATS",
                    detail: format!("{}: {e:?}", group.who),
                }
            })?;
            let at = foes
                .iter()
                .filter(|s| s.entity.as_deref() == Some(group.who.as_str()))
                .chain(foes.iter())
                .map(|s| s.at)
                .find(|at| !taken.contains(at))
                .ok_or_else(|| AppError::Invalid {
                    code: "NO_START_LEFT",
                    detail: id.clone(),
                })?;
            taken.push(at);
            out.push((c, at));
        }
    }
    Ok(out)
}

/// Put the fight's positions back on the board's tokens.
fn sync_tokens(board: &mut Board, fight: &Fight) {
    board.tokens.retain(|t| {
        t.kind == TokenKind::Character
            || !fight.scene.combatants.contains_key(&t.id)
            || fight.positions.contains_key(&t.id)
    });
    for (id, at) in &fight.positions {
        if let Some(t) = board.tokens.iter_mut().find(|t| &t.id == id) {
            t.at = *at;
        } else if let Some(c) = fight.combatant(id) {
            board.tokens.push(Token {
                id: id.clone(),
                kind: match c.side {
                    Side::Party => TokenKind::Character,
                    Side::Opposition => TokenKind::Npc,
                },
                r#ref: id.clone(),
                name: c.name.clone(),
                at: *at,
                hidden: false,
                invisible: false,
            });
        }
    }
    // The fallen and the fled leave the board; so does a dead character.
    board.tokens.retain(|t| {
        (t.kind == TokenKind::Character || fight.positions.contains_key(&t.id))
            && fight.standing.get(&t.id) != Some(&Standing::Dead)
    });
    board.reveal_from_party();
}

async fn append(
    tx: &mut Transaction<'_, Postgres>,
    encounter: Uuid,
    events: &[FightEvent],
) -> Result<(), AppError> {
    for e in events {
        sqlx::query(
            "INSERT INTO encounter_events (encounter_id, seq, event)
             VALUES ($1, COALESCE((SELECT MAX(seq) FROM encounter_events WHERE encounter_id = $1), 0) + 1, $2)",
        )
        .bind(encounter)
        .bind(Json(e))
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// The character behind a party combatant id.
pub(super) fn character_of_combatant(id: &str) -> Option<Uuid> {
    id.strip_prefix("pc-").and_then(|s| Uuid::parse_str(s).ok())
}

/// Keep each character's hit points in play equal to the fight's.
async fn sync_hit_points(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    rules: &RuleSystem,
    fight: &Fight,
) -> Result<(), AppError> {
    for (id, c) in &fight.scene.combatants {
        let Some(character) = character_of_combatant(id) else {
            continue;
        };
        // A dead character is out of play: nothing of theirs moves.
        if fight.standing.get(id) == Some(&Standing::Dead) {
            continue;
        }
        let (sheet, state) = play::in_play_locked(tx, campaign, character, rules).await?;
        let Some(now) = play::combatant(rules, &sheet, &state) else {
            continue;
        };
        let delta = c.hit_points - now.hit_points;
        if delta != 0 {
            play::adjust_in(
                tx,
                campaign,
                rules,
                character,
                Adjustment::HitPoints { delta },
                Actor::Rules,
            )
            .await?;
        }
    }
    Ok(())
}

/// Save the fight after a command: tokens, events, hit points, and the
/// end when it came.
async fn commit_step(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
    rules: &RuleSystem,
    enc: &Encounter,
    fight: &Fight,
    events: &[FightEvent],
) -> Result<(), AppError> {
    let campaign = row.id;
    append(tx, enc.id, events).await?;
    sync_hit_points(tx, campaign, rules, fight).await?;
    let mut board = super::current(&mut **tx, campaign)
        .await?
        .ok_or(AppError::Conflict("NO_MAP_SHOWN"))?;
    sync_tokens(&mut board, fight);
    super::save(tx, campaign, &board).await?;
    let ended = fight.is_over();
    sqlx::query(
        "UPDATE encounters SET fight = $2, version = version + 1, proposal = NULL,
                status = CASE WHEN $3 THEN 'ended' ELSE status END,
                ended_at = CASE WHEN $3 THEN now() ELSE ended_at END
         WHERE id = $1",
    )
    .bind(enc.id)
    .bind(Json(fight))
    .bind(ended)
    .execute(&mut **tx)
    .await?;
    if let (true, Some(end)) = (ended, &fight.end) {
        for (id, xp) in &end.xp {
            if end.dead.contains(id) {
                continue;
            }
            if let (Some(character), true) = (character_of_combatant(id), *xp > 0) {
                play::adjust_in(
                    tx,
                    campaign,
                    rules,
                    character,
                    Adjustment::Xp {
                        delta: i32::try_from(*xp).unwrap_or(i32::MAX),
                    },
                    Actor::Rules,
                )
                .await?;
            }
        }
        let won = end.winner == Some(Side::Party);
        let node = row.story.node(&enc.node);
        let text = match (won, node) {
            (true, Some(n)) if !n.encounter.as_ref().is_none_or(|e| e.on_victory.is_empty()) => n
                .encounter
                .as_ref()
                .map(|e| e.on_victory.clone())
                .unwrap_or_default(),
            (false, Some(n)) if !n.encounter.as_ref().is_none_or(|e| e.on_defeat.is_empty()) => n
                .encounter
                .as_ref()
                .map(|e| e.on_defeat.clone())
                .unwrap_or_default(),
            _ => node.map(|n| n.title.clone()).unwrap_or_default(),
        };
        // The story's outcome text is the GM's: the table gets a plain line.
        knowledge::write(
            tx,
            campaign,
            enc.session_id,
            JournalKind::Note,
            Some(&enc.node),
            &text,
            false,
        )
        .await?;
        knowledge::write(
            tx,
            campaign,
            enc.session_id,
            JournalKind::Fight,
            Some(&enc.node),
            if won {
                "Combat gagné"
            } else {
                "Combat terminé"
            },
            true,
        )
        .await?;
        crate::evening::touch(tx, campaign).await?;
        // A fight on the deck of a boarding hands back to its battle.
        super::battle::deck_fight_over(tx, row, rules, enc.id, end.winner).await?;
    }
    live::touch(tx, campaign, &Topic::Fight).await?;
    Ok(())
}

/// Open the encounter of scene `node`.
///
/// # Errors
///
/// 404; 409 `SESSION_NOT_LIVE`, `FIGHT_IN_PROGRESS`, `RULES_UNKNOWN`,
/// `NO_PARTY`; 400 `UNKNOWN_NODE`, `NO_ENCOUNTER`, `NO_MAP`,
/// `UNKNOWN_OPPONENT`, `OPPONENT_WITHOUT_STATS`, `NO_START_LEFT`.
pub async fn start(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    node_id: &str,
) -> Result<Encounter, AppError> {
    let mut tx = pool.begin().await?;
    let (row, live) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    if live_id(&mut *tx, campaign).await?.is_some()
        || super::battle::live_id(&mut *tx, campaign).await?.is_some()
    {
        return Err(AppError::Conflict("FIGHT_IN_PROGRESS"));
    }
    if row
        .story
        .node(node_id)
        .and_then(|n| n.encounter.as_ref())
        .is_some_and(|e| e.vehicles.is_some())
    {
        return Err(AppError::BadRequest("BATTLE_SCENE"));
    }
    let enc = open_in(&mut tx, &row, live.id, node_id).await?;
    tx.commit().await?;
    Ok(enc)
}

/// Opens the encounter of scene `node_id` inside the caller's
/// transaction, the campaign already locked: a GM's start, or a ship
/// battle's boarding (`super::battle`).
///
/// # Errors
///
/// As [`start`].
pub async fn open_in(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
    session: Uuid,
    node_id: &str,
) -> Result<Encounter, AppError> {
    let campaign = row.id;
    let rules = &rules_of(row)?;
    let node = row
        .story
        .node(node_id)
        .ok_or(AppError::BadRequest("UNKNOWN_NODE"))?;
    // The scene's own map, else the one shown at the table.
    let before = super::current(&mut **tx, campaign).await?;
    let map_id = node
        .map
        .clone()
        .or_else(|| before.as_ref().map(|b| b.map_id.clone()))
        .ok_or(AppError::BadRequest("NO_MAP"))?;
    let map_id = map_id.as_str();
    let map = match before.as_ref().filter(|b| b.map_id == map_id) {
        Some(b) => b.map.clone(),
        None => crate::campaign_maps::playable(&mut **tx, campaign, &row.story, map_id)
            .await?
            .ok_or(AppError::BadRequest("NO_MAP"))?,
    };
    let kept: Vec<Token> = before
        .as_ref()
        .filter(|b| b.map_id == map_id)
        .map(|b| b.tokens.clone())
        .unwrap_or_default();
    let party = super::party_tokens(tx, campaign, &map, &kept).await?;
    if party.is_empty() {
        return Err(AppError::Conflict("NO_PARTY"));
    }
    let mut placements = Vec::new();
    let mut taken: Vec<Cell> = Vec::new();
    for t in &party {
        let character =
            Uuid::parse_str(&t.r#ref).map_err(|e| AppError::internal("token ref", e))?;
        let (sheet, state) = play::in_play_locked(tx, campaign, character, rules).await?;
        let Some(mut c) = play::combatant(rules, &sheet, &state) else {
            continue;
        };
        c.id = t.id.clone();
        c.name = t.name.clone();
        c.side = Side::Party;
        taken.push(t.at);
        placements.push((c, t.at));
    }
    placements.extend(opposition(&row.story, node, rules, &map, &mut taken)?);
    let mut dice = SeededDice::from_os();
    let step =
        Fight::start(rules, &context(rules)?, map.clone(), placements, &mut dice).map_err(|e| {
            AppError::Invalid {
                code: "FIGHT_SETUP",
                detail: format!("{e:?}"),
            }
        })?;
    let mut board = before.filter(|b| b.map_id == map_id).unwrap_or(Board {
        map_id: map_id.to_string(),
        map,
        fog: true,
        revealed: Default::default(),
        tokens: Vec::new(),
    });
    board.tokens = party;
    sync_tokens(&mut board, &step.fight);
    super::save(tx, campaign, &board).await?;
    let stored: Row = sqlx::query_as(&format!(
        "INSERT INTO encounters (campaign_id, session_id, node, fight, loot)
         VALUES ($1, $2, $3, $4, $5) RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(session)
    .bind(node_id)
    .bind(Json(&step.fight))
    .bind(Json(loot_of(&row.story, node, rules)))
    .fetch_one(&mut **tx)
    .await?;
    let enc = encounter(stored);
    append(tx, enc.id, &step.events).await?;
    knowledge::write(
        tx,
        campaign,
        Some(session),
        JournalKind::Fight,
        Some(node_id),
        &node.title,
        true,
    )
    .await?;
    crate::evening::touch(tx, campaign).await?;
    live::touch(tx, campaign, &Topic::Fight).await?;
    Ok(enc)
}

/// One command for the active combatant.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Command {
    /// The cells entered, in order.
    Move {
        path: Vec<Cell>,
    },
    /// A card of the combatant: on targets, at a cell, or alone.
    Act {
        action: String,
        #[serde(default)]
        targets: Vec<String>,
        #[serde(default)]
        cell: Option<Cell>,
        #[serde(default)]
        situations: Vec<String>,
        #[serde(default)]
        choice: Option<usize>,
    },
    /// A carried item, on targets.
    Item {
        item: String,
        #[serde(default)]
        targets: Vec<String>,
    },
    Flee,
    EndTurn,
    /// Dying: roll against death (the whole turn).
    DeathSave,
    /// Try to stabilise a dying ally next to the combatant.
    Stabilize {
        target: String,
    },
}

/// The save difficulty the GM gives when the rules leave it open
/// (INTERPRETATION, the engine's policies use the same).
const GM_SAVE_DIFFICULTY: i32 = 10;

fn play_of(who: &str, cmd: &Command) -> Option<Play> {
    match cmd {
        Command::Act {
            action,
            targets,
            cell,
            situations,
            choice,
        } => {
            let mut p = match (cell, targets.is_empty()) {
                (Some(at), _) => Play::at(who, action, *at),
                (None, true) => Play::alone(who, action),
                (None, false) => {
                    let t: Vec<&str> = targets.iter().map(String::as_str).collect();
                    Play::on(who, action, &t)
                }
            };
            p.situations.clone_from(situations);
            p.choice = *choice;
            p.save_difficulty = Some(GM_SAVE_DIFFICULTY);
            Some(p)
        }
        Command::Item { item, targets } => {
            let t: Vec<&str> = targets.iter().map(String::as_str).collect();
            let mut p = Play::item(who, item, &t);
            p.save_difficulty = Some(GM_SAVE_DIFFICULTY);
            Some(p)
        }
        _ => None,
    }
}

fn refused(r: &impl Serialize) -> AppError {
    AppError::Refused {
        code: "REFUSED",
        refusal: serde_json::to_value(r).unwrap_or_default(),
    }
}

fn run(
    rules: &RuleSystem,
    fight: &Fight,
    who: &str,
    cmd: &Command,
) -> Result<(Fight, Vec<FightEvent>), AppError> {
    let mut dice = SeededDice::from_os();
    let step = match cmd {
        Command::Move { path } => fight.move_along(rules, who, path),
        Command::Act { .. } | Command::Item { .. } => {
            let play = play_of(who, cmd).ok_or(AppError::BadRequest("INVALID_COMMAND"))?;
            fight.act(rules, &play, &mut dice)
        }
        Command::Flee => fight.flee(rules, who, None, &mut dice),
        Command::EndTurn => fight.end_turn(rules, who, &mut dice),
        Command::DeathSave => fight.death_save(rules, who, &mut dice),
        Command::Stabilize { target } => fight.stabilize(rules, who, target, &mut dice),
    }
    .map_err(|r| refused(&r))?;
    Ok((step.fight, step.events))
}

/// player/fight-turn: `player` commands their character on its turn.
///
/// # Errors
///
/// 403 `SPECTATOR`; 409 `NO_FIGHT`, `NOT_YOUR_TURN`, `REFUSED` (with the
/// engine's reason), `SESSION_NOT_LIVE`.
pub async fn command(pool: &PgPool, player: &Player, cmd: &Command) -> Result<(), AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let character: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM characters WHERE player_id = $1 AND status = 'validated'",
    )
    .bind(player.id)
    .fetch_optional(pool)
    .await?;
    let who = character_token(character.ok_or(AppError::Conflict("NOT_YOUR_TURN"))?);
    let mut tx = pool.begin().await?;
    let (row, live) = session::lock_for_player(&mut tx, player, &[Status::Live]).await?;
    let enc = locked_live(&mut tx, player.campaign_id).await?;
    if enc.fight.active() != Some(who.as_str()) {
        return Err(AppError::Conflict("NOT_YOUR_TURN"));
    }
    let rules = &rules_of(&row)?;
    let (fight, events) = run(rules, &enc.fight, &who, cmd)?;
    commit_step(&mut tx, &row, rules, &enc, &fight, &events).await?;
    crate::evening::moment(
        &mut tx,
        live.id,
        player.id,
        crate::evening::MomentKind::Fight,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// What the GM does in a fight.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum GmCommand {
    /// A command for the active adversary.
    Adversary { command: Command },
    /// The co-GM plays the active adversary's whole turn on a copy.
    Propose,
    /// The proposal becomes the fight.
    Accept,
    /// Put a condition of the rules on a combatant, or take it off.
    Condition {
        who: String,
        condition: String,
        #[serde(default)]
        turns: Option<u32>,
        remove: bool,
    },
    /// End the fight now.
    Stop,
    /// The character `who` (a combatant id) dies: the death the dice
    /// proposed, or the GM's own decision for someone down at 0.
    ConfirmDeath { who: String },
    /// Another outcome than the death the dice proposed: `who` is stable.
    Spare { who: String },
}

/// Apply `cmd` of the GM to the live fight.
///
/// # Errors
///
/// 404; 409 `NO_FIGHT`, `NOT_AN_ADVERSARY_TURN`, `NO_PROPOSAL`,
/// `PROPOSAL_STALE`, `REFUSED`; 400 `UNKNOWN_CONDITION`.
pub async fn gm_command(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    cmd: &GmCommand,
) -> Result<Option<Proposal>, AppError> {
    let mut tx = pool.begin().await?;
    let (row, _) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    let enc = locked_live(&mut tx, campaign).await?;
    let rules = &rules_of(&row)?;
    let adversary_turn = || {
        enc.fight
            .active()
            .filter(|w| {
                enc.fight
                    .combatant(w)
                    .is_some_and(|c| c.side == Side::Opposition)
            })
            .map(str::to_string)
            .ok_or(AppError::Conflict("NOT_AN_ADVERSARY_TURN"))
    };
    let mut proposal = None;
    match cmd {
        GmCommand::Adversary { command } => {
            let who = adversary_turn()?;
            let (fight, events) = run(rules, &enc.fight, &who, command)?;
            commit_step(&mut tx, &row, rules, &enc, &fight, &events).await?;
        }
        GmCommand::Propose => {
            let who = adversary_turn()?;
            let p = propose(rules, &enc.fight, &who, enc.version);
            sqlx::query("UPDATE encounters SET proposal = $2 WHERE id = $1")
                .bind(enc.id)
                .bind(Json(&p))
                .execute(&mut *tx)
                .await?;
            live::touch(&mut tx, campaign, &Topic::Desk).await?;
            proposal = Some(p);
        }
        GmCommand::Accept => {
            let p = enc
                .proposal
                .clone()
                .ok_or(AppError::Conflict("NO_PROPOSAL"))?;
            if p.version != enc.version {
                return Err(AppError::Conflict("PROPOSAL_STALE"));
            }
            commit_step(&mut tx, &row, rules, &enc, &p.fight, &p.events).await?;
        }
        GmCommand::Condition {
            who,
            condition,
            turns,
            remove,
        } => {
            let (scene, events) = if *remove {
                conditions::remove_condition(&enc.fight.scene, who, condition)
            } else {
                conditions::named(rules, condition, *turns, "gm")
                    .and_then(|c| conditions::apply_condition(rules, &enc.fight.scene, who, c))
            }
            .map_err(|_| AppError::BadRequest("UNKNOWN_CONDITION"))?;
            let mut fight = enc.fight.clone();
            fight.scene = scene;
            let events: Vec<FightEvent> = events
                .into_iter()
                .map(|event| FightEvent::Rules { event })
                .collect();
            commit_step(&mut tx, &row, rules, &enc, &fight, &events).await?;
        }
        GmCommand::Stop => {
            let step = enc.fight.stop();
            commit_step(&mut tx, &row, rules, &enc, &step.fight, &step.events).await?;
        }
        GmCommand::ConfirmDeath { who } => {
            let character =
                character_of_combatant(who).ok_or(AppError::BadRequest("NOT_A_CHARACTER"))?;
            confirm_in(&mut tx, &row, rules, &enc, who, character).await?;
        }
        GmCommand::Spare { who } => {
            let step = enc
                .fight
                .spare(rules, who, &mut SeededDice::from_os())
                .map_err(|r| refused(&r))?;
            commit_step(&mut tx, &row, rules, &enc, &step.fight, &step.events).await?;
        }
    }
    tx.commit().await?;
    Ok(proposal)
}

/// The GM confirms the death of `character` in fight `enc`: the fight
/// takes them out for good, then the character falls.
async fn confirm_in(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
    rules: &RuleSystem,
    enc: &Encounter,
    who: &str,
    character: Uuid,
) -> Result<(), AppError> {
    let proposed = enc
        .fight
        .combatant(who)
        .and_then(|c| c.death_saves)
        .is_some_and(|d| d.death_due);
    let step = enc
        .fight
        .confirm_death(rules, who, &mut SeededDice::from_os())
        .map_err(|r| refused(&r))?;
    commit_step(tx, row, rules, enc, &step.fight, &step.events).await?;
    crate::players::fate::fall(
        tx,
        row.id,
        rules,
        character,
        if proposed {
            crate::players::fate::Cause::Rules
        } else {
            crate::players::fate::Cause::Gm
        },
        enc.session_id,
        Some(&enc.node),
    )
    .await
}

/// When the live fight of `row` holds `character` (in it, or put out of
/// the scene), the GM's decision of their death goes through the fight:
/// returns whether it did. The caller holds the campaign lock.
///
/// # Errors
///
/// 409 `REFUSED` with the engine's reason (not down at 0); the errors of
/// `players::fate::fall`.
pub async fn confirm_death_of(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
    rules: &RuleSystem,
    character: Uuid,
) -> Result<bool, AppError> {
    if live_id(&mut **tx, row.id).await?.is_none() {
        return Ok(false);
    }
    let enc = locked_live(tx, row.id).await?;
    let who = character_token(character);
    let present = matches!(
        enc.fight.standing.get(&who),
        Some(Standing::InFight | Standing::OutOfScene)
    );
    if !present {
        return Ok(false);
    }
    confirm_in(tx, row, rules, &enc, &who, character).await?;
    Ok(true)
}

/// How many commands a proposed turn may take.
const PROPOSAL_COMMANDS: usize = 12;

/// copilot/propose-adversary-turns: play `who`'s turn on a copy of the
/// fight with the engine's `Focus` policy, dice rolled.
#[must_use]
pub fn propose(rules: &RuleSystem, fight: &Fight, who: &str, version: i32) -> Proposal {
    let mut policy = Focus::default();
    let mut dice = SeededDice::from_os();
    let mut f = fight.clone();
    let mut steps = Vec::new();
    let mut events = Vec::new();
    for _ in 0..PROPOSAL_COMMANDS {
        let decision = policy.decide(rules, &f, who);
        let (step, result) = match decision {
            Decision::Move(path) => (
                ProposedStep::Move { path: path.clone() },
                f.move_along(rules, who, &path),
            ),
            Decision::Act(play) => {
                let targets = match &play.aim {
                    promptus_shared::combat::fight::Aim::Targets(t) => t.clone(),
                    _ => Vec::new(),
                };
                let action = match &play.action {
                    promptus_shared::rules::action::ActionRef::Own(a)
                    | promptus_shared::rules::action::ActionRef::Item(a) => a.clone(),
                };
                (
                    ProposedStep::Act { action, targets },
                    f.act(rules, &play, &mut dice),
                )
            }
            Decision::Flee { difficulty } => (
                ProposedStep::Flee,
                f.flee(rules, who, difficulty, &mut dice),
            ),
            Decision::EndTurn => (ProposedStep::EndTurn, f.end_turn(rules, who, &mut dice)),
        };
        let ending = step == ProposedStep::EndTurn;
        let s = match result {
            Ok(s) => {
                steps.push(step);
                s
            }
            Err(_) => {
                steps.push(ProposedStep::EndTurn);
                match f.end_turn(rules, who, &mut dice) {
                    Ok(s) => s,
                    Err(_) => break,
                }
            }
        };
        events.extend(s.events);
        f = s.fight;
        if ending || f.active() != Some(who) {
            break;
        }
    }
    Proposal {
        who: who.to_string(),
        version,
        steps,
        events,
        fight: f,
    }
}

/// Standing of every combatant, for screens.
#[must_use]
pub fn standing(fight: &Fight, id: &str) -> Standing {
    fight.standing.get(id).copied().unwrap_or(Standing::InFight)
}

/// The reachable cells of the active combatant, with their cost.
#[must_use]
pub fn reachable(rules: &RuleSystem, fight: &Fight, who: &str) -> BTreeMap<Cell, u32> {
    if fight.active() == Some(who) {
        fight.reachable(rules, who)
    } else {
        BTreeMap::new()
    }
}
