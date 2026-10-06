//! Between two sessions, as a player sees it (planche « Entre deux »):
//! built here, by allow-list, like the rest of the projection
//! (`MEMORY.md` §3).
//!
//! Reaches players: what **their own** character gained in the last
//! session (XP, items, purse), and whether the GM published its recap
//! yet; « Précédemment… » of the last published session with what the
//! table learned (clues, revelations) and the threads left open; the
//! chronicle (title, two lines, date of each published session); the
//! next date being planned — the proposed times, the one chosen, when
//! the lobby opens, and the caller's own answer.
//!
//! Never: the GM's recap, the facts' fronts and party state, an
//! unpublished recap or chronicle entry, other players' answers or
//! gains.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::evening::schedule::Plan;
use crate::evening::session::Session;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BetweenView {
    pub last_session: Option<LastSessionView>,
    pub previously: Option<PreviouslyView>,
    /// Published sessions, oldest first.
    pub chronicle: Vec<ChronicleEntryView>,
    pub next: Option<NextSessionView>,
}

/// The caller's last session, from their side.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastSessionView {
    pub number: i32,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_minutes: i64,
    /// The chronicle title, once published.
    pub title: Option<String>,
    pub published: bool,
    pub xp_gained: i32,
    pub gains: Vec<GainView>,
}

/// One thing the caller's character gained (or spent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GainView {
    /// `item` or `resource`.
    pub kind: &'static str,
    pub label: String,
    pub delta: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviouslyView {
    pub number: i32,
    pub text: String,
    /// Clues found that session.
    pub clues: Vec<String>,
    /// What the table now knows.
    pub revelations: Vec<String>,
    /// Promises and debts still to honour.
    pub open_threads: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChronicleEntryView {
    pub number: i32,
    pub title: String,
    pub text: String,
    pub date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextSessionView {
    pub number: i32,
    pub options: Vec<DateTime<Utc>>,
    pub chosen_at: Option<DateTime<Utc>>,
    pub lobby_at: Option<DateTime<Utc>>,
    /// The caller's own answer; `None` before they gave one.
    pub mine: Option<Vec<DateTime<Utc>>>,
    /// How many players answered (never who, nor what).
    pub answered: usize,
}

/// One logged change to the caller's character during the last session
/// (`play_adjustments`, read by the server).
#[derive(Debug, Clone)]
pub struct Change {
    pub kind: String,
    pub label: Option<String>,
    pub before: i32,
    pub after: i32,
}

pub struct BetweenInput<'a> {
    /// Every session, the latest first.
    pub sessions: &'a [Session],
    /// The caller's character's changes during the last ended session.
    pub changes: &'a [Change],
    pub plan: Option<&'a Plan>,
    pub next_number: i32,
    pub caller: uuid::Uuid,
}

/// What a player reads between two sessions.
#[must_use]
pub fn project_between(input: &BetweenInput<'_>) -> BetweenView {
    let ended: Vec<&Session> = input
        .sessions
        .iter()
        .filter(|s| s.status == crate::evening::Status::Ended)
        .collect();
    let last_session = ended.first().map(|s| {
        let mut xp_gained = 0;
        let mut gains: Vec<GainView> = Vec::new();
        for c in input.changes {
            let delta = c.after - c.before;
            match c.kind.as_str() {
                "xp" => xp_gained += delta,
                kind @ ("item" | "resource") => {
                    let label = c.label.clone().unwrap_or_default();
                    let kind = if kind == "item" { "item" } else { "resource" };
                    match gains
                        .iter_mut()
                        .find(|g| g.kind == kind && g.label == label)
                    {
                        Some(g) => g.delta += delta,
                        None => gains.push(GainView { kind, label, delta }),
                    }
                }
                _ => {}
            }
        }
        gains.retain(|g| g.delta != 0);
        LastSessionView {
            number: s.number,
            ended_at: s.ended_at,
            duration_minutes: match (s.started_at, s.ended_at) {
                (Some(a), Some(b)) => (b - a).num_minutes().max(0),
                _ => 0,
            },
            title: s.published.then(|| s.title.clone()),
            published: s.published,
            xp_gained,
            gains,
        }
    });
    let published: Vec<&&Session> = ended.iter().filter(|s| s.published).collect();
    let previously = published.first().map(|s| {
        let facts = s.facts.clone().unwrap_or_default();
        PreviouslyView {
            number: s.number,
            text: s.previously.clone(),
            clues: facts.clues.into_iter().map(|c| c.text).collect(),
            revelations: facts.revelations.into_iter().map(|r| r.statement).collect(),
            open_threads: facts.open_threads,
        }
    });
    let chronicle = published
        .iter()
        .rev()
        .map(|s| ChronicleEntryView {
            number: s.number,
            title: s.title.clone(),
            text: s.chronicle.clone(),
            date: s.started_at.or(s.ended_at),
        })
        .collect();
    let next = input.plan.map(|p| NextSessionView {
        number: input.next_number,
        options: p.options.clone(),
        chosen_at: p.chosen_at,
        lobby_at: p.lobby_at(),
        mine: p
            .answers
            .iter()
            .find(|a| a.player_id == input.caller)
            .map(|a| a.available.clone()),
        answered: p.answers.len(),
    });
    BetweenView {
        last_session,
        previously,
        chronicle,
        next,
    }
}
