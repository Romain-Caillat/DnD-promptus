//! The next session's date as the table sees it (session/schedule-sessions),
//! built here by allow-list like the rest of the projection.
//!
//! Reaches players: the number of the next session, the date the GM
//! fixed and when its lobby opens, the dates still proposed with who said
//! yes (nicknames) and the caller's own answer.
//!
//! Never: the table's Discord webhook, the reminder log, dropped dates.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::schedule::{DateAnswer, DateStatus, SessionDate};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView {
    /// The number the next session will carry.
    pub number: i32,
    /// The date the GM fixed, while it is ahead.
    pub next: Option<NextView>,
    /// Dates the table is asked about, soonest first.
    pub proposed: Vec<ProposedView>,
    /// A spectator sees the dates, answers none.
    pub can_answer: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextView {
    pub id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub minutes: i32,
    pub lobby_opens_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedView {
    pub id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub minutes: i32,
    /// The caller's answer; `None` before they give one.
    pub mine: Option<bool>,
    /// Who said yes.
    pub yes: Vec<String>,
}

/// The schedule for `caller`. `players` maps ids to nicknames.
#[must_use]
pub fn project_schedule(
    number: i32,
    dates: &[SessionDate],
    answers: &[DateAnswer],
    players: &[(Uuid, String)],
    caller: Uuid,
    can_answer: bool,
) -> ScheduleView {
    let nickname = |id: Uuid| {
        players
            .iter()
            .find(|(p, _)| *p == id)
            .map(|(_, n)| n.clone())
    };
    ScheduleView {
        number,
        next: dates
            .iter()
            .find(|d| d.status == DateStatus::Chosen)
            .map(|d| NextView {
                id: d.id,
                starts_at: d.starts_at,
                minutes: d.minutes,
                lobby_opens_at: d.lobby_opens_at(),
            }),
        proposed: dates
            .iter()
            .filter(|d| d.status == DateStatus::Proposed)
            .map(|d| ProposedView {
                id: d.id,
                starts_at: d.starts_at,
                minutes: d.minutes,
                mine: answers
                    .iter()
                    .find(|a| a.date_id == d.id && a.player_id == caller)
                    .map(|a| a.available),
                yes: answers
                    .iter()
                    .filter(|a| a.date_id == d.id && a.available)
                    .filter_map(|a| nickname(a.player_id))
                    .collect(),
            })
            .collect(),
        can_answer,
    }
}
