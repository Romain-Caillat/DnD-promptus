//! Signing in with an email and a code sent to it.
//!
//! The GM types their address and receives a six-digit code valid
//! [`CODE_MINUTES`] minutes; typing it back opens a session. The first
//! time an address signs in, the GM also gives a display name and the
//! account is created then: anyone may become a GM, and what a GM owns
//! (campaigns) stays theirs alone (`auth::guard::owned_by`).
//!
//! Six digits are a small space, so a code dies after [`MAX_ATTEMPTS`]
//! wrong tries, asking again replaces it, and a new one cannot be asked
//! for the same address within [`RESEND_SECONDS`]. Every wrong answer
//! looks the same (`INVALID_CODE`), whether the code is wrong, spent,
//! expired or never asked for.

use chrono::{Duration, Utc};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use super::mailer::Mailer;
use super::session::{self, CurrentGm};
use super::tokens::hash_token;
use crate::error::AppError;

pub const CODE_MINUTES: i64 = 10;
pub const MAX_ATTEMPTS: i32 = 5;
pub const RESEND_SECONDS: i64 = 60;
/// Codes asked for in the last hour, all addresses together, beyond
/// which the server stops sending: the relay is a shared mailbox.
pub const MAX_CODES_PER_HOUR: i64 = 30;
pub const MAX_DISPLAY_NAME_CHARS: usize = 60;
const MAX_EMAIL_CHARS: usize = 254;

fn invalid_code() -> AppError {
    AppError::Unauthorized("INVALID_CODE")
}

/// The address as stored: trimmed and lower-cased. A plausibility check
/// only — the code that reaches it is the real proof.
///
/// # Errors
///
/// 400 `INVALID_EMAIL`.
pub fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_lowercase();
    let plausible = email.chars().count() <= MAX_EMAIL_CHARS
        && !email.chars().any(|c| c.is_whitespace() || c.is_control())
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && !domain.contains('@')
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        });
    if plausible {
        Ok(email)
    } else {
        Err(AppError::BadRequest("INVALID_EMAIL"))
    }
}

fn clean_display_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_DISPLAY_NAME_CHARS {
        return Err(AppError::BadRequest("INVALID_DISPLAY_NAME"));
    }
    Ok(name.to_string())
}

fn code_hash(email: &str, code: &str) -> String {
    hash_token(&format!("{email}:{code}"))
}

/// Send a fresh code to `email`, replacing any earlier one.
///
/// # Errors
///
/// 400 `INVALID_EMAIL`; 429 `CODE_TOO_SOON` within [`RESEND_SECONDS`] of
/// the last code to this address, `TOO_MANY_CODES` past
/// [`MAX_CODES_PER_HOUR`]; 503 `EMAIL_UNAVAILABLE` when the email could
/// not leave (no code is left behind then).
pub async fn request(pool: &PgPool, mailer: &dyn Mailer, email: &str) -> Result<(), AppError> {
    let email = normalize_email(email)?;
    let recent: Option<bool> = sqlx::query_scalar(
        "SELECT created_at > now() - make_interval(secs => $2)
         FROM gm_sign_in_codes WHERE email = $1",
    )
    .bind(&email)
    .bind(RESEND_SECONDS as f64)
    .fetch_optional(pool)
    .await?;
    if recent == Some(true) {
        return Err(AppError::TooManyRequests("CODE_TOO_SOON"));
    }
    let last_hour: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM gm_sign_in_codes WHERE created_at > now() - interval '1 hour'",
    )
    .fetch_one(pool)
    .await?;
    if last_hour >= MAX_CODES_PER_HOUR {
        tracing::warn!("sign-in codes refused: {last_hour} asked for in the last hour");
        return Err(AppError::TooManyRequests("TOO_MANY_CODES"));
    }

    let code = format!("{:06}", rand::rngs::OsRng.gen_range(0..1_000_000));
    sqlx::query(
        "INSERT INTO gm_sign_in_codes (email, code_hash, expires_at) VALUES ($1, $2, $3)
         ON CONFLICT (email) DO UPDATE
         SET code_hash = EXCLUDED.code_hash, attempts = 0,
             created_at = now(), expires_at = EXCLUDED.expires_at",
    )
    .bind(&email)
    .bind(code_hash(&email, &code))
    .bind(Utc::now() + Duration::minutes(CODE_MINUTES))
    .execute(pool)
    .await?;

    if let Err(e) = mailer.send_code(&email, &code).await {
        tracing::error!("sign-in code not sent: {e}");
        sqlx::query("DELETE FROM gm_sign_in_codes WHERE email = $1")
            .bind(&email)
            .execute(pool)
            .await?;
        return Err(AppError::ServiceUnavailable("EMAIL_UNAVAILABLE"));
    }
    Ok(())
}

/// What a right code opens.
pub struct SignedIn {
    pub gm: CurrentGm,
    pub token: String,
    /// The account was created by this sign-in.
    pub created: bool,
}

/// Check `code` for `email` and open a session. An address that has no
/// account yet gets one, named `display_name`.
///
/// # Errors
///
/// 400 `INVALID_EMAIL`, `INVALID_DISPLAY_NAME`; 401 `INVALID_CODE`; 409
/// `DISPLAY_NAME_REQUIRED` when the code is right but the address has no
/// account and no name was given — the code is kept, to be sent again
/// with the name.
pub async fn verify(
    pool: &PgPool,
    email: &str,
    code: &str,
    display_name: Option<&str>,
) -> Result<SignedIn, AppError> {
    let email = normalize_email(email)?;
    let code = code.trim();
    let mut tx = pool.begin().await?;
    // Locked, so two answers racing on one code are counted one by one.
    let row: Option<(String, i32, bool)> = sqlx::query_as(
        "SELECT code_hash, attempts, expires_at <= now()
         FROM gm_sign_in_codes WHERE email = $1 FOR UPDATE",
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((stored, attempts, expired)) = row else {
        return Err(invalid_code());
    };
    if expired || attempts >= MAX_ATTEMPTS {
        sqlx::query("DELETE FROM gm_sign_in_codes WHERE email = $1")
            .bind(&email)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(invalid_code());
    }
    if stored != code_hash(&email, code) {
        sqlx::query("UPDATE gm_sign_in_codes SET attempts = attempts + 1 WHERE email = $1")
            .bind(&email)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(invalid_code());
    }

    let existing: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, display_name FROM gms WHERE email = $1")
            .bind(&email)
            .fetch_optional(&mut *tx)
            .await?;
    let (gm, created) = if let Some((id, display_name)) = existing {
        (CurrentGm { id, display_name }, false)
    } else {
        let Some(name) = display_name else {
            return Err(AppError::Conflict("DISPLAY_NAME_REQUIRED"));
        };
        let display_name = clean_display_name(name)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO gms (display_name, email) VALUES ($1, $2) RETURNING id",
        )
        .bind(&display_name)
        .bind(&email)
        .fetch_one(&mut *tx)
        .await?;
        (CurrentGm { id, display_name }, true)
    };

    sqlx::query("DELETE FROM gm_sign_in_codes WHERE email = $1")
        .bind(&email)
        .execute(&mut *tx)
        .await?;
    let token = session::create_in_tx(&mut tx, gm.id).await?;
    tx.commit().await?;
    if created {
        tracing::info!("new GM account created");
    }
    Ok(SignedIn { gm, token, created })
}

#[cfg(test)]
mod tests {
    use super::normalize_email;

    #[test]
    fn emails_are_trimmed_and_lower_cased() {
        assert_eq!(
            normalize_email("  Romain@Example.ORG ").unwrap(),
            "romain@example.org"
        );
    }

    #[test]
    fn implausible_emails_are_refused() {
        for bad in [
            "",
            "romain",
            "@example.org",
            "romain@",
            "romain@example",
            "a b@c.d",
            "a@b@c.d",
            "a@.c",
            "a@c.",
        ] {
            assert!(normalize_email(bad).is_err(), "{bad:?} accepted");
        }
    }
}
