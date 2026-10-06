//! media/draw-pixel-art-assets, maps/build-tileset-packs — the GM asks
//! for an image, reviews it, and the table sees it once its subject is
//! theirs to see. Each drawing is counted against the AI budget.

mod common;

use axum::http::StatusCode;
use common::{call, call_as_player, imported_campaign, invite_code, join};
use promptus_back::ai::Ai;
use promptus_back::auth::setup::SetupState;
use promptus_back::live::{LiveConfig, LiveHub};
use serde_json::{Value, json};
use uuid::Uuid;

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

#[tokio::test]
async fn an_image_reaches_the_table_only_approved_and_once_its_scene_is_entered() {
    let pool = common::test_pool().await;
    let app = common::app_with_ai(
        pool.clone(),
        SetupState::closed(),
        LiveHub::new(LiveConfig::default()),
        Ai::fake(),
    );
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player")
        .await
        .player_token()
        .unwrap();
    let gm_call = |method: &'static str, path: String, body: Option<Value>| {
        let (app, gm, campaign) = (app.clone(), gm.clone(), campaign.clone());
        async move {
            call(
                &app,
                Some(&gm),
                method,
                &format!("/api/campaigns/{campaign}{path}"),
                body,
            )
            .await
        }
    };
    let seen = || {
        let (app, marc, campaign) = (app.clone(), marc.clone(), campaign.clone());
        async move {
            call_as_player(
                &app,
                Some(&marc),
                "GET",
                &format!("/api/play/{campaign}/media"),
                None,
            )
            .await
            .body["data"]["assets"]
                .as_array()
                .unwrap()
                .clone()
        }
    };
    let id = Uuid::parse_str(&campaign).unwrap();

    // The fixture's budget is zero: nothing is drawn.
    let ask = json!({ "kind": "scene", "subject": "sc_phare" });
    let r = gm_call("POST", "/media".into(), Some(ask.clone())).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "AI_BUDGET_EXCEEDED");

    sqlx::query("UPDATE campaigns SET ai_budget_cents = 100 WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let r = gm_call(
        "POST",
        "/media".into(),
        Some(json!({ "kind": "npc", "subject": "pnj_inconnu" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "UNKNOWN_SUBJECT");

    let r = gm_call("POST", "/media".into(), Some(ask.clone())).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "pending");
    let first = r.body["data"]["id"].as_str().unwrap().to_string();
    let calls: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_calls WHERE campaign_id = $1 AND purpose = 'media.scene'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(calls, 1);

    // The GM sees the image; the table does not, even by id.
    let r = gm_call("GET", format!("/media/{first}/image"), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let r = call_as_player(
        &app,
        Some(&marc),
        "GET",
        &format!("/api/play/{campaign}/media/{first}/image"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // Approved, it still waits for the scene.
    let r = gm_call(
        "POST",
        format!("/media/{first}/decision"),
        Some(json!({ "approve": true })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let r = gm_call(
        "POST",
        format!("/media/{first}/decision"),
        Some(json!({ "approve": false })),
    )
    .await;
    assert_eq!(r.body["error"]["code"], "ALREADY_DECIDED");
    assert!(seen().await.is_empty());

    gm_call("POST", "/session".into(), None).await;
    gm_call("POST", "/session/start".into(), None).await;
    let r = gm_call(
        "POST",
        "/session/reveal".into(),
        Some(json!({ "kind": "scene", "node": "sc_phare" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let shown = seen().await;
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0]["id"], first.as_str());
    assert_eq!(shown[0].get("direction"), None);
    let r = call_as_player(
        &app,
        Some(&marc),
        "GET",
        &format!("/api/play/{campaign}/media/{first}/image"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);

    // A second try, with the GM's word, replaces the first once approved.
    let r = gm_call(
        "POST",
        "/media".into(),
        Some(json!({ "kind": "scene", "subject": "sc_phare", "direction": "plus sombre" })),
    )
    .await;
    let second = r.body["data"]["id"].as_str().unwrap().to_string();
    assert_eq!(seen().await[0]["id"], first.as_str());
    gm_call(
        "POST",
        format!("/media/{second}/decision"),
        Some(json!({ "approve": true })),
    )
    .await;
    let shown = seen().await;
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0]["id"], second.as_str());

    // A material of the world's tileset: every screen may draw with it.
    let r = gm_call(
        "POST",
        "/media".into(),
        Some(json!({ "kind": "tileset", "subject": "pavés" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let list = gm_call("GET", "/media".into(), None).await;
    assert_eq!(list.body["data"]["assets"].as_array().unwrap().len(), 3);
    assert_eq!(list.body["data"]["theme"]["tilesets"][0]["id"], "port-1718");
}
