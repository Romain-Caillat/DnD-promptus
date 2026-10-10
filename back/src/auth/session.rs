//! Server-side GM sessions behind an HttpOnly cookie.
//!
//! The cookie carries a random token; `gm_sessions` stores its hash
//! (`tokens::hash_token`). Signing out deletes the row, so a copied
//! cookie dies with it. The cookie is `HttpOnly` (no script reads it),
//! `SameSite=Strict` (no other site's page can make the browser send it,
//! which is the CSRF defence) and `Secure` whenever the app is served
//! over HTTPS (`PUBLIC_ORIGIN`).

use axum::http::{HeaderMap, header};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::tokens::{hash_token, random_token};
use crate::error::AppError;

pub const COOKIE_NAME: &str = "promptus_gm";
/// The cookie only travels to the API.
const COOKIE_PATH: &str = "/api";
/// A GM signs in with an emailed code about once a month.
pub const SESSION_DAYS: i64 = 30;

/// The signed-in GM, as every GM route sees them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentGm {
    pub id: Uuid,
    pub display_name: String,
}

/// Open a session for `gm_id` inside `tx`. Returns the raw token for
/// the cookie. The GM's expired sessions are swept on the way.
///
/// # Errors
///
/// Fails on a database error.
pub async fn create_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    gm_id: Uuid,
) -> Result<String, AppError> {
    sqlx::query("DELETE FROM gm_sessions WHERE gm_id = $1 AND expires_at <= now()")
        .bind(gm_id)
        .execute(&mut **tx)
        .await?;
    let token = random_token();
    let expires_at: DateTime<Utc> = Utc::now() + Duration::days(SESSION_DAYS);
    sqlx::query("INSERT INTO gm_sessions (gm_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(gm_id)
        .bind(hash_token(&token))
        .bind(expires_at)
        .execute(&mut **tx)
        .await?;
    Ok(token)
}

/// The GM behind a session token, if the session exists and has not
/// expired.
///
/// # Errors
///
/// Fails on a database error.
pub async fn find_gm(pool: &PgPool, token: &str) -> Result<Option<CurrentGm>, AppError> {
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT g.id, g.display_name
         FROM gm_sessions s JOIN gms g ON g.id = s.gm_id
         WHERE s.token_hash = $1 AND s.expires_at > now()",
    )
    .bind(hash_token(token))
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id, display_name)| CurrentGm { id, display_name }))
}

/// End the session behind `token`. Idempotent.
///
/// # Errors
///
/// Fails on a database error.
pub async fn delete(pool: &PgPool, token: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM gm_sessions WHERE token_hash = $1")
        .bind(hash_token(token))
        .execute(pool)
        .await?;
    Ok(())
}

/// The session token sent by the browser, if any.
pub fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    let prefix = format!("{COOKIE_NAME}=");
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|part| part.trim().strip_prefix(prefix.as_str()))
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

/// `Set-Cookie` value that stores a fresh session.
pub fn set_cookie(token: &str, secure: bool) -> String {
    let max_age = SESSION_DAYS * 24 * 60 * 60;
    cookie(token, max_age, secure)
}

/// `Set-Cookie` value that makes the browser forget the session.
pub fn clear_cookie(secure: bool) -> String {
    cookie("", 0, secure)
}

fn cookie(value: &str, max_age: i64, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!(
        "{COOKIE_NAME}={value}; Path={COOKIE_PATH}; HttpOnly; SameSite=Strict; Max-Age={max_age}{secure}"
    )
}
