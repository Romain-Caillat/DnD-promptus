//! copilot/listen-by-voice, over the API, on both witness worlds: the
//! GM dictates, the co-GM hears it and drafts; nothing reaches a phone
//! until the GM shows it; every hearing is counted and none is paid for
//! a request that would be refused anyway.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join, wav_saying};
use promptus_back::ai::fake::FakeProvider;
use promptus_back::ai::{Ai, Pricing};
use promptus_back::live::{LiveConfig, LiveHub};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

/// Each world and an NPC of its cast the GM names aloud.
const WORLDS: [(&str, &str, &str, &str); 2] = [
    ("corsaires", CORSAIRES, "pnj_goulven", "Marguerite Goulven"),
    ("brasier", BRASIER, "pnj_lumen", "LUMEN"),
];

struct Table {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
    lea: String,
}

async fn table(yaml: &str, ai: Ai) -> Table {
    let pool = common::test_pool().await;
    let app = common::app_with_ai(pool.clone(), LiveHub::new(LiveConfig::default()), ai);
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, yaml).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let lea = join(&app, &code, "Léa", "spectator").await;
    let t = Table {
        app,
        pool,
        gm,
        campaign,
        lea: lea.player_token().unwrap(),
    };
    t.budget(100).await;
    t
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn live(&self) {
        let r = self.gm("POST", "/session", None).await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        let r = self.gm("POST", "/session/start", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }

    async fn budget(&self, cents: i32) {
        sqlx::query("UPDATE campaigns SET ai_budget_cents = $2 WHERE id = $1")
            .bind(Uuid::parse_str(&self.campaign).unwrap())
            .bind(cents)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    async fn say(&self, body: Value) -> Reply {
        self.gm("POST", "/session/copilot/voice", Some(body)).await
    }

    /// The calls counted for the campaign, oldest first: (purpose, template).
    async fn calls(&self) -> Vec<(String, Option<String>)> {
        sqlx::query_as(
            "SELECT purpose, template FROM ai_calls WHERE campaign_id = $1 ORDER BY created_at",
        )
        .bind(Uuid::parse_str(&self.campaign).unwrap())
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }
}

#[tokio::test]
async fn the_gm_dictates_and_the_co_gm_drafts_on_both_worlds() {
    for (world, yaml, npc_id, npc) in WORLDS {
        let fake = Arc::new(FakeProvider::default());
        let t = table(yaml, Ai::new(Some(fake.clone()), Pricing::default())).await;
        t.live().await;
        let said = format!("Que répond {npc} si on lui parle du trésor ?");

        let r = t
            .say(json!({ "audio": wav_saying(&said), "kind": "npc", "npc": npc_id }))
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{world}: {}", r.body);
        let heard = &r.body["data"];
        assert_eq!(heard["transcript"], said.as_str(), "{world}");
        assert_eq!(heard["seconds"], 1.0, "{world}");
        // The words are asked to the co-GM as typed words would be.
        let draft = &heard["draft"];
        assert_eq!(draft["status"], "draft", "{world}");
        assert_eq!(draft["kind"], "npc", "{world}");
        assert_eq!(draft["prompt"], said.as_str(), "{world}");
        assert!(
            draft["answer"]["narration"]
                .as_str()
                .unwrap()
                .contains(&said),
            "{world}: {draft}"
        );
        let calls = fake.calls();
        assert_eq!(calls.len(), 2, "{world}: {calls:?}");
        assert_eq!(calls[0], "transcribe", "{world}");
        assert!(calls[1].starts_with("complete:"), "{world}: {calls:?}");
        assert_eq!(
            t.calls().await,
            [
                (
                    "copilot.voice".to_string(),
                    Some("transcribe@1".to_string())
                ),
                ("copilot.npc".to_string(), Some("copilot@1".to_string())),
            ],
            "{world}"
        );

        // Neither the words nor the draft reach a phone.
        let uri = format!("/api/play/{}/evening", t.campaign);
        let view = call_as_player(&t.app, Some(&t.lea), "GET", &uri, None)
            .await
            .body
            .to_string();
        assert!(!view.contains("trésor"), "{world}: {view}");
        // Default kind: the GM's own question.
        let r = t.say(json!({ "audio": wav_saying("Et ensuite ?") })).await;
        assert_eq!(
            r.body["data"]["draft"]["kind"], "free",
            "{world}: {}",
            r.body
        );
    }
}

#[tokio::test]
async fn silence_is_heard_as_nothing_and_still_counted() {
    let fake = Arc::new(FakeProvider::default());
    let t = table(CORSAIRES, Ai::new(Some(fake.clone()), Pricing::default())).await;
    t.live().await;
    let r = t.say(json!({ "audio": wav_saying("") })).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "NOTHING_HEARD");
    // The hearing was paid; no co-GM call followed, no draft was kept.
    assert_eq!(fake.calls(), ["transcribe"]);
    assert_eq!(t.calls().await.len(), 1);
    let drafts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM copilot_drafts WHERE campaign_id = $1")
            .bind(Uuid::parse_str(&t.campaign).unwrap())
            .fetch_one(&t.pool)
            .await
            .unwrap();
    assert_eq!(drafts, 0);
}

#[tokio::test]
async fn nothing_is_sent_for_a_dictation_that_would_be_refused() {
    let fake = Arc::new(FakeProvider::default());
    let t = table(BRASIER, Ai::new(Some(fake.clone()), Pricing::default())).await;
    let voice = json!({ "audio": wav_saying("Le Cure-Dent tremble.") });

    // No live session yet.
    let r = t.say(voice.clone()).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "SESSION_NOT_LIVE");
    t.live().await;

    // Not a WAV, too short, an unknown NPC.
    let r = t.say(json!({ "audio": "T2dnUwAAAAAAAAAA" })).await;
    assert_eq!(r.body["error"]["code"], "AUDIO_INVALID", "{}", r.body);
    let r = t.say(json!({ "audio": "pas du base64 !" })).await;
    assert_eq!(r.body["error"]["code"], "AUDIO_INVALID", "{}", r.body);
    {
        use base64::Engine;
        let one_second = base64::engine::general_purpose::STANDARD
            .decode(wav_saying("x"))
            .unwrap();
        // The header and a tenth of a second of samples.
        let short = base64::engine::general_purpose::STANDARD.encode(&one_second[..44 + 3_200]);
        let r = t.say(json!({ "audio": short })).await;
        assert_eq!(r.body["error"]["code"], "RECORDING_TOO_SHORT", "{}", r.body);
    }
    let r = t
        .say(json!({ "audio": wav_saying("Parle."), "kind": "npc", "npc": "pnj_inconnu" }))
        .await;
    assert_eq!(r.body["error"]["code"], "UNKNOWN_NPC", "{}", r.body);

    // Over the budget: refused before the voice leaves.
    t.budget(0).await;
    let r = t.say(voice.clone()).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "AI_BUDGET_EXCEEDED");
    assert!(fake.calls().is_empty(), "{:?}", fake.calls());
    assert!(t.calls().await.is_empty());

    // A model that answers prose: a readable 502, the hearing counted.
    t.budget(100).await;
    let broken = Arc::new(FakeProvider::broken());
    let app = common::app_with_ai(
        t.pool.clone(),
        LiveHub::new(LiveConfig::default()),
        Ai::new(Some(broken.clone()), Pricing::default()),
    );
    let r = call(
        &app,
        Some(&t.gm),
        "POST",
        &format!("/api/campaigns/{}/session/copilot/voice", t.campaign),
        Some(voice),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_GATEWAY, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "AI_OUTPUT_INVALID");
    assert_eq!(broken.calls(), ["transcribe"]);
    assert_eq!(t.calls().await.len(), 1);
}
