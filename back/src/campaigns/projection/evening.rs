//! The evening as a player (or the shared screen) sees it: built here, by
//! allow-list, like the rest of the projection (`MEMORY.md` §3).
//!
//! Reaches players: the session's number and state, « Précédemment… » of
//! the last ended session, the music playing and when it started, who
//! is in the lobby (nicknames), the campaign view of the current scene
//! (`project_for_players`), the shared lines of the journal (no story
//! ids, no GM line), the player's **own** requests with the GM's answer
//! and the server's roll, the cards they can play, and whether the last
//! session waits for their feedback.
//!
//! Never: the GM's recap and their note of changes, the gaps of
//! knowledge, the rulings, other players' requests, GM-only journal
//! lines, the spotlight.

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::check::RollBreakdown;
use promptus_shared::story::{Campaign, WorldState};
use serde::Serialize;
use uuid::Uuid;

use super::{PlayView, PlayerCampaignView, project_for_players};
use crate::evening::knowledge::{JournalKind, JournalLine};
use crate::evening::requests::{Card, Request, RequestStatus};
use crate::evening::session::{Attendance, Music, Session, Status};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EveningView {
    pub session: Option<SessionView>,
    /// « Précédemment… » of the last ended session, when the GM wrote one.
    pub previously: Option<String>,
    pub music: Option<Music>,
    pub lobby: Vec<LobbySeatView>,
    pub campaign: PlayerCampaignView,
    pub journal: Vec<JournalLineView>,
    /// The caller's own requests in this session, oldest first.
    pub requests: Vec<RequestView>,
    /// What the caller can play in a scene; empty for a spectator or a
    /// character not in play.
    pub cards: Vec<SceneCardView>,
    /// The last ended session asks for this player's answers.
    pub feedback: Option<FeedbackAskView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub number: i32,
    pub status: Status,
    pub started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LobbySeatView {
    pub player_id: Uuid,
    pub nickname: String,
    pub sound_ok: bool,
    pub remote: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalLineView {
    pub kind: JournalKind,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardRef {
    pub kind: &'static str,
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckView {
    pub ability: String,
    pub ability_name: String,
    pub difficulty: i32,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestView {
    pub id: Uuid,
    pub card: CardRef,
    pub text: String,
    pub status: RequestStatus,
    pub gm_reason: Option<String>,
    pub check: Option<CheckView>,
    /// The server's roll: what the die animates.
    pub roll: Option<RollBreakdown>,
    /// The outcome's name in the rules (« Réussite »).
    pub outcome: Option<String>,
    pub contested: bool,
    pub created_at: DateTime<Utc>,
}

/// A card a player can play in a scene.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneCardView {
    /// `ability` or `action`.
    pub kind: &'static str,
    pub id: String,
    pub name: String,
    pub description: String,
    /// The ability's modifier, for an ability card.
    pub modifier: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackAskView {
    pub number: i32,
    pub answered: bool,
}

/// Everything the evening API read, before projection.
pub struct EveningInput<'a> {
    pub campaign: &'a Campaign,
    pub world: &'a WorldState,
    pub rules: Option<&'a RuleSystem>,
    pub current: Option<&'a Session>,
    pub last_ended: Option<&'a Session>,
    /// Who is in the lobby, with their nickname.
    pub lobby: &'a [(Attendance, String)],
    pub journal: &'a [JournalLine],
    /// The caller's own requests.
    pub requests: &'a [Request],
    /// The caller's character in play, if any.
    pub play: Option<&'a PlayView>,
    /// `Some(answered)` when the caller is a player and the last ended
    /// session has no open successor.
    pub feedback_answered: Option<bool>,
}

fn card_ref(rules: Option<&RuleSystem>, card: &Card) -> CardRef {
    match card {
        Card::Ability { ability } => CardRef {
            kind: "ability",
            id: Some(ability.clone()),
            name: card.name(rules),
        },
        Card::Action { action } => CardRef {
            kind: "action",
            id: Some(action.clone()),
            name: card.name(rules),
        },
        Card::Other => CardRef {
            kind: "other",
            id: None,
            name: None,
        },
    }
}

fn request_view(rules: Option<&RuleSystem>, r: &Request) -> RequestView {
    RequestView {
        id: r.id,
        card: card_ref(rules, &r.card),
        text: r.text.clone(),
        status: r.status,
        gm_reason: r.gm_reason.clone(),
        check: r.check.as_ref().map(|c| CheckView {
            ability: c.ability.clone(),
            ability_name: rules
                .and_then(|s| s.abilities.iter().find(|a| a.id == c.ability))
                .map_or_else(|| c.ability.clone(), |a| a.name.clone()),
            difficulty: c.difficulty,
            label: c.label.clone(),
        }),
        roll: r.roll.clone(),
        outcome: match (rules, r.roll.as_ref().and_then(|b| b.band)) {
            (Some(s), Some(band)) => Some(band.name(s).to_string()),
            _ => None,
        },
        contested: r.contested,
        created_at: r.created_at,
    }
}

fn scene_cards(rules: Option<&RuleSystem>, play: Option<&PlayView>) -> Vec<SceneCardView> {
    let (Some(rules), Some(play)) = (rules, play) else {
        return Vec::new();
    };
    let mut cards: Vec<SceneCardView> = rules
        .abilities
        .iter()
        .map(|a| SceneCardView {
            kind: "ability",
            id: a.id.clone(),
            name: a.name.clone(),
            description: a.description.clone(),
            modifier: play
                .abilities
                .iter()
                .find(|x| x.id == a.id)
                .map(|x| x.modifier),
        })
        .collect();
    cards.extend(
        play.cards
            .iter()
            .filter(|c| c.level <= play.level)
            .map(|c| SceneCardView {
                kind: "action",
                id: c.id.clone(),
                name: c.name.clone(),
                description: c.description.clone(),
                modifier: None,
            }),
    );
    cards
}

/// The evening as the caller may see it.
#[must_use]
pub fn project_evening(input: &EveningInput<'_>) -> EveningView {
    let session = input.current.map(|s| SessionView {
        number: s.number,
        status: s.status,
        started_at: s.started_at,
    });
    let previously = input
        .last_ended
        .map(|s| s.previously.clone())
        .filter(|p| !p.trim().is_empty());
    EveningView {
        session,
        previously,
        music: input.current.and_then(|s| s.music.clone()),
        lobby: input
            .lobby
            .iter()
            .map(|(a, nickname)| LobbySeatView {
                player_id: a.player_id,
                nickname: nickname.clone(),
                sound_ok: a.sound_ok,
                remote: a.remote,
            })
            .collect(),
        campaign: project_for_players(input.campaign, input.world),
        journal: input
            .journal
            .iter()
            .filter(|l| l.shared)
            .map(|l| JournalLineView {
                kind: l.kind,
                text: l.text.clone(),
                created_at: l.created_at,
            })
            .collect(),
        requests: input
            .requests
            .iter()
            .map(|r| request_view(input.rules, r))
            .collect(),
        cards: scene_cards(input.rules, input.play),
        feedback: match (input.last_ended, input.feedback_answered) {
            (Some(s), Some(answered)) => Some(FeedbackAskView {
                number: s.number,
                answered,
            }),
            _ => None,
        },
    }
}
