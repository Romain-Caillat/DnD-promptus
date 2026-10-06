//! session/drive-scenes — a player asks, the GM answers, the server rolls.
//!
//! 1. The player sends a card: an ability (« Dextérité »), one of their
//!    class actions, or « Autre… » with their own words.
//! 2. The GM accepts it, refuses it with a reason, or asks for a check:
//!    an ability and a difficulty (a named one of the rules, or a number)
//!    in one gesture. The past rulings for the same situation are offered
//!    (`knowledge::similar_rulings`) and the new one is kept.
//! 3. The player rolls: the **server** rolls (`rules::check`), applies the
//!    XP the outcome grants, and writes the result in the journal. The
//!    phone's die only animates the breakdown it receives
//!    (`ui/roll-faceted-dice`).

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::check::{self, Advantage, OutcomeBand, RollBreakdown, RollTarget};
use promptus_shared::rules::dice::SeededDice;
use promptus_shared::rules::model::RollScope;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::knowledge::{self, JournalKind};
use super::session::{self, Status};
use super::{MomentKind, moment};
use crate::auth::guard::CurrentGm;
use crate::error::AppError;
use crate::players::play::{self, Actor, Adjustment, combatant};
use crate::players::{CharacterStatus, Player, Role};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    Pending,
    Accepted,
    Refused,
    Check,
    Rolled,
    Withdrawn,
}

impl RequestStatus {
    fn parse(s: &str) -> Result<Self, AppError> {
        Ok(match s {
            "pending" => Self::Pending,
            "accepted" => Self::Accepted,
            "refused" => Self::Refused,
            "check" => Self::Check,
            "rolled" => Self::Rolled,
            "withdrawn" => Self::Withdrawn,
            other => return Err(AppError::Internal(format!("request status {other}"))),
        })
    }
}

/// What a player plays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Card {
    /// A check of one ability of the rules.
    Ability { ability: String },
    /// One of the character's class actions.
    Action { action: String },
    /// « Autre… »: the player's own words only.
    Other,
}

impl Card {
    fn key(&self) -> Option<String> {
        match self {
            Self::Ability { ability } => Some(format!("ability:{ability}")),
            Self::Action { action } => Some(format!("action:{action}")),
            Self::Other => None,
        }
    }

    fn from_key(key: Option<&str>) -> Self {
        match key.and_then(|k| k.split_once(':')) {
            Some(("ability", a)) => Self::Ability { ability: a.into() },
            Some(("action", a)) => Self::Action { action: a.into() },
            _ => Self::Other,
        }
    }

    /// The card's name as the rules give it.
    #[must_use]
    pub fn name(&self, rules: Option<&RuleSystem>) -> Option<String> {
        let rules = rules?;
        match self {
            Self::Ability { ability } => rules
                .abilities
                .iter()
                .find(|a| &a.id == ability)
                .map(|a| a.name.clone()),
            Self::Action { action } => rules
                .classes
                .iter()
                .flat_map(|c| c.actions.iter())
                .find(|a| &a.id == action)
                .map(|a| a.name.clone()),
            Self::Other => None,
        }
    }
}

/// A request as the server holds it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub id: Uuid,
    pub session_id: Uuid,
    pub player_id: Uuid,
    pub character_id: Option<Uuid>,
    pub card: Card,
    pub text: String,
    pub status: RequestStatus,
    pub gm_reason: Option<String>,
    pub check: Option<CheckAsked>,
    pub roll: Option<RollBreakdown>,
    pub contested: bool,
    pub created_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub rolled_at: Option<DateTime<Utc>>,
}

/// The check the GM asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckAsked {
    pub ability: String,
    pub difficulty: i32,
    /// The difficulty's name in the rules, when it has one.
    pub label: Option<String>,
}

type Row = (
    Uuid,
    Uuid,
    Uuid,
    Option<Uuid>,
    Option<String>,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<String>,
    Option<Json<RollBreakdown>>,
    bool,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

const COLUMNS: &str = "id, session_id, player_id, character_id, action_id, text, status, \
     gm_reason, check_ability, check_difficulty, check_label, roll, contested, created_at, \
     decided_at, rolled_at";

fn from_row(r: Row) -> Result<Request, AppError> {
    Ok(Request {
        id: r.0,
        session_id: r.1,
        player_id: r.2,
        character_id: r.3,
        card: Card::from_key(r.4.as_deref()),
        text: r.5,
        status: RequestStatus::parse(&r.6)?,
        gm_reason: r.7,
        check: match (r.8, r.9) {
            (Some(ability), Some(difficulty)) => Some(CheckAsked {
                ability,
                difficulty,
                label: r.10,
            }),
            _ => None,
        },
        roll: r.11.map(|j| j.0),
        contested: r.12,
        created_at: r.13,
        decided_at: r.14,
        rolled_at: r.15,
    })
}

/// Every request of `session`, oldest first.
///
/// # Errors
///
/// A database error.
pub async fn of_session(db: impl PgExecutor<'_>, session: Uuid) -> Result<Vec<Request>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM player_requests WHERE session_id = $1 ORDER BY created_at, id"
    ))
    .bind(session)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(from_row).collect()
}

async fn locked(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    id: Uuid,
) -> Result<Request, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM player_requests WHERE session_id = $1 AND id = $2 FOR UPDATE"
    ))
    .bind(session)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(from_row)
        .transpose()?
        .ok_or(AppError::NotFound("NO_SUCH_REQUEST"))
}

/// What a player sends.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ask {
    pub card: Card,
    #[serde(default)]
    pub text: String,
}

const TEXT_MAX: usize = 500;

/// The validated character of `player`, if any.
async fn character_of(db: impl PgExecutor<'_>, player: &Player) -> Result<Option<Uuid>, AppError> {
    let row: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, status FROM characters WHERE player_id = $1")
            .bind(player.id)
            .fetch_optional(db)
            .await?;
    Ok(match row {
        Some((id, status)) if CharacterStatus::parse(&status)? == CharacterStatus::Validated => {
            Some(id)
        }
        _ => None,
    })
}

/// A seated player asks the GM, during the live session.
///
/// # Errors
///
/// 403 `SPECTATOR`; 409 `NO_SESSION`, `SESSION_NOT_LIVE`,
/// `CHARACTER_NOT_VALIDATED`, `TOO_MANY_PENDING`; 400 `EMPTY_TEXT` (an
/// « Autre… » with no words), `TEXT_TOO_LONG`, `UNKNOWN_CARD`.
pub async fn ask(
    pool: &PgPool,
    player: &Player,
    rules: Option<&RuleSystem>,
    ask: &Ask,
) -> Result<Request, AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let text = super::clean_text(&ask.text, TEXT_MAX)?;
    if ask.card == Card::Other && text.is_empty() {
        return Err(AppError::BadRequest("EMPTY_TEXT"));
    }
    if ask.card != Card::Other && ask.card.name(rules).is_none() {
        return Err(AppError::BadRequest("UNKNOWN_CARD"));
    }
    let mut tx = pool.begin().await?;
    let (_, live) = session::lock_for_player(&mut tx, player, &[Status::Live]).await?;
    let character = character_of(&mut *tx, player)
        .await?
        .ok_or(AppError::Conflict("CHARACTER_NOT_VALIDATED"))?;
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM player_requests
         WHERE session_id = $1 AND player_id = $2 AND status IN ('pending', 'check')",
    )
    .bind(live.id)
    .bind(player.id)
    .fetch_one(&mut *tx)
    .await?;
    if pending >= 3 {
        return Err(AppError::Conflict("TOO_MANY_PENDING"));
    }
    let row: Row = sqlx::query_as(&format!(
        "INSERT INTO player_requests (campaign_id, session_id, player_id, character_id, action_id, text)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING {COLUMNS}"
    ))
    .bind(player.campaign_id)
    .bind(live.id)
    .bind(player.id)
    .bind(character)
    .bind(ask.card.key())
    .bind(&text)
    .fetch_one(&mut *tx)
    .await?;
    moment(&mut tx, live.id, player.id, MomentKind::Request).await?;
    super::touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    from_row(row)
}

/// The GM's answer to a request.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Decision {
    Accept {
        #[serde(default)]
        reason: String,
    },
    Refuse {
        reason: String,
    },
    /// A check: an ability and a difficulty — a named one (`moyen`) or a
    /// number the GM gives.
    #[serde(rename_all = "camelCase")]
    Check {
        ability: String,
        #[serde(default)]
        difficulty: Option<String>,
        #[serde(default)]
        value: Option<i32>,
    },
}

const REASON_MAX: usize = 300;

/// The situation a request is about, as a ruling remembers it.
fn situation(rules: Option<&RuleSystem>, req: &Request) -> String {
    if !req.text.is_empty() {
        return req.text.clone();
    }
    req.card.name(rules).unwrap_or_default()
}

/// The GM answers request `id` of the live session.
///
/// # Errors
///
/// As `session::lock_for_gm`; 404 `NO_SUCH_REQUEST`; 409
/// `ALREADY_DECIDED`; 400 `REASON_REQUIRED`, `TEXT_TOO_LONG`,
/// `UNKNOWN_ABILITY`, `UNKNOWN_DIFFICULTY`, `INVALID_DIFFICULTY`.
pub async fn decide(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    rules: Option<&RuleSystem>,
    id: Uuid,
    decision: &Decision,
) -> Result<Request, AppError> {
    let mut tx = pool.begin().await?;
    let (_, live) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    let req = locked(&mut tx, live.id, id).await?;
    if req.status != RequestStatus::Pending {
        return Err(AppError::Conflict("ALREADY_DECIDED"));
    }
    let (status, reason, check) = match decision {
        Decision::Accept { reason } => (
            "accepted",
            Some(super::clean_text(reason, REASON_MAX)?),
            None,
        ),
        Decision::Refuse { reason } => {
            let reason = super::clean_text(reason, REASON_MAX)?;
            if reason.is_empty() {
                return Err(AppError::BadRequest("REASON_REQUIRED"));
            }
            ("refused", Some(reason), None)
        }
        Decision::Check {
            ability,
            difficulty,
            value,
        } => {
            let rules = rules.ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
            if !rules.abilities.iter().any(|a| &a.id == ability) {
                return Err(AppError::BadRequest("UNKNOWN_ABILITY"));
            }
            let asked = match (difficulty, value) {
                (Some(d), _) => {
                    let named = rules
                        .difficulty(d)
                        .ok_or(AppError::BadRequest("UNKNOWN_DIFFICULTY"))?;
                    CheckAsked {
                        ability: ability.clone(),
                        difficulty: named.value,
                        label: Some(named.name.clone()),
                    }
                }
                (None, Some(v)) if (1..=40).contains(v) => CheckAsked {
                    ability: ability.clone(),
                    difficulty: *v,
                    label: rules
                        .difficulties
                        .iter()
                        .find(|d| d.value == *v)
                        .map(|d| d.name.clone()),
                },
                _ => return Err(AppError::BadRequest("INVALID_DIFFICULTY")),
            };
            knowledge::rule(
                &mut tx,
                campaign,
                live.id,
                &situation(Some(rules), &req),
                &asked.ability,
                asked.difficulty,
            )
            .await?;
            ("check", None, Some(asked))
        }
    };
    let row: Row = sqlx::query_as(&format!(
        "UPDATE player_requests
         SET status = $2, gm_reason = NULLIF($3, ''), check_ability = $4,
             check_difficulty = $5, check_label = $6, decided_at = now()
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(status)
    .bind(reason)
    .bind(check.as_ref().map(|c| c.ability.clone()))
    .bind(check.as_ref().map(|c| c.difficulty))
    .bind(check.as_ref().and_then(|c| c.label.clone()))
    .fetch_one(&mut *tx)
    .await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    from_row(row)
}

/// The player rolls the check the GM asked for. The server rolls the
/// rules' die, adds the character's modifiers, applies the XP the
/// outcome grants, and writes the result in the journal.
///
/// # Errors
///
/// 409 `NO_SESSION`, `SESSION_NOT_LIVE`, `NOT_A_CHECK`, `RULES_UNKNOWN`,
/// `NO_PLAY_SHEET`; 404 `NO_SUCH_REQUEST` (another player's included).
pub async fn roll(
    pool: &PgPool,
    player: &Player,
    rules: Option<&RuleSystem>,
    id: Uuid,
) -> Result<Request, AppError> {
    let rules = rules.ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
    let mut tx = pool.begin().await?;
    let (_, live) = session::lock_for_player(&mut tx, player, &[Status::Live]).await?;
    let req = locked(&mut tx, live.id, id).await?;
    if req.player_id != player.id {
        return Err(AppError::NotFound("NO_SUCH_REQUEST"));
    }
    let (Some(asked), RequestStatus::Check) = (&req.check, req.status) else {
        return Err(AppError::Conflict("NOT_A_CHECK"));
    };
    let character = req
        .character_id
        .ok_or(AppError::Conflict("NO_PLAY_SHEET"))?;
    let (sheet, state) =
        play::in_play_locked(&mut tx, player.campaign_id, character, rules).await?;
    let c = combatant(rules, &sheet, &state).ok_or(AppError::Conflict("NO_PLAY_SHEET"))?;
    let target = RollTarget::Difficulty {
        id: rules
            .difficulties
            .iter()
            .find(|d| d.value == asked.difficulty)
            .map(|d| d.id.clone()),
        value: asked.difficulty,
    };
    let breakdown = check::ability_check(
        rules,
        &c,
        &asked.ability,
        RollScope::Checks,
        Some(target),
        Advantage::Normal,
        &mut SeededDice::from_os(),
    )
    .map_err(|e| AppError::Internal(format!("check: {e:?}")))?;
    let band = breakdown.band.unwrap_or(OutcomeBand::Failure);
    let xp = band.xp(rules);
    if xp > 0 {
        play::adjust_in(
            &mut tx,
            player.campaign_id,
            rules,
            character,
            Adjustment::Xp {
                delta: i32::try_from(xp).unwrap_or(i32::MAX),
            },
            Actor::Rules,
        )
        .await?;
    }
    let ability = rules
        .abilities
        .iter()
        .find(|a| a.id == asked.ability)
        .map_or(asked.ability.clone(), |a| a.name.clone());
    let what = situation(Some(rules), &req);
    let line = format!(
        "{} — {}{} : {} {} {} → {}",
        sheet.name,
        what,
        if what.is_empty() { "" } else { "," },
        ability,
        breakdown.total,
        asked
            .label
            .clone()
            .unwrap_or_else(|| asked.difficulty.to_string()),
        band.name(rules),
    );
    knowledge::write(
        &mut tx,
        player.campaign_id,
        Some(live.id),
        JournalKind::Roll,
        None,
        line.trim_start_matches(" — "),
        true,
    )
    .await?;
    let row: Row = sqlx::query_as(&format!(
        "UPDATE player_requests SET status = 'rolled', roll = $2, rolled_at = now()
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(Json(&breakdown))
    .fetch_one(&mut *tx)
    .await?;
    moment(&mut tx, live.id, player.id, MomentKind::Roll).await?;
    super::touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    from_row(row)
}

/// The player withdraws a request the GM has not answered, or contests
/// one the GM refused or a check they rolled (a measure the GM reads
/// after the evening, `feedback`).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Follow {
    Withdraw,
    Contest,
}

/// Apply `follow` to the player's own request `id`.
///
/// # Errors
///
/// 404 `NO_SUCH_REQUEST`; 409 `NO_SESSION`, `SESSION_NOT_LIVE`,
/// `ALREADY_DECIDED` (withdrawing an answered request),
/// `NOT_CONTESTABLE`.
pub async fn follow(
    pool: &PgPool,
    player: &Player,
    id: Uuid,
    follow: Follow,
) -> Result<Request, AppError> {
    let mut tx = pool.begin().await?;
    let (_, live) = session::lock_for_player(&mut tx, player, &[Status::Live]).await?;
    let req = locked(&mut tx, live.id, id).await?;
    if req.player_id != player.id {
        return Err(AppError::NotFound("NO_SUCH_REQUEST"));
    }
    let sql = match follow {
        Follow::Withdraw if req.status == RequestStatus::Pending => {
            "UPDATE player_requests SET status = 'withdrawn' WHERE id = $1"
        }
        Follow::Withdraw => return Err(AppError::Conflict("ALREADY_DECIDED")),
        Follow::Contest if matches!(req.status, RequestStatus::Refused | RequestStatus::Rolled) => {
            "UPDATE player_requests SET contested = true WHERE id = $1"
        }
        Follow::Contest => return Err(AppError::Conflict("NOT_CONTESTABLE")),
    };
    sqlx::query(sql).bind(id).execute(&mut *tx).await?;
    super::touch(&mut tx, player.campaign_id).await?;
    let req = locked(&mut tx, live.id, id).await?;
    tx.commit().await?;
    Ok(req)
}
