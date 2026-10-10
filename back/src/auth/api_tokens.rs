//! platform/connect-claude-mcp — a GM's personal access tokens.
//!
//! A GM mints a named token from their space (shown once, stored
//! hashed) and hands it to a program on their own machine — the local
//! MCP server Claude launches (`mcp/`, `docs/claude-mcp.md`). That
//! program sends it as `Authorization: Bearer <token>` and reaches the
//! routes of [`TOKEN_ROUTES`] only, as the GM who minted it.
//!
//! The lookup is by the SHA-256 of the presented secret on a unique
//! index (`tokens::hash_token`): the secret itself is never compared
//! byte by byte, so its timing reveals nothing about a stored token.
//! Each accepted call stamps `last_used_at`; a revoked token is refused
//! at once (401), like an unknown one.

use axum::http::Method;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::guard::{CurrentGm, OwnedByGm, owned_by};
use super::tokens::{hash_token, random_token};
use crate::error::AppError;

/// What a token reaches: campaign preparation only, read and write,
/// with the same checks as the review screen. Paths are the router's
/// own (`MatchedPath`). Creating a campaign from a YAML file is in: the
/// GM may write a campaign with Claude and have it land as a new one.
///
/// Deliberately **not** here: signing in and out, accounts, GM
/// invitations, the tokens themselves (a token never mints a token),
/// creating an empty campaign, archiving or setting one, declaring it playable
/// (`story/validate` is the GM's decision — the validator's report
/// already comes back with every read and edit), the table and its
/// players, the live session and its socket, the player routes, and
/// every route that spends AI budget (generation, workshop, maps
/// generated, images, the co-GM).
pub const TOKEN_ROUTES: &[(&str, &str)] = &[
    ("GET", "/api/campaigns"),
    ("POST", "/api/campaigns/import"),
    ("GET", "/api/campaigns/{id}"),
    ("GET", "/api/campaigns/{id}/export"),
    ("PUT", "/api/campaigns/{id}/import"),
    ("POST", "/api/campaigns/{id}/story/edits"),
    ("GET", "/api/campaigns/{id}/readiness"),
];

/// Whether a token may call `method` on the route matched as `path`.
pub fn allows(method: &Method, path: Option<&str>) -> bool {
    let Some(path) = path else { return false };
    TOKEN_ROUTES
        .iter()
        .any(|(m, p)| *p == path && method.as_str() == *m)
}

/// What the token secrets start with, so one is recognised in a config
/// file or a leak scanner.
pub const TOKEN_PREFIX: &str = "promptus_";
/// A GM's live tokens at most: one per machine or tool is plenty.
pub const MAX_TOKENS: i64 = 20;
pub const MAX_NAME_CHARS: usize = 60;

/// A token as its GM sees it in the list. Never the secret.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiToken {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub gm_id: Uuid,
}

impl OwnedByGm for ApiToken {
    fn owner_id(&self) -> Uuid {
        self.gm_id
    }
}

/// A freshly minted token: the only time the secret is shown.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MintedToken {
    #[serde(flatten)]
    pub token: ApiToken,
    pub secret: String,
}

type TokenRow = (Uuid, String, DateTime<Utc>, Option<DateTime<Utc>>, Uuid);

fn from_row((id, name, created_at, last_used_at, gm_id): TokenRow) -> ApiToken {
    ApiToken {
        id,
        name,
        created_at,
        last_used_at,
        gm_id,
    }
}

fn clean_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(AppError::BadRequest("INVALID_TOKEN_NAME"));
    }
    Ok(name.to_string())
}

/// Mint a token named `name` for `gm`.
///
/// # Errors
///
/// 400 `INVALID_TOKEN_NAME` (blank or over 60 characters); 409
/// `TOO_MANY_TOKENS` past [`MAX_TOKENS`] live ones; a database error.
pub async fn create(pool: &PgPool, gm: &CurrentGm, name: &str) -> Result<MintedToken, AppError> {
    let name = clean_name(name)?;
    let mut tx = pool.begin().await?;
    // Serialises a GM's own mints, so the cap holds under a double click.
    sqlx::query("SELECT id FROM gms WHERE id = $1 FOR UPDATE")
        .bind(gm.id)
        .execute(&mut *tx)
        .await?;
    let live: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM gm_api_tokens WHERE gm_id = $1 AND revoked_at IS NULL",
    )
    .bind(gm.id)
    .fetch_one(&mut *tx)
    .await?;
    if live >= MAX_TOKENS {
        return Err(AppError::Conflict("TOO_MANY_TOKENS"));
    }
    let secret = format!("{TOKEN_PREFIX}{}", random_token());
    let row: TokenRow = sqlx::query_as(
        "INSERT INTO gm_api_tokens (gm_id, name, token_hash) VALUES ($1, $2, $3)
         RETURNING id, name, created_at, last_used_at, gm_id",
    )
    .bind(gm.id)
    .bind(&name)
    .bind(hash_token(&secret))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(MintedToken {
        token: from_row(row),
        secret,
    })
}

/// `gm`'s live tokens, newest first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(pool: &PgPool, gm: &CurrentGm) -> Result<Vec<ApiToken>, AppError> {
    let rows: Vec<TokenRow> = sqlx::query_as(
        "SELECT id, name, created_at, last_used_at, gm_id FROM gm_api_tokens
         WHERE gm_id = $1 AND revoked_at IS NULL
         ORDER BY created_at DESC, id",
    )
    .bind(gm.id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(from_row).collect())
}

/// Revoke one of `gm`'s live tokens: it is refused from the next call.
///
/// # Errors
///
/// 404 `NOT_FOUND` when it does not exist, is already revoked, or
/// belongs to another GM.
pub async fn revoke(pool: &PgPool, gm: &CurrentGm, id: Uuid) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let row: Option<TokenRow> = sqlx::query_as(
        "SELECT id, name, created_at, last_used_at, gm_id FROM gm_api_tokens
         WHERE id = $1 AND revoked_at IS NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let token = owned_by(row.map(from_row), gm)?;
    sqlx::query("UPDATE gm_api_tokens SET revoked_at = now() WHERE id = $1")
        .bind(token.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// The GM behind a live token, stamping its last use; `None` for an
/// unknown or revoked one.
///
/// # Errors
///
/// Fails on a database error.
pub async fn find_gm(pool: &PgPool, secret: &str) -> Result<Option<CurrentGm>, AppError> {
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "UPDATE gm_api_tokens t SET last_used_at = now()
         FROM gms g
         WHERE t.token_hash = $1 AND t.revoked_at IS NULL AND g.id = t.gm_id
         RETURNING g.id, g.display_name",
    )
    .bind(hash_token(secret))
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id, display_name)| CurrentGm { id, display_name }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_listed_preparation_routes_are_allowed() {
        assert!(allows(&Method::GET, Some("/api/campaigns")));
        assert!(allows(
            &Method::POST,
            Some("/api/campaigns/{id}/story/edits")
        ));
        assert!(allows(&Method::PUT, Some("/api/campaigns/{id}/import")));
        // Same path, another method: creating a campaign is not prep.
        assert!(!allows(&Method::POST, Some("/api/campaigns")));
        assert!(!allows(
            &Method::POST,
            Some("/api/campaigns/{id}/story/validate")
        ));
        assert!(!allows(&Method::POST, Some("/api/gm-tokens")));
        assert!(!allows(&Method::GET, None));
    }

    #[test]
    fn names_are_trimmed_and_bounded() {
        assert_eq!(clean_name("  Mac  ").unwrap(), "Mac");
        assert!(clean_name("   ").is_err());
        assert!(clean_name(&"x".repeat(MAX_NAME_CHARS + 1)).is_err());
    }
}
