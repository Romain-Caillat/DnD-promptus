//! Where sign-in codes go: an email, over SMTP.
//!
//! The sender is a trait so tests record what would leave instead of
//! sending it — the same seam as the AI provider and the Discord
//! notifier. Without `SMTP_HOST` (development) the code is written to
//! the server log instead, which only whoever runs the server can read.

use std::sync::{Arc, Mutex};

use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::ai::BoxFuture;
use crate::auth::codes::CODE_MINUTES;

/// Sends one sign-in code to one address.
pub trait Mailer: Send + Sync {
    /// # Errors
    ///
    /// What went wrong, for the server log.
    fn send_code<'a>(&'a self, to: &'a str, code: &'a str) -> BoxFuture<'a, Result<(), String>>;
}

/// SMTP settings (`SMTP_*`, `.env.example`).
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub password: Option<String>,
    pub from: String,
}

/// The mailer the configuration asks for.
///
/// # Errors
///
/// Fails when `SMTP_FROM` is not an address or the relay cannot be set up.
pub fn from_config(smtp: Option<&SmtpConfig>) -> Result<Arc<dyn Mailer>, String> {
    match smtp {
        Some(config) => Ok(Arc::new(Smtp::new(config)?)),
        None => {
            tracing::warn!("SMTP_HOST is not set: sign-in codes are written to this log");
            Ok(Arc::new(LogMailer))
        }
    }
}

struct Smtp {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl Smtp {
    fn new(config: &SmtpConfig) -> Result<Self, String> {
        let from: Mailbox = config
            .from
            .parse()
            .map_err(|e| format!("SMTP_FROM {:?}: {e}", config.from))?;
        // Port 465 speaks TLS from the first byte; any other port
        // (587) upgrades with STARTTLS, which is required.
        let builder = if config.port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
        }
        .map_err(|e| format!("SMTP_HOST {:?}: {e}", config.host))?
        .port(config.port);
        let builder = match (&config.user, &config.password) {
            (Some(user), Some(password)) => {
                builder.credentials(Credentials::new(user.clone(), password.clone()))
            }
            _ => builder,
        };
        Ok(Self {
            transport: builder.build(),
            from,
        })
    }
}

impl Mailer for Smtp {
    fn send_code<'a>(&'a self, to: &'a str, code: &'a str) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let to: Mailbox = to.parse().map_err(|e| format!("recipient: {e}"))?;
            // A Message-ID on the sender's own domain: lettre adds none
            // by default, and a missing or foreign one sends the code to
            // the spam folder.
            let message_id = format!("<{}@{}>", uuid::Uuid::new_v4(), self.from.email.domain());
            let message = Message::builder()
                .from(self.from.clone())
                .to(to)
                .message_id(Some(message_id))
                .subject(format!("Ton code de connexion Promptus : {code}"))
                .multipart(MultiPart::alternative_plain_html(
                    code_text(code),
                    code_html(code),
                ))
                .map_err(|e| format!("message: {e}"))?;
            self.transport
                .send(message)
                .await
                .map(|_| ())
                .map_err(|e| format!("smtp: {e}"))
        })
    }
}

/// The body of the email, in French like the rest of the app.
fn code_text(code: &str) -> String {
    format!(
        "Ton code de connexion à Promptus : {code}\n\n\
         Il expire dans {CODE_MINUTES} minutes.\n\n\
         Si tu n'as pas demandé ce code, ignore cet email : personne ne peut \
         se connecter sans lui."
    )
}

/// The same, for mail apps that show HTML. `code` is six digits: nothing
/// to escape.
fn code_html(code: &str) -> String {
    format!(
        "<!doctype html><html lang=\"fr\"><body style=\"font-family:sans-serif;max-width:420px;margin:0 auto;padding:24px\">\
         <h2 style=\"margin:0 0 12px\">Promptus</h2>\
         <p>Ton code de connexion :</p>\
         <p style=\"font-size:32px;font-weight:bold;letter-spacing:8px;padding:16px;background:#f4f4f5;border-radius:8px;text-align:center\">{code}</p>\
         <p style=\"color:#71717a;font-size:14px\">Il expire dans {CODE_MINUTES} minutes. Si tu n'as pas demandé ce code, ignore cet email : personne ne peut se connecter sans lui.</p>\
         </body></html>"
    )
}

/// Development without SMTP: the code goes to the server log.
struct LogMailer;

impl Mailer for LogMailer {
    fn send_code<'a>(&'a self, to: &'a str, code: &'a str) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            tracing::warn!("sign-in code for {to}: {code}");
            Ok(())
        })
    }
}

/// Tests: keeps every code that would have been sent, as `(to, code)`.
#[derive(Clone, Default)]
pub struct Recorder {
    sent: Arc<Mutex<Vec<(String, String)>>>,
    failing: bool,
}

impl Recorder {
    /// A recorder whose sends all fail, as an unreachable relay would.
    #[must_use]
    pub fn failing() -> Self {
        Self {
            failing: true,
            ..Self::default()
        }
    }

    /// Everything sent so far, oldest first.
    #[must_use]
    pub fn sent(&self) -> Vec<(String, String)> {
        self.sent.lock().expect("recorder lock").clone()
    }

    /// The last code sent to `to`.
    #[must_use]
    pub fn last_code(&self, to: &str) -> Option<String> {
        self.sent()
            .into_iter()
            .rev()
            .find(|(addr, _)| addr == to)
            .map(|(_, code)| code)
    }
}

impl Mailer for Recorder {
    fn send_code<'a>(&'a self, to: &'a str, code: &'a str) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            if self.failing {
                return Err("relay unreachable".to_string());
            }
            self.sent
                .lock()
                .expect("recorder lock")
                .push((to.to_string(), code.to_string()));
            Ok(())
        })
    }
}
