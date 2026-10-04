//! GM invitations: how every GM after the first one gets an account.
//!
//! A signed-in GM mints a code (shown once, stored hashed), valid seven
//! days and for one account. The invited person types it, or opens the
//! link carrying it, on the account creation page. Only the GM who
//! minted an invitation sees or revokes it.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::guard::{CurrentGm, OwnedByGm, owned_by};
use super::tokens::{hash_token, random_token};
use crate::error::AppError;

pub const INVITE_DAYS: i64 = 7;

/// A pending invitation, as its GM sees it. Never the code.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(skip)]
    pub created_by: Uuid,
}

impl OwnedByGm for Invite {
    fn owner_id(&self) -> Uuid {
        self.created_by
    }
}

/// A freshly minted invitation: the only time the code is shown.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MintedInvite {
    #[serde(flatten)]
    pub invite: Invite,
    pub code: String,
}

type InviteRow = (Uuid, DateTime<Utc>, DateTime<Utc>, Uuid);

fn from_row((id, created_at, expires_at, created_by): InviteRow) -> Invite {
    Invite {
        id,
        created_at,
        expires_at,
        created_by,
    }
}

/// Mint an invitation for `gm`.
///
/// # Errors
///
/// Fails on a database error.
pub async fn create(pool: &PgPool, gm: &CurrentGm) -> Result<MintedInvite, AppError> {
    let code = random_token();
    let expires_at = Utc::now() + Duration::days(INVITE_DAYS);
    let row: InviteRow = sqlx::query_as(
        "INSERT INTO gm_invites (token_hash, created_by, expires_at) VALUES ($1, $2, $3)
         RETURNING id, created_at, expires_at, created_by",
    )
    .bind(hash_token(&code))
    .bind(gm.id)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;
    Ok(MintedInvite {
        invite: from_row(row),
        code,
    })
}

/// `gm`'s invitations still waiting to be used, newest first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list_pending(pool: &PgPool, gm: &CurrentGm) -> Result<Vec<Invite>, AppError> {
    let rows: Vec<InviteRow> = sqlx::query_as(
        "SELECT id, created_at, expires_at, created_by FROM gm_invites
         WHERE created_by = $1 AND used_at IS NULL AND expires_at > now()
         ORDER BY created_at DESC",
    )
    .bind(gm.id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(from_row).collect())
}

/// Revoke one of `gm`'s unused invitations.
///
/// # Errors
///
/// 404 `NOT_FOUND` when it does not exist, was already used, or belongs
/// to another GM.
pub async fn revoke(pool: &PgPool, gm: &CurrentGm, id: Uuid) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let row: Option<InviteRow> = sqlx::query_as(
        "SELECT id, created_at, expires_at, created_by FROM gm_invites
         WHERE id = $1 AND used_at IS NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let invite = owned_by(row.map(from_row), gm)?;
    sqlx::query("DELETE FROM gm_invites WHERE id = $1")
        .bind(invite.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// The id of the usable invitation behind `code`, if any.
///
/// # Errors
///
/// Fails on a database error.
pub async fn find_usable(pool: &PgPool, code: &str) -> Result<Option<Uuid>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM gm_invites
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()",
    )
    .bind(hash_token(code))
    .fetch_optional(pool)
    .await?)
}

/// Spend invitation `id` on the account `gm_id`, inside the transaction
/// that creates it. `false` when it was used, revoked or expired in the
/// meantime: the caller must then roll back.
///
/// # Errors
///
/// Fails on a database error.
pub async fn spend_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    gm_id: Uuid,
) -> Result<bool, AppError> {
    let spent = sqlx::query(
        "UPDATE gm_invites SET used_at = now(), used_by = $2
         WHERE id = $1 AND used_at IS NULL AND expires_at > now()",
    )
    .bind(id)
    .bind(gm_id)
    .execute(&mut **tx)
    .await?;
    Ok(spent.rows_affected() == 1)
}
