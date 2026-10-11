//! platform/stream-prep-edits — every preparation write takes a revision
//! and leaves a line in the journal; the GM catches up on what changed
//! since a revision and takes one back while nothing covered it; the
//! revision and Claude's activity reach the GM's sockets only.

mod common;

use axum::http::StatusCode;
use common::live::{Socket, next, next_of, open_player, open_screen, ready, table};
use common::{call, call_as_screen, call_with_token};
use promptus_back::live::LiveConfig;
use serde_json::{Value, json};

fn set_name(target: &str, name: &str) -> Value {
    json!({ "edits": [{ "op": "set", "target": target, "field": "name", "value": name }] })
}

/// A token for Claude, with the hand: its writes land directly, which is
/// what the journal and the activity are about (otherwise they are
/// proposed, `claude_proposals_test.rs`).
async fn claude(t: &common::live::Table) -> String {
    let (_, secret) = common::api_token(&t.router, &t.token, "Claude").await;
    common::claude_hand(&t.router, &t.token, &t.campaign.to_string()).await;
    secret
}

async fn npc_name(t: &common::live::Table, id: &str) -> String {
    let r = call(
        &t.router,
        Some(&t.token),
        "GET",
        &format!("/api/campaigns/{}", t.campaign),
        None,
    )
    .await;
    r.body["data"]["story"]["npcs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()["name"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn each_writer_leaves_its_line_and_the_gm_catches_up_since_a_revision() {
    let t = table(LiveConfig::default()).await;
    let secret = claude(&t).await;
    let edits = format!("/api/campaigns/{}/story/edits", t.campaign);

    let r = call(
        &t.router,
        Some(&t.token),
        "POST",
        &edits,
        Some(set_name("pnj_gwen", "Gwen la rousse")),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["campaign"]["revision"], 1);
    let r = call_with_token(
        &t.router,
        &secret,
        "POST",
        &edits,
        Some(set_name("lieu_phare", "Le phare noir")),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    // An edit that changes nothing takes no revision.
    let r = call_with_token(
        &t.router,
        &secret,
        "POST",
        &edits,
        Some(set_name("lieu_phare", "Le phare noir")),
    )
    .await;
    assert_eq!(r.body["data"]["campaign"]["revision"], 2, "{}", r.body);

    let r = call(
        &t.router,
        Some(&t.token),
        "GET",
        &format!("/api/campaigns/{}/revisions?since=1", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let data = &r.body["data"];
    assert_eq!(data["revision"], 2);
    let entries = data["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "only what came after 1: {data}");
    assert_eq!(entries[0]["revision"], 2);
    assert_eq!(entries[0]["author"], "token");
    assert_eq!(
        entries[0]["changes"],
        json!([{ "op": "set", "kind": "location", "id": "lieu_phare", "name": "Le phare noir" }])
    );
    assert_eq!(entries[0]["undoable"], true);

    let r = call(
        &t.router,
        Some(&t.token),
        "GET",
        &format!("/api/campaigns/{}/revisions?since=0", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["entries"][0]["author"], "gm");
}

#[tokio::test]
async fn an_undo_puts_back_what_one_revision_touched_until_a_later_one_covers_it() {
    let t = table(LiveConfig::default()).await;
    let secret = claude(&t).await;
    let edits = format!("/api/campaigns/{}/story/edits", t.campaign);
    let undo = |rev: i64| format!("/api/campaigns/{}/revisions/{rev}/undo", t.campaign);
    let gwen = npc_name(&t, "pnj_gwen").await;

    // Claude renames Gwen (1); the GM then renames Loïc (2).
    call_with_token(
        &t.router,
        &secret,
        "POST",
        &edits,
        Some(set_name("pnj_gwen", "Écrit par Claude")),
    )
    .await;
    call(
        &t.router,
        Some(&t.token),
        "POST",
        &edits,
        Some(set_name("pnj_loic", "Écrit par le MJ")),
    )
    .await;

    // A token never takes a revision back.
    let r = call_with_token(&t.router, &secret, "POST", &undo(1), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.body);

    // Nothing after 1 touched Gwen: the undo restores her, keeps Loïc.
    let r = call(&t.router, Some(&t.token), "POST", &undo(1), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["revision"], 3);
    assert_eq!(npc_name(&t, "pnj_gwen").await, gwen);
    assert_eq!(npc_name(&t, "pnj_loic").await, "Écrit par le MJ");

    // The undo itself touched Gwen: 1 is covered now.
    let r = call(&t.router, Some(&t.token), "POST", &undo(1), None).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "REVISION_COVERED");
    // So is 2 once the GM writes Loïc again.
    call(
        &t.router,
        Some(&t.token),
        "POST",
        &edits,
        Some(set_name("pnj_loic", "Loïc encore")),
    )
    .await;
    let r = call(&t.router, Some(&t.token), "POST", &undo(2), None).await;
    assert_eq!(r.body["error"]["code"], "REVISION_COVERED", "{}", r.body);

    let r = call(
        &t.router,
        Some(&t.token),
        "GET",
        &format!("/api/campaigns/{}/revisions", t.campaign),
        None,
    )
    .await;
    let entries = r.body["data"]["entries"].as_array().unwrap().clone();
    let by_rev = |n: i64| entries.iter().find(|e| e["revision"] == n).unwrap().clone();
    assert_eq!(by_rev(1)["undone"], true);
    assert_eq!(by_rev(3)["undoes"], 1);
    assert_eq!(by_rev(2)["undoable"], false);
    assert_eq!(by_rev(4)["undoable"], true);
}

/// Read `ws` up to the world change `version` (the sentinel), and return
/// every message type and topic seen on the way.
async fn seen_until_world(ws: &mut Socket, version: i64) -> Vec<String> {
    let mut seen = Vec::new();
    loop {
        let msg = next(ws).await;
        if msg["type"] == "changed" && msg["topic"] == "world" && msg["version"] == version {
            return seen;
        }
        seen.push(format!(
            "{}:{}",
            msg["type"].as_str().unwrap_or(""),
            msg["topic"].as_str().unwrap_or("")
        ));
    }
}

#[tokio::test]
async fn the_revision_and_claudes_activity_reach_the_gm_and_no_player_or_screen() {
    let t = table(LiveConfig::default()).await;
    let secret = claude(&t).await;
    let edits = format!("/api/campaigns/{}/story/edits", t.campaign);
    // One revision before anyone connects, so the versions sent on
    // connection hold the `prep` topic.
    call(
        &t.router,
        Some(&t.token),
        "POST",
        &edits,
        Some(set_name("pnj_gwen", "Gwen")),
    )
    .await;

    let (_, device) = t.seat("Marc").await;
    let r = call_as_screen(&t.router, None, "POST", "/api/tv", None).await;
    let tv = r.screen_token().unwrap();
    let code = r.body["data"]["code"].as_str().unwrap().to_string();
    let r = call(
        &t.router,
        Some(&t.token),
        "POST",
        &format!("/api/campaigns/{}/screens", t.campaign),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);

    let mut gm = t.connect().await;
    let mut phone = open_player(t.addr, Some(&device), t.campaign)
        .await
        .unwrap();
    let mut screen = open_screen(t.addr, &tv).await.unwrap();
    assert_eq!(ready(&mut gm).await["prep"], 1);
    for ws in [&mut phone, &mut screen] {
        let versions = ready(ws).await;
        assert!(versions.get("prep").is_none(), "{versions}");
        assert!(versions.get("proposals").is_none(), "{versions}");
        assert_eq!(
            versions["story"], 1,
            "the story itself is theirs to refetch"
        );
    }

    // Claude writes; then a world change every socket hears, the sentinel.
    let r = call_with_token(
        &t.router,
        &secret,
        "POST",
        &edits,
        Some(set_name("lieu_phare", "Le phare noir")),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    t.bump().await;

    let heard = seen_until_world(&mut gm, 1).await;
    assert!(heard.contains(&"changed:prep".to_string()), "{heard:?}");
    assert!(heard.contains(&"activity:".to_string()), "{heard:?}");
    for ws in [&mut phone, &mut screen] {
        let seen = seen_until_world(ws, 1).await;
        assert!(seen.contains(&"changed:story".to_string()), "{seen:?}");
        assert!(
            !seen
                .iter()
                .any(|s| s.starts_with("activity") || s.ends_with(":prep")),
            "{seen:?}"
        );
    }
}

#[tokio::test]
async fn claudes_activity_names_the_revision_it_wrote() {
    let t = table(LiveConfig::default()).await;
    let secret = claude(&t).await;
    let edits = format!("/api/campaigns/{}/story/edits", t.campaign);
    let mut gm = t.connect().await;
    ready(&mut gm).await;
    // The GM's own write is no activity; Claude's next one is.
    call(
        &t.router,
        Some(&t.token),
        "POST",
        &edits,
        Some(set_name("pnj_gwen", "Gwen")),
    )
    .await;
    call_with_token(
        &t.router,
        &secret,
        "POST",
        &edits,
        Some(set_name("lieu_phare", "Le phare noir")),
    )
    .await;
    assert_eq!(
        next_of(&mut gm, "activity").await,
        json!({ "type": "activity", "author": "token", "revision": 2 })
    );
}

/// Bump the world (the sentinel) and return its new version.
async fn sentinel(t: &common::live::Table) -> i64 {
    t.bump().await;
    sqlx::query_scalar(
        "SELECT version FROM live_versions WHERE campaign_id = $1 AND topic = 'world'",
    )
    .bind(t.campaign)
    .fetch_one(&t.pool)
    .await
    .unwrap()
}

/// A seated player's phone and a paired shared screen, connected.
async fn phone_and_screen(t: &common::live::Table) -> (Socket, Socket) {
    let (_, device) = t.seat("Marc").await;
    let r = call_as_screen(&t.router, None, "POST", "/api/tv", None).await;
    let tv = r.screen_token().unwrap();
    let code = r.body["data"]["code"].as_str().unwrap().to_string();
    let r = call(
        &t.router,
        Some(&t.token),
        "POST",
        &format!("/api/campaigns/{}/screens", t.campaign),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let phone = open_player(t.addr, Some(&device), t.campaign)
        .await
        .unwrap();
    let screen = open_screen(t.addr, &tv).await.unwrap();
    (phone, screen)
}

/// Nothing but presence: no topic moved, no activity.
fn quiet(seen: &[String]) -> bool {
    !seen
        .iter()
        .any(|s| s.starts_with("changed:") || s.starts_with("activity"))
}

#[tokio::test]
async fn claudes_map_writes_reach_the_gm_and_no_player_or_screen() {
    let t = table(LiveConfig::default()).await;
    let (_, secret) = common::api_token(&t.router, &t.token, "Claude").await;
    let campaign = t.campaign.to_string();
    let r = call(
        &t.router,
        Some(&t.token),
        "POST",
        &format!("/api/campaigns/{campaign}/maps"),
        Some(json!({ "name": "La crypte", "width": 8, "height": 6 })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let map = r.body["data"]["map"]["id"].as_str().unwrap().to_string();
    let uri = format!("/api/campaigns/{campaign}/maps/{map}");
    let revision = |t: &common::live::Table| {
        let router = t.router.clone();
        let token = t.token.clone();
        let uri = uri.clone();
        async move {
            call(&router, Some(&token), "GET", &uri, None).await.body["data"]["revision"].clone()
        }
    };
    let wall = |x: i32| json!({ "op": "paint", "cells": [[x, 1]], "kind": { "terrain": "pierre", "wall": true } });

    let mut gm = t.connect().await;
    let (mut phone, mut screen) = phone_and_screen(&t).await;
    assert!(ready(&mut gm).await.get("desk").is_some());
    for ws in [&mut phone, &mut screen] {
        let versions = ready(ws).await;
        assert!(versions.get("desk").is_none(), "{versions}");
    }

    // Proposed: the GM hears of the batch, nobody else of anything.
    let r = call_with_token(
        &t.router,
        &secret,
        "POST",
        &format!("{uri}/ops"),
        Some(json!({ "revision": revision(&t).await, "ops": [wall(1)], "title": "Un mur" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    let v = sentinel(&t).await;
    let heard = seen_until_world(&mut gm, v).await;
    assert!(
        heard.contains(&"changed:proposals".to_string()),
        "{heard:?}"
    );
    for ws in [&mut phone, &mut screen] {
        let seen = seen_until_world(ws, v).await;
        assert!(quiet(&seen), "{seen:?}");
    }

    // With the hand, written directly: the GM hears the map and Claude's
    // activity, nobody else of anything.
    common::claude_hand(&t.router, &t.token, &campaign).await;
    let v = sentinel(&t).await;
    for ws in [&mut gm, &mut phone, &mut screen] {
        seen_until_world(ws, v).await;
    }
    let r = call_with_token(
        &t.router,
        &secret,
        "POST",
        &format!("{uri}/ops"),
        Some(json!({ "revision": revision(&t).await, "ops": [wall(2)] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let v = sentinel(&t).await;
    let heard = seen_until_world(&mut gm, v).await;
    assert!(heard.contains(&"changed:desk".to_string()), "{heard:?}");
    assert!(heard.contains(&"changed:prep".to_string()), "{heard:?}");
    assert!(heard.contains(&"activity:".to_string()), "{heard:?}");
    for ws in [&mut phone, &mut screen] {
        let seen = seen_until_world(ws, v).await;
        assert!(quiet(&seen), "{seen:?}");
    }
}
