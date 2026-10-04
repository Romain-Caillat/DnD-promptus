//! The relying party (WebAuthn) and its pending ceremonies.
//!
//! Same approach as Devotion: a ceremony is a challenge handed out, then
//! the signed answer. Ceremonies are kept **server-side, in memory**:
//! - each has a random id, lives 10 minutes and is taken out on first
//!   use, so a replayed answer finds nothing;
//! - each is bound to its purpose (a sign-in challenge never answers a
//!   registration) and a registration to the account it will create;
//! - the table is capped, so unauthenticated starts cannot grow it.
//!
//! **User verification is required** (Face ID, a fingerprint, a security
//! key's PIN — not a mere touch), and credentials are **discoverable**:
//! signing in names no account, the authenticator says whose it is.
//!
//! The relying party is `PUBLIC_ORIGIN` and its host (or
//! `WEBAUTHN_RP_ID`). Passkeys are bound to that domain: changing it
//! means registering them again.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Serialize;
use uuid::Uuid;
use webauthn_rs::prelude::{
    AuthenticationResult, CreationChallengeResponse, DiscoverableAuthentication, DiscoverableKey,
    Passkey, PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Url, Webauthn, WebauthnBuilder,
};
use webauthn_rs_proto::ResidentKeyRequirement;

use super::tokens::random_token;
use crate::error::AppError;

// Longer than the browser's own 5-minute prompt, so an answer given at
// the last second of the prompt is not refused as expired.
const CEREMONY_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_CEREMONIES: usize = 10_000;
const RP_NAME: &str = "Promptus";

/// What a registration will create once its passkey comes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// The first GM of the instance, admitted by the setup code.
    Setup,
    /// A GM invited by another one: the invitation's id.
    Invite(Uuid),
}

/// A registration in progress: the account is chosen now (its id is the
/// passkey's user handle) and created only when the passkey verifies.
pub struct PendingAccount {
    pub gm_id: Uuid,
    pub display_name: String,
    pub admission: Admission,
}

enum Ceremony {
    Register {
        account: PendingAccount,
        state: PasskeyRegistration,
    },
    SignIn {
        state: DiscoverableAuthentication,
    },
}

struct Entry {
    ceremony: Ceremony,
    expires: Instant,
}

struct Inner {
    webauthn: Webauthn,
    pending: Mutex<HashMap<String, Entry>>,
}

/// The server's relying party plus its pending ceremonies. Cheap to clone.
#[derive(Clone)]
pub struct Passkeys {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for Passkeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Passkeys").finish_non_exhaustive()
    }
}

/// A challenge for the browser, and the id to answer it under.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Challenge<T: Serialize> {
    pub ceremony_id: String,
    pub options: T,
}

/// Every refused answer looks the same to the client; the reason goes
/// to the log.
fn invalid_passkey() -> AppError {
    AppError::Unauthorized("INVALID_PASSKEY")
}

/// How a credential id is stored (`gm_passkeys.credential_id`).
pub fn credential_key(raw: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(raw)
}

impl Passkeys {
    /// A relying party for `rp_id`, used from `origin`.
    ///
    /// # Errors
    ///
    /// Fails when the origin is not a URL or does not belong to `rp_id`.
    pub fn new(rp_id: &str, origin: &str) -> Result<Self, String> {
        let url = Url::parse(origin).map_err(|e| format!("origin '{origin}': {e}"))?;
        let webauthn = WebauthnBuilder::new(rp_id, &url)
            .map_err(|e| format!("relying party '{rp_id}' for '{origin}': {e}"))?
            .rp_name(RP_NAME)
            .build()
            .map_err(|e| format!("relying party: {e}"))?;
        Ok(Self {
            inner: Arc::new(Inner {
                webauthn,
                pending: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// From `PUBLIC_ORIGIN` and an optional explicit relying party id.
    ///
    /// # Errors
    ///
    /// Fails when no usable relying party can be built: GMs could not
    /// sign in at all, so the server must not start.
    pub fn from_origin(origin: &str, rp_id: Option<&str>) -> Result<Self, String> {
        let rp_id = match rp_id {
            Some(id) => id.to_string(),
            None => Url::parse(origin)
                .ok()
                .and_then(|u| u.host_str().map(str::to_string))
                .ok_or_else(|| format!("PUBLIC_ORIGIN '{origin}' has no host"))?,
        };
        Self::new(&rp_id, origin)
    }

    fn put(&self, ceremony: Ceremony) -> Result<String, AppError> {
        let now = Instant::now();
        let mut map = self.inner.pending.lock().expect("ceremony lock");
        if map.len() >= MAX_CEREMONIES {
            map.retain(|_, e| e.expires > now);
            if map.len() >= MAX_CEREMONIES {
                tracing::warn!("passkey ceremony table full; refusing a new challenge");
                return Err(AppError::ServiceUnavailable("TOO_MANY_CEREMONIES"));
            }
        }
        let id = random_token();
        map.insert(
            id.clone(),
            Entry {
                ceremony,
                expires: now + CEREMONY_TTL,
            },
        );
        Ok(id)
    }

    /// Single use: the ceremony leaves the table whatever happens next.
    fn take(&self, id: &str) -> Option<Ceremony> {
        let mut map = self.inner.pending.lock().expect("ceremony lock");
        let Some(entry) = map.remove(id) else {
            tracing::info!("passkey ceremony refused: unknown or already used");
            return None;
        };
        if entry.expires <= Instant::now() {
            tracing::info!("passkey ceremony refused: expired");
            return None;
        }
        Some(entry.ceremony)
    }

    /// Hand out a registration challenge for an account to be created.
    ///
    /// # Errors
    ///
    /// Fails when the ceremony table is full.
    pub fn begin_registration(
        &self,
        account: PendingAccount,
    ) -> Result<Challenge<CreationChallengeResponse>, AppError> {
        let (mut options, state) = self
            .inner
            .webauthn
            .start_passkey_registration(
                account.gm_id,
                &account.display_name,
                &account.display_name,
                None,
            )
            .map_err(|e| AppError::internal("start passkey registration", e))?;
        // Discoverable, so signing in later needs no identifier.
        if let Some(sel) = options.public_key.authenticator_selection.as_mut() {
            sel.resident_key = Some(ResidentKeyRequirement::Required);
            sel.require_resident_key = true;
        }
        let ceremony_id = self.put(Ceremony::Register { account, state })?;
        Ok(Challenge {
            ceremony_id,
            options,
        })
    }

    /// Verify a registration answer. Returns the account it was started
    /// for and its new passkey.
    ///
    /// # Errors
    ///
    /// 401 `INVALID_PASSKEY` on an unknown, spent, expired or forged
    /// answer.
    pub fn finish_registration(
        &self,
        ceremony_id: &str,
        credential: &RegisterPublicKeyCredential,
    ) -> Result<(PendingAccount, Passkey), AppError> {
        let Some(Ceremony::Register { account, state }) = self.take(ceremony_id) else {
            return Err(invalid_passkey());
        };
        let passkey = self
            .inner
            .webauthn
            .finish_passkey_registration(credential, &state)
            .map_err(|e| {
                tracing::info!("passkey registration refused: {e}");
                invalid_passkey()
            })?;
        Ok((account, passkey))
    }

    /// Hand out a sign-in challenge (any passkey of this site).
    ///
    /// # Errors
    ///
    /// Fails when the ceremony table is full.
    pub fn begin_sign_in(&self) -> Result<Challenge<RequestChallengeResponse>, AppError> {
        let (mut options, state) = self
            .inner
            .webauthn
            .start_discoverable_authentication()
            .map_err(|e| AppError::internal("start passkey sign-in", e))?;
        // The sign-in button asks at once; no autofill-style mediation.
        options.mediation = None;
        let ceremony_id = self.put(Ceremony::SignIn { state })?;
        Ok(Challenge {
            ceremony_id,
            options,
        })
    }

    /// The sign-in ceremony an answer refers to, and the GM id its
    /// authenticator claims (the user handle). Nothing is verified yet:
    /// the caller loads that GM's passkey and calls [`Self::verify_sign_in`].
    ///
    /// # Errors
    ///
    /// 401 `INVALID_PASSKEY` on an unknown, spent or expired ceremony, or
    /// an answer without a usable user handle.
    pub fn identify(
        &self,
        ceremony_id: &str,
        credential: &PublicKeyCredential,
    ) -> Result<(Uuid, DiscoverableAuthentication), AppError> {
        let Some(Ceremony::SignIn { state }) = self.take(ceremony_id) else {
            return Err(invalid_passkey());
        };
        let (gm_id, _) = self
            .inner
            .webauthn
            .identify_discoverable_authentication(credential)
            .map_err(|e| {
                tracing::info!("passkey sign-in refused: {e}");
                invalid_passkey()
            })?;
        Ok((gm_id, state))
    }

    /// Check the signature against the stored passkey, with user
    /// verification.
    ///
    /// # Errors
    ///
    /// 401 `INVALID_PASSKEY` when the assertion does not verify.
    pub fn verify_sign_in(
        &self,
        credential: &PublicKeyCredential,
        state: DiscoverableAuthentication,
        passkey: &Passkey,
    ) -> Result<AuthenticationResult, AppError> {
        let result = self
            .inner
            .webauthn
            .finish_discoverable_authentication(
                credential,
                state,
                &[DiscoverableKey::from(passkey)],
            )
            .map_err(|e| {
                tracing::info!("passkey assertion refused: {e}");
                invalid_passkey()
            })?;
        if !result.user_verified() {
            tracing::info!("passkey assertion refused: no user verification");
            return Err(invalid_passkey());
        }
        Ok(result)
    }
}
