//! session/collect-player-feedback — the lessons of each evening, in
//! writing, while they are fresh.
//!
//! At the end of a session each player answers three questions on their
//! phone (rules clear? a moment of your own? do you know what your
//! character wants next time?) and may add a word. The GM reads the
//! answers on one screen next to what Promptus measured — each player's
//! longest stretch without a moment, the rulings they contested, what the
//! next scenes needed that the table did not know when the session ended
//! — and writes down what they change.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::knowledge::Gap;
use super::session::{self, Status};
use super::spotlight;
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;
use crate::players::{Player, Role};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    Yes,
    Partly,
    No,
}

impl Answer {
    fn as_str(self) -> &'static str {
        match self {
            Self::Yes => "yes",
            Self::Partly => "partly",
            Self::No => "no",
        }
    }

    fn parse(s: &str) -> Result<Self, AppError> {
        Ok(match s {
            "yes" => Self::Yes,
            "partly" => Self::Partly,
            "no" => Self::No,
            other => return Err(AppError::Internal(format!("feedback answer {other}"))),
        })
    }
}

/// What a player answers.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Answers {
    pub rules_clear: Answer,
    pub had_moment: Answer,
    pub knows_next: Answer,
    #[serde(default)]
    pub comment: String,
}

const COMMENT_MAX: usize = 1_000;

/// The session a player answers about: the last one ended.
///
/// # Errors
///
/// 409 `NO_ENDED_SESSION`; a database error.
pub async fn session_to_answer(
    pool: &PgPool,
    player: &Player,
) -> Result<session::Session, AppError> {
    session::last_ended(pool, player.campaign_id)
        .await?
        .ok_or(AppError::Conflict("NO_ENDED_SESSION"))
}

/// Whether `player` already answered `session`.
///
/// # Errors
///
/// A database error.
pub async fn answered(pool: &PgPool, session: Uuid, player: Uuid) -> Result<bool, AppError> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM session_feedback WHERE session_id = $1 AND player_id = $2",
    )
    .bind(session)
    .bind(player)
    .fetch_one(pool)
    .await?;
    Ok(n > 0)
}

/// A player answers about the last ended session; answering again
/// replaces the answers (a thumb slipped).
///
/// # Errors
///
/// 403 `SPECTATOR`; 409 `NO_ENDED_SESSION`; 400 `TEXT_TOO_LONG`.
pub async fn answer(pool: &PgPool, player: &Player, answers: &Answers) -> Result<(), AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let comment = super::clean_text(&answers.comment, COMMENT_MAX)?;
    let mut tx = pool.begin().await?;
    campaigns::lock(&mut tx, player.campaign_id)
        .await?
        .ok_or(AppError::Unauthorized("NOT_JOINED"))?;
    let ended = session::last_ended(&mut *tx, player.campaign_id)
        .await?
        .ok_or(AppError::Conflict("NO_ENDED_SESSION"))?;
    sqlx::query(
        "INSERT INTO session_feedback (session_id, player_id, rules_clear, had_moment, knows_next, comment)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (session_id, player_id) DO UPDATE
           SET rules_clear = EXCLUDED.rules_clear, had_moment = EXCLUDED.had_moment,
               knows_next = EXCLUDED.knows_next, comment = EXCLUDED.comment, created_at = now()",
    )
    .bind(ended.id)
    .bind(player.id)
    .bind(answers.rules_clear.as_str())
    .bind(answers.had_moment.as_str())
    .bind(answers.knows_next.as_str())
    .bind(&comment)
    .execute(&mut *tx)
    .await?;
    super::touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(())
}

/// One player's line on the GM's feedback screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerReport {
    pub player_id: Uuid,
    pub nickname: String,
    pub character_name: String,
    pub answers: Option<Answers>,
    pub answered_at: Option<DateTime<Utc>>,
    /// Longest stretch without a moment, in minutes.
    pub longest_idle_minutes: i64,
    pub requests: i64,
    pub refused: i64,
    pub contested: i64,
}

/// Everything about one session's feedback, on one screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub session_id: Uuid,
    pub number: i32,
    pub status: Status,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub players: Vec<PlayerReport>,
    /// What the next scenes needed and the table did not know at the end.
    pub gaps: Vec<Gap>,
    pub gm_changes: String,
}

/// The feedback screen of session `id` of `campaign`.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, or the session is
/// not of it; a database error.
pub async fn report(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
) -> Result<Report, AppError> {
    owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let s = session::by_id(pool, campaign, id).await?;
    type Row = (Uuid, String, String, i64, i64, i64);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT p.id, p.nickname, COALESCE(c.sheet->>'name', ''),
                (SELECT COUNT(*) FROM player_requests r WHERE r.session_id = $2 AND r.player_id = p.id),
                (SELECT COUNT(*) FROM player_requests r WHERE r.session_id = $2 AND r.player_id = p.id AND r.status = 'refused'),
                (SELECT COUNT(*) FROM player_requests r WHERE r.session_id = $2 AND r.player_id = p.id AND r.contested)
         FROM players p JOIN characters c ON c.player_id = p.id
         WHERE p.campaign_id = $1 AND p.role = 'player' AND c.status = 'validated'
         ORDER BY p.created_at",
    )
    .bind(campaign)
    .bind(s.id)
    .fetch_all(pool)
    .await?;
    let ids: Vec<Uuid> = rows.iter().map(|r| r.0).collect();
    let idle = spotlight::longest_idle(pool, &s, &ids, Utc::now()).await?;
    type AnswerRow = (Uuid, String, String, String, String, DateTime<Utc>);
    let answers: Vec<AnswerRow> = sqlx::query_as(
        "SELECT player_id, rules_clear, had_moment, knows_next, comment, created_at
         FROM session_feedback WHERE session_id = $1",
    )
    .bind(s.id)
    .fetch_all(pool)
    .await?;
    let mut players = Vec::with_capacity(rows.len());
    for (player_id, nickname, character_name, requests, refused, contested) in rows {
        let given = answers.iter().find(|a| a.0 == player_id);
        let parsed = match given {
            Some((_, r, m, k, comment, _)) => Some(Answers {
                rules_clear: Answer::parse(r)?,
                had_moment: Answer::parse(m)?,
                knows_next: Answer::parse(k)?,
                comment: comment.clone(),
            }),
            None => None,
        };
        players.push(PlayerReport {
            player_id,
            nickname,
            character_name,
            answers: parsed,
            answered_at: given.map(|a| a.5),
            longest_idle_minutes: idle
                .iter()
                .find(|(p, _)| *p == player_id)
                .map_or(0, |(_, m)| *m),
            requests,
            refused,
            contested,
        });
    }
    Ok(Report {
        session_id: s.id,
        number: s.number,
        status: s.status,
        started_at: s.started_at,
        ended_at: s.ended_at,
        players,
        gaps: s.gaps_at_end.clone(),
        gm_changes: s.gm_changes.clone(),
    })
}

/// The GM's note of what they change after reading the feedback.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's, or the session is
/// not of it; 400 `TEXT_TOO_LONG`.
pub async fn note_changes(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
    text: &str,
) -> Result<(), AppError> {
    let text = super::clean_text(text, 4_000)?;
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    session::by_id(&mut *tx, campaign, id).await?;
    sqlx::query("UPDATE game_sessions SET gm_changes = $2 WHERE id = $1")
        .bind(id)
        .bind(&text)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
