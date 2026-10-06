//! OpenRouter, the first [`Provider`]: its OpenAI-compatible chat API
//! for text, and the same endpoint with `modalities: ["image", "text"]`
//! for images, which come back as base64 data URLs.
//!
//! OpenRouter reports the real cost of each call when asked
//! (`usage: { include: true }`), in dollars; it is stored in millionths.

use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    AiError, BoxFuture, ImageRequest, ImageResponse, LlmRequest, LlmResponse, Provider, Usage,
};

pub const BASE_URL: &str = "https://openrouter.ai/api/v1";
pub const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4.5";
pub const DEFAULT_IMAGE_MODEL: &str = "google/gemini-2.5-flash-image";

/// How long a call may take before it is given up.
const TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone)]
pub struct Settings {
    pub base_url: String,
    pub model: String,
    pub image_model: String,
    /// Sent as `HTTP-Referer`, OpenRouter's app attribution.
    pub app_url: Option<String>,
}

pub struct OpenRouter {
    key: String,
    settings: Settings,
    http: reqwest::Client,
}

impl OpenRouter {
    /// # Panics
    ///
    /// When the TLS backend cannot initialise (never with rustls).
    #[must_use]
    pub fn new(key: String, settings: Settings) -> Self {
        let http = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .expect("an HTTP client with rustls builds");
        Self {
            key,
            settings,
            http,
        }
    }

    async fn chat(&self, body: Value) -> Result<Completion, AiError> {
        let url = format!(
            "{}/chat/completions",
            self.settings.base_url.trim_end_matches('/')
        );
        let mut request = self
            .http
            .post(url)
            .bearer_auth(&self.key)
            .header("X-Title", "Promptus")
            .json(&body);
        if let Some(app) = &self.settings.app_url {
            request = request.header("HTTP-Referer", app);
        }
        let response = request
            .send()
            .await
            .map_err(|e| AiError::Unreachable(e.to_string()))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| AiError::Unreachable(e.to_string()))?;
        let parsed: Completion = serde_json::from_str(&text).map_err(|_| AiError::Refused {
            status: status.as_u16(),
            message: text.chars().take(300).collect(),
        })?;
        if !status.is_success() || parsed.error.is_some() {
            return Err(AiError::Refused {
                status: status.as_u16(),
                message: parsed
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| text.chars().take(300).collect()),
            });
        }
        Ok(parsed)
    }
}

#[derive(Debug, Deserialize)]
struct Completion {
    model: Option<String>,
    #[serde(default)]
    choices: Vec<Choice>,
    usage: Option<ApiUsage>,
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Option<ChoiceMessage>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
    #[serde(default)]
    images: Vec<ImagePart>,
}

#[derive(Debug, Deserialize)]
struct ImagePart {
    image_url: ImageUrl,
}

#[derive(Debug, Deserialize)]
struct ImageUrl {
    url: String,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    cost: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

fn usage(u: Option<&ApiUsage>) -> Usage {
    Usage {
        prompt_tokens: u.and_then(|u| u.prompt_tokens).unwrap_or(0),
        completion_tokens: u.and_then(|u| u.completion_tokens).unwrap_or(0),
        cost_micros: u
            .and_then(|u| u.cost)
            .filter(|c| c.is_finite() && *c >= 0.0)
            .map_or(0, |c| (c * 1_000_000.0).round() as i64),
    }
}

/// `data:image/png;base64,…` → (mime, bytes).
fn decode_data_url(url: &str) -> Option<(String, Vec<u8>)> {
    let rest = url.strip_prefix("data:")?;
    let (meta, data) = rest.split_once(',')?;
    let mime = meta.strip_suffix(";base64")?;
    Some((mime.to_string(), STANDARD.decode(data).ok()?))
}

impl Provider for OpenRouter {
    fn name(&self) -> &'static str {
        "openrouter"
    }

    fn complete<'a>(&'a self, req: &'a LlmRequest) -> BoxFuture<'a, Result<LlmResponse, AiError>> {
        Box::pin(async move {
            let model = req
                .model
                .clone()
                .unwrap_or_else(|| self.settings.model.clone());
            let mut body = json!({
                "model": model,
                "messages": req.messages,
                "temperature": req.temperature,
                "max_tokens": req.max_tokens,
                "usage": { "include": true },
            });
            if req.json {
                body["response_format"] = json!({ "type": "json_object" });
            }
            let completion = self.chat(body).await?;
            let choice = completion.choices.first();
            let text = choice
                .and_then(|c| c.message.as_ref())
                .and_then(|m| m.content.clone())
                .unwrap_or_default();
            if text.trim().is_empty() {
                return Err(AiError::Schema(format!(
                    "réponse vide ({})",
                    choice
                        .and_then(|c| c.finish_reason.clone())
                        .unwrap_or_else(|| "sans raison".into())
                )));
            }
            Ok(LlmResponse {
                text,
                model: completion.model.clone().unwrap_or(model),
                usage: usage(completion.usage.as_ref()),
            })
        })
    }

    fn image<'a>(&'a self, req: &'a ImageRequest) -> BoxFuture<'a, Result<ImageResponse, AiError>> {
        Box::pin(async move {
            let model = req
                .model
                .clone()
                .unwrap_or_else(|| self.settings.image_model.clone());
            let body = json!({
                "model": model,
                "messages": [{ "role": "user", "content": req.prompt }],
                "modalities": ["image", "text"],
                "usage": { "include": true },
            });
            let completion = self.chat(body).await?;
            let (mime, bytes) = completion
                .choices
                .first()
                .and_then(|c| c.message.as_ref())
                .and_then(|m| m.images.first())
                .and_then(|i| decode_data_url(&i.image_url.url))
                .ok_or_else(|| AiError::Schema("aucune image dans la réponse".into()))?;
            Ok(ImageResponse {
                bytes,
                mime,
                model: completion.model.clone().unwrap_or(model),
                usage: usage(completion.usage.as_ref()),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_data_url_is_decoded_and_anything_else_is_not() {
        let (mime, bytes) = decode_data_url("data:image/png;base64,iVBORw==").unwrap();
        assert_eq!(mime, "image/png");
        assert_eq!(bytes, [0x89, b'P', b'N', b'G']);
        assert!(decode_data_url("https://example.com/a.png").is_none());
        assert!(decode_data_url("data:image/png,raw").is_none());
    }

    #[test]
    fn the_reported_cost_is_kept_in_millionths_of_a_dollar() {
        let u = ApiUsage {
            prompt_tokens: Some(1200),
            completion_tokens: Some(300),
            cost: Some(0.004_512),
        };
        assert_eq!(
            usage(Some(&u)),
            Usage {
                prompt_tokens: 1200,
                completion_tokens: 300,
                cost_micros: 4_512
            }
        );
    }
}
