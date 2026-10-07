//! copilot/listen-by-voice — the GM dictates to the co-GM.
//!
//! The GM's screen records their voice while they hold the microphone
//! (push to talk: the table's own voices stay on Discord, `MEMORY.md`
//! §1) and sends it as a 16-bit PCM WAV file. A model that hears audio
//! writes it down (template `transcribe.v1`, a counted call), helped by
//! the campaign's proper nouns; the words then go to the co-GM exactly
//! as if the GM had typed them (`drafts::ask`), and its answer is an
//! ordinary draft: the GM edits it, shows it or drops it. The transcript
//! itself never reaches a player — it is the draft's prompt, on the GM's
//! screen only.

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use promptus_shared::story::Campaign;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::drafts::{self, Draft};
use super::{Ask, Kind};
use crate::ai::{self, Ai, TranscribeRequest, ledger, templates};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::error::AppError;
use crate::evening::session::{self, Status};
use crate::players;

/// The longest dictation: a minute is plenty to ask the co-GM, and it
/// bounds both the upload and the bill.
pub const MAX_SECONDS: f32 = 60.0;
/// Below this, the GM touched the button by mistake.
const MIN_SECONDS: f32 = 0.3;
/// A minute at 48 kHz stereo 16-bit (what a client that does not
/// downsample sends), plus the header: the largest file accepted.
pub const MAX_WAV_BYTES: usize = 48_000 * 2 * 2 * 60 + 1_024;

/// What the GM's screen sends.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Dictation {
    /// The WAV file, base64.
    pub audio: String,
    /// What to ask the co-GM with these words; the GM's own question
    /// (`free`) when absent.
    #[serde(default)]
    pub kind: Option<Kind>,
    #[serde(default)]
    pub npc: Option<String>,
}

/// What the co-GM heard, and its draft.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Heard {
    pub transcript: String,
    pub seconds: f32,
    pub draft: Draft,
}

/// The length of a 16-bit PCM WAV file, in seconds.
///
/// # Errors
///
/// `AUDIO_INVALID` when it is not a RIFF/WAVE file with a PCM 16-bit
/// `fmt ` chunk followed by a `data` chunk.
pub fn wav_seconds(wav: &[u8]) -> Result<f32, &'static str> {
    const INVALID: &str = "AUDIO_INVALID";
    if wav.len() < 12 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return Err(INVALID);
    }
    let u16_at = |at: usize| u16::from_le_bytes([wav[at], wav[at + 1]]);
    let u32_at = |at: usize| u32::from_le_bytes([wav[at], wav[at + 1], wav[at + 2], wav[at + 3]]);
    let mut at = 12;
    let mut byte_rate: Option<u32> = None;
    while at + 8 <= wav.len() {
        let id = &wav[at..at + 4];
        let len = u32_at(at + 4) as usize;
        let body = at + 8;
        if id == b"fmt " {
            if len < 16 || body + 16 > wav.len() {
                return Err(INVALID);
            }
            let format = u16_at(body);
            let channels = u16_at(body + 2);
            let rate = u32_at(body + 4);
            let bits = u16_at(body + 14);
            if format != 1 || bits != 16 || channels == 0 || rate == 0 {
                return Err(INVALID);
            }
            byte_rate = Some(rate * u32::from(channels) * 2);
        } else if id == b"data" {
            let rate = byte_rate.ok_or(INVALID)?;
            // A recorder that could not seek writes a placeholder length:
            // the bytes actually there are what counts.
            let samples = len.min(wav.len() - body);
            return Ok(samples as f32 / rate as f32);
        }
        // Chunks are padded to an even length.
        at = body + len + (len & 1);
    }
    Err(INVALID)
}

/// The campaign's proper nouns, for the model to spell them right: the
/// title, the party, the NPCs, adversaries, places, factions and items,
/// and the names at the table (nicknames and characters).
#[must_use]
pub fn vocabulary(story: &Campaign, table: &[String]) -> String {
    let mut names: Vec<String> = Vec::new();
    let mut add = |n: &str| {
        let n = n.trim();
        if !n.is_empty() && !names.iter().any(|x| x == n) {
            names.push(n.to_string());
        }
    };
    add(&story.title);
    story.party.iter().for_each(|p| add(&p.name));
    story.npcs.iter().for_each(|n| add(&n.name));
    story.adversaries.iter().for_each(|a| add(&a.name));
    story.locations.iter().for_each(|l| add(&l.name));
    story.factions.iter().for_each(|f| add(&f.name));
    story.items.iter().for_each(|i| add(&i.name));
    table.iter().for_each(|t| add(t));
    // The prompt stays small: the first 200 names are the ones that
    // matter (cast first).
    names.truncate(200);
    names.join(", ")
}

#[derive(Debug, Deserialize)]
struct Transcript {
    text: String,
}

/// Hear the GM, then ask the co-GM with their words.
///
/// # Errors
///
/// 404 when the campaign is missing or another GM's; 409
/// `SESSION_NOT_LIVE`, `AI_BUDGET_EXCEEDED` (no call made); 400
/// `AUDIO_INVALID`, `RECORDING_TOO_SHORT`, `RECORDING_TOO_LONG`,
/// `NOTHING_HEARD` (the transcription is still counted), `UNKNOWN_NPC`;
/// 503 `AI_NOT_CONFIGURED`; 502 `AI_UNAVAILABLE`, `AI_OUTPUT_INVALID`.
pub async fn listen(
    pool: &PgPool,
    ai: &Ai,
    gm: &CurrentGm,
    campaign: Uuid,
    dictation: &Dictation,
) -> Result<Heard, AppError> {
    let wav = STANDARD
        .decode(dictation.audio.trim())
        .map_err(|_| AppError::BadRequest("AUDIO_INVALID"))?;
    if wav.len() > MAX_WAV_BYTES {
        return Err(AppError::BadRequest("RECORDING_TOO_LONG"));
    }
    let seconds = wav_seconds(&wav).map_err(AppError::BadRequest)?;
    if seconds < MIN_SECONDS {
        return Err(AppError::BadRequest("RECORDING_TOO_SHORT"));
    }
    if seconds > MAX_SECONDS + 1.0 {
        return Err(AppError::BadRequest("RECORDING_TOO_LONG"));
    }
    // Everything the ask would refuse is refused before the voice is
    // sent: no transcription is paid for nothing.
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    session::current(pool, campaign)
        .await?
        .filter(|s| s.status == Status::Live)
        .ok_or(AppError::Conflict("SESSION_NOT_LIVE"))?;
    if let Some(npc) = &dictation.npc
        && row.story.npc(npc).is_none()
    {
        return Err(AppError::BadRequest("UNKNOWN_NPC"));
    }

    let table: Vec<String> = players::seats(pool, campaign)
        .await?
        .into_iter()
        .flat_map(|s| [Some(s.nickname), s.character.map(|c| c.name)])
        .flatten()
        .collect();
    let vars: BTreeMap<&str, String> = [("vocabulary", vocabulary(&row.story, &table))]
        .into_iter()
        .collect();
    let messages = templates::TRANSCRIBE
        .render(&vars)
        .map_err(|e| AppError::internal("transcribe template", e))?;
    let reply = ai
        .transcribe(
            pool,
            campaign,
            "copilot.voice",
            &templates::TRANSCRIBE,
            &TranscribeRequest {
                messages,
                wav,
                seconds,
                model: None,
            },
        )
        .await?;
    let heard: Transcript = ai::parse_json(&reply.text).map_err(|e| ledger::app_error(&e))?;
    let transcript: String = heard
        .text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(drafts::PROMPT_MAX)
        .collect();
    if transcript.is_empty() {
        return Err(AppError::BadRequest("NOTHING_HEARD"));
    }
    let draft = drafts::ask(
        pool,
        ai,
        gm,
        campaign,
        &Ask {
            kind: dictation.kind.unwrap_or(Kind::Free),
            npc: dictation.npc.clone(),
            prompt: transcript.clone(),
        },
    )
    .await?;
    Ok(Heard {
        transcript,
        seconds,
        draft,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mono 16-bit WAV at `rate` holding `samples`.
    fn wav(rate: u32, channels: u16, samples: &[u8]) -> Vec<u8> {
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&(36 + samples.len() as u32).to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&channels.to_le_bytes());
        w.extend_from_slice(&rate.to_le_bytes());
        w.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        w.extend_from_slice(&(channels * 2).to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&(samples.len() as u32).to_le_bytes());
        w.extend_from_slice(samples);
        w
    }

    #[test]
    fn a_wav_says_how_long_it_lasts() {
        // Two seconds of 16 kHz mono, then one second of 8 kHz stereo.
        assert_eq!(wav_seconds(&wav(16_000, 1, &vec![0; 64_000])), Ok(2.0));
        assert_eq!(wav_seconds(&wav(8_000, 2, &vec![0; 32_000])), Ok(1.0));
        // A placeholder length longer than the file: the bytes count.
        let mut w = wav(16_000, 1, &vec![0; 16_000]);
        let at = w.len() - 16_000 - 4;
        w[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(wav_seconds(&w), Ok(0.5));
    }

    #[test]
    fn anything_but_16_bit_pcm_wav_is_refused() {
        assert_eq!(wav_seconds(b"OggS\0\0\0\0\0\0\0\0"), Err("AUDIO_INVALID"));
        assert_eq!(wav_seconds(b""), Err("AUDIO_INVALID"));
        let mut float = wav(16_000, 1, &[0; 10]);
        float[20] = 3; // IEEE float
        assert_eq!(wav_seconds(&float), Err("AUDIO_INVALID"));
        // A header cut before its data chunk.
        let cut = wav(16_000, 1, &[]);
        assert_eq!(wav_seconds(&cut[..36]), Err("AUDIO_INVALID"));
    }

    #[test]
    fn the_vocabulary_lists_each_name_once_cast_first() {
        let c: Campaign = promptus_shared::story::from_yaml(include_str!(
            "../../../content/campaigns/corsaires/campagne.yaml"
        ))
        .unwrap();
        let v = vocabulary(&c, &["Marc".into(), "Borin".into(), "Marc".into()]);
        assert!(v.starts_with(&c.title), "{v}");
        assert!(v.contains(&c.npcs[0].name), "{v}");
        assert_eq!(v.matches("Marc").count(), 1, "{v}");
        assert!(v.ends_with("Marc, Borin"), "{v}");
    }
}
