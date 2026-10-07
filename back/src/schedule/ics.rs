//! The chosen date as a calendar event (RFC 5545): a player adds it to
//! their phone's calendar, which reminds them the day before and an hour
//! before — the same two reminders as the table's channel — with the
//! link to the lobby. Only player text: the campaign's title, the session
//! number, the time and the link.

use chrono::{DateTime, Utc};

use super::SessionDate;

/// Text as a property value: backslash, `;`, `,` and line breaks escaped.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

/// Lines are folded at 75 octets, never inside a character.
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut width = 0;
    for c in line.chars() {
        let len = c.len_utf8();
        if width + len > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(c);
        width += len;
    }
    out
}

fn stamp(at: DateTime<Utc>) -> String {
    at.format("%Y%m%dT%H%M%SZ").to_string()
}

/// The calendar holding session `number` of `title` on `d`.
#[must_use]
pub fn calendar(
    title: &str,
    number: i32,
    d: &SessionDate,
    link: &str,
    now: DateTime<Utc>,
) -> String {
    let summary = escape(&format!("{title} · séance {number}"));
    let alarm = escape(&format!("Séance {number} de « {title} »"));
    let description = escape(&format!("Le salon ouvre un quart d'heure avant : {link}"));
    let lines = [
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//Promptus//Promptus//FR".to_string(),
        "CALSCALE:GREGORIAN".to_string(),
        "METHOD:PUBLISH".to_string(),
        "BEGIN:VEVENT".to_string(),
        format!("UID:{}@promptus", d.id),
        format!("DTSTAMP:{}", stamp(now)),
        format!("DTSTART:{}", stamp(d.starts_at)),
        format!("DTEND:{}", stamp(d.ends_at())),
        format!("SUMMARY:{summary}"),
        format!("DESCRIPTION:{description}"),
        format!("URL:{link}"),
        "BEGIN:VALARM".to_string(),
        "ACTION:DISPLAY".to_string(),
        format!("DESCRIPTION:{alarm}"),
        "TRIGGER:-P1D".to_string(),
        "END:VALARM".to_string(),
        "BEGIN:VALARM".to_string(),
        "ACTION:DISPLAY".to_string(),
        format!("DESCRIPTION:{alarm}"),
        "TRIGGER:-PT1H".to_string(),
        "END:VALARM".to_string(),
        "END:VEVENT".to_string(),
        "END:VCALENDAR".to_string(),
    ];
    lines.iter().map(|l| fold(l) + "\r\n").collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::DateStatus;
    use uuid::Uuid;

    #[test]
    fn the_event_holds_the_time_two_alarms_and_the_way_in() {
        let d = SessionDate {
            id: Uuid::nil(),
            campaign_id: Uuid::nil(),
            starts_at: DateTime::parse_from_rfc3339("2026-10-10T18:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            minutes: 150,
            status: DateStatus::Chosen,
            chosen_at: None,
            eve_reminded_at: None,
            hour_reminded_at: None,
            lobby_opened_at: None,
        };
        let link = "https://promptus.example/partie/00000000-0000-0000-0000-000000000000";
        let ics = calendar("Corsaires, acte 2; la mer", 4, &d, link, d.starts_at);
        assert!(ics.contains("DTSTART:20261010T183000Z\r\n"), "{ics}");
        assert!(ics.contains("DTEND:20261010T210000Z\r\n"), "{ics}");
        assert!(
            ics.contains("SUMMARY:Corsaires\\, acte 2\\; la mer · séance 4"),
            "{ics}"
        );
        assert!(
            ics.contains("TRIGGER:-P1D") && ics.contains("TRIGGER:-PT1H"),
            "{ics}"
        );
        // Every physical line fits in 75 octets, and unfolds back to the link.
        assert!(ics.split("\r\n").all(|l| l.len() <= 75), "{ics}");
        assert!(
            ics.replace("\r\n ", "").contains(&format!("URL:{link}")),
            "{ics}"
        );
    }
}
