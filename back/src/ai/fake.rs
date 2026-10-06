//! The deterministic provider: same request, same answer, no network.
//!
//! It answers the server's own templates the way a good model would, from
//! what the prompt contains — the co-GM's suggestions name the first clue
//! still to find, the first exit, the first front and the first NPC of
//! the context, plus one invented id that the server must drop — so the
//! whole chain (template → provider → schema → sanitising → ledger) runs
//! in tests and in development without a key (`AI_PROVIDER=fake`).
//! Images are small pixel patterns drawn from the prompt's hash.

use std::sync::Mutex;

use promptus_shared::sprite::Image;
use serde_json::json;

use super::{
    AiError, BoxFuture, ImageRequest, ImageResponse, LlmRequest, LlmResponse, Provider, Role, Usage,
};

/// What the fake answers with for one call.
#[derive(Debug, Default)]
pub struct FakeProvider {
    /// Every call, in order: `complete:<first words of the system>` or
    /// `image`.
    calls: Mutex<Vec<String>>,
    /// Answer text that is not JSON (to test schema errors).
    pub broken: bool,
    /// What each call reports it cost; 0 means 1 000 µ$ (a tenth of a cent).
    pub cost_micros: i64,
}

impl FakeProvider {
    /// A provider whose answers are prose, not the JSON asked for.
    #[must_use]
    pub fn broken() -> Self {
        Self {
            broken: true,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn record(&self, call: String) {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(call);
    }

    fn usage(&self) -> Usage {
        Usage {
            prompt_tokens: 1_000,
            completion_tokens: 400,
            cost_micros: if self.cost_micros > 0 {
                self.cost_micros
            } else {
                1_000
            },
        }
    }
}

/// The id after `prefix` on the first line of `text` that starts with it.
fn first_id(text: &str, prefix: &str, suffix: Option<&str>) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(prefix)?;
        if let Some(s) = suffix
            && !line.contains(s)
        {
            return None;
        }
        rest.split_whitespace().next().map(str::to_string)
    })
}

fn after_line<'a>(text: &'a str, header: &str) -> Option<&'a str> {
    let at = text.find(header)?;
    text[at + header.len()..].lines().nth(1)
}

fn copilot_answer(prompt: &str) -> serde_json::Value {
    let precision = prompt
        .lines()
        .find_map(|l| l.strip_prefix("Précision du MJ : "))
        .unwrap_or("")
        .trim()
        .to_string();
    let clue = first_id(prompt, "- indice ", Some("(à trouver)"));
    let npc = first_id(prompt, "- pnj ", None);
    let front = first_id(prompt, "- front ", None);
    let exit = after_line(prompt, "Sorties :")
        .and_then(|l| l.trim_start().strip_prefix("- "))
        .and_then(|l| l.split_whitespace().next())
        .map(str::to_string);
    let mut narration =
        String::from("Le silence retombe, lourd, et chacun guette le prochain geste.");
    if !precision.is_empty() {
        narration = format!("{narration} ({precision})");
    }
    let mut suggestions = Vec::new();
    if let Some(c) = &clue {
        suggestions.push(json!({ "label": "Laisser trouver un indice", "why": "La piste s’essouffle.", "action": { "type": "reveal_clue", "clue": c } }));
    }
    if let Some(f) = &front {
        suggestions.push(json!({ "label": "Faire avancer la menace", "action": { "type": "advance_front", "front": f } }));
    }
    if let Some(n) = &exit {
        suggestions.push(json!({ "label": "Ouvrir la scène suivante", "action": { "type": "enter_scene", "node": n } }));
    }
    suggestions.push(json!({ "label": "Action fantaisiste", "action": { "type": "reveal_clue", "clue": "indice-invente" } }));
    let npc_lines = npc
        .map(|id| vec![json!({ "npc": id, "text": "Je n’ai rien vu, et vous non plus." })])
        .unwrap_or_default();
    json!({
        "narration": narration,
        "npcLines": npc_lines,
        "suggestions": suggestions,
        "gmNote": "Donnez la main à celui qui n’a pas joué depuis longtemps.",
    })
}

fn recap_answer(prompt: &str) -> serde_json::Value {
    let journal: Vec<&str> = prompt
        .split("# Ce que la table sait (journal des joueurs)")
        .nth(1)
        .and_then(|s| s.split("# Ce que le MJ sait en plus").next())
        .map(|s| {
            s.lines()
                .filter_map(|l| l.trim().strip_prefix("- "))
                .collect()
        })
        .unwrap_or_default();
    let told = if journal.is_empty() {
        "rien de notable".to_string()
    } else {
        journal.join(" ; ")
    };
    json!({
        "players": format!("Précédemment… {told}."),
        "gm": journal.iter().map(|l| format!("- {l}")).collect::<Vec<_>>().join("\n"),
    })
}

/// A 32 × 32 pixel pattern, mirrored like a crest, from the prompt's
/// FNV-1a hash.
fn pattern(prompt: &str) -> Vec<u8> {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in prompt.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    let (w, h) = (32u32, 32u32);
    let ink = [
        (hash >> 8) as u8 | 0x40,
        (hash >> 16) as u8 | 0x40,
        (hash >> 24) as u8 | 0x40,
    ];
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let mx = if x < w / 2 { x } else { w - 1 - x };
            let bit = (hash.rotate_left((mx / 4 + (y / 4) * 4) % 64)) & 1 == 1;
            let px = if bit { ink } else { [0x14, 0x14, 0x18] };
            rgba.extend_from_slice(&[px[0], px[1], px[2], 0xff]);
        }
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
    .to_png()
}

impl Provider for FakeProvider {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn complete<'a>(&'a self, req: &'a LlmRequest) -> BoxFuture<'a, Result<LlmResponse, AiError>> {
        Box::pin(async move {
            let system = req
                .messages
                .iter()
                .find(|m| m.role == Role::System)
                .map(|m| m.content.as_str())
                .unwrap_or("");
            let prompt = req
                .messages
                .iter()
                .rev()
                .find(|m| m.role == Role::User)
                .map(|m| m.content.as_str())
                .unwrap_or("");
            let words: String = system
                .split_whitespace()
                .take(4)
                .collect::<Vec<_>>()
                .join(" ");
            self.record(format!("complete:{words}"));
            let text = if self.broken {
                "Voici ma réponse : { pas du json".to_string()
            } else if system.contains("co-MJ") {
                copilot_answer(prompt).to_string()
            } else if system.contains("récapitulatifs") {
                recap_answer(prompt).to_string()
            } else {
                return Err(AiError::Refused {
                    status: 400,
                    message: format!("faux fournisseur : prompt inconnu ({words})"),
                });
            };
            Ok(LlmResponse {
                text,
                model: req.model.clone().unwrap_or_else(|| "fake/promptus".into()),
                usage: self.usage(),
            })
        })
    }

    fn image<'a>(&'a self, req: &'a ImageRequest) -> BoxFuture<'a, Result<ImageResponse, AiError>> {
        Box::pin(async move {
            self.record("image".into());
            Ok(ImageResponse {
                bytes: pattern(&req.prompt),
                mime: "image/png".into(),
                model: req.model.clone().unwrap_or_else(|| "fake/pixel".into()),
                usage: self.usage(),
            })
        })
    }
}
