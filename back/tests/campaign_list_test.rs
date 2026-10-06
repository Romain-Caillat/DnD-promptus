//! campaign/list-campaigns — done when a GM creates a campaign, finds it
//! again in their list, reopens it, and another GM account does not see
//! it.

mod common;

use axum::http::StatusCode;
use common::call;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

fn new_campaign() -> Value {
    json!({
        "title": "  Les Cendres de Valombre ",
        "world": " Fantasy, une ville minière ",
        "rules": { "id": "corsaires", "version": 1 },
        "pitch": "Une ville minière qui enterre ses morts deux fois.",
        "playerHook": "Le maire vous a fait venir.",
        "playerCount": 5,
        "aiBudgetCents": 1000
    })
}

fn settings(title: &str, hook: &str, players: i64, cents: i64) -> Value {
    json!({
        "title": title,
        "world": "Fantasy",
        "pitch": "Le maire paie le culte.",
        "playerHook": hook,
        "playerCount": players,
        "aiBudgetCents": cents
    })
}

fn card<'a>(list: &'a Value, id: &str) -> Option<&'a Value> {
    list["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
}

async fn story_version(pool: &PgPool, campaign: &str) -> Option<i64> {
    sqlx::query_scalar(
        "SELECT version FROM live_versions WHERE campaign_id = $1 AND topic = 'story'",
    )
    .bind(Uuid::parse_str(campaign).unwrap())
    .fetch_optional(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn a_gm_creates_finds_and_reopens_a_campaign_another_gm_never_sees() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, romain) = common::signed_in_gm(&pool, "Romain").await;
    let (_, marc) = common::signed_in_gm(&pool, "Marc").await;

    let r = call(
        &app,
        Some(&romain),
        "POST",
        "/api/campaigns",
        Some(new_campaign()),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let id = r.body["data"]["id"].as_str().unwrap().to_string();

    // Found again on the list, with what its card shows.
    let list = call(&app, Some(&romain), "GET", "/api/campaigns", None).await;
    let found = card(&list.body, &id).expect("the new campaign is listed");
    assert_eq!(found["title"], "Les Cendres de Valombre");
    assert_eq!(found["world"], "Fantasy, une ville minière");
    assert_eq!(found["rules"], json!({ "id": "corsaires", "version": 1 }));
    assert_eq!(found["playerCount"], 5);
    assert_eq!(found["playersSeated"], 0);
    assert_eq!(found["archivedAt"], Value::Null);

    // Reopened whole: story, pitch and the GM's settings.
    let r = call(
        &app,
        Some(&romain),
        "GET",
        &format!("/api/campaigns/{id}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let story = &r.body["data"]["story"];
    assert_eq!(
        story["bible"]["pitch"],
        "Une ville minière qui enterre ses morts deux fois."
    );
    assert_eq!(story["bible"]["player_hook"], "Le maire vous a fait venir.");
    assert_eq!(
        r.body["data"]["settings"],
        json!({ "playerCount": 5, "aiBudgetCents": 1000 })
    );

    // Marc's list does not have it, and his own campaigns are his alone.
    let r = call(
        &app,
        Some(&marc),
        "POST",
        "/api/campaigns",
        Some(new_campaign()),
    )
    .await;
    let his = r.body["data"]["id"].as_str().unwrap().to_string();
    let list = call(&app, Some(&marc), "GET", "/api/campaigns", None).await;
    assert!(card(&list.body, &id).is_none());
    assert!(card(&list.body, &his).is_some());
    let list = call(&app, Some(&romain), "GET", "/api/campaigns", None).await;
    assert!(card(&list.body, &his).is_none());
    let r = call(
        &app,
        Some(&marc),
        "GET",
        &format!("/api/campaigns/{id}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_list_counts_seated_players_and_puts_the_latest_activity_first() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns",
        Some(new_campaign()),
    )
    .await;
    let older = r.body["data"]["id"].as_str().unwrap().to_string();
    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns",
        Some(new_campaign()),
    )
    .await;
    let newer = r.body["data"]["id"].as_str().unwrap().to_string();
    // Created a moment apart: the second is the more recent.
    sqlx::query("UPDATE campaigns SET updated_at = now() - interval '1 day' WHERE id = $1")
        .bind(Uuid::parse_str(&older).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    // Two players and a spectator join the older one: it becomes the
    // latest activity, and only the players are seated.
    let code = common::invite_code(&app, &token, &older).await;
    for (nickname, role) in [
        ("Marc", "player"),
        ("Camille", "player"),
        ("Hugo", "spectator"),
    ] {
        let r = common::join(&app, &code, nickname, role).await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    }

    let list = call(&app, Some(&token), "GET", "/api/campaigns", None).await;
    let ids: Vec<&str> = list.body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [older.as_str(), newer.as_str()]);
    assert_eq!(card(&list.body, &older).unwrap()["playersSeated"], 2);
    assert_eq!(card(&list.body, &newer).unwrap()["playersSeated"], 0);
}

#[tokio::test]
async fn settings_change_the_campaign_and_tell_players_only_what_they_see() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns",
        Some(new_campaign()),
    )
    .await;
    let id = r.body["data"]["id"].as_str().unwrap().to_string();
    let story_id = r.body["data"]["story"]["id"].clone();
    let uri = format!("/api/campaigns/{id}/settings");

    let r = call(
        &app,
        Some(&token),
        "PUT",
        &uri,
        Some(settings(" Valombre ", "Une lettre vous attend.", 6, 2500)),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let story = &r.body["data"]["story"];
    assert_eq!(story["title"], "Valombre");
    assert_eq!(story["world"], "Fantasy");
    assert_eq!(story["bible"]["pitch"], "Le maire paie le culte.");
    // The id is stable: renaming a campaign does not rename it.
    assert_eq!(story["id"], story_id);
    assert_eq!(
        r.body["data"]["settings"],
        json!({ "playerCount": 6, "aiBudgetCents": 2500 })
    );
    let after_story = story_version(&pool, &id).await;
    assert!(
        after_story.is_some(),
        "players read the title and hook: the story topic moves"
    );

    // The players' preview reads the new title and hook.
    let r = call(
        &app,
        Some(&token),
        "GET",
        &format!("/api/campaigns/{id}/player-view"),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["title"], "Valombre");
    assert_eq!(r.body["data"]["playerHook"], "Une lettre vous attend.");

    // Only the GM's planning changes: players see nothing new.
    let r = call(
        &app,
        Some(&token),
        "PUT",
        &uri,
        Some(settings("Valombre", "Une lettre vous attend.", 4, 0)),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(story_version(&pool, &id).await, after_story);

    // Out of range is refused and changes nothing.
    for (body, code) in [
        (settings("  ", "", 4, 0), "TITLE_REQUIRED"),
        (settings("Valombre", "", 0, 0), "INVALID_PLAYER_COUNT"),
        (settings("Valombre", "", 13, 0), "INVALID_PLAYER_COUNT"),
        (settings("Valombre", "", 4, -1), "INVALID_AI_BUDGET"),
    ] {
        let r = call(&app, Some(&token), "PUT", &uri, Some(body)).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST);
        assert_eq!(r.body["error"]["code"], code);
    }
    let r = call(
        &app,
        Some(&token),
        "GET",
        &format!("/api/campaigns/{id}"),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["story"]["title"], "Valombre");
    assert_eq!(r.body["data"]["settings"]["playerCount"], 4);
}

#[tokio::test]
async fn an_archived_campaign_stays_listed_as_archived_and_comes_back() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let r = call(
        &app,
        Some(&token),
        "POST",
        "/api/campaigns",
        Some(new_campaign()),
    )
    .await;
    let id = r.body["data"]["id"].as_str().unwrap().to_string();
    let uri = format!("/api/campaigns/{id}/archive");

    let r = call(
        &app,
        Some(&token),
        "PUT",
        &uri,
        Some(json!({ "archived": true })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let first = r.body["data"]["archivedAt"].clone();
    assert!(first.is_string());
    // Archiving again keeps the date it was shelved.
    let r = call(
        &app,
        Some(&token),
        "PUT",
        &uri,
        Some(json!({ "archived": true })),
    )
    .await;
    assert_eq!(r.body["data"]["archivedAt"], first);
    let list = call(&app, Some(&token), "GET", "/api/campaigns", None).await;
    assert_eq!(card(&list.body, &id).unwrap()["archivedAt"], first);
    // Still reopened: archived is a shelf, not a deletion.
    let r = call(
        &app,
        Some(&token),
        "GET",
        &format!("/api/campaigns/{id}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);

    let r = call(
        &app,
        Some(&token),
        "PUT",
        &uri,
        Some(json!({ "archived": false })),
    )
    .await;
    assert_eq!(r.body["data"]["archivedAt"], Value::Null);
    let list = call(&app, Some(&token), "GET", "/api/campaigns", None).await;
    assert_eq!(card(&list.body, &id).unwrap()["archivedAt"], Value::Null);
}

#[tokio::test]
async fn the_rule_presets_bring_their_stat_names() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let r = call(&app, Some(&token), "GET", "/api/rule-systems", None).await;
    assert_eq!(r.status, StatusCode::OK);
    let presets = r.body["data"].as_array().unwrap();
    let ids: Vec<(&str, u64)> = presets
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p["version"].as_u64().unwrap()))
        .collect();
    assert_eq!(ids, [("corsaires", 1), ("brasier", 1)]);
    let corsaires = &presets[0];
    assert_eq!(corsaires["name"], "Corsaires de la Couronne");
    assert_eq!(
        corsaires["abilities"][0],
        json!({ "abbr": "FOR", "name": "Force" })
    );
    assert_eq!(corsaires["abilities"].as_array().unwrap().len(), 6);
    assert_eq!(corsaires["hitPoints"]["abbr"], "PV");
    // Every preset a GM is offered creates a campaign.
    for p in presets {
        let r = call(
            &app,
            Some(&token),
            "POST",
            "/api/campaigns",
            Some(json!({ "title": "Essai", "rules": { "id": p["id"], "version": p["version"] } })),
        )
        .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        assert_eq!(r.body["data"]["settings"]["playerCount"], 6);
    }
}
