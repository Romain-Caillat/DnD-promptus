//! OpenRouter, the first [`Provider`]: its OpenAI-compatible chat API
//! for text, and the same endpoint with `modalities: ["image", "text"]`
//! for images, which come back as base64 data URLs. Video has its own
//! asynchronous API (`/videos`): a job is submitted, polled until it is
//! `completed` or `failed`, and its file downloaded. A transcription is a
//! chat call whose user message carries the WAV as an `input_audio` part
//! (base64), sent to a model that hears audio.
//!
//! OpenRouter reports the real cost of each call when asked
//! (`usage: { include: true }`), in dollars; it is stored in millionths.

use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    AiError, BoxFuture, ImageRequest, ImageResponse, LlmRequest, LlmResponse, Provider, Role,
    TranscribeRequest, TranscribeResponse, Usage, VideoRequest, VideoResponse,
};

pub const BASE_URL: &str = "https://openrouter.ai/api/v1";
pub const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4.5";
pub const DEFAULT_IMAGE_MODEL: &str = "google/gemini-2.5-flash-image";
pub const DEFAULT_VIDEO_MODEL: &str = "google/veo-3.1";
/// A model that takes audio in and is cheap per minute.
pub const DEFAULT_AUDIO_MODEL: &str = "google/gemini-2.5-flash";

/// How long a call may take before it is given up.
const TIMEOUT: Duration = Duration::from_secs(180);
/// How often a video job is polled, and for how long at most.
const VIDEO_POLL: Duration = Duration::from_secs(10);
const VIDEO_WAIT: Duration = Duration::from_secs(20 * 60);

#[derive(Debug, Clone)]
pub struct Settings {
    pub base_url: String,
    pub model: String,
    pub image_model: String,
    pub video_model: String,
    pub audio_model: String,
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

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.settings.base_url.trim_end_matches('/'))
    }

    /// A call to OpenRouter's API, its status and text.
    async fn send(&self, request: reqwest::RequestBuilder) -> Result<(u16, String), AiError> {
        let mut request = request.bearer_auth(&self.key).header("X-Title", "Promptus");
        if let Some(app) = &self.settings.app_url {
            request = request.header("HTTP-Referer", app);
        }
        let response = request
            .send()
            .await
            .map_err(|e| AiError::Unreachable(e.to_string()))?;
        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| AiError::Unreachable(e.to_string()))?;
        Ok((status, text))
    }

    /// A video job's state.
    async fn video_job(&self, request: reqwest::RequestBuilder) -> Result<VideoJob, AiError> {
        let (status, text) = self.send(request).await?;
        let job: VideoJob = serde_json::from_str(&text).map_err(|_| AiError::Refused {
            status,
            message: text.chars().take(300).collect(),
        })?;
        if !(200..300).contains(&status) || (job.error.is_some() && job.status.is_none()) {
            return Err(AiError::Refused {
                status,
                message: job
                    .error_text()
                    .unwrap_or_else(|| text.chars().take(300).collect()),
            });
        }
        Ok(job)
    }

    /// The finished video's bytes and type: from its unsigned URL, or
    /// from OpenRouter's content endpoint when there is none.
    async fn download(&self, job: &VideoJob) -> Result<(Vec<u8>, String), AiError> {
        let request = match job.unsigned_urls.first() {
            // A signed storage URL: no key goes with it.
            Some(url) => self.http.get(url),
            None => self
                .http
                .get(self.url(&format!("/videos/{}/content?index=0", job.id)))
                .bearer_auth(&self.key),
        };
        let response = request
            .send()
            .await
            .map_err(|e| AiError::Unreachable(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(AiError::Refused {
                status: status.as_u16(),
                message: "le fichier de la vidéo n’a pas pu être téléchargé".into(),
            });
        }
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.split(';').next().unwrap_or(v).trim().to_string())
            .filter(|v| v.starts_with("video/"))
            .unwrap_or_else(|| "video/mp4".into());
        let bytes = response
            .bytes()
            .await
            .map_err(|e| AiError::Unreachable(e.to_string()))?;
        if bytes.is_empty() {
            return Err(AiError::Schema("vidéo vide".into()));
        }
        Ok((bytes.to_vec(), mime))
    }

    async fn chat(&self, body: Value) -> Result<Completion, AiError> {
        let (status, text) = self
            .send(self.http.post(self.url("/chat/completions")).json(&body))
            .await?;
        let parsed: Completion = serde_json::from_str(&text).map_err(|_| AiError::Refused {
            status,
            message: text.chars().take(300).collect(),
        })?;
        if !(200..300).contains(&status) || parsed.error.is_some() {
            return Err(AiError::Refused {
                status,
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

/// A video job, as submitted or polled: `pending`, `in_progress`,
/// `completed` or `failed`.
#[derive(Debug, Deserialize)]
struct VideoJob {
    #[serde(default)]
    id: String,
    status: Option<String>,
    #[serde(default)]
    unsigned_urls: Vec<String>,
    usage: Option<ApiUsage>,
    /// A string or `{ message }`.
    error: Option<Value>,
}

impl VideoJob {
    fn error_text(&self) -> Option<String> {
        self.error.as_ref().map(|e| match e {
            Value::String(s) => s.clone(),
            other => other
                .get("message")
                .and_then(Value::as_str)
                .map_or_else(|| other.to_string(), str::to_string),
        })
    }
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

    fn video<'a>(&'a self, req: &'a VideoRequest) -> BoxFuture<'a, Result<VideoResponse, AiError>> {
        Box::pin(self.generate_video(req))
    }

    fn transcribe<'a>(
        &'a self,
        req: &'a TranscribeRequest,
    ) -> BoxFuture<'a, Result<TranscribeResponse, AiError>> {
        Box::pin(async move {
            let model = req
                .model
                .clone()
                .unwrap_or_else(|| self.settings.audio_model.clone());
            let body = json!({
                "model": model,
                "messages": audio_messages(req),
                "temperature": 0,
                "max_tokens": TRANSCRIPT_TOKENS,
                "response_format": { "type": "json_object" },
                "usage": { "include": true },
            });
            let completion = self.chat(body).await?;
            let text = completion
                .choices
                .first()
                .and_then(|c| c.message.as_ref())
                .and_then(|m| m.content.clone())
                .unwrap_or_default();
            if text.trim().is_empty() {
                return Err(AiError::Schema("transcription vide".into()));
            }
            Ok(TranscribeResponse {
                text,
                model: completion.model.clone().unwrap_or(model),
                usage: usage(completion.usage.as_ref()),
            })
        })
    }
}

/// The most a transcription may answer, in tokens: a minute of speech
/// is about 150 words.
pub const TRANSCRIPT_TOKENS: u32 = 600;

/// The template's messages, the recording appended to the last user
/// message as an `input_audio` part (OpenRouter's multimodal format).
fn audio_messages(req: &TranscribeRequest) -> Vec<Value> {
    let last_user = req.messages.iter().rposition(|m| m.role == Role::User);
    req.messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            if Some(i) == last_user {
                json!({
                    "role": m.role,
                    "content": [
                        { "type": "text", "text": m.content },
                        { "type": "input_audio",
                          "input_audio": { "data": STANDARD.encode(&req.wav), "format": "wav" } },
                    ],
                })
            } else {
                json!({ "role": m.role, "content": m.content })
            }
        })
        .collect()
}

impl OpenRouter {
    async fn generate_video(&self, req: &VideoRequest) -> Result<VideoResponse, AiError> {
        let model = req
            .model
            .clone()
            .unwrap_or_else(|| self.settings.video_model.clone());
        let body = json!({
            "model": model,
            "prompt": req.prompt,
            "duration": req.seconds,
            "aspect_ratio": "16:9",
            "resolution": "720p",
        });
        let mut job = self
            .video_job(self.http.post(self.url("/videos")).json(&body))
            .await?;
        if job.id.is_empty() {
            return Err(AiError::Schema("aucun identifiant de tâche vidéo".into()));
        }
        let started = tokio::time::Instant::now();
        loop {
            match job.status.as_deref() {
                Some("completed") => break,
                Some("failed") => {
                    return Err(AiError::Refused {
                        status: 200,
                        message: job
                            .error_text()
                            .unwrap_or_else(|| "la génération de la vidéo a échoué".into()),
                    });
                }
                _ if started.elapsed() > VIDEO_WAIT => {
                    return Err(AiError::Unreachable(
                        "la vidéo n’est pas prête après vingt minutes".into(),
                    ));
                }
                _ => {}
            }
            tokio::time::sleep(VIDEO_POLL).await;
            let id = job.id.clone();
            job = self
                .video_job(self.http.get(self.url(&format!("/videos/{id}"))))
                .await?;
            if job.id.is_empty() {
                job.id = id;
            }
        }
        let (bytes, mime) = self.download(&job).await?;
        Ok(VideoResponse {
            bytes,
            mime,
            model,
            usage: usage(job.usage.as_ref()),
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
    fn a_video_job_reads_as_documented_and_says_why_it_failed() {
        let done: VideoJob = serde_json::from_str(
            r#"{"generation_id":"gen-xyz789","id":"job-abc123",
                "polling_url":"/api/v1/videos/job-abc123","status":"completed",
                "unsigned_urls":["https://storage.example.com/video.mp4"],
                "usage":{"cost":0.5}}"#,
        )
        .unwrap();
        assert_eq!(done.status.as_deref(), Some("completed"));
        assert_eq!(
            done.unsigned_urls[0],
            "https://storage.example.com/video.mp4"
        );
        assert_eq!(usage(done.usage.as_ref()).cost_micros, 500_000);
        let failed: VideoJob = serde_json::from_str(
            r#"{"id":"j","status":"failed","error":{"message":"contenu refusé"}}"#,
        )
        .unwrap();
        assert_eq!(failed.error_text().as_deref(), Some("contenu refusé"));
        let plain: VideoJob = serde_json::from_str(r#"{"error":"clé invalide"}"#).unwrap();
        assert_eq!(plain.error_text().as_deref(), Some("clé invalide"));
    }

    #[test]
    fn the_recording_rides_with_the_last_user_message_as_base64_wav() {
        let req = TranscribeRequest {
            messages: vec![
                super::super::Message::system("Tu transcris."),
                super::super::Message::user("Noms : Vaubernier"),
            ],
            wav: b"RIFF".to_vec(),
            seconds: 1.0,
            model: None,
        };
        let m = audio_messages(&req);
        assert_eq!(
            m[0],
            json!({ "role": "system", "content": "Tu transcris." })
        );
        assert_eq!(m[1]["content"][0]["text"], "Noms : Vaubernier");
        assert_eq!(m[1]["content"][1]["type"], "input_audio");
        assert_eq!(m[1]["content"][1]["input_audio"]["format"], "wav");
        assert_eq!(m[1]["content"][1]["input_audio"]["data"], "UklGRg==");
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
