//! engine/save-against-death, player/face-death — a character's death
//! and what comes after (migration `025_character_deaths.sql`).
//!
//! A death is always the GM's: under the `death_saves` rule the dice
//! propose it in a fight and the GM confirms it (`board::fight`,
//! `GmCommand::ConfirmDeath`), under any rule the GM may decide it for a
//! character down at 0 hit points ([`declare`]). Either way [`fall`]
//! makes it so: the character is `fallen` — out of play for good, kept
//! for the chronicle —, the death is a row of `character_deaths`, the
//! table's journal says it, the token leaves the map.
//!
//! Then the player writes their character's last words ([`last_words`]),
//! shown to the whole table, and chooses what they do next
//! ([`choose_next`]): watch the evening, make a new character — at the
//! party's level: it starts with the dead one's XP —, or wait for a hook
//! from the GM.

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::sprite::CharacterLook;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::play::{self, Actor, Kind, Record};
use super::{CharacterSheet, Player, Role, touch_character};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;
use crate::evening::knowledge::{self, JournalKind};
use crate::live::{self, Topic};

/// Longest last words, in characters: a sentence, not a speech.
pub const LAST_WORDS_MAX: usize = 280;

/// Who made the death.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    /// The death saves proposed it; the GM confirmed.
    Rules,
    /// The GM decided it.
    Gm,
}

impl Cause {
    fn as_str(self) -> &'static str {
        match self {
            Self::Rules => "rules",
            Self::Gm => "gm",
        }
    }
}

/// What the player does after their character's death.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Next {
    /// Follow the rest of the evening as a spectator.
    Watch,
    /// Make a new character, at the party's level.
    New,
    /// Wait for the GM to bring them back in (a hook).
    Hook,
}

impl Next {
    fn as_str(self) -> &'static str {
        match self {
            Self::Watch => "watch",
            Self::New => "new",
            Self::Hook => "hook",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "watch" => Some(Self::Watch),
            "new" => Some(Self::New),
            "hook" => Some(Self::Hook),
            _ => None,
        }
    }
}

/// A dead character, as its player and the GM read it. Nothing GM-only:
/// who decided it is the table's to know.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fallen {
    pub character_id: Uuid,
    pub player_id: Uuid,
    pub nickname: String,
    pub name: String,
    pub class_id: Option<String>,
    pub look: Option<CharacterLook>,
    pub level: u32,
    pub cause: String,
    pub last_words: Option<String>,
    pub next: Option<Next>,
    pub died_at: DateTime<Utc>,
}

type FallenRow = (
    Uuid,
    Uuid,
    String,
    Json<CharacterSheet>,
    i32,
    String,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
);

const FALLEN_COLUMNS: &str = "d.character_id, d.player_id, p.nickname, c.sheet, d.level, d.cause,
     d.last_words, d.next, d.died_at";

fn fallen((id, player, nickname, sheet, level, cause, words, next, died_at): FallenRow) -> Fallen {
    Fallen {
        character_id: id,
        player_id: player,
        nickname,
        name: sheet.0.name.clone(),
        class_id: sheet.0.class_id.clone(),
        look: sheet.0.look.clone(),
        level: u32::try_from(level).unwrap_or(1),
        cause,
        last_words: words,
        next: next.as_deref().and_then(Next::parse),
        died_at,
    }
}

/// The latest death of `player`'s characters, if any.
///
/// # Errors
///
/// A database error.
pub async fn latest_of(pool: &PgPool, player: Uuid) -> Result<Option<Fallen>, AppError> {
    let row: Option<FallenRow> = sqlx::query_as(&format!(
        "SELECT {FALLEN_COLUMNS}
         FROM character_deaths d
         JOIN characters c ON c.id = d.character_id
         JOIN players p ON p.id = d.player_id
         WHERE d.player_id = $1 ORDER BY d.died_at DESC LIMIT 1"
    ))
    .bind(player)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(fallen))
}

/// Every death of `campaign`, oldest first: the table's « Tombés ».
///
/// # Errors
///
/// A database error.
pub async fn of_campaign(pool: &PgPool, campaign: Uuid) -> Result<Vec<Fallen>, AppError> {
    let rows: Vec<FallenRow> = sqlx::query_as(&format!(
        "SELECT {FALLEN_COLUMNS}
         FROM character_deaths d
         JOIN characters c ON c.id = d.character_id
         JOIN players p ON p.id = d.player_id
         WHERE d.campaign_id = $1 ORDER BY d.died_at, d.character_id"
    ))
    .bind(campaign)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(fallen).collect())
}

/// Character `id` of `campaign` dies, in the caller's transaction and
/// under the campaign lock it holds: `fallen`, its death recorded and
/// logged, the table's journal told, its token off the map.
///
/// # Errors
///
/// 404 `NOT_FOUND`; 409 `CHARACTER_NOT_VALIDATED`, `NO_PLAY_SHEET`,
/// `NOT_DOWN` (above 0 hit points).
pub async fn fall(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    rules: &RuleSystem,
    id: Uuid,
    cause: Cause,
    session: Option<Uuid>,
    node: Option<&str>,
) -> Result<(), AppError> {
    let (sheet, state) = play::in_play_locked(tx, campaign, id, rules).await?;
    let c = play::combatant(rules, &sheet, &state).ok_or(AppError::Conflict("NO_PLAY_SHEET"))?;
    if c.hit_points > 0 {
        return Err(AppError::Conflict("NOT_DOWN"));
    }
    let level = c.level(rules).unwrap_or(1);
    sqlx::query("UPDATE characters SET status = 'fallen' WHERE id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "INSERT INTO character_deaths (character_id, campaign_id, player_id, session_id, cause, level, node)
         SELECT id, campaign_id, player_id, $2, $3, $4, $5 FROM characters WHERE id = $1",
    )
    .bind(id)
    .bind(session)
    .bind(cause.as_str())
    .bind(i32::try_from(level).unwrap_or(1))
    .bind(node)
    .execute(&mut **tx)
    .await?;
    let record = Record {
        kind: Kind::Death,
        label: Some(sheet.name.clone()),
        before: i32::try_from(level).unwrap_or(1),
        after: 0,
    };
    play::log(tx, campaign, id, Actor::Gm, &record).await?;
    knowledge::write(
        tx,
        campaign,
        session,
        JournalKind::Note,
        node,
        &format!("Mort de {}.", sheet.name),
        true,
    )
    .await?;
    if let Some(mut board) = crate::board::current(&mut **tx, campaign).await? {
        let token = id.to_string();
        let before = board.tokens.len();
        board.tokens.retain(|t| t.r#ref != token);
        if board.tokens.len() != before {
            crate::board::save(tx, campaign, &board).await?;
            live::touch(tx, campaign, &Topic::Map).await?;
        }
    }
    touch_character(tx, campaign, id).await?;
    crate::evening::touch(tx, campaign).await?;
    Ok(())
}

/// The GM decides that character `id`, down at 0 hit points, dies —
/// under any rule (rule 1: the GM has the last word). In a fight, the
/// fight takes it too (`Fight::confirm_death`).
///
/// # Errors
///
/// 404 when the campaign is another GM's or the character not at its
/// table; 409 `RULES_UNKNOWN` and the errors of [`fall`].
pub async fn declare(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: Uuid,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let row = owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let rules = crate::board::fight::rules_of(&row)?;
    let in_fight = crate::board::fight::confirm_death_of(&mut tx, &row, &rules, id).await?;
    if !in_fight {
        let session = crate::evening::session::current(&mut *tx, campaign)
            .await?
            .map(|s| s.id);
        fall(&mut tx, campaign, &rules, id, Cause::Gm, session, None).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// A death row: the character, its last words, the player's choice,
/// the session it happened in, the character's name.
type DeathRow<Name> = (Uuid, Option<String>, Option<String>, Option<Uuid>, Name);

/// The latest death of `player`'s characters, its row locked.
async fn locked_death(
    tx: &mut Transaction<'_, Postgres>,
    player: &Player,
) -> Result<DeathRow<String>, AppError> {
    let row: Option<DeathRow<Option<String>>> = sqlx::query_as(
        "SELECT d.character_id, d.last_words, d.next, d.session_id, c.sheet->>'name'
             FROM character_deaths d JOIN characters c ON c.id = d.character_id
             WHERE d.player_id = $1 ORDER BY d.died_at DESC LIMIT 1 FOR UPDATE OF d",
    )
    .bind(player.id)
    .fetch_optional(&mut **tx)
    .await?;
    let (id, words, next, session, name) = row.ok_or(AppError::NotFound("NO_DEATH"))?;
    Ok((id, words, next, session, name.unwrap_or_default()))
}

/// The player's last words for their dead character, once: shown to the
/// whole table in the journal, kept for the chronicle.
///
/// # Errors
///
/// 404 `NO_DEATH`; 409 `ALREADY_SAID`; 400 `INVALID_WORDS`.
pub async fn last_words(pool: &PgPool, player: &Player, text: &str) -> Result<(), AppError> {
    let words = text.trim();
    if words.is_empty()
        || words.chars().count() > LAST_WORDS_MAX
        || words.chars().any(char::is_control)
    {
        return Err(AppError::BadRequest("INVALID_WORDS"));
    }
    let mut tx = pool.begin().await?;
    campaigns::lock(&mut tx, player.campaign_id)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    let (id, said, _, session, name) = locked_death(&mut tx, player).await?;
    if said.is_some() {
        return Err(AppError::Conflict("ALREADY_SAID"));
    }
    sqlx::query(
        "UPDATE character_deaths SET last_words = $2, said_at = now() WHERE character_id = $1",
    )
    .bind(id)
    .bind(words)
    .execute(&mut *tx)
    .await?;
    knowledge::write(
        &mut tx,
        player.campaign_id,
        session,
        JournalKind::Note,
        None,
        &format!("Derniers mots de {name} : « {words} »"),
        true,
    )
    .await?;
    touch_character(&mut tx, player.campaign_id, id).await?;
    crate::evening::touch(&mut tx, player.campaign_id).await?;
    tx.commit().await?;
    Ok(())
}

/// What the player does now. `New` makes a fresh draft character that
/// joins at the dead one's XP once the GM validates it; that choice is
/// final, the other two can be changed.
///
/// # Errors
///
/// 403 `SPECTATOR`; 404 `NO_DEATH`; 409 `ALREADY_CHOSEN` after `New`,
/// `HAS_CHARACTER` when a living character already sits there.
pub async fn choose_next(pool: &PgPool, player: &Player, next: Next) -> Result<(), AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let mut tx = pool.begin().await?;
    campaigns::lock(&mut tx, player.campaign_id)
        .await?
        .ok_or(AppError::NotFound("NOT_FOUND"))?;
    let (dead, _, chosen, _, _) = locked_death(&mut tx, player).await?;
    if chosen.as_deref() == Some(Next::New.as_str()) {
        return Err(AppError::Conflict("ALREADY_CHOSEN"));
    }
    if next == Next::New {
        let living: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM characters WHERE player_id = $1 AND status <> 'fallen'",
        )
        .bind(player.id)
        .fetch_optional(&mut *tx)
        .await?;
        if living.is_some() {
            return Err(AppError::Conflict("HAS_CHARACTER"));
        }
        let xp: Option<i32> =
            sqlx::query_scalar("SELECT total_xp FROM character_play WHERE character_id = $1")
                .bind(dead)
                .fetch_optional(&mut *tx)
                .await?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO characters (campaign_id, player_id, sheet, starting_xp)
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(player.campaign_id)
        .bind(player.id)
        .bind(Json(CharacterSheet::default()))
        .bind(xp.unwrap_or(0))
        .fetch_one(&mut *tx)
        .await?;
        touch_character(&mut tx, player.campaign_id, id).await?;
    }
    sqlx::query("UPDATE character_deaths SET next = $2 WHERE character_id = $1")
        .bind(dead)
        .bind(next.as_str())
        .execute(&mut *tx)
        .await?;
    touch_character(&mut tx, player.campaign_id, dead).await?;
    tx.commit().await?;
    Ok(())
}
