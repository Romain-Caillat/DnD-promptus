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

/// Wait until no row of `campaign` is still being drawn.
async fn drawn(pool: &sqlx::PgPool, campaign: Uuid) {
    for _ in 0..200 {
        let drawing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM media_assets WHERE campaign_id = $1 AND status = 'drawing'",
        )
        .bind(campaign)
        .fetch_one(pool)
        .await
        .unwrap();
        if drawing == 0 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("still drawing");
}

#[tokio::test]
async fn a_batch_draws_what_is_missing_and_an_act_gets_its_video_in_the_background() {
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
    let id = Uuid::parse_str(&campaign).unwrap();
    let gm_call = |method: &'static str, path: &'static str, body: Option<Value>| {
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
    // 1 $: every image fits (0.04 $ each), not the act's video (4 $).
    sqlx::query("UPDATE campaigns SET ai_budget_cents = 100 WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let plan = gm_call("GET", "/media", None).await.body["data"]["plan"].clone();
    let images = plan["images"].as_array().unwrap().len();
    assert!(images >= 8, "{plan}");
    assert_eq!(
        plan["videos"],
        json!([{ "kind": "intro", "subject": "acte_1" }])
    );
    assert_eq!(plan["running"], false);

    let r = gm_call("POST", "/media/batch", Some(json!({ "videos": true }))).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "AI_BUDGET_EXCEEDED");
    let r = gm_call("POST", "/media/batch", Some(json!({ "videos": false }))).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["data"]["queued"].as_array().unwrap().len(), images);
    assert_eq!(r.body["data"]["queued"][0]["status"], "drawing");
    drawn(&pool, id).await;

    let list = gm_call("GET", "/media", None).await.body["data"].clone();
    let assets = list["assets"].as_array().unwrap();
    assert_eq!(assets.len(), images);
    assert!(assets.iter().all(|a| a["status"] == "pending"), "{list}");
    assert_eq!(list["plan"]["images"], json!([]));
    let r = gm_call("POST", "/media/batch", Some(json!({ "videos": false }))).await;
    assert_eq!(r.body["error"]["code"], "NOTHING_TO_DRAW");

    // The act's introduction, drawn in the background.
    sqlx::query("UPDATE campaigns SET ai_budget_cents = 1000 WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let r = gm_call(
        "POST",
        "/media",
        Some(json!({ "kind": "intro", "subject": "acte_1", "direction": "à l'aube" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "drawing");
    let video = r.body["data"]["id"].as_str().unwrap().to_string();
    drawn(&pool, id).await;
    let (status, mime): (String, String) =
        sqlx::query_as("SELECT status, mime FROM media_assets WHERE id = $1")
            .bind(Uuid::parse_str(&video).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((status.as_str(), mime.as_str()), ("pending", "video/mp4"));
    let videos: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_calls WHERE campaign_id = $1 AND kind = 'video' AND purpose = 'media.intro'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(videos, 1);

    // Approved, the table sees it once a scene of the act is entered.
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("/api/campaigns/{campaign}/media/{video}/decision"),
        Some(json!({ "approve": true })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let seen = || async {
        call_as_player(
            &app,
            Some(&marc),
            "GET",
            &format!("/api/play/{campaign}/media"),
            None,
        )
        .await
        .body["data"]["assets"]
            .clone()
    };
    assert_eq!(seen().await, json!([]));
    gm_call("POST", "/session", None).await;
    gm_call("POST", "/session/start", None).await;
    gm_call(
        "POST",
        "/session/reveal",
        Some(json!({ "kind": "scene", "node": "sc_taverne" })),
    )
    .await;
    assert_eq!(
        seen().await,
        json!([{ "id": video, "kind": "intro", "subject": "acte_1" }])
    );

    // A drawing cut off by a restart says so instead of spinning forever.
    sqlx::query(
        "INSERT INTO media_assets (campaign_id, kind, subject, status) VALUES ($1, 'npc', 'pnj_gwen', 'drawing')",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    let list = gm_call("GET", "/media", None).await.body["data"].clone();
    let lost = &list["assets"][0];
    assert_eq!(lost["status"], "rejected", "{list}");
    assert!(lost["error"].as_str().unwrap().contains("Interrompu"));
}
