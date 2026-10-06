//! ai/generate-campaign — done when a pitch produces a valid campaign
//! (validator green) that the GM reviews and applies; V1's
//! `generation.test.ts` (unit and integration) ported: the steps in
//! order, a repair kept, a retry on bad JSON, a budget that stops the
//! job, and a draft applied once.

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call};
use promptus_back::ai::fake::FakeProvider;
use promptus_back::ai::{Ai, Pricing, Provider};
use promptus_back::auth::setup::SetupState;
use promptus_back::live::{LiveConfig, LiveHub};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const PITCH: &str = "Des gobelins pillent un village minier.";

fn app_with(pool: &PgPool, provider: &Arc<FakeProvider>) -> Router {
    common::app_with_ai(
        pool.clone(),
        SetupState::closed(),
        LiveHub::new(LiveConfig::default()),
        Ai::new(
            Some(provider.clone() as Arc<dyn Provider>),
            Pricing::default(),
        ),
    )
}

/// A campaign to prepare, with `budget_cents` to spend on AI.
async fn new_campaign(app: &Router, token: &str, budget_cents: i32) -> String {
    let r = call(
        app,
        Some(token),
        "POST",
        "/api/campaigns",
        Some(json!({
            "title": "La mine",
            "rules": { "id": "corsaires", "version": 1 },
            "aiBudgetCents": budget_cents
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["data"]["id"].as_str().unwrap().to_string()
}

async fn start(app: &Router, token: &str, campaign: &str, pitch: &str) -> Reply {
    call(
        app,
        Some(token),
        "POST",
        &format!("/api/campaigns/{campaign}/generation"),
        Some(json!({ "pitch": pitch, "tone": "Âpre", "length": "one_shot" })),
    )
    .await
}

/// The job once it is no longer running.
async fn finished(app: &Router, token: &str, campaign: &str) -> Value {
    for _ in 0..500 {
        let r = call(
            app,
            Some(token),
            "GET",
            &format!("/api/campaigns/{campaign}/generation"),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let job = r.body["data"]["jobs"][0].clone();
        if job["status"] != "running" {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the generation never finished");
}

fn statuses(job: &Value) -> Vec<String> {
    job["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            format!(
                "{}:{}",
                s["id"].as_str().unwrap(),
                s["status"].as_str().unwrap()
            )
        })
        .collect()
}

#[tokio::test]
async fn a_pitch_becomes_a_campaign_the_gm_reviews_and_applies() {
    let pool = common::test_pool().await;
    let provider = Arc::new(FakeProvider::default());
    let app = app_with(&pool, &provider);
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = new_campaign(&app, &token, 500).await;

    // What one more job costs, before starting it.
    let r = call(
        &app,
        Some(&token),
        "GET",
        &format!("/api/campaigns/{campaign}/generation"),
        None,
    )
    .await;
    assert!(r.body["data"]["estimateMicros"].as_i64().unwrap() > 0);
    assert_eq!(r.body["data"]["validated"], false);

    let r = start(&app, &token, &campaign, PITCH).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "running");
    // One job at a time.
    let again = start(&app, &token, &campaign, PITCH).await;
    assert_eq!(again.body["error"]["code"], "GENERATION_RUNNING");

    let job = finished(&app, &token, &campaign).await;
    assert_eq!(job["status"], "succeeded", "{job}");
    assert_eq!(
        statuses(&job),
        ["cast:done", "scenes:done", "check:done"],
        "{job}"
    );
    assert_eq!(provider.calls(), ["generation:cast", "generation:scenes"]);
    // Validator green, rule system included.
    assert_eq!(job["issues"], json!([]), "{}", job["issues"]);
    assert_eq!(job["steps"][1]["counts"]["nodes"], 7);
    // The invented faction, NPC and scene were removed.
    let removed = job["removed"].as_array().unwrap();
    assert_eq!(removed.len(), 3, "{removed:?}");
    assert!(removed.iter().all(|r| r["code"] == "REF_DANGLING"));
    let draft = &job["draft"];
    assert_eq!(draft["title"], "La mine");
    assert!(draft["bible"]["pitch"].as_str().unwrap().starts_with(PITCH));
    assert_eq!(
        draft["adversaries"][0]["stats"]["from_rules"],
        "marin_de_gueule_rouge"
    );
    assert_eq!(
        draft["adversaries"][1]["stats"]["from_rules"],
        "gueule_rouge"
    );
    assert!(job["costMicros"].as_i64().unwrap() > 0);

    // Nothing applied yet.
    let base = format!("/api/campaigns/{campaign}");
    let r = call(&app, Some(&token), "GET", &base, None).await;
    assert!(r.body["data"]["story"]["nodes"].is_null(), "{}", r.body);

    // The GM applies it: the campaign holds the draft, still to review.
    let id = job["id"].as_str().unwrap();
    let r = call(
        &app,
        Some(&token),
        "POST",
        &format!("{base}/generation/{id}/apply"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["job"]["status"], "applied");
    let detail = &r.body["data"]["campaign"];
    assert_eq!(detail["story"]["nodes"].as_array().unwrap().len(), 7);
    assert_eq!(detail["issues"], json!([]));
    assert!(detail["validatedAt"].is_null());
    let r = call(
        &app,
        Some(&token),
        "POST",
        &format!("{base}/generation/{id}/apply"),
        None,
    )
    .await;
    assert_eq!(r.body["error"]["code"], "GENERATION_ALREADY_APPLIED");

    // Reviewed and validated, it opens a session; then no more
    // generation on it.
    let r = call(
        &app,
        Some(&token),
        "POST",
        &format!("{base}/story/validate"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = call(&app, Some(&token), "POST", &format!("{base}/session"), None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let r = start(&app, &token, &campaign, PITCH).await;
    assert_eq!(r.body["error"]["code"], "CAMPAIGN_ALREADY_VALIDATED");

    // Every call was counted.
    let calls: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_calls WHERE campaign_id = $1 AND purpose = 'generation'",
    )
    .bind(Uuid::parse_str(&campaign).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(calls, 2);
}

#[tokio::test]
async fn the_validator_s_findings_go_back_to_the_model_and_its_repair_is_kept() {
    let pool = common::test_pool().await;
    let mut fake = FakeProvider::default();
    fake.few_clues = true;
    let provider = Arc::new(fake);
    let app = app_with(&pool, &provider);
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = new_campaign(&app, &token, 500).await;
    let r = start(&app, &token, &campaign, PITCH).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);

    let job = finished(&app, &token, &campaign).await;
    assert_eq!(job["status"], "succeeded", "{job}");
    assert_eq!(
        provider.calls(),
        ["generation:cast", "generation:scenes", "generation:repair"]
    );
    assert_eq!(job["issues"], json!([]), "{}", job["issues"]);
    assert_eq!(job["repairs"], 1);
    // The repair's invented id was dropped.
    assert_eq!(job["dropped"], 1);
    assert_eq!(job["steps"][2]["counts"]["repairs"], 1);
    // Two clues to the traitor and two to the den, back to three scenes.
    assert_eq!(job["draft"]["clues"].as_array().unwrap().len(), 7);
}

#[tokio::test]
async fn an_answer_out_of_format_is_sent_back_once() {
    let pool = common::test_pool().await;
    let mut fake = FakeProvider::default();
    fake.bad_json_first = true;
    let provider = Arc::new(fake);
    let app = app_with(&pool, &provider);
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = new_campaign(&app, &token, 500).await;
    start(&app, &token, &campaign, PITCH).await;

    let job = finished(&app, &token, &campaign).await;
    assert_eq!(job["status"], "succeeded", "{job}");
    assert_eq!(
        provider.calls(),
        ["generation:cast", "generation:cast", "generation:scenes"]
    );
    assert_eq!(job["issues"], json!([]));
}

#[tokio::test]
async fn a_model_that_never_answers_in_format_fails_the_job() {
    let pool = common::test_pool().await;
    let provider = Arc::new(FakeProvider::broken());
    let app = app_with(&pool, &provider);
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = new_campaign(&app, &token, 500).await;
    start(&app, &token, &campaign, PITCH).await;

    let job = finished(&app, &token, &campaign).await;
    assert_eq!(job["status"], "failed");
    assert_eq!(job["error"], "AI_OUTPUT_INVALID");
    assert!(job["detail"].is_string());
    assert_eq!(
        statuses(&job),
        ["cast:failed", "scenes:pending", "check:pending"]
    );
    assert_eq!(provider.calls().len(), 2);
    // A failed job is not applied.
    let id = job["id"].as_str().unwrap();
    let r = call(
        &app,
        Some(&token),
        "POST",
        &format!("/api/campaigns/{campaign}/generation/{id}/apply"),
        None,
    )
    .await;
    assert_eq!(r.body["error"]["code"], "GENERATION_NOT_READY");
}

#[tokio::test]
async fn the_budget_refuses_the_job_or_stops_it() {
    let pool = common::test_pool().await;
    // Each call costs 0.45 $ on a 0.50 $ budget: the first fits, the
    // second would pass it and is never made.
    let mut fake = FakeProvider::default();
    fake.cost_micros = 450_000;
    let provider = Arc::new(fake);
    let app = app_with(&pool, &provider);
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;

    // A budget below the estimate: refused before any call.
    let poor = new_campaign(&app, &token, 1).await;
    let r = start(&app, &token, &poor, PITCH).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "AI_BUDGET_EXCEEDED");
    assert!(provider.calls().is_empty());

    let campaign = new_campaign(&app, &token, 50).await;
    let r = start(&app, &token, &campaign, PITCH).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    let job = finished(&app, &token, &campaign).await;
    assert_eq!(job["status"], "failed");
    assert_eq!(job["error"], "AI_BUDGET_EXCEEDED");
    assert_eq!(provider.calls(), ["generation:cast"]);
    assert_eq!(
        statuses(&job),
        ["cast:done", "scenes:failed", "check:pending"]
    );
}

#[tokio::test]
async fn another_gm_sees_and_applies_nothing() {
    let pool = common::test_pool().await;
    let provider = Arc::new(FakeProvider::default());
    let app = app_with(&pool, &provider);
    let (_, owner) = common::signed_in_gm(&pool, "Romain").await;
    let (_, other) = common::signed_in_gm(&pool, "Autre").await;
    let campaign = new_campaign(&app, &owner, 500).await;
    start(&app, &owner, &campaign, PITCH).await;
    let job = finished(&app, &owner, &campaign).await;
    let id = job["id"].as_str().unwrap();
    for (method, path) in [
        ("GET", format!("/api/campaigns/{campaign}/generation")),
        ("POST", format!("/api/campaigns/{campaign}/generation")),
        (
            "POST",
            format!("/api/campaigns/{campaign}/generation/{id}/apply"),
        ),
    ] {
        let body =
            (method == "POST" && path.ends_with("/generation")).then(|| json!({ "pitch": PITCH }));
        let r = call(&app, Some(&other), method, &path, body).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{method} {path}");
    }
    // A pitch too short is refused before anything.
    let r = start(&app, &owner, &campaign, "Court").await;
    assert_eq!(r.body["error"]["code"], "PITCH_TOO_SHORT");
}
