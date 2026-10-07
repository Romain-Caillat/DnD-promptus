//! ai/route-llm-provider — every call to a model goes through here.
//!
//! The game never names a provider: it asks [`Ai`] for a completion or an
//! image, and [`Ai`] holds whichever [`Provider`] the server was started
//! with — OpenRouter ([`openrouter`]) in production, the deterministic
//! [`fake`] provider in tests and in development without a key. Changing
//! model or provider is configuration (`.env.example`), not code.
//!
//! Three rules hold for every call:
//! - its prompt comes from a **versioned template** ([`templates`]), and
//!   the template's id and version are recorded with the call;
//! - its output is **parsed against a schema** (a serde type) before
//!   anything reads it: an answer out of schema is an error the GM can
//!   read ([`parse_json`]), never a half-filled value;
//! - it is **counted** ([`ledger`], `ai/count-ai-calls`): estimated and
//!   checked against the campaign's budget before it leaves, recorded
//!   with its real cost when it comes back (`MEMORY.md` §3).
//!
//! Video (`media/generate-images-and-video`) follows OpenRouter's
//! asynchronous video API: a job is submitted, polled until done, and
//! its file downloaded ([`openrouter`]).
//!
//! Transcription (`copilot/listen-by-voice`) sends the GM's recorded
//! words, as a WAV file, to a model that hears audio; it answers the
//! text, parsed against a schema like any other call.

pub mod fake;
pub mod ledger;
pub mod openrouter;
pub mod templates;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// A boxed future, so [`Provider`] stays object-safe.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
}

/// A chat completion to run.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmRequest {
    pub messages: Vec<Message>,
    /// The provider's model id; `None` takes the server's default.
    pub model: Option<String>,
    pub temperature: f32,
    /// The most the answer may cost in tokens: the estimate bills it whole.
    pub max_tokens: u32,
    /// Ask for one JSON object.
    pub json: bool,
}

impl LlmRequest {
    /// A JSON answer to `messages`, with the defaults the co-GM uses.
    #[must_use]
    pub fn json(messages: Vec<Message>) -> Self {
        Self {
            messages,
            model: None,
            temperature: 0.8,
            max_tokens: 2_000,
            json: true,
        }
    }

    /// Characters of every message: what the estimate turns into tokens.
    #[must_use]
    pub fn prompt_chars(&self) -> usize {
        self.messages
            .iter()
            .map(|m| m.content.chars().count())
            .sum()
    }
}

/// What a call consumed, as the provider reports it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    /// Millionths of a US dollar.
    pub cost_micros: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmResponse {
    pub text: String,
    pub model: String,
    pub usage: Usage,
}

/// An image to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRequest {
    pub prompt: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageResponse {
    pub bytes: Vec<u8>,
    /// `image/png`, `image/jpeg`…
    pub mime: String,
    pub model: String,
    pub usage: Usage,
}

/// A short video to generate (an act's introduction).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoRequest {
    pub prompt: String,
    pub model: Option<String>,
    /// Length asked for, in seconds.
    pub seconds: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoResponse {
    pub bytes: Vec<u8>,
    /// `video/mp4`…
    pub mime: String,
    pub model: String,
    pub usage: Usage,
}

/// What the GM said, to write down (`copilot/listen-by-voice`).
#[derive(Debug, Clone, PartialEq)]
pub struct TranscribeRequest {
    /// The system and user messages of the transcription template: the
    /// instruction and the campaign's names to spell right.
    pub messages: Vec<Message>,
    /// A 16-bit PCM WAV file: every audio model takes it.
    pub wav: Vec<u8>,
    /// Length of the recording, for the estimate.
    pub seconds: f32,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranscribeResponse {
    /// The model's answer, JSON to parse (`{ "text": … }`).
    pub text: String,
    pub model: String,
    pub usage: Usage,
}

/// Why a call failed. `Display` is written for the GM's eyes (French):
/// it is what the screen shows next to the failed draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// The server has no provider configured.
    NotConfigured,
    /// The provider could not be reached, or did not answer in time.
    Unreachable(String),
    /// The provider refused the call (status and its message).
    Refused { status: u16, message: String },
    /// The answer did not match what was asked for.
    Schema(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => {
                write!(f, "Aucun fournisseur d’IA n’est configuré sur ce serveur.")
            }
            Self::Unreachable(e) => write!(f, "Le fournisseur d’IA ne répond pas : {e}"),
            Self::Refused { status, message } => {
                write!(
                    f,
                    "Le fournisseur d’IA a refusé l’appel ({status}) : {message}"
                )
            }
            Self::Schema(e) => write!(
                f,
                "La réponse du modèle ne respecte pas le format attendu : {e}"
            ),
        }
    }
}

/// A model provider. Object-safe: the server holds an `Arc<dyn Provider>`.
pub trait Provider: Send + Sync {
    /// Short name recorded with each call (`openrouter`, `fake`).
    fn name(&self) -> &'static str;
    fn complete<'a>(&'a self, req: &'a LlmRequest) -> BoxFuture<'a, Result<LlmResponse, AiError>>;
    fn image<'a>(&'a self, req: &'a ImageRequest) -> BoxFuture<'a, Result<ImageResponse, AiError>>;
    fn video<'a>(&'a self, req: &'a VideoRequest) -> BoxFuture<'a, Result<VideoResponse, AiError>>;
    fn transcribe<'a>(
        &'a self,
        req: &'a TranscribeRequest,
    ) -> BoxFuture<'a, Result<TranscribeResponse, AiError>>;
}

/// What a call is estimated to cost before it leaves (`ledger`). Prices
/// are the server's configuration, in millionths of a dollar; the
/// defaults are on the expensive side so an estimate errs towards
/// refusing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pricing {
    pub input_per_mtok_micros: i64,
    pub output_per_mtok_micros: i64,
    pub per_image_micros: i64,
    pub per_video_micros: i64,
    /// One minute of recorded voice heard by the model (its audio
    /// tokens), the written answer billed on top as output tokens.
    pub per_audio_minute_micros: i64,
}

impl Default for Pricing {
    fn default() -> Self {
        Self {
            input_per_mtok_micros: 3_000_000,
            output_per_mtok_micros: 15_000_000,
            per_image_micros: 40_000,
            // An 8-second clip at about 0.50 $ a second.
            per_video_micros: 4_000_000,
            // Audio models bill about 2 000 tokens a minute, around
            // 0.002 $; five times that, to err towards refusing.
            per_audio_minute_micros: 10_000,
        }
    }
}

impl Pricing {
    /// The most `req` may cost: its prompt (4 characters a token,
    /// rounded up) plus every token it may answer.
    #[must_use]
    pub fn llm(&self, req: &LlmRequest) -> i64 {
        let prompt_tokens = (req.prompt_chars() as i64 + 3) / 4;
        ceil_div(prompt_tokens * self.input_per_mtok_micros, 1_000_000)
            + ceil_div(
                i64::from(req.max_tokens) * self.output_per_mtok_micros,
                1_000_000,
            )
    }

    #[must_use]
    pub fn image(&self) -> i64 {
        self.per_image_micros
    }

    #[must_use]
    pub fn video(&self) -> i64 {
        self.per_video_micros
    }

    /// The most a transcription may cost: every started minute of
    /// voice, the instruction, and the whole answer budget.
    #[must_use]
    pub fn transcribe(&self, req: &TranscribeRequest, max_answer_tokens: u32) -> i64 {
        let minutes = (f64::from(req.seconds.max(0.0)) / 60.0).ceil().max(1.0) as i64;
        let text = LlmRequest {
            messages: req.messages.clone(),
            model: None,
            temperature: 0.0,
            max_tokens: max_answer_tokens,
            json: true,
        };
        minutes * self.per_audio_minute_micros + self.llm(&text)
    }
}

fn ceil_div(a: i64, b: i64) -> i64 {
    (a + b - 1) / b
}

/// The server's AI: a provider (or none) and its prices.
#[derive(Clone)]
pub struct Ai {
    provider: Option<Arc<dyn Provider>>,
    pub pricing: Pricing,
}

impl Ai {
    #[must_use]
    pub fn new(provider: Option<Arc<dyn Provider>>, pricing: Pricing) -> Self {
        Self { provider, pricing }
    }

    /// No provider: every call answers [`AiError::NotConfigured`].
    #[must_use]
    pub fn none() -> Self {
        Self::new(None, Pricing::default())
    }

    /// The deterministic provider, for tests and keyless development.
    #[must_use]
    pub fn fake() -> Self {
        Self::new(
            Some(Arc::new(fake::FakeProvider::default())),
            Pricing::default(),
        )
    }

    /// From the environment (`.env.example`): `AI_PROVIDER=fake` for the
    /// fake provider, otherwise OpenRouter when `OPENROUTER_API_KEY` is
    /// set, otherwise none. Prices from `AI_PRICE_INPUT_PER_MTOK`,
    /// `AI_PRICE_OUTPUT_PER_MTOK`, `AI_PRICE_PER_IMAGE` and
    /// `AI_PRICE_PER_VIDEO` and `AI_PRICE_PER_AUDIO_MINUTE`, in dollars.
    #[must_use]
    pub fn from_env() -> Self {
        let dollars = |name: &str, default: i64| {
            std::env::var(name)
                .ok()
                .and_then(|v| v.trim().parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v >= 0.0)
                .map_or(default, |v| (v * 1_000_000.0).round() as i64)
        };
        let base = Pricing::default();
        let pricing = Pricing {
            input_per_mtok_micros: dollars("AI_PRICE_INPUT_PER_MTOK", base.input_per_mtok_micros),
            output_per_mtok_micros: dollars(
                "AI_PRICE_OUTPUT_PER_MTOK",
                base.output_per_mtok_micros,
            ),
            per_image_micros: dollars("AI_PRICE_PER_IMAGE", base.per_image_micros),
            per_video_micros: dollars("AI_PRICE_PER_VIDEO", base.per_video_micros),
            per_audio_minute_micros: dollars(
                "AI_PRICE_PER_AUDIO_MINUTE",
                base.per_audio_minute_micros,
            ),
        };
        let env = |name: &str| {
            std::env::var(name)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let provider: Option<Arc<dyn Provider>> = if env("AI_PROVIDER").as_deref() == Some("fake") {
            Some(Arc::new(fake::FakeProvider::default()))
        } else {
            env("OPENROUTER_API_KEY").map(|key| {
                Arc::new(openrouter::OpenRouter::new(
                    key,
                    openrouter::Settings {
                        base_url: env("OPENROUTER_BASE_URL")
                            .unwrap_or_else(|| openrouter::BASE_URL.to_string()),
                        model: env("OPENROUTER_MODEL")
                            .unwrap_or_else(|| openrouter::DEFAULT_MODEL.to_string()),
                        image_model: env("OPENROUTER_IMAGE_MODEL")
                            .unwrap_or_else(|| openrouter::DEFAULT_IMAGE_MODEL.to_string()),
                        video_model: env("OPENROUTER_VIDEO_MODEL")
                            .unwrap_or_else(|| openrouter::DEFAULT_VIDEO_MODEL.to_string()),
                        audio_model: env("OPENROUTER_AUDIO_MODEL")
                            .unwrap_or_else(|| openrouter::DEFAULT_AUDIO_MODEL.to_string()),
                        app_url: env("PUBLIC_ORIGIN"),
                    },
                )) as Arc<dyn Provider>
            })
        };
        Self::new(provider, pricing)
    }

    /// The provider, or [`AiError::NotConfigured`].
    ///
    /// # Errors
    ///
    /// When the server has none.
    pub fn provider(&self) -> Result<&dyn Provider, AiError> {
        self.provider.as_deref().ok_or(AiError::NotConfigured)
    }

    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.provider.is_some()
    }
}

/// Read one JSON object of type `T` out of a model's text: the object
/// itself, or one inside a ```json fence or surrounded by prose.
///
/// # Errors
///
/// [`AiError::Schema`] when there is no object, it is not JSON, or it
/// does not match `T` — with serde's own description of where.
pub fn parse_json<T: DeserializeOwned>(text: &str) -> Result<T, AiError> {
    let candidate = text
        .find("```")
        .and_then(|start| {
            let rest = &text[start + 3..];
            let rest = rest.strip_prefix("json").unwrap_or(rest);
            rest.find("```").map(|end| &rest[..end])
        })
        .unwrap_or(text);
    let (Some(start), Some(end)) = (candidate.find('{'), candidate.rfind('}')) else {
        return Err(AiError::Schema("aucun objet JSON dans la réponse".into()));
    };
    if end <= start {
        return Err(AiError::Schema("aucun objet JSON dans la réponse".into()));
    }
    serde_json::from_str(&candidate[start..=end]).map_err(|e| AiError::Schema(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Answer {
        narration: String,
    }

    #[test]
    fn a_json_object_is_found_in_a_fence_or_in_prose() {
        let fenced = "Voici :\n```json\n{\"narration\": \"Le vent\"}\n```\nBonne partie";
        assert_eq!(parse_json::<Answer>(fenced).unwrap().narration, "Le vent");
        let prose = "Bien sûr ! {\"narration\": \"La nuit\"} Voilà.";
        assert_eq!(parse_json::<Answer>(prose).unwrap().narration, "La nuit");
    }

    #[test]
    fn an_answer_out_of_schema_is_refused_with_where_it_went_wrong() {
        let wrong_type = parse_json::<Answer>("{\"narration\": 42}").unwrap_err();
        let AiError::Schema(detail) = &wrong_type else {
            panic!("{wrong_type:?}")
        };
        assert!(detail.contains("invalid type"), "{detail}");
        assert!(wrong_type.to_string().contains("format attendu"));
        assert!(matches!(
            parse_json::<Answer>("pas de json"),
            Err(AiError::Schema(_))
        ));
        assert!(matches!(
            parse_json::<Answer>("{\"narration\": \"x\", \"secret\": 1}"),
            Err(AiError::Schema(_))
        ));
    }

    #[test]
    fn the_estimate_bills_the_whole_answer_budget() {
        let pricing = Pricing {
            input_per_mtok_micros: 1_000_000,
            output_per_mtok_micros: 2_000_000,
            per_image_micros: 5,
            per_video_micros: 7,
            per_audio_minute_micros: 11,
        };
        let mut req = LlmRequest::json(vec![Message::user("x".repeat(4_000))]);
        req.max_tokens = 500;
        // 1 000 prompt tokens at 1 $/Mtok + 500 answer tokens at 2 $/Mtok.
        assert_eq!(pricing.llm(&req), 1_000 + 1_000);
        // 61 seconds of voice bill two minutes, plus the instruction.
        let voice = TranscribeRequest {
            messages: vec![Message::user("x".repeat(400))],
            wav: Vec::new(),
            seconds: 61.0,
            model: None,
        };
        assert_eq!(pricing.transcribe(&voice, 500), 2 * 11 + 100 + 1_000);
    }
}
