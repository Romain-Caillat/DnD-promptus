//! After a death the GM confirmed (`player/face-death`, planche
//! « Mourir »): the character is marked dead with the evening it fell
//! in; its player says their last words — read at the table and kept in
//! the shared journal — then watches, or creates another character, who
//! enters play at the group's level once the GM validates them.

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::progression::level_for;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{Player, Role, create_character_at, touch_character};
use crate::error::AppError;
use crate::evening::knowledge::{self, JournalKind};
use crate::players::play;

/// Last words, at most this many characters.
pub const LAST_WORDS_MAX: usize = 280;

/// A player's fallen character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fallen {
    pub id: Uuid,
    pub name: String,
    pub last_words: String,
    pub died_at: DateTime<Utc>,
    pub died_session: Option<Uuid>,
}

type FallenRow = (Uuid, String, String, DateTime<Utc>, Option<Uuid>);

const FALLEN_COLUMNS: &str = "id, COALESCE(sheet->>'name', ''), last_words, died_at, died_session";

fn fallen((id, name, last_words, died_at, died_session): FallenRow) -> Fallen {
    Fallen {
        id,
        name,
        last_words,
        died_at,
        died_session,
    }
}

/// `player`'s last fallen character, if one fell.
///
/// # Errors
///
/// A database error.
pub async fn fallen_of(pool: &PgPool, player: &Player) -> Result<Option<Fallen>, AppError> {
    let row: Option<FallenRow> = sqlx::query_as(&format!(
        "SELECT {FALLEN_COLUMNS} FROM characters
         WHERE player_id = $1 AND status = 'dead' ORDER BY died_at DESC LIMIT 1"
    ))
    .bind(player.id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(fallen))
}

/// Mark the character behind a fight's dead combatant: out of play for
/// good, the evening it fell in kept. A shared line tells the table.
///
/// # Errors
///
/// A database error.
pub(crate) async fn mark_dead(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    character: Uuid,
    session: Option<Uuid>,
) -> Result<(), AppError> {
    let name: Option<String> = sqlx::query_scalar(
        "UPDATE characters SET status = 'dead', died_at = now(), died_session = $3
         WHERE id = $1 AND campaign_id = $2 AND status = 'validated'
         RETURNING COALESCE(sheet->>'name', '')",
    )
    .bind(character)
    .bind(campaign)
    .bind(session)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(name) = name else {
        return Ok(());
    };
    knowledge::write(
        tx,
        campaign,
        session,
        JournalKind::Fight,
        None,
        &format!("{name} est tombé."),
        true,
    )
    .await?;
    touch_character(tx, campaign, character).await?;
    crate::evening::touch(tx, campaign).await?;
    Ok(())
}

/// The fallen character's last words, said once: kept on the character
/// and read to the whole table in the journal.
///
/// # Errors
///
/// 403 `SPECTATOR`; 404 `NO_FALLEN`; 400 `INVALID_LAST_WORDS`; 409
/// `LAST_WORDS_SAID`.
pub async fn say_last_words(pool: &PgPool, player: &Player, text: &str) -> Result<(), AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let text = text.trim();
    if text.is_empty() || text.chars().count() > LAST_WORDS_MAX {
        return Err(AppError::BadRequest("INVALID_LAST_WORDS"));
    }
    let mut tx = pool.begin().await?;
    let row: Option<FallenRow> = sqlx::query_as(&format!(
        "SELECT {FALLEN_COLUMNS} FROM characters
         WHERE player_id = $1 AND status = 'dead' ORDER BY died_at DESC LIMIT 1 FOR UPDATE"
    ))
    .bind(player.id)
    .fetch_optional(&mut *tx)
    .await?;
    let f = row.map(fallen).ok_or(AppError::NotFound("NO_FALLEN"))?;
    if !f.last_words.is_empty() {
        return Err(AppError::Conflict("LAST_WORDS_SAID"));
    }
    sqlx::query("UPDATE characters SET last_words = $2 WHERE id = $1")
        .bind(f.id)
        .bind(text)
        .execute(&mut *tx)
        .await?;
    knowledge::write(
        &mut tx,
        player.campaign_id,
        f.died_session,
        JournalKind::Narration,
        None,
        &format!("{} : « {text} »", f.name),
        true,
    )
    .await?;
    touch_character(&mut tx, player.campaign_id, f.id).await?;
    crate::evening::touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(())
}

/// The XP that makes the group's level: the lowest level among the
/// characters in play (INTERPRETATION: a newcomer never outranks
/// anyone), at its threshold.
fn group_xp(rules: &RuleSystem, party_xp: &[u32]) -> u32 {
    let level = party_xp
        .iter()
        .map(|xp| level_for(rules, *xp))
        .min()
        .unwrap_or(1);
    rules
        .progression
        .levels
        .iter()
        .filter(|l| l.level <= level)
        .map(|l| l.xp)
        .max()
        .unwrap_or(0)
}

/// After a death, a new character to write: a fresh draft that will
/// enter play at the group's level.
///
/// # Errors
///
/// 403 `SPECTATOR`; 409 `NO_FALLEN` when none of the player's
/// characters fell, `CHARACTER_EXISTS` when they already have one.
pub async fn new_character(
    pool: &PgPool,
    player: &Player,
    rules: &RuleSystem,
) -> Result<(), AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let mut tx = pool.begin().await?;
    crate::campaigns::lock(&mut tx, player.campaign_id)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    let (dead, living): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*) FILTER (WHERE status = 'dead'), COUNT(*) FILTER (WHERE status <> 'dead')
         FROM characters WHERE player_id = $1",
    )
    .bind(player.id)
    .fetch_one(&mut *tx)
    .await?;
    if living > 0 {
        return Err(AppError::Conflict("CHARACTER_EXISTS"));
    }
    if dead == 0 {
        return Err(AppError::Conflict("NO_FALLEN"));
    }
    let party: Vec<u32> = play::in_play(&mut *tx, player.campaign_id)
        .await?
        .iter()
        .map(|c| c.state.as_ref().map_or(0, |s| s.total_xp))
        .collect();
    create_character_at(&mut tx, player, group_xp(rules, &party)).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use promptus_shared::story::RuleSystemRef;

    #[test]
    fn a_newcomer_starts_at_the_lowest_level_of_the_group() {
        let r = crate::content::preset(&RuleSystemRef {
            id: "corsaires".into(),
            version: 1,
        })
        .unwrap();
        let levels = &r.progression.levels;
        let l2 = levels.iter().find(|l| l.level == 2).unwrap().xp;
        let l3 = levels.iter().find(|l| l.level == 3).unwrap().xp;
        assert_eq!(group_xp(r, &[]), 0);
        assert_eq!(group_xp(r, &[l3 + 1, l2 + 1]), l2);
        assert_eq!(group_xp(r, &[l3, l3 + 2]), l3);
    }
}
