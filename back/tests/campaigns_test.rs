//! campaign/model-story-graph — a campaign imports from YAML, validates,
//! round-trips through export, is stored under its GM and is invisible
//! to every other GM.

mod common;

use axum::http::StatusCode;
use common::call;
use promptus_back::auth::guard::CurrentGm;
use promptus_back::campaigns;
use promptus_shared::story::{Campaign, FlagValue, from_yaml};
use serde_json::{Value, json};
use uuid::Uuid;

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

fn fixture_json() -> Value {
    serde_json::to_value(from_yaml(FIXTURE).unwrap()).unwrap()
}

fn issue_codes(detail: &Value) -> Vec<String> {
    detail["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["code"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn a_yaml_campaign_imports_validates_round_trips_and_belongs_to_its_gm() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, token) = common::signed_in_gm(&pool, "Romain").await;

    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns/import",
        Some(json!({ "yaml": FIXTURE })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let id = r.body["data"]["id"].as_str().unwrap().to_string();
    assert_eq!(r.body["data"]["issues"], json!([]));
    assert_eq!(r.body["data"]["story"], fixture_json());

    // Stored under its GM.
    let owner: Uuid = sqlx::query_scalar("SELECT gm_id FROM campaigns WHERE id = $1")
        .bind(Uuid::parse_str(&id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(owner, gm);

    // Read back whole.
    let r = call(
        &app,
        Some(&token),
        "GET",
        &format!("/api/campaigns/{id}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["data"]["story"], fixture_json());
    assert_eq!(r.body["data"]["world"], json!({}));

    // Export → import gives the same campaign, and the same text again.
    let r = call(
        &app,
        Some(&token),
        "GET",
        &format!("/api/campaigns/{id}/export"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let exported = r.body["data"]["yaml"].as_str().unwrap().to_string();
    assert_eq!(from_yaml(&exported).unwrap(), from_yaml(FIXTURE).unwrap());
    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns/import",
        Some(json!({ "yaml": exported })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    let copy = r.body["data"]["id"].as_str().unwrap().to_string();
    assert_ne!(copy, id);
    assert_eq!(r.body["data"]["story"], fixture_json());
    let r = call(
        &app,
        Some(&token),
        "GET",
        &format!("/api/campaigns/{copy}/export"),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["yaml"].as_str().unwrap(), exported);

    // Both are listed for their GM.
    let r = call(&app, Some(&token), "GET", "/api/campaigns", None).await;
    let listed: Vec<&str> = r.body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert!(listed.contains(&id.as_str()) && listed.contains(&copy.as_str()));
    let first = &r.body["data"][0];
    assert_eq!(first["title"], "Le Phare de Kerbrume");
    assert_eq!(first["world"], "Côte bretonne, 1718");
}

#[tokio::test]
async fn another_gm_gets_404_on_every_campaign_route() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, romain) = common::signed_in_gm(&pool, "Romain").await;
    let (_, marc) = common::signed_in_gm(&pool, "Marc").await;
    let r = call(
        &app,
        Some(&romain),
        "POST",
        "/api/campaigns/import",
        Some(json!({ "yaml": FIXTURE })),
    )
    .await;
    let id = r.body["data"]["id"].as_str().unwrap().to_string();

    let other = FIXTURE.replace("title: Le Phare de Kerbrume", "title: Volé");
    let missing = Uuid::new_v4().to_string();
    for target in [id.as_str(), missing.as_str(), "not-a-uuid"] {
        for (method, path, body) in [
            ("GET", format!("/api/campaigns/{target}"), None),
            ("GET", format!("/api/campaigns/{target}/export"), None),
            ("GET", format!("/api/campaigns/{target}/player-view"), None),
            (
                "PUT",
                format!("/api/campaigns/{target}/import"),
                Some(json!({ "yaml": other })),
            ),
        ] {
            let r = call(&app, Some(&marc), method, &path, body).await;
            assert_eq!(
                r.status,
                StatusCode::NOT_FOUND,
                "{method} {path}: {}",
                r.body
            );
            assert_eq!(r.body["error"]["code"], "NOT_FOUND");
        }
    }
    // Not listed for Marc, and untouched by his PUT.
    let r = call(&app, Some(&marc), "GET", "/api/campaigns", None).await;
    assert!(
        r.body["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["id"] != id.as_str())
    );
    let r = call(
        &app,
        Some(&romain),
        "GET",
        &format!("/api/campaigns/{id}"),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["story"]["title"], "Le Phare de Kerbrume");
}

#[tokio::test]
async fn a_text_that_is_not_a_campaign_is_refused_with_where() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, token) = common::signed_in_gm(&pool, "Romain").await;

    for bad in [
        FIXTURE.replace("    wants: Que quelqu'un", "    veut: Que quelqu'un"),
        "title: [unclosed".to_string(),
        FIXTURE.replace("rules:\n  id: corsaires\n  version: 1\n", ""),
    ] {
        assert_ne!(bad, FIXTURE);
        let r = call(
            &app,
            Some(&token),
            "POST",
            "/api/campaigns/import",
            Some(json!({ "yaml": bad })),
        )
        .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
        assert_eq!(r.body["error"]["code"], "INVALID_CAMPAIGN_YAML");
        let message = r.body["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("line") || message.contains("missing field"),
            "{message}"
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM campaigns WHERE gm_id = $1")
        .bind(gm)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn issues_are_reported_but_never_block_a_write() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let mut flawed = from_yaml(FIXTURE).unwrap();
    flawed.clues.retain(|c| c.id != "cl_carte");
    flawed.clues[0].node = "sc_disparue".into();
    let yaml = promptus_shared::story::to_yaml(&flawed).unwrap();

    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns/import",
        Some(json!({ "yaml": yaml })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let codes = issue_codes(&r.body["data"]);
    assert!(codes.contains(&"THREE_CLUE_RULE".to_string()), "{codes:?}");
    assert!(codes.contains(&"REF_DANGLING".to_string()), "{codes:?}");
    let issue = r.body["data"]["issues"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["code"] == "REF_DANGLING")
        .unwrap();
    assert_eq!(issue["severity"], "error");
    assert_eq!(issue["path"], "clues[0].node");
}

#[tokio::test]
async fn reimporting_replaces_the_story_and_keeps_the_world() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, token) = common::signed_in_gm(&pool, "Romain").await;
    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns/import",
        Some(json!({ "yaml": FIXTURE })),
    )
    .await;
    let id = r.body["data"]["id"].as_str().unwrap().to_string();
    let uuid = Uuid::parse_str(&id).unwrap();
    let current = CurrentGm {
        id: gm,
        display_name: "Romain".into(),
    };
    campaigns::update_world(&pool, &current, uuid, |c, w| {
        w.reveal_clue(c, "cl_gwen").unwrap();
        w.set_flag("brume", FlagValue::Bool(true));
        Ok(())
    })
    .await
    .unwrap();

    let edited = FIXTURE.replace(
        "title: Le Phare de Kerbrume",
        "title: Le Phare de Kerbrume (révisé)",
    );
    let r = call(
        &app,
        Some(&token),
        "PUT",
        &format!("/api/campaigns/{id}/import"),
        Some(json!({ "yaml": edited })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(
        r.body["data"]["story"]["title"],
        "Le Phare de Kerbrume (révisé)"
    );
    assert_eq!(r.body["data"]["world"]["found_clues"], json!(["cl_gwen"]));
    assert_eq!(r.body["data"]["world"]["flags"]["brume"], json!(true));

    // A refused re-import changes nothing.
    let r = call(
        &app,
        Some(&token),
        "PUT",
        &format!("/api/campaigns/{id}/import"),
        Some(json!({ "yaml": "nope: 1" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let row = campaigns::find(&pool, uuid).await.unwrap().unwrap();
    assert_eq!(row.story.title, "Le Phare de Kerbrume (révisé)");
}

#[tokio::test]
async fn a_new_empty_campaign_names_its_rule_system() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns",
        Some(json!({ "title": "  Le Brasier ", "world": "Espace", "rules": { "id": "brasier", "version": 2 } })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let story: Campaign = serde_json::from_value(r.body["data"]["story"].clone()).unwrap();
    assert_eq!(story.id, "le-brasier");
    assert_eq!(story.title, "Le Brasier");
    assert_eq!(
        (story.rules.id.as_str(), story.rules.version),
        ("brasier", 2)
    );
    assert!(story.nodes.is_empty());
    assert_eq!(r.body["data"]["issues"], json!([]));

    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns",
        Some(json!({ "title": "   ", "rules": { "id": "brasier", "version": 2 } })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "TITLE_REQUIRED");
}

#[tokio::test]
async fn the_gm_previews_what_players_see() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, token) = common::signed_in_gm(&pool, "Romain").await;
    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns/import",
        Some(json!({ "yaml": FIXTURE })),
    )
    .await;
    let id = r.body["data"]["id"].as_str().unwrap().to_string();
    let current = CurrentGm {
        id: gm,
        display_name: "Romain".into(),
    };
    campaigns::update_world(&pool, &current, Uuid::parse_str(&id).unwrap(), |c, w| {
        w.enter_node(c, "sc_crique").unwrap();
        Ok(())
    })
    .await
    .unwrap();
    let r = call(
        &app,
        Some(&token),
        "GET",
        &format!("/api/campaigns/{id}/player-view"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["data"]["scene"]["title"], "La crique aux Morts");
    let body = r.body.to_string();
    assert!(!body.contains("Corentin"), "{body}");
    assert!(!body.contains("statement"), "{body}");
}
