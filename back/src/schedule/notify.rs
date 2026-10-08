//! Where the table's reminders go: the campaign's Discord channel,
//! through a webhook the GM pastes (`session/schedule-sessions`).
//!
//! Romain's table already lives on Discord, on every phone: a message in
//! its channel is a notification on Marc's phone, and its link opens the
//! lobby in one touch. The sender is a trait so tests record what would
//! leave instead of posting it — the same seam as the AI provider.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;

use crate::ai::BoxFuture;
use crate::error::AppError;

/// Posts one message to one webhook.
pub trait Sender: Send + Sync {
    /// # Errors
    ///
    /// What went wrong, in a line the GM can read in the reminder log.
    fn post<'a>(&'a self, webhook: &'a str, content: &'a str) -> BoxFuture<'a, Result<(), String>>;
}

/// The table's notifications: who sends, and where links lead.
#[derive(Clone)]
pub struct Notifier {
    sender: Arc<dyn Sender>,
    /// The origin people open the app at (`PUBLIC_ORIGIN`).
    origin: String,
}

impl Notifier {
    #[must_use]
    pub fn new(sender: Arc<dyn Sender>, origin: &str) -> Self {
        Self {
            sender,
            origin: origin.trim_end_matches('/').to_string(),
        }
    }

    /// Posting to Discord for real.
    #[must_use]
    pub fn discord(origin: &str) -> Self {
        Self::new(Arc::new(Discord::default()), origin)
    }

    /// Recording instead of posting (tests).
    #[must_use]
    pub fn recorder(origin: &str) -> (Self, Arc<Recorder>) {
        let recorder = Arc::new(Recorder::default());
        (Self::new(recorder.clone(), origin), recorder)
    }

    /// Where a player lands from a reminder: their home in the campaign,
    /// which opens on the lobby when it is open.
    #[must_use]
    pub fn link(&self, campaign: uuid::Uuid) -> String {
        format!("{}/partie/{campaign}", self.origin)
    }

    /// # Errors
    ///
    /// As [`Sender::post`].
    pub async fn post(&self, webhook: &str, content: &str) -> Result<(), String> {
        self.sender.post(webhook, content).await
    }
}

/// Discord's webhooks over HTTPS.
struct Discord {
    http: reqwest::Client,
}

impl Default for Discord {
    fn default() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Sender for Discord {
    fn post<'a>(&'a self, webhook: &'a str, content: &'a str) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            // No mention is ever pinged, whatever the text holds.
            let body = json!({ "content": content, "allowed_mentions": { "parse": [] } });
            let r = self
                .http
                .post(webhook)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Discord injoignable : {e}"))?;
            if r.status().is_success() {
                Ok(())
            } else {
                Err(format!("Discord a refusé : {}", r.status()))
            }
        })
    }
}

/// What would have been posted, for tests; `failing` refuses everything.
#[derive(Default)]
pub struct Recorder {
    sent: Mutex<Vec<(String, String)>>,
    pub failing: std::sync::atomic::AtomicBool,
}

impl Recorder {
    /// Every (webhook, message) posted so far.
    #[must_use]
    pub fn sent(&self) -> Vec<(String, String)> {
        self.sent.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Sender for Recorder {
    fn post<'a>(&'a self, webhook: &'a str, content: &'a str) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            if self.failing.load(std::sync::atomic::Ordering::Relaxed) {
                return Err("refusé (test)".into());
            }
            if let Ok(mut s) = self.sent.lock() {
                s.push((webhook.to_string(), content.to_string()));
            }
            Ok(())
        })
    }
}

/// A webhook URL the server will post to: Discord's own, over HTTPS —
/// anything else would let a GM make the server call any address.
///
/// # Errors
///
/// 400 `INVALID_WEBHOOK`.
pub fn check_webhook(raw: &str) -> Result<String, AppError> {
    let url =
        reqwest::Url::parse(raw.trim()).map_err(|_| AppError::BadRequest("INVALID_WEBHOOK"))?;
    let host_ok = matches!(
        url.host_str(),
        Some("discord.com" | "discordapp.com" | "ptb.discord.com" | "canary.discord.com")
    );
    if url.scheme() != "https"
        || !host_ok
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || !url.path().starts_with("/api/webhooks/")
    {
        return Err(AppError::BadRequest("INVALID_WEBHOOK"));
    }
    Ok(url.to_string())
}

/// The end of a webhook, enough for the GM to recognise it.
#[must_use]
pub fn masked(webhook: &str) -> String {
    let tail: String = webhook
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_discord_webhook_over_https_is_accepted() {
        assert!(check_webhook("https://discord.com/api/webhooks/123/abc").is_ok());
        assert!(check_webhook(" https://discordapp.com/api/webhooks/1/x ").is_ok());
        for bad in [
            "http://discord.com/api/webhooks/123/abc",
            "https://discord.com.evil.net/api/webhooks/1/x",
            "https://evil.net/api/webhooks/1/x",
            "https://discord.com/channels/1/2",
            "https://discord.com:8443/api/webhooks/1/x",
            "https://user@discord.com/api/webhooks/1/x",
            "http://127.0.0.1/api/webhooks/1/x",
            "pas une adresse",
        ] {
            assert!(check_webhook(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_gm_sees_only_the_end_of_the_secret() {
        assert_eq!(
            masked("https://discord.com/api/webhooks/1/abcdWXYZ"),
            "…WXYZ"
        );
    }
}
