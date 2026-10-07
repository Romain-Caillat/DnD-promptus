//! The shared screen (`session/pair-shared-screen`, planche « Lancer »):
//! a TV in the room, or a window the GM shares on Discord. It joins the
//! table as a spectator seat marked `screen`, so everything it shows
//! comes from the player projection a spectator gets — never a GM note,
//! a monster's hit points or a co-GM proposal.
//!
//! A TV opens `/tv`: the server gives it a four-character code and a
//! secret; the GM types the code (or scans the QR), the server seats the
//! screen and leaves its device token for the TV to collect with its
//! secret. A window the GM opens is paired at once.

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{PLAYER_COLUMNS, Player, PlayerRow, player_from_row};
use crate::auth::tokens::{hash_token, random_token};
use crate::error::AppError;
use crate::live::{self, Topic};

/// How long a TV's code waits for the GM.
const CODE_MINUTES: i64 = 15;

/// Letters and digits that read the same from a sofa: no 0/O, 1/I.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// A TV waiting to be paired.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pairing {
    pub code: String,
    /// The TV's own key to ask whether it was paired.
    pub secret: String,
}

/// What a waiting TV learns when it asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairingState {
    Waiting {
        code: String,
    },
    /// Paired: the seat's device token, handed over once.
    Paired {
        campaign: Uuid,
        token: String,
    },
}

/// A screen seated at a table, for the GM.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screen {
    pub id: Uuid,
    pub name: String,
    pub paired_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

fn new_code() -> String {
    let mut rng = rand::rngs::OsRng;
    (0..4)
        .map(|_| char::from(CODE_ALPHABET[rng.gen_range(0..CODE_ALPHABET.len())]))
        .collect()
}

async fn forget_stale(tx: &mut Transaction<'_, Postgres>) -> Result<(), AppError> {
    sqlx::query("DELETE FROM tv_pairings WHERE created_at < now() - make_interval(mins => $1)")
        .bind(i32::try_from(CODE_MINUTES).unwrap_or(15))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// A TV asks for a code to show.
///
/// # Errors
///
/// A database error.
pub async fn start(pool: &PgPool) -> Result<Pairing, AppError> {
    let mut tx = pool.begin().await?;
    forget_stale(&mut tx).await?;
    let secret = random_token();
    // A code may collide with one still waiting: draw again.
    for _ in 0..20 {
        let code = new_code();
        let inserted = sqlx::query(
            "INSERT INTO tv_pairings (secret_hash, code) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(hash_token(&secret))
        .bind(&code)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if inserted == 1 {
            tx.commit().await?;
            return Ok(Pairing { code, secret });
        }
    }
    Err(AppError::Conflict("TV_BUSY"))
}

/// Where a TV's pairing stands. A paired TV collects its token once.
///
/// # Errors
///
/// 404 `TV_PAIRING_UNKNOWN` for an unknown or expired secret.
pub async fn poll(pool: &PgPool, secret: &str) -> Result<PairingState, AppError> {
    let mut tx = pool.begin().await?;
    forget_stale(&mut tx).await?;
    let row: Option<(String, Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT code, campaign_id, player_token FROM tv_pairings WHERE secret_hash = $1 FOR UPDATE",
    )
    .bind(hash_token(secret))
    .fetch_optional(&mut *tx)
    .await?;
    let state = match row {
        None => return Err(AppError::NotFound("TV_PAIRING_UNKNOWN")),
        Some((code, None, _)) => PairingState::Waiting { code },
        Some((_, Some(campaign), Some(token))) => {
            sqlx::query("DELETE FROM tv_pairings WHERE secret_hash = $1")
                .bind(hash_token(secret))
                .execute(&mut *tx)
                .await?;
            PairingState::Paired { campaign, token }
        }
        Some(_) => return Err(AppError::internal("tv pairing", "paired without a token")),
    };
    tx.commit().await?;
    Ok(state)
}

/// The waiting code of `secret`, for its QR.
///
/// # Errors
///
/// 404 `TV_PAIRING_UNKNOWN`.
pub async fn code_of(pool: &PgPool, secret: &str) -> Result<String, AppError> {
    sqlx::query_scalar(
        "SELECT code FROM tv_pairings WHERE secret_hash = $1 AND campaign_id IS NULL",
    )
    .bind(hash_token(secret))
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("TV_PAIRING_UNKNOWN"))
}

/// Seat a screen at `campaign`: a spectator marked `screen`, named
/// « TV », « TV 2 »… Returns its device token.
async fn seat(tx: &mut Transaction<'_, Postgres>, campaign: Uuid) -> Result<String, AppError> {
    let taken: Vec<String> =
        sqlx::query_scalar("SELECT lower(nickname) FROM players WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_all(&mut **tx)
            .await?;
    let name = (1..)
        .map(|n| {
            if n == 1 {
                "TV".to_string()
            } else {
                format!("TV {n}")
            }
        })
        .find(|n| !taken.contains(&n.to_lowercase()))
        .unwrap_or_default();
    let token = random_token();
    let row: PlayerRow = sqlx::query_as(&format!(
        "INSERT INTO players (campaign_id, nickname, role, token_hash, screen)
         VALUES ($1, $2, 'spectator', $3, true) RETURNING {PLAYER_COLUMNS}"
    ))
    .bind(campaign)
    .bind(&name)
    .bind(hash_token(&token))
    .fetch_one(&mut **tx)
    .await?;
    let _: Player = player_from_row(row)?;
    live::touch(tx, campaign, &Topic::Table).await?;
    Ok(token)
}

/// The GM types the code a TV shows. The caller has checked the GM owns
/// `campaign`.
///
/// # Errors
///
/// 404 `TV_CODE_UNKNOWN` when no TV waits with that code.
pub async fn pair(pool: &PgPool, campaign: Uuid, code: &str) -> Result<(), AppError> {
    let code = code.trim().to_uppercase();
    let mut tx = pool.begin().await?;
    forget_stale(&mut tx).await?;
    let waiting: Option<String> = sqlx::query_scalar(
        "SELECT secret_hash FROM tv_pairings WHERE code = $1 AND campaign_id IS NULL FOR UPDATE",
    )
    .bind(&code)
    .fetch_optional(&mut *tx)
    .await?;
    let secret_hash = waiting.ok_or(AppError::NotFound("TV_CODE_UNKNOWN"))?;
    let token = seat(&mut tx, campaign).await?;
    sqlx::query(
        "UPDATE tv_pairings SET campaign_id = $2, player_token = $3 WHERE secret_hash = $1",
    )
    .bind(&secret_hash)
    .bind(campaign)
    .bind(&token)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// A window the GM opens to share on Discord: paired at once; its secret
/// goes in the address the GM's browser opens.
///
/// # Errors
///
/// A database error.
pub async fn open_window(pool: &PgPool, campaign: Uuid) -> Result<String, AppError> {
    let mut tx = pool.begin().await?;
    forget_stale(&mut tx).await?;
    let token = seat(&mut tx, campaign).await?;
    let secret = random_token();
    // Its code is never shown; it only has to be well formed.
    sqlx::query(
        "INSERT INTO tv_pairings (secret_hash, code, campaign_id, player_token)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(hash_token(&secret))
    .bind(new_code())
    .bind(campaign)
    .bind(&token)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(secret)
}

/// The screens seated at `campaign`.
///
/// # Errors
///
/// A database error.
pub async fn list(pool: &PgPool, campaign: Uuid) -> Result<Vec<Screen>, AppError> {
    let rows: Vec<(Uuid, String, DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, nickname, created_at, last_seen_at FROM players
         WHERE campaign_id = $1 AND screen ORDER BY created_at",
    )
    .bind(campaign)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, paired_at, last_seen_at)| Screen {
            id,
            name,
            paired_at,
            last_seen_at,
        })
        .collect())
}

/// Forget a screen: its token stops working.
///
/// # Errors
///
/// 404 `NOT_FOUND` when `screen` is no screen of `campaign`.
pub async fn forget(pool: &PgPool, campaign: Uuid, screen: Uuid) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let gone = sqlx::query("DELETE FROM players WHERE id = $1 AND campaign_id = $2 AND screen")
        .bind(screen)
        .bind(campaign)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if gone == 0 {
        return Err(AppError::NotFound("NOT_FOUND"));
    }
    live::touch(&mut tx, campaign, &Topic::Table).await?;
    tx.commit().await?;
    Ok(())
}

/// The QR a waiting TV shows: the address where the GM, signed in on a
/// phone, pairs it.
#[must_use]
pub fn qr_svg(url: &str) -> String {
    match qrcode::QrCode::new(url.as_bytes()) {
        Ok(code) => code
            .render::<qrcode::render::svg::Color<'_>>()
            .min_dimensions(240, 240)
            .quiet_zone(true)
            .build(),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_read_from_a_sofa() {
        for _ in 0..200 {
            let c = new_code();
            assert_eq!(c.len(), 4);
            assert!(c.bytes().all(|b| CODE_ALPHABET.contains(&b)), "{c}");
            assert!(!c.contains(['0', 'O', '1', 'I']), "{c}");
        }
    }

    #[test]
    fn the_qr_is_an_svg() {
        let svg = qr_svg("https://promptus.example/tv/jumeler?code=K7QF");
        assert!(svg.starts_with("<?xml") || svg.starts_with("<svg"), "{svg}");
        assert!(svg.contains("</svg>"));
    }
}
