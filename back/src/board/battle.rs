//! engine/support-vehicle-combat — a ship battle at the table
//! (`battles`).
//!
//! The GM opens the battle of a scene (`encounter.vehicles`): the party
//! ship and the enemy ships on the battle map's starts, the validated
//! characters aboard as the crew (each seated by class, its id the
//! character's token `pc-<uuid>`, the same as in a fight), the ship's own
//! holder (LUMEN) at its station, initiative rolled
//! (`vehicle::Battle`). Every command is checked and resolved by the
//! engine under the campaign lock, and refused with the engine's reason
//! otherwise.
//!
//! On the party ship's turn the whole crew acts: each player at their
//! own station, the GM for the ship's holder or an absent player. On an
//! enemy ship's turn the GM manoeuvres and fires, or asks the co-GM to
//! play the whole turn on a copy (`vehicle::policy::propose_enemy_turn`)
//! and accepts it as is — nothing applies without the GM.
//!
//! A boarding pauses the battle (`status = 'boarding'`) and opens the
//! scene's own encounter on its deck map ([`super::fight::open_in`]).
//! When that fight ends the battle resumes ([`deck_fight_over`]): the
//! boarders won when their side won the deck fight; a fight stopped
//! without a winner throws them back. At the end, the XP of the crew's
//! rolls is granted and the scene's outcome goes to the GM's journal.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use promptus_shared::maps::{Cell, Direction};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::dice::SeededDice;
use promptus_shared::rules::sheet::Side;
use promptus_shared::vehicle::policy::{Order, propose_enemy_turn};
use promptus_shared::vehicle::scenario::VehicleScenario;
use promptus_shared::vehicle::{
    Aim, Battle, BattleEvent, BattleRefusal, Crew, Facing, ShipStanding, Step,
};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::character_token;
use super::fight::{character_of_combatant, rules_of};
use crate::auth::guard::CurrentGm;
use crate::campaigns::CampaignRow;
use crate::error::AppError;
use crate::evening::knowledge::{self, JournalKind};
use crate::evening::session::{self, Status};
use crate::live::{self, Topic};
use crate::players::play::{self, Actor, Adjustment};
use crate::players::{Player, Role};

/// The co-GM's enemy turn, played on a copy of the battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    /// The enemy unit (a ship, or a squad) whose turn it is.
    pub unit: String,
    /// The battle version it was played from: accepting a stale one is
    /// refused.
    pub version: i32,
    pub orders: Vec<Order>,
    pub events: Vec<BattleEvent>,
    pub battle: Battle,
}

/// A battle as the server holds it.
#[derive(Debug, Clone)]
pub struct StoredBattle {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub node: String,
    /// `live`, `boarding` or `ended`.
    pub status: String,
    pub version: i32,
    pub battle: Battle,
    pub proposal: Option<Proposal>,
    pub boarding_encounter: Option<Uuid>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

type Row = (
    Uuid,
    Option<Uuid>,
    String,
    String,
    i32,
    Json<Battle>,
    Option<Json<Proposal>>,
    Option<Uuid>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);

const COLUMNS: &str = "id, session_id, node, status, version, battle, proposal, \
                       boarding_encounter, started_at, ended_at";

fn stored(r: Row) -> StoredBattle {
    StoredBattle {
        id: r.0,
        session_id: r.1,
        node: r.2,
        status: r.3,
        version: r.4,
        battle: r.5.0,
        proposal: r.6.map(|p| p.0),
        boarding_encounter: r.7,
        started_at: r.8,
        ended_at: r.9,
    }
}

/// The id of the battle of `campaign` not over yet (live or boarding).
///
/// # Errors
///
/// A database error.
pub async fn live_id(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Option<Uuid>, AppError> {
    Ok(
        sqlx::query_scalar("SELECT id FROM battles WHERE campaign_id = $1 AND status <> 'ended'")
            .bind(campaign)
            .fetch_optional(db)
            .await?,
    )
}

/// The battle of `campaign` not over yet, or the last one.
///
/// # Errors
///
/// A database error.
pub async fn latest(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
) -> Result<Option<StoredBattle>, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM battles WHERE campaign_id = $1
         ORDER BY (status <> 'ended') DESC, started_at DESC LIMIT 1"
    ))
    .bind(campaign)
    .fetch_optional(db)
    .await?;
    Ok(row.map(stored))
}

/// The events of `battle`, oldest first (the last `limit`).
///
/// # Errors
///
/// A database error.
pub async fn events(
    db: impl PgExecutor<'_>,
    battle: Uuid,
    limit: i64,
) -> Result<Vec<BattleEvent>, AppError> {
    let rows: Vec<Json<BattleEvent>> = sqlx::query_scalar(
        "SELECT event FROM (SELECT seq, event FROM battle_events WHERE battle_id = $1
                            ORDER BY seq DESC LIMIT $2) e ORDER BY seq",
    )
    .bind(battle)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|j| j.0).collect())
}

async fn locked_open(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
) -> Result<StoredBattle, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM battles WHERE campaign_id = $1 AND status <> 'ended' FOR UPDATE"
    ))
    .bind(campaign)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(stored).ok_or(AppError::Conflict("NO_BATTLE"))
}

async fn append(
    tx: &mut Transaction<'_, Postgres>,
    battle: Uuid,
    events: &[BattleEvent],
) -> Result<(), AppError> {
    for e in events {
        sqlx::query(
            "INSERT INTO battle_events (battle_id, seq, event)
             VALUES ($1, COALESCE((SELECT MAX(seq) FROM battle_events WHERE battle_id = $1), 0) + 1, $2)",
        )
        .bind(battle)
        .bind(Json(e))
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

fn refused(r: &BattleRefusal) -> AppError {
    AppError::Refused {
        code: "REFUSED",
        refusal: serde_json::to_value(r).unwrap_or_default(),
    }
}

/// The validated characters of `campaign` as the crew, in the order
/// they joined.
async fn crew_aboard(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    rules: &RuleSystem,
) -> Result<Vec<Crew>, AppError> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT c.id, COALESCE(c.sheet->>'name', p.nickname)
         FROM characters c JOIN players p ON p.id = c.player_id
         WHERE p.campaign_id = $1 AND p.role = 'player' AND c.status = 'validated'
         ORDER BY p.created_at",
    )
    .bind(campaign)
    .fetch_all(&mut **tx)
    .await?;
    let mut crew = Vec::new();
    for (id, name) in rows {
        let (sheet, state) = play::in_play_locked(tx, campaign, id, rules).await?;
        let Some(mut c) = play::combatant(rules, &sheet, &state) else {
            continue;
        };
        c.id = character_token(id);
        c.name = name;
        c.side = Side::Party;
        crew.push(Crew::from_combatant(rules, &c, sheet.class_id.clone()));
    }
    Ok(crew)
}

/// Open the ship battle of scene `node`.
///
/// # Errors
///
/// 404; 409 `SESSION_NOT_LIVE`, `FIGHT_IN_PROGRESS`, `RULES_UNKNOWN`,
/// `NO_PARTY`; 400 `UNKNOWN_NODE`, `NO_BATTLE_HERE`, `NO_MAP`; 422
/// `BATTLE_SETUP` (a ship the rules lack, no start left on the map).
pub async fn start(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    node_id: &str,
) -> Result<StoredBattle, AppError> {
    let mut tx = pool.begin().await?;
    let (row, live) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    if live_id(&mut *tx, campaign).await?.is_some()
        || super::fight::live_id(&mut *tx, campaign).await?.is_some()
    {
        return Err(AppError::Conflict("FIGHT_IN_PROGRESS"));
    }
    let rules = rules_of(&row)?;
    let node = row
        .story
        .node(node_id)
        .ok_or(AppError::BadRequest("UNKNOWN_NODE"))?;
    let spec = node
        .encounter
        .as_ref()
        .and_then(|e| e.vehicles.as_ref())
        .ok_or(AppError::BadRequest("NO_BATTLE_HERE"))?;
    let map = crate::campaign_maps::playable(&mut *tx, campaign, &row.story, &spec.map)
        .await?
        .ok_or(AppError::BadRequest("NO_MAP"))?;
    let crew = crew_aboard(&mut tx, campaign, &rules).await?;
    if crew.is_empty() {
        return Err(AppError::Conflict("NO_PARTY"));
    }
    let setup_error = |detail: String| AppError::Invalid {
        code: "BATTLE_SETUP",
        detail,
    };
    let (map, setup) = VehicleScenario::from_story(&row.story, node_id, spec)
        .setup(&rules, &map, crew)
        .map_err(|e| setup_error(e.to_string()))?;
    let mut dice = SeededDice::from_os();
    let step =
        Battle::start(&rules, map, setup, &mut dice).map_err(|e| setup_error(format!("{e:?}")))?;
    let row_out: Row = sqlx::query_as(&format!(
        "INSERT INTO battles (campaign_id, session_id, node, battle)
         VALUES ($1, $2, $3, $4) RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(live.id)
    .bind(node_id)
    .bind(Json(&step.battle))
    .fetch_one(&mut *tx)
    .await?;
    let b = stored(row_out);
    append(&mut tx, b.id, &step.events).await?;
    knowledge::write(
        &mut tx,
        campaign,
        Some(live.id),
        JournalKind::Fight,
        Some(node_id),
        &node.title,
        true,
    )
    .await?;
    crate::evening::touch(&mut tx, campaign).await?;
    live::touch(&mut tx, campaign, &Topic::Fight).await?;
    tx.commit().await?;
    Ok(b)
}

/// Save the battle after a command: events, state, and the end when it
/// came (XP of the crew's rolls, the scene's outcome to the journal).
async fn commit_step(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
    rules: &RuleSystem,
    b: &StoredBattle,
    next: &Battle,
    events: &[BattleEvent],
) -> Result<(), AppError> {
    let campaign = row.id;
    append(tx, b.id, events).await?;
    let status = if next.is_over() {
        "ended"
    } else if next.boarding.is_some() {
        "boarding"
    } else {
        "live"
    };
    sqlx::query(
        "UPDATE battles SET battle = $2, version = version + 1, proposal = NULL, status = $3,
                ended_at = CASE WHEN $3 = 'ended' THEN now() ELSE ended_at END,
                boarding_encounter = CASE WHEN $3 = 'boarding' THEN boarding_encounter END
         WHERE id = $1",
    )
    .bind(b.id)
    .bind(Json(next))
    .bind(status)
    .execute(&mut **tx)
    .await?;
    if let (Some(end), false) = (&next.end, b.battle.is_over()) {
        for (id, xp) in &end.xp {
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
        let node = row.story.node(&b.node);
        let outcome = node.and_then(|n| n.encounter.as_ref()).map(|e| {
            if won {
                e.on_victory.clone()
            } else {
                e.on_defeat.clone()
            }
        });
        let text = outcome
            .filter(|t| !t.is_empty())
            .or_else(|| node.map(|n| n.title.clone()))
            .unwrap_or_default();
        // The story's outcome text is the GM's: the table gets a plain line.
        knowledge::write(
            tx,
            campaign,
            b.session_id,
            JournalKind::Note,
            Some(&b.node),
            &text,
            false,
        )
        .await?;
        knowledge::write(
            tx,
            campaign,
            b.session_id,
            JournalKind::Fight,
            Some(&b.node),
            if won {
                "Combat de vaisseau gagné"
            } else {
                "Combat de vaisseau terminé"
            },
            true,
        )
        .await?;
        crate::evening::touch(tx, campaign).await?;
    }
    live::touch(tx, campaign, &Topic::Fight).await?;
    Ok(())
}

/// What a crew member does at their station.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CrewCommand {
    /// Move to another station (`None`: leave it), paid in actions.
    Station {
        #[serde(default)]
        station: Option<String>,
    },
    /// One of the station's actions.
    Act {
        action: String,
        #[serde(default)]
        aim: Aim,
    },
    /// Done for this turn.
    Pass,
}

fn crew_step(
    rules: &RuleSystem,
    battle: &Battle,
    who: &str,
    cmd: &CrewCommand,
) -> Result<Step, AppError> {
    let mut dice = SeededDice::from_os();
    match cmd {
        CrewCommand::Station { station } => battle.take_station(rules, who, station.as_deref()),
        CrewCommand::Act { action, aim } => battle.crew_act(rules, who, action, aim, &mut dice),
        CrewCommand::Pass => battle.pass(rules, who, &mut dice),
    }
    .map_err(|r| refused(&r))
}

/// A player's command at their own station.
///
/// # Errors
///
/// 403 `SPECTATOR`; 409 `NO_BATTLE`, `NOT_ABOARD`, `REFUSED` (with the
/// engine's reason), `SESSION_NOT_LIVE`.
pub async fn command(pool: &PgPool, player: &Player, cmd: &CrewCommand) -> Result<(), AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let character: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM characters WHERE player_id = $1 AND status = 'validated'",
    )
    .bind(player.id)
    .fetch_optional(pool)
    .await?;
    let who = character_token(character.ok_or(AppError::Conflict("NOT_ABOARD"))?);
    let mut tx = pool.begin().await?;
    let (row, live) = session::lock_for_player(&mut tx, player, &[Status::Live]).await?;
    let b = locked_open(&mut tx, player.campaign_id).await?;
    if b.battle.crew_member(&who).is_none() {
        return Err(AppError::Conflict("NOT_ABOARD"));
    }
    let rules = rules_of(&row)?;
    let step = crew_step(&rules, &b.battle, &who, cmd)?;
    commit_step(&mut tx, &row, &rules, &b, &step.battle, &step.events).await?;
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

/// What the GM does in a battle.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum GmCommand {
    /// Seat a crew member at a station (or none), at no cost.
    Seat {
        crew: String,
        #[serde(default)]
        station: Option<String>,
    },
    /// A command for a crew member: the ship's holder (LUMEN), or a
    /// player away from their screen.
    Crew {
        crew: String,
        command: CrewCommand,
    },
    /// The active enemy ship's manoeuvre.
    Maneuver {
        ship: String,
        to: Cell,
        facing: Facing,
    },
    /// The active enemy ship's shot.
    Fire {
        ship: String,
        weapon: String,
        target: String,
    },
    EndTurn,
    /// The co-GM plays the active enemy unit's whole turn on a copy.
    Propose,
    /// The proposal becomes the battle.
    Accept,
    /// Set a ship's gauges.
    Adjust {
        ship: String,
        #[serde(default)]
        hull: Option<i32>,
        #[serde(default)]
        screen: Option<i32>,
        #[serde(default)]
        morale: Option<i32>,
    },
    /// Take a ship out: it strikes, flees, sinks.
    Strike {
        ship: String,
        standing: ShipStanding,
    },
    /// Turn the wind (or the pull).
    Current {
        #[serde(default)]
        from: Option<Direction>,
    },
    /// Grapple: the fight moves to the deck.
    Board {
        attacker: String,
        defender: String,
    },
    /// End the battle now.
    Stop,
}

/// Apply `cmd` of the GM to the battle in progress.
///
/// # Errors
///
/// 404; 409 `NO_BATTLE`, `NOT_AN_ENEMY_TURN`, `BOARDING`, `NO_PROPOSAL`,
/// `PROPOSAL_STALE`, `REFUSED`, and the codes of
/// [`super::fight::open_in`] for a boarding.
pub async fn gm_command(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    cmd: &GmCommand,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let (row, live) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    let b = locked_open(&mut tx, campaign).await?;
    let rules = rules_of(&row)?;
    let mut dice = SeededDice::from_os();
    let r = |x: Result<Step, BattleRefusal>| x.map_err(|e| refused(&e));
    let step = match cmd {
        GmCommand::Seat { crew, station } => r(b.battle.seat(&rules, crew, station.as_deref()))?,
        GmCommand::Crew { crew, command } => crew_step(&rules, &b.battle, crew, command)?,
        GmCommand::Maneuver { ship, to, facing } => {
            r(b.battle.enemy_maneuver(&rules, ship, *to, *facing))?
        }
        GmCommand::Fire {
            ship,
            weapon,
            target,
        } => r(b.battle.enemy_fire(&rules, ship, weapon, target, &mut dice))?,
        GmCommand::EndTurn => r(b.battle.end_turn(&rules, &mut dice))?,
        GmCommand::Propose => {
            if b.battle.boarding.is_some() {
                return Err(AppError::Conflict("BOARDING"));
            }
            let unit = b
                .battle
                .active()
                .filter(|_| !b.battle.crew_turn())
                .map(|u| u.name.clone())
                .ok_or(AppError::Conflict("NOT_AN_ENEMY_TURN"))?;
            let (orders, events, next) = propose_enemy_turn(&rules, &b.battle, &mut dice);
            let p = Proposal {
                unit,
                version: b.version,
                orders,
                events,
                battle: next,
            };
            sqlx::query("UPDATE battles SET proposal = $2 WHERE id = $1")
                .bind(b.id)
                .bind(Json(&p))
                .execute(&mut *tx)
                .await?;
            live::touch(&mut tx, campaign, &Topic::Desk).await?;
            tx.commit().await?;
            return Ok(());
        }
        GmCommand::Accept => {
            let p = b
                .proposal
                .clone()
                .ok_or(AppError::Conflict("NO_PROPOSAL"))?;
            if p.version != b.version {
                return Err(AppError::Conflict("PROPOSAL_STALE"));
            }
            Step {
                battle: p.battle,
                events: p.events,
            }
        }
        GmCommand::Adjust {
            ship,
            hull,
            screen,
            morale,
        } => r(b.battle.adjust(ship, *hull, *screen, *morale))?,
        GmCommand::Strike { ship, standing } => r(b.battle.strike(ship, *standing))?,
        GmCommand::Current { from } => r(b.battle.set_current(*from))?,
        GmCommand::Board { attacker, defender } => {
            let step = r(b.battle.board(&rules, attacker, defender))?;
            let enc = super::fight::open_in(&mut tx, &row, live.id, &b.node).await?;
            sqlx::query("UPDATE battles SET boarding_encounter = $2 WHERE id = $1")
                .bind(b.id)
                .bind(enc.id)
                .execute(&mut *tx)
                .await?;
            step
        }
        GmCommand::Stop => b.battle.stop(),
    };
    commit_step(&mut tx, &row, &rules, &b, &step.battle, &step.events).await?;
    tx.commit().await?;
    Ok(())
}

/// The fight on the deck of a boarding is over: the battle it paused
/// resumes, the boarders winning when their side won the fight (a fight
/// stopped without a winner throws them back). Nothing when `encounter`
/// is not a boarding.
///
/// # Errors
///
/// A database error.
pub async fn deck_fight_over(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
    rules: &RuleSystem,
    encounter: Uuid,
    winner: Option<Side>,
) -> Result<(), AppError> {
    let found: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM battles
         WHERE campaign_id = $1 AND status = 'boarding' AND boarding_encounter = $2 FOR UPDATE"
    ))
    .bind(row.id)
    .bind(encounter)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(b) = found.map(stored) else {
        return Ok(());
    };
    let attacker_side = b
        .battle
        .boarding
        .as_ref()
        .and_then(|x| b.battle.ship(&x.attacker))
        .map(|s| s.side);
    let attacker_won = winner.is_some() && winner == attacker_side;
    let step = b
        .battle
        .end_boarding(attacker_won)
        .map_err(|r| refused(&r))?;
    commit_step(tx, row, rules, &b, &step.battle, &step.events).await
}

/// Cells each ship of the active enemy unit may still manoeuvre to, for
/// the GM's screen.
#[must_use]
pub fn enemy_reach(rules: &RuleSystem, battle: &Battle) -> BTreeMap<String, Vec<(Cell, u32)>> {
    let mut out = BTreeMap::new();
    let (Some(v), Some(unit)) = (rules.vehicles.as_ref(), battle.active()) else {
        return out;
    };
    if unit.side == Side::Party || battle.boarding.is_some() {
        return out;
    }
    for id in &unit.ships {
        if battle.ship(id).is_some_and(|s| s.afloat() && !s.moved) {
            out.insert(
                id.clone(),
                battle.reachable(v, id, 1).into_iter().collect::<Vec<_>>(),
            );
        }
    }
    out
}

/// What each ship of the active enemy unit can fire at now: its
/// weapons, each with the party ships in arc and range.
#[must_use]
pub fn enemy_fire_options(rules: &RuleSystem, battle: &Battle) -> Vec<serde_json::Value> {
    let (Some(v), Some(unit)) = (rules.vehicles.as_ref(), battle.active()) else {
        return Vec::new();
    };
    if unit.side == Side::Party || battle.boarding.is_some() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for id in &unit.ships {
        let Some(s) = battle.ship(id).filter(|s| s.afloat() && !s.fired) else {
            continue;
        };
        for t in battle
            .ships
            .iter()
            .filter(|t| t.side != s.side && t.afloat())
        {
            for w in battle.weapons_on(v, s, t) {
                out.push(serde_json::json!({
                    "ship": s.id,
                    "weapon": w.id,
                    "name": w.name,
                    "target": t.id,
                }));
            }
        }
    }
    out
}
