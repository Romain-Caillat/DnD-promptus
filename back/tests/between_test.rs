//! Between two sessions, over the API: session/write-recaps (the facts,
//! the drafts, the GM's reading, then publishing), player/play-between-
//! sessions (what Marc gained, the recap and the chronicle on his
//! phone), session/schedule-sessions (dates proposed, answered, chosen;
//! the reminder; the lobby opening on time).

mod common;

use axum::Router;
use axum::http::StatusCode;
use chrono::{Duration, DurationRound, Utc};
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use promptus_back::evening::schedule;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

struct Table {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
    marc: String,
    character: String,
    lea: String,
}

/// Romain's table: Marc plays Borin (validated), Léa watches.
async fn table() -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let marc_id = Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    let character: Uuid = sqlx::query_scalar(
        "UPDATE characters SET status = 'validated',
                sheet = '{\"name\":\"Borin\",\"classId\":\"bretteur\"}'
         WHERE player_id = $1 RETURNING id",
    )
    .bind(marc_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let lea = join(&app, &code, "Léa", "spectator").await;
    Table {
        app,
        pool,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
        character: character.to_string(),
        lea: lea.player_token().unwrap(),
    }
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn player(&self, token: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/play/{}{path}", self.campaign);
        call_as_player(&self.app, Some(token), method, &uri, body).await
    }

    async fn ok(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        let r = self.gm(method, path, body).await;
        assert!(r.status.is_success(), "{method} {path}: {}", r.body);
        r.body
    }

    async fn between(&self) -> Value {
        let r = self.player(&self.marc, "GET", "/between", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }
}

#[tokio::test]
async fn the_recap_waits_for_the_gm_then_reaches_marcs_phone() {
    let t = table().await;
    t.ok("POST", "/session", None).await;
    t.ok("POST", "/session/start", None).await;
    // The evening: a clue found, the wreckers' clock moves (the GM's
    // secret), Borin earns XP and gold and a lantern, a promise is made.
    t.ok(
        "POST",
        "/session/reveal",
        Some(json!({ "kind": "clue", "clue": "cl_gwen" })),
    )
    .await;
    t.ok(
        "POST",
        "/session/reveal",
        Some(json!({ "kind": "front", "front": "front_naufrageurs", "delta": 1 })),
    )
    .await;
    let adjust = format!("/characters/{}/adjust", t.character);
    for body in [
        json!({ "kind": "xp", "delta": 3 }),
        json!({ "kind": "resource", "resource": "or", "delta": 34 }),
        json!({ "kind": "giveItem", "name": "Lanterne du phare" }),
    ] {
        t.ok("POST", &adjust, Some(body)).await;
    }
    t.ok(
        "POST",
        "/session/journal",
        Some(json!({ "kind": "promise", "text": "Retrouver le frère de Gwen.", "shared": true })),
    )
    .await;

    // Ended without a word: the drafts come from what happened.
    let ended = t.ok("POST", "/session/end", Some(json!({}))).await["data"].clone();
    let session = ended["id"].as_str().unwrap().to_string();
    assert_eq!(ended["published"], false);
    assert!(
        ended["recap"]
            .as_str()
            .unwrap()
            .contains("Les naufrageurs de Corentin")
    );
    let previously = ended["previously"].as_str().unwrap();
    assert!(
        previously.contains("Gwen a vu les hommes de Corentin"),
        "{previously}"
    );
    assert!(
        !previously.contains("naufrageurs de Corentin"),
        "{previously}"
    );
    assert_eq!(
        ended["facts"]["openThreads"][0],
        "Retrouver le frère de Gwen."
    );

    // The next morning, before Romain rereads: Marc sees his gains, not
    // the recap.
    let b = t.between().await;
    let last = &b["lastSession"];
    assert_eq!(last["number"], 1);
    assert_eq!(last["published"], false);
    assert_eq!(last["xpGained"], 3);
    assert!(last["title"].is_null());
    let gains = last["gains"].as_array().unwrap();
    assert!(
        gains.contains(&json!({ "kind": "resource", "label": "Pièces d'or", "delta": 34 })),
        "{gains:?}"
    );
    assert!(
        gains.contains(&json!({ "kind": "item", "label": "Lanterne du phare", "delta": 1 })),
        "{gains:?}"
    );
    assert!(b["previously"].is_null());
    assert_eq!(b["chronicle"], json!([]));

    // Romain rereads, names the entry, publishes.
    t.ok(
        "PUT",
        &format!("/sessions/{session}/recap"),
        Some(json!({
            "recap": "Corentin se méfie.",
            "previously": "Vous avez appris ce que Gwen a vu.",
            "title": "La nuit sans phare",
            "chronicle": "Gwen parle. Une promesse est faite."
        })),
    )
    .await;
    t.ok("POST", &format!("/sessions/{session}/publish"), None)
        .await;

    let b = t.between().await;
    let text = b.to_string();
    assert_eq!(
        b["previously"]["text"],
        "Vous avez appris ce que Gwen a vu."
    );
    assert_eq!(
        b["previously"]["clues"][0],
        "Gwen a vu les hommes de Corentin monter au phare la nuit du premier naufrage."
    );
    assert_eq!(
        b["previously"]["revelations"][0],
        "Corentin éteint le phare pour faire échouer les navires."
    );
    assert_eq!(
        b["previously"]["openThreads"][0],
        "Retrouver le frère de Gwen."
    );
    assert_eq!(b["chronicle"][0]["title"], "La nuit sans phare");
    assert_eq!(
        b["chronicle"][0]["text"],
        "Gwen parle. Une promesse est faite."
    );
    assert_eq!(b["lastSession"]["title"], "La nuit sans phare");
    // Neither the GM's recap nor the front that moved.
    assert!(!text.contains("Corentin se méfie"), "{text}");
    assert!(!text.contains("naufrageurs de Corentin"), "{text}");
    assert!(!text.contains("feu éteint"), "{text}");
    let evening = t.player(&t.marc, "GET", "/evening", None).await.body;
    assert_eq!(
        evening["data"]["previously"],
        "Vous avez appris ce que Gwen a vu."
    );

    // Another GM's session, or one still open, cannot be published.
    t.ok("POST", "/session", None).await;
    let open: String = t.gm("GET", "/session", None).await.body["data"]["session"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = t
        .gm("POST", &format!("/sessions/{open}/publish"), None)
        .await;
    assert_eq!(r.body["error"]["code"], "SESSION_NOT_ENDED");
}

#[tokio::test]
async fn romain_proposes_dates_marc_answers_and_the_lobby_opens_on_time() {
    let t = table().await;
    let day = |d: i64| {
        (Utc::now() + Duration::days(d))
            .duration_trunc(Duration::minutes(1))
            .unwrap()
    };
    let (wed, thu, fri) = (day(3), day(4), day(5));

    // Nothing planned yet.
    let r = t
        .player(
            &t.marc,
            "PUT",
            "/availability",
            Some(json!({ "available": [thu] })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NO_PLAN");
    let r = t
        .gm(
            "PUT",
            "/plan",
            Some(json!({ "options": [Utc::now() - Duration::hours(1)] })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "DATE_IN_PAST");

    let plan = t
        .ok("PUT", "/plan", Some(json!({ "options": [fri, wed, thu] })))
        .await;
    assert_eq!(plan["data"]["options"], json!([wed, thu, fri]));
    assert_eq!(plan["data"]["lobbyMinutes"], 15);

    // Marc answers with a tap; Léa only watches.
    let r = t
        .player(
            &t.marc,
            "PUT",
            "/availability",
            Some(json!({ "available": [thu, fri] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["next"]["mine"], json!([thu, fri]));
    assert_eq!(r.body["data"]["next"]["answered"], 1);
    assert_eq!(r.body["data"]["next"]["number"], 1);
    let r = t
        .player(
            &t.lea,
            "PUT",
            "/availability",
            Some(json!({ "available": [thu] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = t
        .player(
            &t.marc,
            "PUT",
            "/availability",
            Some(json!({ "available": [day(9)] })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NOT_PROPOSED");
    let plan = t.ok("GET", "/plan", None).await;
    assert_eq!(plan["data"]["answers"][0]["available"], json!([thu, fri]));

    // No reminder before a date is chosen; then one, an hour ahead,
    // leading to Marc's table.
    let ics = format!("/api/play/{}/next-session.ics", t.campaign);
    let r = call_as_player(&t.app, Some(&t.marc), "GET", &ics, None).await;
    assert_eq!(r.body["error"]["code"], "NO_NEXT_SESSION");
    let r = t
        .gm("PUT", "/plan/choice", Some(json!({ "at": day(8) })))
        .await;
    assert_eq!(r.body["error"]["code"], "NOT_PROPOSED");
    let plan = t
        .ok("PUT", "/plan/choice", Some(json!({ "at": thu })))
        .await;
    assert_eq!(plan["data"]["chosenAt"], json!(thu));
    let res = common::send_raw(&t.app, Some(&t.marc), "GET", &ics).await;
    assert_eq!(res.0, StatusCode::OK);
    let text = res.1;
    assert!(
        text.contains(&format!("DTSTART:{}", thu.format("%Y%m%dT%H%M%SZ"))),
        "{text}"
    );
    assert!(text.contains("TRIGGER:-PT1H"), "{text}");
    assert!(text.contains(&format!("/partie/{}", t.campaign)), "{text}");
    assert!(text.contains("SUMMARY:Le Phare de Kerbrume"), "{text}");
    let b = t.between().await;
    assert_eq!(b["next"]["lobbyAt"], json!(thu - Duration::minutes(15)));

    // Proposing again without Thursday unchooses it.
    let plan = t
        .ok("PUT", "/plan", Some(json!({ "options": [wed, fri] })))
        .await;
    assert!(plan["data"]["chosenAt"].is_null());
    assert_eq!(plan["data"]["answers"][0]["available"], json!([fri]));
    t.ok("PUT", "/plan/choice", Some(json!({ "at": fri })))
        .await;

    // Friday, a quarter of an hour before: the lobby opens by itself,
    // once, and the plan is spent.
    let campaign = Uuid::parse_str(&t.campaign).unwrap();
    let early = schedule::tick(&t.pool, fri - Duration::minutes(16))
        .await
        .unwrap();
    assert!(!early.contains(&campaign));
    let opened = schedule::tick(&t.pool, fri - Duration::minutes(15))
        .await
        .unwrap();
    assert!(opened.contains(&campaign));
    let again = schedule::tick(&t.pool, fri).await.unwrap();
    assert!(!again.contains(&campaign));
    let screen = t.ok("GET", "/session", None).await;
    assert_eq!(screen["data"]["session"]["status"], "lobby");
    assert_eq!(screen["data"]["session"]["number"], 1);
    assert!(t.ok("GET", "/plan", None).await["data"].is_null());
}
