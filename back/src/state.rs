use axum::extract::FromRef;
use sqlx::PgPool;

use crate::ai::Ai;
use std::sync::Arc;

use crate::auth::mailer::Mailer;
use crate::live::LiveHub;
use crate::schedule::Notifier;

/// Everything a handler can reach.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth: Auth,
    /// The live channel: campaign rooms and presence.
    pub live: LiveHub,
    /// The model provider and its prices (`ai`).
    pub ai: Ai,
    /// Where the table's reminders go (`schedule`).
    pub notifier: Notifier,
}

/// GM authentication: where sign-in codes are sent, and how the session
/// cookie is written.
#[derive(Clone)]
pub struct Auth {
    pub mailer: Arc<dyn Mailer>,
    /// `Secure` cookie flag: on whenever the app is served over HTTPS.
    pub secure_cookie: bool,
}

impl Auth {
    /// For the public origin the app is opened at.
    #[must_use]
    pub fn new(public_origin: &str, mailer: Arc<dyn Mailer>) -> Self {
        Self {
            mailer,
            secure_cookie: public_origin.starts_with("https://"),
        }
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}
