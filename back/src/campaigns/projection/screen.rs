//! The evening as the shared screen (TV) shows it (tv/show-evening),
//! built here by allow-list like the rest of the projection
//! (`MEMORY.md` §3).
//!
//! The screen is « every player at once »: it is built from what any
//! player may see, never from one player's own data. It reuses the
//! players' campaign view ([`project_for_players`]) and the spectator's
//! grid (`board::project_board` with no character, built by the route),
//! then the GM's switches ([`Shows`]) take away more.
//!
//! Reaches the screen: the campaign's title; the session's number and
//! state; « Précédemment… » of the last ended session; the music; the
//! party (character name, nickname, look, here or not, hit points — the
//! party's hit points are the table's, as in a fight); the current scene
//! as players see it; the grid as a spectator sees it; the shared lines
//! of the journal of this session (what every phone's journal shows);
//! the checks rolled this session, reduced to what the shared journal
//! line already says — who, which ability, the difficulty's name, the
//! dice and the outcome — so the die can roll on the TV; after the
//! session, its shared lines (« Ce soir »).
//!
//! Never: a request not rolled, the GM's answer to a request (a request
//! rolled reaches the screen only as far as its shared journal line
//! already tells every phone), anyone's sheet or bag, a spectator, the
//! GM's journal lines, recaps, notes, the spotlight, anything the
//! players' projection refuses.

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::check::RollBreakdown;
use promptus_shared::sprite::CharacterLook;
use promptus_shared::story::{Campaign, WorldState};
use serde::Serialize;
use uuid::Uuid;

use super::board::BoardView;
use super::evening::SessionView;
use super::{SceneView, project_for_players, project_play};
use crate::evening::knowledge::{JournalKind, JournalLine};
use crate::evening::requests::{Request, RequestStatus};
use crate::evening::session::{Attendance, Music, Session};
use crate::players::play::InPlay;
use crate::screens::Shows;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenView {
    pub title: String,
    /// What the GM lets the screen show (already applied below).
    pub shows: Shows,
    pub session: Option<SessionView>,
    pub previously: Option<String>,
    pub music: Option<Music>,
    pub party: Vec<ScreenSeat>,
    pub scene: Option<SceneView>,
    /// The grid as a spectator sees it.
    pub board: Option<BoardView>,
    /// The shared lines of the current session's journal, oldest first.
    pub moments: Vec<Moment>,
    /// The checks rolled this session, oldest first.
    pub rolls: Vec<ScreenRoll>,
    /// After a session: its shared lines.
    pub tonight: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenSeat {
    pub name: String,
    pub nickname: String,
    pub look: Option<CharacterLook>,
    /// Said « I am here » in the session's lobby.
    pub here: bool,
    pub hit_points: Option<i32>,
    pub max_hit_points: Option<i32>,
}

/// A shared line of the journal: a big moment when its kind is one.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moment {
    pub id: Uuid,
    pub kind: JournalKind,
    pub text: String,
    pub at: DateTime<Utc>,
}

/// A check rolled at the table: what the die shows, never what was
/// asked or answered.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenRoll {
    pub id: Uuid,
    pub character: String,
    pub ability: String,
    /// The difficulty's name, or its number.
    pub difficulty: String,
    pub roll: RollBreakdown,
    pub outcome: Option<String>,
    pub at: DateTime<Utc>,
}

/// Everything the screen route read, before projection.
pub struct ScreenInput<'a> {
    pub campaign: &'a Campaign,
    pub world: &'a WorldState,
    pub rules: Option<&'a RuleSystem>,
    pub shows: Shows,
    pub current: Option<&'a Session>,
    pub last_ended: Option<&'a Session>,
    pub attendance: &'a [Attendance],
    pub in_play: &'a [InPlay],
    /// The shared journal (`knowledge::journal(…, true)`).
    pub journal: &'a [JournalLine],
    /// The current session's requests (every player's: only rolled
    /// checks are kept, cut down).
    pub requests: &'a [Request],
    /// The spectator's grid, when one is shown.
    pub board: Option<BoardView>,
}

/// Most recent moments and rolls kept: enough for the feed and for the
/// moments a screen that just reconnected has not played yet.
const RECENT: usize = 20;
/// Lines of « Ce soir ».
const TONIGHT: usize = 6;

fn last<T>(mut v: Vec<T>, n: usize) -> Vec<T> {
    let cut = v.len().saturating_sub(n);
    v.split_off(cut)
}

/// The evening as the shared screen may show it now.
#[must_use]
pub fn project_screen(input: ScreenInput<'_>) -> ScreenView {
    let shows = input.shows;
    let campaign = project_for_players(input.campaign, input.world);
    let current = input.current;
    let shared_of = |session: Uuid| {
        input
            .journal
            .iter()
            .filter(move |l| l.shared && l.session_id == Some(session))
    };
    let party = if shows.party {
        input
            .in_play
            .iter()
            .map(|c| {
                let play = input
                    .rules
                    .and_then(|r| project_play(r, &c.sheet, c.state.as_ref()));
                ScreenSeat {
                    name: c.sheet.name.clone(),
                    nickname: c.nickname.clone(),
                    look: c.sheet.look.clone(),
                    here: input.attendance.iter().any(|a| a.player_id == c.player_id),
                    hit_points: play.as_ref().map(|p| p.hit_points),
                    max_hit_points: play.as_ref().map(|p| p.max_hit_points),
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    let moments = match (shows.moments, current) {
        (true, Some(s)) => last(
            shared_of(s.id)
                .map(|l| Moment {
                    id: l.id,
                    kind: l.kind,
                    text: l.text.clone(),
                    at: l.created_at,
                })
                .collect(),
            RECENT,
        ),
        _ => Vec::new(),
    };
    let rolls = if shows.moments {
        last(
            input
                .requests
                .iter()
                .filter(|r| r.status == RequestStatus::Rolled)
                .filter_map(|r| roll_of(input.rules, input.in_play, r))
                .collect(),
            RECENT,
        )
    } else {
        Vec::new()
    };
    let tonight = match (shows.moments, current, input.last_ended) {
        (true, None, Some(s)) => last(
            shared_of(s.id)
                .filter(|l| {
                    matches!(
                        l.kind,
                        JournalKind::Clue
                            | JournalKind::Npc
                            | JournalKind::Item
                            | JournalKind::Loot
                            | JournalKind::Fight
                    )
                })
                .map(|l| l.text.clone())
                .collect(),
            TONIGHT,
        ),
        _ => Vec::new(),
    };
    ScreenView {
        title: campaign.title.clone(),
        shows,
        session: current.map(|s| SessionView {
            number: s.number,
            status: s.status,
            started_at: s.started_at,
        }),
        previously: input
            .last_ended
            .filter(|_| shows.scene)
            .map(|s| s.previously.clone())
            .filter(|p| !p.trim().is_empty()),
        music: current.and_then(|s| s.music.clone()),
        party,
        scene: campaign.scene.filter(|_| shows.scene),
        board: input.board.filter(|_| shows.map),
        moments,
        rolls,
        tonight,
    }
}

fn roll_of(rules: Option<&RuleSystem>, in_play: &[InPlay], r: &Request) -> Option<ScreenRoll> {
    let roll = r.roll.clone()?;
    let check = r.check.as_ref()?;
    let character = in_play
        .iter()
        .find(|c| Some(c.id) == r.character_id)
        .map(|c| c.sheet.name.clone())?;
    Some(ScreenRoll {
        id: r.id,
        character,
        ability: rules
            .and_then(|s| s.abilities.iter().find(|a| a.id == check.ability))
            .map_or_else(|| check.ability.clone(), |a| a.name.clone()),
        difficulty: check
            .label
            .clone()
            .unwrap_or_else(|| check.difficulty.to_string()),
        outcome: match (rules, roll.band) {
            (Some(s), Some(band)) => Some(band.name(s).to_string()),
            _ => None,
        },
        roll,
        at: r.rolled_at.unwrap_or(r.created_at),
    })
}
