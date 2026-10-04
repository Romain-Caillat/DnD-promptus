//! First-account setup code.
//!
//! The app is published on the Internet for players, so reaching a fresh
//! instance first must not be enough to own it. While no GM exists the
//! server opens setup with a **setup code**: random, minted at boot and
//! printed in the server log (or pinned by the installer through
//! `GM_SETUP_TOKEN`). Only someone who can read that log — whoever runs
//! the server — can create the first account.
//!
//! The code lives in memory only. Setup closes for good once any GM
//! exists: the account is created under a table lock that re-checks it,
//! so two racing setups cannot both succeed. Every later GM is admitted
//! by an invitation from an existing GM (`invites`).

use std::sync::{Arc, Mutex};

use sqlx::PgPool;

use super::tokens::{hash_token, random_token};
use crate::error::AppError;

/// Shortest installer-pinned code accepted; anything weaker is replaced
/// by a random one.
const MIN_PINNED_LEN: usize = 16;

/// The setup code while setup is open, `None` once it is closed.
#[derive(Clone, Default)]
pub struct SetupState {
    code: Arc<Mutex<Option<String>>>,
}

impl SetupState {
    /// Setup open with this code.
    pub fn open_with(code: String) -> Self {
        Self {
            code: Arc::new(Mutex::new(Some(code))),
        }
    }

    /// Setup closed: a GM already exists.
    pub fn closed() -> Self {
        Self::default()
    }

    /// Open setup when no GM exists yet. `pinned` is `GM_SETUP_TOKEN`,
    /// used when long enough.
    ///
    /// # Errors
    ///
    /// Fails on a database error.
    pub async fn init(pool: &PgPool, pinned: Option<String>) -> Result<Self, AppError> {
        if gm_count(pool).await? > 0 {
            return Ok(Self::closed());
        }
        let code = match pinned {
            Some(p) if p.len() >= MIN_PINNED_LEN => p,
            Some(_) => {
                tracing::warn!(
                    "GM_SETUP_TOKEN is shorter than {MIN_PINNED_LEN} characters; using a random setup code instead"
                );
                random_token()
            }
            None => random_token(),
        };
        Ok(Self::open_with(code))
    }

    /// The current code, for the boot log line.
    pub fn code(&self) -> Option<String> {
        self.code.lock().expect("setup code lock").clone()
    }

    pub fn close(&self) {
        *self.code.lock().expect("setup code lock") = None;
    }

    /// Whether `candidate` is the open setup code. Both sides are hashed
    /// first, so the comparison never runs over the secret itself byte
    /// by byte.
    pub fn matches(&self, candidate: &str) -> bool {
        let guard = self.code.lock().expect("setup code lock");
        guard
            .as_deref()
            .is_some_and(|code| hash_token(code) == hash_token(candidate))
    }
}

/// How many GM accounts exist.
///
/// # Errors
///
/// Fails on a database error.
pub async fn gm_count(pool: &PgPool) -> Result<i64, AppError> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM gms")
        .fetch_one(pool)
        .await?)
}
