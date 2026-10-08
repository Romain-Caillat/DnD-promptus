//! session/pair-shared-screen — the shared screen of a table: a TV in
//! the living room, or a window the GM shares on Discord (migration
//! `036_shared_screens.sql`).
//!
//! A TV opens `/tv` and says hello ([`hello`]): it gets a random token
//! (kept on the device by `auth::screen`, hashed here) and a four-letter
//! code that lives [`CODE_MINUTES`]. The GM types that code on their
//! screen ([`pair`]) and the TV is bound to the campaign until the GM
//! forgets it ([`forget`]): next evening it reconnects by itself. When
//! everyone plays remotely, the GM opens a window instead ([`window`]):
//! their own browser gets a screen paired at birth, with no code.
//!
//! A screen sees what every player may see and nothing more: its answer
//! is built by `campaigns::projection::screen`, narrowed further by what
//! the GM lets it show ([`Shows`]).

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::auth::tokens::{hash_token, random_token};
use crate::error::AppError;
use crate::live::{self, Topic};

/// How long a code shown on a TV may be typed.
pub const CODE_MINUTES: i64 = 10;
/// Characters a code is drawn from: no 0/O, 1/I/L, nothing a TV across
/// the room makes ambiguous.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const CODE_LEN: usize = 4;
/// Screens waiting for a GM at once, for the whole server: past this a
/// new one is refused (the hello route is public).
const MAX_WAITING: i64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenKind {
    /// A device that showed a code: a TV, an old laptop on the TV.
    Tv,
    /// The GM's own browser, for a window shared on Discord.
    Window,
}

impl ScreenKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Tv => "tv",
            Self::Window => "window",
        }
    }

    fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "tv" => Ok(Self::Tv),
            "window" => Ok(Self::Window),
            other => Err(AppError::Internal(format!("screen kind {other}"))),
        }
    }
}

/// One screen, as the server holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    pub id: Uuid,
    pub kind: ScreenKind,
    /// While waiting for a GM.
    pub code: Option<String>,
    pub code_expires_at: Option<DateTime<Utc>>,
    /// Once paired.
    pub campaign_id: Option<Uuid>,
    pub paired_at: Option<DateTime<Utc>>,
}

type Row = (
    Uuid,
    String,
    Option<String>,
    Option<DateTime<Utc>>,
    Option<Uuid>,
    Option<DateTime<Utc>>,
);

const COLUMNS: &str = "id, kind, code, code_expires_at, campaign_id, paired_at";

fn from_row(
    (id, kind, code, code_expires_at, campaign_id, paired_at): Row,
) -> Result<Screen, AppError> {
    Ok(Screen {
        id,
        kind: ScreenKind::parse(&kind)?,
        code,
        code_expires_at,
        campaign_id,
        paired_at,
    })
}

/// What the GM lets the shared screens of a campaign show. Each switch
/// only takes away from the players' projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Shows {
    /// The scene read aloud, its image, « Précédemment… ».
    pub scene: bool,
    /// The map as the table sees it, and the fight on it.
    pub map: bool,
    /// The party: who plays whom, their hearts.
    pub party: bool,
    /// The big moments: dice rolled, clues found, loot, the feed line.
    pub moments: bool,
}

impl Default for Shows {
    fn default() -> Self {
        Self {
            scene: true,
            map: true,
            party: true,
            moments: true,
        }
    }
}

/// What a screen holding `token` is now, if the token is one.
///
/// # Errors
///
/// A database error.
pub async fn find_by_token(
    db: impl PgExecutor<'_>,
    tokens: &[String],
) -> Result<Option<Screen>, AppError> {
    if tokens.is_empty() {
        return Ok(None);
    }
    let hashes: Vec<String> = tokens.iter().map(|t| hash_token(t)).collect();
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM shared_screens WHERE token_hash = ANY($1) LIMIT 1"
    ))
    .bind(&hashes)
    .fetch_optional(db)
    .await?;
    row.map(from_row).transpose()
}

fn new_code() -> String {
    let mut rng = rand::thread_rng();
    (0..CODE_LEN)
        .map(|_| char::from(CODE_ALPHABET[rng.gen_range(0..CODE_ALPHABET.len())]))
        .collect()
}

/// A fresh code for `screen`, retried on the rare clash with another
/// live code.
async fn give_code(pool: &PgPool, screen: Uuid) -> Result<Screen, AppError> {
    // Codes no one can type any more are cleared first, so they free
    // their letters for the unique index.
    sqlx::query(
        "UPDATE shared_screens SET code = NULL
         WHERE code IS NOT NULL AND code_expires_at < now()",
    )
    .execute(pool)
    .await?;
    for _ in 0..8 {
        let row: Result<Row, sqlx::Error> = sqlx::query_as(&format!(
            "UPDATE shared_screens
             SET code = $2, code_expires_at = now() + make_interval(mins => $3)
             WHERE id = $1 AND campaign_id IS NULL RETURNING {COLUMNS}"
        ))
        .bind(screen)
        .bind(new_code())
        .bind(i32::try_from(CODE_MINUTES).unwrap_or(10))
        .fetch_one(pool)
        .await;
        match row {
            Ok(row) => return from_row(row),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {}
            Err(e) => return Err(e.into()),
        }
    }
    Err(AppError::Internal("no free screen code".into()))
}

/// What a screen says hello with: who it is now, and a token when it is
/// a new one.
pub struct Hello {
    pub screen: Screen,
    /// Set when a new screen was made: the device must keep it.
    pub token: Option<String>,
}

/// A screen says hello. A known one is told where it stands — paired,
/// or waiting with a code that is renewed once it expired; an unknown
/// one becomes a new waiting screen with a token and a code.
///
/// # Errors
///
/// 503 `TOO_MANY_SCREENS` when too many screens already wait; a
/// database error.
pub async fn hello(pool: &PgPool, tokens: &[String]) -> Result<Hello, AppError> {
    if let Some(screen) = find_by_token(pool, tokens).await? {
        let expired = screen.campaign_id.is_none()
            && screen.code_expires_at.is_none_or(|at| at <= Utc::now());
        let screen = if expired {
            give_code(pool, screen.id).await?
        } else {
            screen
        };
        return Ok(Hello {
            screen,
            token: None,
        });
    }
    // Screens that waited for nobody for a day go, then the cap.
    sqlx::query(
        "DELETE FROM shared_screens
         WHERE campaign_id IS NULL AND created_at < now() - interval '1 day'",
    )
    .execute(pool)
    .await?;
    let waiting: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM shared_screens
         WHERE campaign_id IS NULL AND code_expires_at > now()",
    )
    .fetch_one(pool)
    .await?;
    if waiting >= MAX_WAITING {
        return Err(AppError::ServiceUnavailable("TOO_MANY_SCREENS"));
    }
    let token = random_token();
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO shared_screens (token_hash, kind) VALUES ($1, 'tv') RETURNING id",
    )
    .bind(hash_token(&token))
    .fetch_one(pool)
    .await?;
    let screen = give_code(pool, id).await?;
    Ok(Hello {
        screen,
        token: Some(token),
    })
}

/// Normalise what the GM typed: case and spaces do not matter.
fn typed_code(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(char::to_uppercase)
        .collect()
}

/// The GM pairs the TV showing `code` with `campaign` (owned: checked by
/// the route). The code is spent.
///
/// # Errors
///
/// 404 `NO_SUCH_CODE` for a code no waiting screen shows (expired,
/// already used, mistyped); a database error.
pub async fn pair(pool: &PgPool, campaign: Uuid, code: &str) -> Result<Screen, AppError> {
    let code = typed_code(code);
    if code.len() != CODE_LEN {
        return Err(AppError::NotFound("NO_SUCH_CODE"));
    }
    let mut tx = pool.begin().await?;
    let row: Option<Row> = sqlx::query_as(&format!(
        "UPDATE shared_screens
         SET campaign_id = $2, paired_at = now(), code = NULL, code_expires_at = NULL
         WHERE code = $1 AND code_expires_at > now() AND campaign_id IS NULL
         RETURNING {COLUMNS}"
    ))
    .bind(&code)
    .bind(campaign)
    .fetch_optional(&mut *tx)
    .await?;
    let row = row.ok_or(AppError::NotFound("NO_SUCH_CODE"))?;
    live::touch(&mut tx, campaign, &Topic::Screens).await?;
    tx.commit().await?;
    from_row(row)
}

/// A window for `campaign` in the GM's own browser: a screen paired at
/// birth. The campaign keeps one window: an older one is forgotten.
/// Returns the screen and the token the browser must keep.
///
/// # Errors
///
/// A database error.
pub async fn window(pool: &PgPool, campaign: Uuid) -> Result<(Screen, String), AppError> {
    let token = random_token();
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM shared_screens WHERE campaign_id = $1 AND kind = 'window'")
        .bind(campaign)
        .execute(&mut *tx)
        .await?;
    let row: Row = sqlx::query_as(&format!(
        "INSERT INTO shared_screens (token_hash, kind, campaign_id, paired_at)
         VALUES ($1, $2, $3, now()) RETURNING {COLUMNS}"
    ))
    .bind(hash_token(&token))
    .bind(ScreenKind::Window.as_str())
    .bind(campaign)
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Screens).await?;
    tx.commit().await?;
    Ok((from_row(row)?, token))
}

/// The screens paired with `campaign`, oldest first.
///
/// # Errors
///
/// A database error.
pub async fn of_campaign(pool: &PgPool, campaign: Uuid) -> Result<Vec<Screen>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM shared_screens WHERE campaign_id = $1 ORDER BY paired_at, id"
    ))
    .bind(campaign)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(from_row).collect()
}

/// The GM forgets a screen of `campaign`: its token opens nothing any
/// more, and the TV goes back to showing a code.
///
/// # Errors
///
/// 404 `NO_SUCH_SCREEN` for a screen not paired with `campaign`.
pub async fn forget(pool: &PgPool, campaign: Uuid, screen: Uuid) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let gone = sqlx::query("DELETE FROM shared_screens WHERE id = $1 AND campaign_id = $2")
        .bind(screen)
        .bind(campaign)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if gone == 0 {
        return Err(AppError::NotFound("NO_SUCH_SCREEN"));
    }
    live::touch(&mut tx, campaign, &Topic::Screens).await?;
    tx.commit().await?;
    Ok(())
}

/// What the screens of `campaign` may show.
///
/// # Errors
///
/// A database error.
pub async fn shows(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Shows, AppError> {
    let row: Option<Json<Shows>> =
        sqlx::query_scalar("SELECT shows FROM shared_screen_settings WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_optional(db)
            .await?;
    Ok(row.map(|Json(s)| s).unwrap_or_default())
}

/// The GM sets what the screens of `campaign` may show; the screens
/// refetch.
///
/// # Errors
///
/// A database error.
pub async fn set_shows(pool: &PgPool, campaign: Uuid, shows: Shows) -> Result<Shows, AppError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO shared_screen_settings (campaign_id, shows) VALUES ($1, $2)
         ON CONFLICT (campaign_id) DO UPDATE SET shows = EXCLUDED.shows",
    )
    .bind(campaign)
    .bind(Json(shows))
    .execute(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Screens).await?;
    tx.commit().await?;
    Ok(shows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_four_unambiguous_characters_and_typing_is_forgiving() {
        for _ in 0..200 {
            let c = new_code();
            assert_eq!(c.len(), CODE_LEN);
            assert!(c.bytes().all(|b| CODE_ALPHABET.contains(&b)), "{c}");
            assert!(!c.contains(['0', 'O', '1', 'I', 'L']), "{c}");
        }
        assert_eq!(typed_code(" k7-qf "), "K7QF");
        assert_eq!(typed_code("k 7 q f"), "K7QF");
    }
}
