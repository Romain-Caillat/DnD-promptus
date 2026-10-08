//! Between two sessions, as a player sees it (player/play-between-sessions,
//! planche « Entre deux »): built here, by allow-list, like the rest of
//! the projection (`MEMORY.md` §3).
//!
//! Reaches the player: the last ended session's number, when it ended,
//! how long it was played, the scene it ended on (the players were in
//! it), its « Précédemment… » once published, and the shared journal
//! lines of that evening (clues, people, items); for their **own**
//! character, the XP that evening brought as a number, the level before
//! and after, and the shared lines that name them (loot handed to them,
//! a purchase); the level-up moment — points left to spend and the
//! cards the new level opened; the chronicle — one entry per ended
//! session — and the threads still open (shared promises and debts).
//!
//! Never: the GM's recap, their note of changes, the gaps of knowledge,
//! GM-only journal lines, story ids, another player's sheet, nor any
//! line of the GM's history of adjustments (`play_adjustments`): only
//! sums of the caller's own XP are read from it, server-side.

use chrono::{DateTime, Utc};
use promptus_shared::story::{Campaign, WorldState};
use serde::Serialize;

use super::evening::SessionView;
use super::{AbilityScoreView, ActionCardView, PlayView, project_for_players};
use crate::evening::knowledge::{JournalKind, JournalLine};
use crate::evening::session::Session;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BetweenView {
    /// The session open now (lobby or live), if any: the between screen
    /// then gives way to the evening.
    pub open: Option<SessionView>,
    pub last: Option<LastSessionView>,
    /// Present while the caller has upgrade points to spend.
    pub level_up: Option<LevelUpView>,
    /// One entry per ended session, the first session first.
    pub chronicle: Vec<ChronicleEntryView>,
    /// Promises and debts the table made, the oldest first.
    pub open_threads: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastSessionView {
    pub number: i32,
    pub ended_at: Option<DateTime<Utc>>,
    /// From the start to the end of the evening, in minutes.
    pub minutes: Option<i64>,
    /// The scene the evening ended in.
    pub title: Option<String>,
    /// « Précédemment… », once the GM published it.
    pub previously: Option<String>,
    /// What the group learnt that evening: shared clue, NPC and item lines.
    pub learnt: Vec<String>,
    /// The caller's own evening; `None` for a spectator.
    pub mine: Option<MyEveningView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyEveningView {
    /// XP earned during the evening (what the GM took back counted out).
    pub xp_gained: i64,
    pub level_before: u32,
    pub level: u32,
    pub total_xp: u32,
    pub xp_bar: u32,
    pub xp_bar_max: u32,
    pub next_level_xp: Option<u32>,
    /// The shared lines of that evening about the caller's character:
    /// loot handed to them, what they bought or were given.
    pub got: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelUpView {
    pub level: u32,
    /// Points earned and not spent: one each is +1 to a score.
    pub points: u32,
    pub abilities: Vec<AbilityScoreView>,
    /// Cards the levels reached since the last evening started opened.
    pub new_cards: Vec<ActionCardView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChronicleEntryView {
    pub number: i32,
    pub ended_at: Option<DateTime<Utc>>,
    pub title: Option<String>,
    pub text: Option<String>,
}

/// The caller's character, read before projection.
pub struct Mine<'a> {
    pub play: &'a PlayView,
    /// XP earned between the last session's start and its end.
    pub xp_in_session: i64,
    /// XP earned since the last session started, until now.
    pub xp_since_start: i64,
    /// The level a total XP makes, under the campaign's rules.
    pub level_for: &'a (dyn Fn(u32) -> u32 + Sync),
    /// The shared lines of the last session about this character.
    pub got: Vec<String>,
}

/// Everything the between API read.
pub struct BetweenInput<'a> {
    pub campaign: &'a Campaign,
    pub current: Option<&'a Session>,
    /// Ended sessions, the first first, with the world they ended in.
    pub ended: &'a [(Session, Option<WorldState>)],
    /// The shared journal, oldest first.
    pub journal: &'a [JournalLine],
    pub mine: Option<Mine<'a>>,
}

fn title(campaign: &Campaign, world: Option<&WorldState>) -> Option<String> {
    world
        .and_then(|w| project_for_players(campaign, w).scene)
        .map(|s| s.title)
}

/// The between screen as the caller may see it.
#[must_use]
pub fn project_between(input: &BetweenInput<'_>) -> BetweenView {
    let shared = || input.journal.iter().filter(|l| l.shared);
    let last = input.ended.last().map(|(s, world)| {
        let learnt = shared()
            .filter(|l| l.session_id == Some(s.id))
            .filter(|l| {
                matches!(
                    l.kind,
                    JournalKind::Clue | JournalKind::Npc | JournalKind::Item
                )
            })
            .map(|l| l.text.clone())
            .collect();
        let mine = input.mine.as_ref().map(|m| {
            let since = u32::try_from(m.xp_since_start.max(0)).unwrap_or(u32::MAX);
            MyEveningView {
                xp_gained: m.xp_in_session,
                level_before: (m.level_for)(m.play.total_xp.saturating_sub(since)),
                level: m.play.level,
                total_xp: m.play.total_xp,
                xp_bar: m.play.xp_bar,
                xp_bar_max: m.play.xp_bar_max,
                next_level_xp: m.play.next_level_xp,
                got: m.got.clone(),
            }
        });
        LastSessionView {
            number: s.number,
            ended_at: s.ended_at,
            minutes: match (s.started_at, s.ended_at) {
                (Some(a), Some(b)) => Some((b - a).num_minutes()),
                _ => None,
            },
            title: title(input.campaign, world.as_ref()),
            previously: s.published_previously().map(str::to_string),
            learnt,
            mine,
        }
    });
    let level_up = input.mine.as_ref().and_then(|m| {
        let play = m.play;
        if play.upgrade_points == 0 {
            return None;
        }
        let since = u32::try_from(m.xp_since_start.max(0)).unwrap_or(u32::MAX);
        let before = if input.ended.is_empty() {
            play.level
        } else {
            (m.level_for)(play.total_xp.saturating_sub(since))
        };
        Some(LevelUpView {
            level: play.level,
            points: play.upgrade_points,
            abilities: play.abilities.clone(),
            new_cards: play
                .cards
                .iter()
                .filter(|c| c.level > before && c.level <= play.level)
                .cloned()
                .collect(),
        })
    });
    BetweenView {
        open: input.current.map(|s| SessionView {
            number: s.number,
            status: s.status,
            started_at: s.started_at,
        }),
        last,
        level_up,
        chronicle: input
            .ended
            .iter()
            .map(|(s, world)| {
                // Only what the GM published (session/write-recaps): the
                // entry's own title, else the scene the evening ended in.
                let published = s.published_chronicle();
                ChronicleEntryView {
                    number: s.number,
                    ended_at: s.ended_at,
                    title: published
                        .map(|(t, _)| t)
                        .filter(|t| !t.is_empty())
                        .map(str::to_string)
                        .or_else(|| title(input.campaign, world.as_ref())),
                    text: published.map(|(_, text)| text.to_string()),
                }
            })
            .collect(),
        open_threads: shared()
            .filter(|l| matches!(l.kind, JournalKind::Promise | JournalKind::Debt))
            .map(|l| l.text.clone())
            .collect(),
    }
}
