//! copilot/check-character-sheets and copilot/co-write-backstory — the
//! co-GM writes the player's three answers up, drafts the GM's word on a
//! sheet over the rules, and proposes secret hooks tied only to what
//! exists. Every call is counted; nothing is stored behind the GM's back.

mod common;

use axum::http::StatusCode;
use common::marked::FIXTURE;
use common::{call, call_as_player, imported_campaign, invite_code, join};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn calls(pool: &PgPool, campaign: &str, purpose: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM ai_calls WHERE campaign_id = $1 AND purpose = $2")
        .bind(Uuid::parse_str(campaign).unwrap())
        .bind(purpose)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn marc_writes_his_story_with_the_co_gm_and_romain_gets_a_word_and_hooks() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    sqlx::query("UPDATE campaigns SET ai_budget_cents = 100 WHERE id = $1")
        .bind(Uuid::parse_str(&campaign).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let code = invite_code(&app, &gm, &campaign).await;
    let r = join(&app, &code, "Marc", "player").await;
    let token = r.player_token().unwrap();
    let character = r.body["data"]["character"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let play = format!("/api/play/{campaign}/character");
    let base = format!("/api/campaigns/{campaign}/characters/{character}");

    // Moment 7 of « Créer »: three answers, one paragraph, his own words.
    let r = call_as_player(
        &app,
        Some(&token),
        "PUT",
        &play,
        Some(json!({ "name": "Borin" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let empty = json!({ "origin": " ", "loss": "", "quest": "" });
    let r = call_as_player(
        &app,
        Some(&token),
        "POST",
        &format!("{play}/backstory"),
        Some(empty),
    )
    .await;
    assert_eq!(r.body["error"]["code"], "EMPTY_TEXT");
    let answers = json!({
        "origin": "De la mine d’argent de Valombre",
        "loss": "Son frère Dorn, dans la mine",
        "quest": "Pourquoi la mine paie encore",
    });
    let r = call_as_player(
        &app,
        Some(&token),
        "POST",
        &format!("{play}/backstory"),
        Some(answers),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let text = r.body["data"]["text"].as_str().unwrap();
    assert!(
        text.starts_with("Borin.") && text.contains("Valombre") && text.contains("Dorn"),
        "{text}"
    );
    assert_eq!(calls(&pool, &campaign, "backstory.write").await, 1);

    // Sent with Force 18: the co-GM drafts the word, the GM decides.
    sqlx::query("UPDATE characters SET status = 'submitted', sheet = $2 WHERE id = $1")
        .bind(Uuid::parse_str(&character).unwrap())
        .bind(json!({
            "name": "Borin", "classId": "bretteur",
            "abilities": { "FOR": 18, "DEX": 14, "CON": 10, "INT": 8, "SAG": 10, "CHA": 9 },
            "backstory": { "origin": "De la mine d’argent de Valombre", "text": text },
        }))
        .execute(&pool)
        .await
        .unwrap();
    let r = call(&app, Some(&gm), "POST", &format!("{base}/note-draft"), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(
        r.body["data"]["note"].as_str().unwrap().contains("dépasse"),
        "{}",
        r.body
    );
    assert_eq!(calls(&pool, &campaign, "sheet.note").await, 1);
    // Drafting stores nothing: the sheet still waits for the GM.
    let review = call(&app, Some(&gm), "GET", &base, None).await;
    assert_eq!(review.body["data"]["status"], "submitted");
    assert_eq!(review.body["data"]["gmNote"], json!(null));
    // Once sent, the story is no longer the player's to rewrite.
    let r = call_as_player(
        &app,
        Some(&token),
        "POST",
        &format!("{play}/backstory"),
        Some(json!({ "origin": "Ailleurs" })),
    )
    .await;
    assert_eq!(r.body["error"]["code"], "CHARACTER_LOCKED");

    // Hooks: tied to a scene that exists; the invented link and the
    // untitled hook are dropped; nothing is kept until the GM adds one.
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("{base}/hooks/propose"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let hooks = r.body["data"]["hooks"].as_array().unwrap();
    assert_eq!(hooks.len(), 1, "{}", r.body);
    assert_eq!(r.body["data"]["dropped"], 2);
    let targets = call(
        &app,
        Some(&gm),
        "GET",
        &format!("/api/campaigns/{campaign}/hooks"),
        None,
    )
    .await;
    assert_eq!(
        hooks[0]["links"],
        json!([targets.body["data"]["targets"][0]["id"]])
    );
    assert_eq!(targets.body["data"]["hooks"], json!([]));
    assert_eq!(calls(&pool, &campaign, "hooks.propose").await, 1);

    // No budget left, no call.
    sqlx::query("UPDATE campaigns SET ai_budget_cents = 0 WHERE id = $1")
        .bind(Uuid::parse_str(&campaign).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let r = call(&app, Some(&gm), "POST", &format!("{base}/note-draft"), None).await;
    assert_eq!(r.body["error"]["code"], "AI_BUDGET_EXCEEDED");
}

#[tokio::test]
async fn hooks_need_a_story_and_a_draft_note_needs_a_sent_sheet() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let r = join(&app, &code, "Marc", "player").await;
    let character = r.body["data"]["character"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let base = format!("/api/campaigns/{campaign}/characters/{character}");
    let r = call(&app, Some(&gm), "POST", &format!("{base}/note-draft"), None).await;
    assert_eq!(r.body["error"]["code"], "CHARACTER_NOT_SUBMITTED");
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("{base}/hooks/propose"),
        None,
    )
    .await;
    assert_eq!(r.body["error"]["code"], "NO_BACKSTORY");
}
