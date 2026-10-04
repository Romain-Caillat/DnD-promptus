//! Creating a GM account and signing in, on top of the ceremonies.

use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, Passkey, PublicKeyCredential, RegisterPublicKeyCredential,
};

use super::invites;
use super::passkeys::{Admission, Challenge, PendingAccount, credential_key};
use super::session::{self, CurrentGm};
use super::setup;
use crate::error::AppError;
use crate::state::Auth;

pub const MAX_DISPLAY_NAME_CHARS: usize = 60;

/// A wrong, spent, expired or revoked code — and a setup code once setup
/// has closed — all look the same.
fn invalid_code() -> AppError {
    AppError::Forbidden("INVALID_REGISTRATION_CODE")
}

fn clean_display_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_DISPLAY_NAME_CHARS {
        return Err(AppError::BadRequest("INVALID_DISPLAY_NAME"));
    }
    Ok(name.to_string())
}

/// Step 1 of creating an account: check the code (the setup code while
/// no GM exists, otherwise an invitation) and hand out the passkey
/// registration challenge.
///
/// # Errors
///
/// 400 `INVALID_DISPLAY_NAME`; 403 `INVALID_REGISTRATION_CODE`.
pub async fn begin_registration(
    pool: &PgPool,
    auth: &Auth,
    code: &str,
    display_name: &str,
) -> Result<Challenge<CreationChallengeResponse>, AppError> {
    let display_name = clean_display_name(display_name)?;
    let code = code.trim();
    let admission = if auth.setup.matches(code) {
        if setup::gm_count(pool).await? > 0 {
            auth.setup.close();
            return Err(invalid_code());
        }
        Admission::Setup
    } else if let Some(id) = invites::find_usable(pool, code).await? {
        Admission::Invite(id)
    } else {
        return Err(invalid_code());
    };
    auth.passkeys.begin_registration(PendingAccount {
        gm_id: Uuid::new_v4(),
        display_name,
        admission,
    })
}

/// Step 2: the passkey came back. Creates, in one transaction, the GM,
/// their passkey and a session; spends the invitation or closes setup.
/// Returns the GM and the session token.
///
/// # Errors
///
/// 401 `INVALID_PASSKEY` on an unknown, spent or forged answer; 403
/// `INVALID_REGISTRATION_CODE` when the code stopped being valid in the
/// meantime (another account took the setup, the invitation was used or
/// revoked); 400 `PASSKEY_ALREADY_REGISTERED`.
pub async fn finish_registration(
    pool: &PgPool,
    auth: &Auth,
    ceremony_id: &str,
    credential: &RegisterPublicKeyCredential,
) -> Result<(CurrentGm, String), AppError> {
    let (account, passkey) = auth.passkeys.finish_registration(ceremony_id, credential)?;
    let mut tx = pool.begin().await?;
    if account.admission == Admission::Setup {
        // Serialises account creation: a racing setup (or an invited
        // account) waits here, then the count below sees it.
        sqlx::query("LOCK TABLE gms IN SHARE ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await?;
        let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gms")
            .fetch_one(&mut *tx)
            .await?;
        if existing > 0 {
            auth.setup.close();
            return Err(invalid_code());
        }
    }
    sqlx::query("INSERT INTO gms (id, display_name) VALUES ($1, $2)")
        .bind(account.gm_id)
        .bind(&account.display_name)
        .execute(&mut *tx)
        .await?;
    if let Admission::Invite(invite_id) = account.admission
        && !invites::spend_in_tx(&mut tx, invite_id, account.gm_id).await?
    {
        return Err(invalid_code());
    }
    insert_passkey(&mut tx, account.gm_id, &passkey).await?;
    let token = session::create_in_tx(&mut tx, account.gm_id).await?;
    tx.commit().await?;
    if account.admission == Admission::Setup {
        auth.setup.close();
        tracing::info!("first GM account created; setup is closed");
    }
    Ok((
        CurrentGm {
            id: account.gm_id,
            display_name: account.display_name,
        },
        token,
    ))
}

async fn insert_passkey(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    gm_id: Uuid,
    passkey: &Passkey,
) -> Result<(), AppError> {
    let json =
        serde_json::to_value(passkey).map_err(|e| AppError::internal("serialize passkey", e))?;
    let inserted = sqlx::query(
        "INSERT INTO gm_passkeys (gm_id, credential_id, passkey) VALUES ($1, $2, $3)
         ON CONFLICT (credential_id) DO NOTHING",
    )
    .bind(gm_id)
    .bind(credential_key(passkey.cred_id().as_ref()))
    .bind(json)
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(AppError::BadRequest("PASSKEY_ALREADY_REGISTERED"));
    }
    Ok(())
}

/// Sign in with a passkey: whoever's credential signed the challenge
/// gets a session. Returns the GM and the session token.
///
/// # Errors
///
/// 401 `INVALID_PASSKEY` on an unknown, spent or forged answer, a
/// credential no GM holds, or a missing user verification.
pub async fn finish_sign_in(
    pool: &PgPool,
    auth: &Auth,
    ceremony_id: &str,
    credential: &PublicKeyCredential,
) -> Result<(CurrentGm, String), AppError> {
    let (gm_id, state) = auth.passkeys.identify(ceremony_id, credential)?;
    let mut tx = pool.begin().await?;
    // The row is locked so two sign-ins with the same key cannot both
    // move the signature counter from the same value.
    let row: Option<(Uuid, serde_json::Value, String)> = sqlx::query_as(
        "SELECT p.id, p.passkey, g.display_name
         FROM gm_passkeys p JOIN gms g ON g.id = p.gm_id
         WHERE p.gm_id = $1 AND p.credential_id = $2
         FOR UPDATE OF p",
    )
    .bind(gm_id)
    .bind(credential_key(credential.raw_id.as_ref()))
    .fetch_optional(&mut *tx)
    .await?;
    let Some((passkey_id, stored, display_name)) = row else {
        tracing::info!("passkey sign-in refused: no such credential for this GM");
        return Err(AppError::Unauthorized("INVALID_PASSKEY"));
    };
    let mut passkey: Passkey =
        serde_json::from_value(stored).map_err(|e| AppError::internal("stored passkey", e))?;
    let result = auth.passkeys.verify_sign_in(credential, state, &passkey)?;
    passkey.update_credential(&result);
    let json =
        serde_json::to_value(&passkey).map_err(|e| AppError::internal("serialize passkey", e))?;
    sqlx::query("UPDATE gm_passkeys SET passkey = $2, last_used_at = now() WHERE id = $1")
        .bind(passkey_id)
        .bind(json)
        .execute(&mut *tx)
        .await?;
    let token = session::create_in_tx(&mut tx, gm_id).await?;
    tx.commit().await?;
    Ok((
        CurrentGm {
            id: gm_id,
            display_name,
        },
        token,
    ))
}
