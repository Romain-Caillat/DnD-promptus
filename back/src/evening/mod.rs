//! The evening: one session of play, from the lobby to « Terminer la
//! session » (migration `011_evening.sql`).
//!
//! - [`session`]: open the lobby, start, end; the chronicle;
//! - [`scenes`]: what the GM shows — a scene, a clue, an NPC, a front's
//!   clock, the music;
//! - [`requests`]: what players ask, the GM's answer, the check rolled
//!   by the server;
//! - [`knowledge`]: what the table knows (the journal), the rulings
//!   given, what the next scenes need that the table does not know;
//! - [`spotlight`]: who has not had a moment for a while;
//! - [`feedback`]: the players' thirty seconds at the end, and what
//!   Promptus measured.
//!
//! Every write takes the campaign row lock and re-reads inside the
//! transaction (`MEMORY.md` §3), then touches the `session` topic (and
//! `world` when the world moved) so every screen refetches through the
//! API. What players receive is built by `campaigns::projection::evening`.

pub mod feedback;
pub mod knowledge;
pub mod requests;
pub mod scenes;
pub mod session;
pub mod spotlight;

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::AppError;
use crate::live::{self, Topic};

pub use session::{Music, Session, Status};

/// Tell every screen the evening moved, when `tx` commits.
///
/// # Errors
///
/// A database error.
pub async fn touch(tx: &mut Transaction<'_, Postgres>, campaign: Uuid) -> Result<(), AppError> {
    live::touch(tx, campaign, &Topic::Session).await?;
    Ok(())
}

/// Text a person typed: trimmed, at most `max` characters.
///
/// # Errors
///
/// 400 `TEXT_TOO_LONG`.
pub fn clean_text(raw: &str, max: usize) -> Result<String, AppError> {
    let text = raw.trim();
    if text.chars().count() > max {
        return Err(AppError::BadRequest("TEXT_TOO_LONG"));
    }
    Ok(text.to_string())
}

/// A moment a player had (`player_moments`): the spotlight and the
/// feedback measures read them.
///
/// # Errors
///
/// A database error.
pub async fn moment(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    player: Uuid,
    kind: MomentKind,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO player_moments (session_id, player_id, kind) VALUES ($1, $2, $3)")
        .bind(session)
        .bind(player)
        .bind(kind.as_str())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MomentKind {
    Request,
    Roll,
    Move,
    Fight,
    Spotlight,
}

impl MomentKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Roll => "roll",
            Self::Move => "move",
            Self::Fight => "fight",
            Self::Spotlight => "spotlight",
        }
    }
}
