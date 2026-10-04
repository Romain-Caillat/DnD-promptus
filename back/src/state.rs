use axum::extract::FromRef;
use sqlx::PgPool;

use crate::auth::passkeys::Passkeys;
use crate::auth::setup::SetupState;

/// Everything a handler can reach.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth: Auth,
}

/// GM authentication: the relying party, the setup code, and how the
/// session cookie is written.
#[derive(Clone)]
pub struct Auth {
    pub passkeys: Passkeys,
    pub setup: SetupState,
    /// `Secure` cookie flag: on whenever the app is served over HTTPS.
    pub secure_cookie: bool,
}

impl Auth {
    /// From the public origin the app is opened at.
    ///
    /// # Errors
    ///
    /// Fails when no relying party can be built for that origin.
    pub fn new(
        public_origin: &str,
        rp_id: Option<&str>,
        setup: SetupState,
    ) -> Result<Self, String> {
        Ok(Self {
            passkeys: Passkeys::from_origin(public_origin, rp_id)?,
            setup,
            secure_cookie: public_origin.starts_with("https://"),
        })
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}
