//! session/write-recaps — what a session changed, read when it ends:
//! the worlds at its start and end, its fights, the party, the threads
//! the table took on. The facts and the drafts written from them are
//! `promptus_shared::story::recap`; the GM rereads, then publishes
//! (`session::publish`).

use chrono::Utc;
use promptus_shared::combat::Fight;
use promptus_shared::rules::sheet::Side;
use promptus_shared::story::WorldState;
use promptus_shared::story::recap::{PartyFact, RecapFacts, SessionRecord, session_facts};
use sqlx::types::Json;
use sqlx::{Postgres, Transaction};

use super::session::Session;
use crate::campaigns::CampaignRow;
use crate::error::AppError;
use crate::players::play::{PlayState, combatant, in_play};

/// The facts of `session` of `row`, as the world stands now.
///
/// # Errors
///
/// A database error.
pub async fn facts_of(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
    session: &Session,
) -> Result<RecapFacts, AppError> {
    // The world when the lobby opened; for a session opened before that
    // was kept, the world the previous one left.
    let start: Option<Option<Json<WorldState>>> = sqlx::query_scalar(
        "SELECT COALESCE(s.world_at_start,
                         (SELECT p.world_at_end FROM game_sessions p
                          WHERE p.campaign_id = s.campaign_id AND p.number < s.number
                          ORDER BY p.number DESC LIMIT 1))
         FROM game_sessions s WHERE s.id = $1",
    )
    .bind(session.id)
    .fetch_optional(&mut **tx)
    .await?;
    let before = start.flatten().map(|w| w.0).unwrap_or_default();

    let fights: Vec<Json<Fight>> = sqlx::query_scalar(
        "SELECT fight FROM encounters WHERE session_id = $1 ORDER BY started_at",
    )
    .bind(session.id)
    .fetch_all(&mut **tx)
    .await?;
    // Named as the players saw them on the board.
    let fallen = fights
        .iter()
        .flat_map(|f| f.0.scene.combatants.values())
        .filter(|c| c.side == Side::Opposition && c.hit_points <= 0)
        .map(|c| c.name.clone())
        .collect();

    let party = match row.rules() {
        Some(rules) => in_play(&mut **tx, row.id)
            .await?
            .into_iter()
            .filter_map(|p| {
                let state = p.state.unwrap_or_else(|| PlayState::start(rules, &p.sheet));
                let c = combatant(rules, &p.sheet, &state)?;
                Some(PartyFact {
                    name: p.sheet.name.clone(),
                    hit_points: c.hit_points,
                    max_hit_points: c.max_hit_points(rules).ok()?,
                })
            })
            .collect(),
        None => Vec::new(),
    };

    let open_threads: Vec<String> = sqlx::query_scalar(
        "SELECT text FROM table_journal
         WHERE session_id = $1 AND shared AND kind IN ('promise', 'debt')
         ORDER BY created_at",
    )
    .bind(session.id)
    .fetch_all(&mut **tx)
    .await?;

    let duration_minutes = session
        .started_at
        .map_or(0, |at| (Utc::now() - at).num_minutes());
    Ok(session_facts(
        &row.story,
        &before,
        &row.world,
        SessionRecord {
            duration_minutes,
            fights: u32::try_from(fights.len()).unwrap_or(u32::MAX),
            fallen,
            party,
            open_threads,
        },
    ))
}
