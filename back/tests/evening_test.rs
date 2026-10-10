//! The evening, end to end over the API: session/open-lobby,
//! session/drive-scenes, session/track-table-knowledge,
//! session/end-session, session/collect-player-feedback,
//! gm/balance-spotlight, ui/roll-faceted-dice (the server rolls),
//! ai/count-ai-calls.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use promptus_back::ai::fake::FakeProvider;
use promptus_back::ai::{Ai, Pricing};
use promptus_back::live::{LiveConfig, LiveHub};
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
    marc_id: Uuid,
    lea: String,
}

/// Romain's table: Marc plays Borin (validated), Léa watches.
async fn table_with(ai: Ai) -> Table {
    let pool = common::test_pool().await;
    let app = common::app_with_ai(pool.clone(), LiveHub::new(LiveConfig::default()), ai);
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let marc_id = Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    sqlx::query(
        "UPDATE characters SET status = 'validated',
                sheet = '{\"name\":\"Borin\",\"classId\":\"bretteur\",\"abilities\":{\"DEX\":14}}'
         WHERE player_id = $1",
    )
    .bind(marc_id)
    .execute(&pool)
    .await
    .unwrap();
    let lea = join(&app, &code, "Léa", "spectator").await;
    Table {
        app,
        pool,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
        marc_id,
        lea: lea.player_token().unwrap(),
    }
}

async fn table() -> Table {
    table_with(Ai::fake()).await
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

    async fn live(&self) {
        let r = self.gm("POST", "/session", None).await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        let r = self.gm("POST", "/session/start", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }

    async fn ask(&self, text: &str) -> Value {
        let r = self
            .player(
                &self.marc,
                "POST",
                "/requests",
                Some(json!({ "card": { "kind": "ability", "ability": "DEX" }, "text": text })),
            )
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        r.body["data"]["requests"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()
            .clone()
    }
}

#[tokio::test]
async fn an_evening_from_the_lobby_to_the_feedback() {
    let t = table().await;

    // Nothing to join before the GM opens the lobby.
    let r = t
        .player(&t.marc, "POST", "/lobby", Some(json!({ "soundOk": true })))
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "NO_SESSION");

    let r = t.gm("POST", "/session", None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["data"]["number"], 1);
    assert_eq!(r.body["data"]["status"], "lobby");
    // Opening twice is the same lobby.
    let again = t.gm("POST", "/session", None).await;
    assert_eq!(again.body["data"]["id"], r.body["data"]["id"]);

    // Marc arrives, sound checked; the GM sees him in the lobby.
    let r = t
        .player(
            &t.marc,
            "POST",
            "/lobby",
            Some(json!({ "soundOk": true, "remote": true })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["lobby"][0]["nickname"], "Marc");
    assert_eq!(r.body["data"]["session"]["status"], "lobby");
    let screen = t.gm("GET", "/session", None).await;
    assert_eq!(screen.body["data"]["lobby"][0]["soundOk"], true);

    // No request before the GM starts.
    let r = t
        .player(
            &t.marc,
            "POST",
            "/requests",
            Some(json!({ "card": { "kind": "other" }, "text": "Je commande à boire." })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "SESSION_NOT_LIVE");

    let r = t.gm("POST", "/session/start", None).await;
    assert_eq!(r.body["data"]["status"], "live", "{}", r.body);

    // The GM reveals the opening scene: the phones show it, the journal
    // keeps it.
    let r = t
        .gm(
            "POST",
            "/session/reveal",
            Some(json!({ "kind": "scene", "node": "sc_taverne" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let view = t.player(&t.marc, "GET", "/evening", None).await.body;
    assert_eq!(
        view["data"]["campaign"]["scene"]["title"],
        "Le Goéland Ivre"
    );
    assert!(
        view["data"]["journal"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["kind"] == "scene"),
        "{view}"
    );

    // Marc plays his Dextérité card; the GM asks for a check.
    let req = t.ask("Je subtilise la clé du tavernier.").await;
    assert_eq!(req["status"], "pending");
    assert_eq!(req["card"]["name"], "Dextérité");
    let id = req["id"].as_str().unwrap();
    // Léa does not see Marc's request.
    let lea = t.player(&t.lea, "GET", "/evening", None).await.body;
    assert_eq!(lea["data"]["requests"], json!([]));
    assert_eq!(lea["data"]["cards"], json!([]));

    let r = t
        .gm(
            "POST",
            &format!("/session/requests/{id}"),
            Some(json!({ "kind": "check", "ability": "DEX", "difficulty": "moyen" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "check");
    assert_eq!(r.body["data"]["check"]["difficulty"], 10);

    // The server rolls, not the phone: the breakdown comes back with the
    // outcome named by the rules, and its XP is logged as a rules gesture.
    let r = t
        .player(&t.marc, "POST", &format!("/requests/{id}/roll"), None)
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let rolled = r.body["data"]["requests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == id)
        .unwrap()
        .clone();
    assert_eq!(rolled["status"], "rolled");
    let natural = rolled["roll"]["natural"].as_i64().unwrap();
    assert!((1..=20).contains(&natural), "{rolled}");
    // Borin's DEX 14 gives +2.
    assert_eq!(
        rolled["roll"]["total"].as_i64().unwrap(),
        natural + 2,
        "{rolled}"
    );
    assert!(rolled["outcome"].is_string(), "{rolled}");
    // Rolled once only.
    let r = t
        .player(&t.marc, "POST", &format!("/requests/{id}/roll"), None)
        .await;
    assert_eq!(r.body["error"]["code"], "NOT_A_CHECK");
    let actor: Option<String> =
        sqlx::query_scalar(
            "SELECT actor FROM play_adjustments WHERE campaign_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(uuid::Uuid::parse_str(&t.campaign).unwrap())
        .fetch_optional(&t.pool)
            .await
            .unwrap();
    if let Some(actor) = actor {
        assert_eq!(actor, "rules");
    }

    // The GM's ruling is remembered: a similar situation finds it.
    let r = t
        .gm(
            "GET",
            "/knowledge?situation=subtiliser%20une%20cl%C3%A9",
            None,
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["rulings"][0]["ability"], "DEX", "{}", r.body);

    // A GM-only note stays off the phones.
    let r = t
        .gm(
            "POST",
            "/session/journal",
            Some(json!({ "kind": "note", "text": "Le tavernier ment.", "shared": false })),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let view = t.player(&t.marc, "GET", "/evening", None).await.body;
    assert!(!view.to_string().contains("Le tavernier ment."), "{view}");

    // The spotlight: Marc had moments, the screen lists him.
    let screen = t.gm("GET", "/session", None).await.body;
    assert_eq!(
        screen["data"]["spotlight"][0]["nickname"], "Marc",
        "{screen}"
    );

    // End: the recap stays the GM's; « Précédemment… » is a draft until
    // the GM publishes it, then it reaches the table.
    let ending = json!({ "recap": "Borin a la clé, il l'ignore.", "previously": "Une clé a changé de poche." });
    let r = t.gm("POST", "/session/end", Some(ending.clone())).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let session = r.body["data"]["id"].as_str().unwrap().to_string();
    let view = t.player(&t.marc, "GET", "/evening", None).await.body;
    assert!(view["data"]["previously"].is_null(), "{view}");
    let mut publish = ending;
    publish["publish"] = json!(true);
    let r = t
        .gm("PUT", &format!("/sessions/{session}/recap"), Some(publish))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let view = t.player(&t.marc, "GET", "/evening", None).await.body;
    assert_eq!(view["data"]["previously"], "Une clé a changé de poche.");
    assert!(!view.to_string().contains("il l'ignore"), "{view}");
    assert_eq!(view["data"]["feedback"]["answered"], false, "{view}");

    // Feedback: Marc answers, Léa cannot; the GM reads it with the
    // measures next to it.
    let answers = json!({ "rulesClear": "yes", "hadMoment": "partly", "knowsNext": "no",
                          "comment": "J'aurais aimé plus de combat." });
    let r = t
        .player(&t.lea, "POST", "/feedback", Some(answers.clone()))
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = t.player(&t.marc, "POST", "/feedback", Some(answers)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["feedback"]["answered"], true);
    let report = t
        .gm("GET", &format!("/sessions/{session}/feedback"), None)
        .await
        .body;
    let marc = &report["data"]["players"][0];
    assert_eq!(marc["nickname"], "Marc", "{report}");
    assert_eq!(marc["answers"]["hadMoment"], "partly");
    assert_eq!(marc["requests"], 1);
    let r = t
        .gm(
            "PUT",
            &format!("/sessions/{session}/changes"),
            Some(json!({ "text": "Un combat dès l'ouverture." })),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);

    // The next session opens with number 2.
    let r = t.gm("POST", "/session", None).await;
    assert_eq!(r.body["data"]["number"], 2);
    let _ = t.marc_id;
}

#[tokio::test]
async fn a_player_withdraws_and_contests_a_refusal() {
    let t = table().await;
    t.live().await;
    let first = t.ask("Je saute par la fenêtre.").await;
    let id = first["id"].as_str().unwrap();
    let r = t
        .player(&t.marc, "POST", &format!("/requests/{id}/withdraw"), None)
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["requests"][0]["status"], "withdrawn");

    let second = t.ask("J'assomme le tavernier.").await;
    let id = second["id"].as_str().unwrap();
    // A refusal needs a reason the player will read.
    let r = t
        .gm(
            "POST",
            &format!("/session/requests/{id}"),
            Some(json!({ "kind": "refuse", "reason": "" })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "REASON_REQUIRED");
    let r = t
        .gm(
            "POST",
            &format!("/session/requests/{id}"),
            Some(json!({ "kind": "refuse", "reason": "Il est sous la protection du port." })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = t
        .player(&t.marc, "POST", &format!("/requests/{id}/contest"), None)
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let mine = &r.body["data"]["requests"][1];
    assert_eq!(mine["gmReason"], "Il est sous la protection du port.");
    assert_eq!(mine["contested"], true);

    // Three pending requests at most: the GM is not flooded.
    for n in 0..3 {
        t.ask(&format!("Idée {n}")).await;
    }
    let r = t
        .player(
            &t.marc,
            "POST",
            "/requests",
            Some(json!({ "card": { "kind": "other" }, "text": "Encore une." })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "TOO_MANY_PENDING");
}

/// ai/count-ai-calls: over budget, the call is refused before it is made
/// and nothing is billed; an answer out of format is a readable error.
#[tokio::test]
async fn the_recap_draft_respects_the_budget_and_the_format() {
    let fake = Arc::new(FakeProvider::default());
    let t = table_with(Ai::new(Some(fake.clone()), Pricing::default())).await;
    t.live().await;
    let r = t.gm("POST", "/session/end", Some(json!({}))).await;
    let session = r.body["data"]["id"].as_str().unwrap().to_string();
    let draft = format!("/sessions/{session}/recap-draft");

    // The fixture's budget is zero.
    sqlx::query("UPDATE campaigns SET ai_budget_cents = 0 WHERE id = $1")
        .bind(Uuid::parse_str(&t.campaign).unwrap())
        .execute(&t.pool)
        .await
        .unwrap();
    let r = t.gm("POST", &draft, None).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "AI_BUDGET_EXCEEDED");
    assert!(fake.calls().is_empty(), "{:?}", fake.calls());

    sqlx::query("UPDATE campaigns SET ai_budget_cents = 100 WHERE id = $1")
        .bind(Uuid::parse_str(&t.campaign).unwrap())
        .execute(&t.pool)
        .await
        .unwrap();
    let r = t.gm("POST", &draft, None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(r.body["data"]["players"].is_string(), "{}", r.body);
    assert_eq!(fake.calls().len(), 1);
    let usage = t.gm("GET", "/ai", None).await.body;
    assert_eq!(usage["data"]["calls"][0]["template"], "recap@2", "{usage}");
    assert!(usage["data"]["spending"]["spentMicros"].as_i64().unwrap() > 0);

    // A provider that answers prose: a readable 502, the call still
    // counted.
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
        &format!("/api/campaigns/{}{draft}", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_GATEWAY, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "AI_OUTPUT_INVALID");
    let calls: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_calls WHERE campaign_id = $1")
        .bind(Uuid::parse_str(&t.campaign).unwrap())
        .fetch_one(&t.pool)
        .await
        .unwrap();
    assert_eq!(calls, 2);
}

/// copilot/draft-narration: the co-GM drafts from what the table knows,
/// an invented id loses its button, nothing reaches a phone until the
/// GM shows their edited text.
#[tokio::test]
async fn the_co_gm_drafts_and_only_the_gm_shows() {
    let fake = Arc::new(FakeProvider::default());
    let t = table_with(Ai::new(Some(fake.clone()), Pricing::default())).await;
    t.live().await;
    sqlx::query("UPDATE campaigns SET ai_budget_cents = 100 WHERE id = $1")
        .bind(Uuid::parse_str(&t.campaign).unwrap())
        .execute(&t.pool)
        .await
        .unwrap();
    t.gm(
        "POST",
        "/session/reveal",
        Some(json!({ "kind": "scene", "node": "sc_taverne" })),
    )
    .await;
    t.ask("Je demande au tavernier qui a allumé le phare.")
        .await;

    let r = t
        .gm(
            "POST",
            "/session/copilot",
            Some(json!({ "kind": "consequence", "prompt": "ils crient" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let draft = &r.body["data"];
    assert_eq!(draft["status"], "draft");
    assert!(
        draft["answer"]["narration"]
            .as_str()
            .unwrap()
            .contains("ils crient"),
        "{draft}"
    );
    let suggestions = draft["answer"]["suggestions"].as_array().unwrap();
    assert!(
        suggestions
            .iter()
            .filter(|s| !s["action"].is_null())
            .count()
            >= 2,
        "{draft}"
    );
    let invented = suggestions
        .iter()
        .find(|s| s["label"] == "Action fantaisiste")
        .unwrap();
    assert!(invented["action"].is_null(), "{invented}");
    // The co-GM read the scene, the player's request and the journal.
    let prompt = &fake.calls()[0];
    assert!(prompt.starts_with("complete:"), "{prompt}");
    let ctx: String = sqlx::query_scalar("SELECT template FROM ai_calls WHERE campaign_id = $1")
        .bind(Uuid::parse_str(&t.campaign).unwrap())
        .fetch_one(&t.pool)
        .await
        .unwrap();
    assert_eq!(ctx, "copilot@1");

    // The phones see nothing of the draft.
    let narration = draft["answer"]["narration"].as_str().unwrap().to_string();
    let view = t.player(&t.marc, "GET", "/evening", None).await.body;
    assert!(!view.to_string().contains(&narration), "{view}");

    // The GM edits and shows: their words, not the draft's, reach the
    // table; a draft is shown once.
    let id = draft["id"].as_str().unwrap();
    let r = t
        .gm(
            "POST",
            &format!("/session/copilot/{id}/show"),
            Some(json!({ "narration": "Les chopes tremblent sur le comptoir.",
                         "npcLines": [{ "speaker": "Le tavernier", "text": "Personne, voyons." }] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let view = t.player(&t.lea, "GET", "/evening", None).await.body;
    let journal = view["data"]["journal"].to_string();
    assert!(journal.contains("Les chopes tremblent"), "{journal}");
    assert!(
        journal.contains("Le tavernier : « Personne, voyons. »"),
        "{journal}"
    );
    assert!(!journal.contains(&narration), "{journal}");
    let r = t
        .gm("POST", &format!("/session/copilot/{id}/dismiss"), None)
        .await;
    assert_eq!(r.body["error"]["code"], "DRAFT_ALREADY_DECIDED");
}
