//! campaign/import-pixellab-assets — Claude brings PixelLab images in,
//! pending the GM's word, and never regenerates what the theme holds.

mod common;

use axum::http::StatusCode;
use base64::Engine as _;
use common::{call, call_with_token};
use promptus_shared::elements::{Rgba, read_png, write_png};
use serde_json::json;

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

fn data_url(pixels: Vec<u8>, width: u32, height: u32) -> String {
    let bytes = write_png(&Rgba {
        width,
        height,
        pixels,
    })
    .unwrap();
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

#[tokio::test]
async fn claude_lists_what_the_quay_lacks_and_brings_one_element_in_pending() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, session) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = common::imported_campaign(&app, &session, FIXTURE).await;
    let (_, token) = common::api_token(&app, &session, "Claude").await;

    // What the quay of the Corsaires' world needs: its floors, walls and props.
    let r = call_with_token(
        &app,
        &token,
        "GET",
        &format!("/api/campaigns/{campaign}/elements?map=quai-port-louis"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let needs = r.body["data"]["needs"].as_array().unwrap();
    assert_eq!(r.body["data"]["tileset"], "port-1718");
    assert!(r.body["data"]["palette"].as_array().unwrap().len() > 10);
    let paves = needs
        .iter()
        .find(|n| n["subject"] == "port-1718/material/pavés")
        .expect("the cobbles are needed");
    assert_eq!(paves["status"], "missing");
    assert_eq!(paves["form"], "texture");
    let total = r.body["data"]["generations"].as_u64().unwrap();
    assert_eq!(
        total,
        needs
            .iter()
            .map(|n| n["generations"].as_u64().unwrap())
            .sum::<u64>()
    );

    // A bright red texture comes back onto the theme's palette, pending.
    let url = data_url([250, 0, 0, 255].repeat(4), 2, 2);
    let r = call_with_token(
        &app,
        &token,
        "POST",
        &format!("/api/campaigns/{campaign}/media/import"),
        Some(json!({
            "kind": "element", "subject": "port-1718/material/pavés", "url": url,
            "description": "pavés mouillés, vue de face", "generations": 1
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "pending");
    assert_eq!(r.body["data"]["source"], "pixellab");
    assert_eq!(r.body["data"]["meta"]["form"], "texture");
    let id = r.body["data"]["id"].as_str().unwrap().to_string();
    let stored: Vec<u8> = sqlx::query_scalar("SELECT image FROM media_assets WHERE id = $1::uuid")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let px = read_png(&stored).unwrap().pixels;
    assert_ne!(
        &px[..3],
        &[250, 0, 0],
        "the colour was brought onto the palette"
    );

    // Never generated twice: the same slot is refused, unless replaced.
    let again = json!({ "kind": "element", "subject": "port-1718/material/pavés", "url": url });
    let r = call_with_token(
        &app,
        &token,
        "POST",
        &format!("/api/campaigns/{campaign}/media/import"),
        Some(again.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "ELEMENT_EXISTS");
    let r = call_with_token(
        &app,
        &token,
        "GET",
        &format!("/api/campaigns/{campaign}/elements?map=quai-port-louis"),
        None,
    )
    .await;
    let paves = r.body["data"]["needs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["subject"] == "port-1718/material/pavés")
        .unwrap()
        .clone();
    assert_eq!(
        (paves["status"].as_str(), paves["generations"].as_u64()),
        (Some("pending"), Some(0))
    );

    // The GM keeps it, in Promptus: the token never decides.
    let r = call_with_token(
        &app,
        &token,
        "POST",
        &format!("/api/campaigns/{campaign}/media/{id}/decision"),
        Some(json!({ "approve": true })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = call(
        &app,
        Some(&session),
        "POST",
        &format!("/api/campaigns/{campaign}/media/{id}/decision"),
        Some(json!({ "approve": true })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);

    // Replacing waits for the GM again, the kept one stays until then.
    let mut replace = again;
    replace["replace"] = json!(true);
    let r = call_with_token(
        &app,
        &token,
        "POST",
        &format!("/api/campaigns/{campaign}/media/import"),
        Some(replace),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let kept: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM media_assets WHERE id = $1::uuid AND status = 'approved'",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(kept, 1);
}

#[tokio::test]
async fn a_wrong_slot_or_address_brings_nothing_in() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, session) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = common::imported_campaign(&app, &session, FIXTURE).await;
    let uri = format!("/api/campaigns/{campaign}/media/import");
    let tiny = data_url(vec![0, 0, 0, 255], 1, 1);
    let cases = [
        (
            json!({ "kind": "element", "subject": "port-1718/prop/caisse", "url": tiny }),
            "ELEMENT_UNKNOWN",
        ),
        (
            json!({ "kind": "element", "subject": "port-1718/material/lave", "url": tiny }),
            "ELEMENT_UNKNOWN",
        ),
        (
            json!({ "kind": "element", "subject": "port-1718/wall", "url": "https://example.com/x.png" }),
            "IMAGE_URL_REFUSED",
        ),
        (
            json!({ "kind": "element", "subject": "port-1718/wall", "url": "data:image/png;base64,aGVsbG8=" }),
            "IMAGE_INVALID",
        ),
        (
            json!({ "kind": "element", "subject": "port-1718/wall" }),
            "IMPORT_MALFORMED",
        ),
        (
            json!({ "kind": "scene", "subject": "sc_crique", "url": tiny }),
            "IMPORT_MALFORMED",
        ),
        // No scene draws a facade yet: nothing to generate for it.
        (
            json!({ "kind": "element", "subject": "port-1718/facade/taverne", "url": tiny }),
            "ELEMENT_UNKNOWN",
        ),
        (
            json!({ "kind": "character", "subject": "pnj_gwen", "frames": [
                { "url": tiny, "direction": "south", "index": 3_000_000_000_u64 },
            ] }),
            "BAD_FRAME",
        ),
    ];
    for (body, code) in cases {
        let r = call(&app, Some(&session), "POST", &uri, Some(body.clone())).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body}: {}", r.body);
        assert_eq!(r.body["error"]["code"], code, "{body}");
    }
    let r = call(
        &app,
        Some(&session),
        "POST",
        &uri,
        Some(json!({ "kind": "character", "subject": "pnj_personne", "frames": [{ "url": tiny, "direction": "south" }] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM media_assets WHERE campaign_id = $1::uuid")
            .bind(&campaign)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn an_npc_arrives_with_its_frames_and_stays_hidden_from_players() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, session) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = common::imported_campaign(&app, &session, FIXTURE).await;
    let frame = |r: u8| data_url(vec![r, 10, 10, 255], 1, 1);
    let r = call(
        &app,
        Some(&session),
        "POST",
        &format!("/api/campaigns/{campaign}/media/import"),
        Some(
            json!({ "kind": "character", "subject": "pnj_gwen", "frames": [
            { "url": frame(10), "direction": "east", "animation": "walk", "index": 1 },
            { "url": frame(20), "direction": "south" },
            { "url": frame(30), "direction": "east", "animation": "walk", "index": 0 },
        ] }),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let asset = &r.body["data"];
    assert_eq!(asset["meta"]["form"], "character");
    assert_eq!(asset["meta"]["name"], "Gwen Le Bihan");
    let frames = asset["meta"]["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 3);
    // Ordered by animation, direction, index: idle first, then the walk.
    assert_eq!(frames[0]["animation"], "idle");
    assert_eq!(
        (frames[1]["index"].as_u64(), frames[2]["index"].as_u64()),
        (Some(0), Some(1))
    );
    let id = asset["id"].as_str().unwrap();
    // The figure shown is resting, facing us; frame 1 is the walk's first.
    let r = call(
        &app,
        Some(&session),
        "GET",
        &format!("/api/campaigns/{campaign}/media/{id}/image"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let r = call(
        &app,
        Some(&session),
        "GET",
        &format!("/api/campaigns/{campaign}/media/{id}/frames/1"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let r = call(
        &app,
        Some(&session),
        "GET",
        &format!("/api/campaigns/{campaign}/media/{id}/frames/3"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // Kept, but Gwen not met yet: no player sees her figure.
    let r = call(
        &app,
        Some(&session),
        "POST",
        &format!("/api/campaigns/{campaign}/media/{id}/decision"),
        Some(json!({ "approve": true })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let code = common::invite_code(&app, &session, &campaign).await;
    let player = common::join(&app, &code, "Marc", "player").await;
    let token = player.player_token().unwrap();
    let r = common::call_as_player(
        &app,
        Some(&token),
        "GET",
        &format!("/api/play/{campaign}/media"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(
        r.body["data"]["assets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["id"] != id),
        "{}",
        r.body
    );
}

#[tokio::test]
async fn frames_are_served_in_the_order_meta_numbers_them() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, session) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = common::imported_campaign(&app, &session, FIXTURE).await;
    let frame = |r: u8| data_url(vec![r, 10, 10, 255], 1, 1);
    // `_` sorts before `a` in bytes, after it in most collations.
    let r = call(
        &app,
        Some(&session),
        "POST",
        &format!("/api/campaigns/{campaign}/media/import"),
        Some(
            json!({ "kind": "character", "subject": "pnj_gwen", "frames": [
            { "url": frame(40), "direction": "south", "animation": "walka" },
            { "url": frame(80), "direction": "south", "animation": "walk_b" },
        ] }),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let meta = &r.body["data"]["meta"]["frames"];
    assert_eq!(meta[0]["animation"], "walk_b");
    let id = r.body["data"]["id"].as_str().unwrap();
    let r = call(
        &app,
        Some(&session),
        "GET",
        &format!("/api/campaigns/{campaign}/media/{id}/frames/0"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let stored: Vec<u8> = sqlx::query_scalar(
        "SELECT image FROM media_frames WHERE asset_id = $1::uuid AND animation = 'walk_b'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(read_png(&stored).unwrap().pixels[0], 80);
    let served = sqlx::query_scalar::<_, Vec<u8>>(
        r#"SELECT image FROM media_frames WHERE asset_id = $1::uuid
           ORDER BY animation COLLATE "C", direction COLLATE "C", idx LIMIT 1"#,
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(served, stored);
}

#[tokio::test]
async fn a_fight_on_the_map_brings_its_opponents_figures_into_the_needs() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, session) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = common::imported_campaign(&app, &session, FIXTURE).await;
    let r = call(
        &app,
        Some(&session),
        "POST",
        &format!("/api/campaigns/{campaign}/story/edits"),
        Some(
            json!({ "edits": [{ "op": "set", "target": "sc_crique", "field": "encounter.anchor",
            "value": { "map": "quai-port-louis", "at": [12, 6] } }] }),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = call(
        &app,
        Some(&session),
        "GET",
        &format!("/api/campaigns/{campaign}/elements?map=quai-port-louis"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let figures: Vec<&str> = r.body["data"]["needs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["kind"] == "character")
        .map(|n| n["subject"].as_str().unwrap())
        .collect();
    assert!(figures.contains(&"adv_contrebandier"), "{figures:?}");
    assert!(figures.contains(&"pnj_corentin"), "{figures:?}");
}
