//! session/schedule-sessions over the API, on both witness worlds: the
//! GM proposes dates, the players answer, the GM fixes one and the table
//! is told; the clock reminds the day before and an hour before, then
//! opens the lobby a quarter of an hour before — each once, ever. The
//! table's Discord channel is a recorder here: what would have been
//! posted is read back.

mod common;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::Router;
use axum::http::StatusCode;
use chrono::{DateTime, Duration, DurationRound, Utc};
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use promptus_back::ai::Ai;
use promptus_back::live::{LiveConfig, LiveHub};
use promptus_back::schedule::notify::Recorder;
use promptus_back::schedule::{self, Kind, Notifier};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");
const WEBHOOK: &str = "https://discord.com/api/webhooks/123456/secret-token-XYZW";

struct Table {
    app: Router,
    pool: PgPool,
    notifier: Notifier,
    recorder: Arc<Recorder>,
    gm: String,
    campaign: String,
    marc: String,
    hugo: String,
    lea: String,
}

impl Table {
    fn id(&self) -> Uuid {
        Uuid::parse_str(&self.campaign).unwrap()
    }

    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn player(&self, token: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/play/{}{path}", self.campaign);
        call_as_player(&self.app, Some(token), method, &uri, body).await
    }

    async fn tick(&self, now: DateTime<Utc>) -> schedule::Tick {
        schedule::tick(&self.pool, &self.notifier, now, Some(self.id()))
            .await
            .unwrap()
    }

    fn sent(&self) -> Vec<String> {
        self.recorder.sent().into_iter().map(|(_, m)| m).collect()
    }
}

async fn table(yaml: &str) -> Table {
    let pool = common::test_pool().await;
    let (notifier, recorder) = Notifier::recorder(common::ORIGIN);
    let app = common::app_with_notifier(
        pool.clone(),
        LiveHub::new(LiveConfig::default()),
        Ai::fake(),
        notifier.clone(),
    );
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, yaml).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let hugo = join(&app, &code, "Hugo", "player").await;
    let lea = join(&app, &code, "Léa", "spectator").await;
    Table {
        app,
        pool,
        notifier,
        recorder,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
        hugo: hugo.player_token().unwrap(),
        lea: lea.player_token().unwrap(),
    }
}

/// Thursday in `days`, 18:30 UTC.
fn evening_in(days: i64) -> DateTime<Utc> {
    (Utc::now() + Duration::days(days))
        .duration_trunc(Duration::days(1))
        .unwrap()
        + Duration::minutes(18 * 60 + 30)
}

async fn from_proposal_to_lobby(yaml: &str, title: &str) {
    let t = table(yaml).await;

    // The channel: only Discord's own webhooks; the GM sees its end only.
    let r = t
        .gm(
            "PUT",
            "/schedule/discord",
            Some(json!({ "webhook": "http://127.0.0.1/api/webhooks/1/x" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "INVALID_WEBHOOK");
    let r = t
        .gm(
            "PUT",
            "/schedule/discord",
            Some(json!({ "webhook": WEBHOOK })),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);

    // Two dates proposed; one in the past is refused.
    let r = t
        .gm(
            "POST",
            "/schedule/dates",
            Some(json!({ "startsAt": Utc::now() - Duration::hours(1) })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "DATE_IN_PAST");
    let thursday = evening_in(3);
    let friday = evening_in(4);
    let mut ids = Vec::new();
    for at in [thursday, friday] {
        let r = t
            .gm(
                "POST",
                "/schedule/dates",
                Some(json!({ "startsAt": at, "minutes": 150 })),
            )
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        ids.push(r.body["data"]["id"].as_str().unwrap().to_string());
    }
    let (thu, fri) = (&ids[0], &ids[1]);

    // The players answer; a spectator watches, answers nothing.
    for (token, date, yes) in [
        (&t.marc, thu, true),
        (&t.marc, fri, false),
        (&t.hugo, thu, true),
        (&t.hugo, fri, true),
    ] {
        let r = t
            .player(
                token,
                "PUT",
                &format!("/schedule/{date}"),
                Some(json!({ "available": yes })),
            )
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }
    let r = t
        .player(
            &t.lea,
            "PUT",
            &format!("/schedule/{thu}"),
            Some(json!({ "available": true })),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.body["error"]["code"], "SPECTATOR");
    let mine = t.player(&t.marc, "GET", "/schedule", None).await;
    let proposed = &mine.body["data"]["proposed"];
    assert_eq!(proposed[0]["mine"], true, "{}", mine.text);
    assert_eq!(proposed[1]["mine"], false);
    assert_eq!(proposed[0]["yes"], json!(["Marc", "Hugo"]));
    assert_eq!(proposed[1]["yes"], json!(["Hugo"]));
    assert_eq!(mine.body["data"]["number"], 1);
    assert!(!mine.text.contains("discord.com"), "{}", mine.text);
    let screen = t.gm("GET", "/schedule", None).await.body;
    assert_eq!(
        screen["data"]["dates"][0]["answers"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(screen["data"]["discord"]["configured"], true);
    assert_eq!(screen["data"]["discord"]["hint"], "…XYZW");
    assert!(!screen.to_string().contains("secret-token"), "{screen}");

    // The GM fixes Thursday: Friday closes, the table is told, with the
    // way in.
    let r = t
        .gm("POST", &format!("/schedule/dates/{thu}/choose"), None)
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let sent = t.sent();
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(
        sent[0].contains(title) && sent[0].contains(&format!("<t:{}:F>", thursday.timestamp())),
        "{}",
        sent[0]
    );
    let link = format!("{}/partie/{}", common::ORIGIN, t.campaign);
    assert!(sent[0].ends_with(&link), "{}", sent[0]);
    assert_eq!(t.recorder.sent()[0].0, WEBHOOK);
    let mine = t.player(&t.marc, "GET", "/schedule", None).await.body;
    assert_eq!(mine["data"]["proposed"], json!([]));
    assert_eq!(mine["data"]["next"]["id"], thu.as_str());
    let lobby_at = thursday - Duration::minutes(15);
    assert_eq!(
        mine["data"]["next"]["lobbyOpensAt"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap(),
        lobby_at
    );

    // The phone's calendar: the time, two alarms, the way in.
    let ics = t.player(&t.marc, "GET", "/schedule.ics", None).await;
    assert_eq!(ics.status, StatusCode::OK);
    let unfolded = ics.text.replace("\r\n ", "");
    assert!(
        unfolded.contains(&format!("DTSTART:{}", thursday.format("%Y%m%dT%H%M%SZ"))),
        "{unfolded}"
    );
    assert!(unfolded.contains("TRIGGER:-P1D") && unfolded.contains("TRIGGER:-PT1H"));
    assert!(unfolded.contains(&format!("URL:{link}")), "{unfolded}");

    // The clock: nothing two days before; the eve reminder once.
    assert_eq!(
        t.tick(thursday - Duration::hours(48)).await,
        schedule::Tick::default()
    );
    let eve = t.tick(thursday - Duration::hours(23)).await;
    assert_eq!(
        eve.reminded.iter().map(|r| r.1).collect::<Vec<_>>(),
        [Kind::Eve]
    );
    assert_eq!(
        t.tick(thursday - Duration::hours(22)).await,
        schedule::Tick::default()
    );
    assert!(
        t.sent()[1].starts_with("Rappel : séance 1"),
        "{}",
        t.sent()[1]
    );

    // An hour before; then the lobby opens on its own, and the phones see it.
    let hour = t.tick(thursday - Duration::minutes(50)).await;
    assert_eq!(
        hour.reminded.iter().map(|r| r.1).collect::<Vec<_>>(),
        [Kind::Hour]
    );
    let lobby = t.tick(thursday - Duration::minutes(10)).await;
    assert_eq!(lobby.opened, [t.id()]);
    assert_eq!(
        t.tick(thursday - Duration::minutes(5)).await,
        schedule::Tick::default()
    );
    assert!(
        t.sent()[3].starts_with(&format!("Le salon de « {title} » est ouvert")),
        "{}",
        t.sent()[3]
    );
    assert_eq!(t.sent().len(), 4);
    let evening = t.player(&t.marc, "GET", "/evening", None).await.body;
    assert_eq!(evening["data"]["session"]["status"], "lobby", "{evening}");
    assert_eq!(evening["data"]["session"]["number"], 1);
    // While the lobby is open, the date still names that session.
    assert_eq!(
        t.player(&t.marc, "GET", "/schedule", None).await.body["data"]["number"],
        1
    );
    let log = t.gm("GET", "/schedule", None).await.body;
    let kinds: Vec<&str> = log["data"]["dates"][0]["log"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["chosen", "eve", "hour", "lobby"]);

    // Once played, the date closes: nothing is ahead.
    t.tick(thursday + Duration::hours(3)).await;
    let mine = t.player(&t.marc, "GET", "/schedule", None).await.body;
    assert!(mine["data"]["next"].is_null(), "{mine}");
}

#[tokio::test]
async fn corsaires_from_the_proposed_dates_to_the_lobby() {
    let title = promptus_shared::story::from_yaml(CORSAIRES).unwrap().title;
    from_proposal_to_lobby(CORSAIRES, &title).await;
}

#[tokio::test]
async fn brasier_from_the_proposed_dates_to_the_lobby() {
    let title = promptus_shared::story::from_yaml(BRASIER).unwrap().title;
    from_proposal_to_lobby(BRASIER, &title).await;
}

/// A server down through the eve and the hour sends only the lobby; a
/// campaign the GM unvalidated keeps its lobby closed and says why; a
/// channel that refuses is logged, not retried.
#[tokio::test]
async fn the_clock_catches_up_once_and_says_what_failed() {
    let t = table(CORSAIRES).await;
    t.gm(
        "PUT",
        "/schedule/discord",
        Some(json!({ "webhook": WEBHOOK })),
    )
    .await;
    let at = evening_in(2);
    let id = t
        .gm("POST", "/schedule/dates", Some(json!({ "startsAt": at })))
        .await
        .body["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    t.gm("POST", &format!("/schedule/dates/{id}/choose"), None)
        .await;
    sqlx::query("UPDATE campaigns SET validated_at = NULL WHERE id = $1")
        .bind(t.id())
        .execute(&t.pool)
        .await
        .unwrap();
    t.recorder.failing.store(true, Ordering::Relaxed);

    // Down since the day before: only the hour reminder, refused.
    let hour = t.tick(at - Duration::minutes(50)).await;
    assert_eq!(
        hour.reminded.iter().map(|r| r.1).collect::<Vec<_>>(),
        [Kind::Hour]
    );
    let tick = t.tick(at - Duration::minutes(5)).await;
    assert!(tick.opened.is_empty(), "{tick:?}");
    assert_eq!(
        t.tick(at - Duration::minutes(4)).await,
        schedule::Tick::default()
    );
    let screen = t.gm("GET", "/schedule", None).await.body;
    let log: Vec<(String, bool, String)> = screen["data"]["dates"][0]["log"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["kind"].as_str().unwrap().to_string(),
                l["ok"].as_bool().unwrap(),
                l["detail"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(log.len(), 3, "{log:?}");
    assert_eq!(log[0].0, "chosen");
    assert!(log[0].1, "{log:?}");
    assert_eq!(
        log[1],
        ("hour".to_string(), false, "refusé (test)".to_string())
    );
    assert_eq!(
        log[2],
        (
            "lobby".to_string(),
            false,
            "CAMPAIGN_NOT_VALIDATED".to_string()
        )
    );
    let r = t.gm("GET", "/session", None).await.body;
    assert!(r["data"]["session"].is_null(), "{r}");
}
