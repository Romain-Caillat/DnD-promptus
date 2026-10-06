//! The grid at the table, over the API: player/explore-map,
//! maps/reveal-fog-and-hidden, gm/run-combat, player/fight-turn,
//! copilot/propose-adversary-turns, player/receive-rewards — on the
//! Corsaires' quay fight (`sc_quai`, `quai-port-louis`).

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");

struct Table {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
    marc: String,
    lea: String,
    borin: Uuid,
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
}

/// Romain's Corsaires, live, Marc playing Borin the bretteur, Léa
/// watching; the quay shown.
async fn quay() -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, CORSAIRES).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let marc_id = Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    let borin: Uuid = sqlx::query_scalar(
        "UPDATE characters SET status = 'validated',
                sheet = '{\"name\":\"Borin\",\"classId\":\"bretteur\"}'
         WHERE player_id = $1 RETURNING id",
    )
    .bind(marc_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let lea = join(&app, &code, "Léa", "spectator").await;
    let t = Table {
        app,
        pool,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
        lea: lea.player_token().unwrap(),
        borin,
    };
    assert_eq!(
        t.gm("POST", "/session", None).await.status,
        StatusCode::CREATED
    );
    t.gm("POST", "/session/start", None).await;
    let r = t
        .gm("POST", "/board", Some(json!({ "map": "quai-port-louis" })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    t
}

/// Text that only the GM's copy of the quay holds.
const GM_ONLY: [&str; 4] = [
    // The map's GM notes.
    "Combat tutoriel",
    // The hidden trapdoor, its check and its notes.
    "trappe-contrebande",
    "rhum de contrebande",
    // The ambush layer.
    "Embuscade de Gueule-Rouge",
];

fn assert_no_gm_text(what: &str, body: &Value) {
    let json = body.to_string();
    for t in GM_ONLY {
        assert!(!json.contains(t), "{what} leaked {t:?}: {json}");
    }
}

#[tokio::test]
async fn players_walk_under_the_fog_and_see_nothing_hidden() {
    let t = quay().await;
    let tok = format!("pc-{}", t.borin);

    let view = t.player(&t.marc, "GET", "/board", None).await.body;
    assert_no_gm_text("Marc's board", &view);
    let data = &view["data"];
    assert_eq!(data["fog"], true);
    // Borin stands on the first party start, and is the only token: the
    // ambushers are on a GM layer, not on the board yet.
    let tokens = data["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 1, "{tokens:?}");
    assert_eq!(tokens[0]["id"], tok);
    assert_eq!(tokens[0]["mine"], true);
    assert_eq!(tokens[0]["at"], json!([0, 5]));
    // La Mâchoire's deck is far beyond the fog's ten cells.
    assert!(
        !data["map"].to_string().contains("La Mâchoire"),
        "{}",
        data["map"]
    );
    let reach = data["reachable"].as_array().unwrap();
    assert!(reach.iter().any(|c| c["at"] == json!([1, 5])), "{reach:?}");

    // A path that jumps is refused, a step is walked, and the fog lifts
    // ahead.
    let r = t
        .player(
            &t.marc,
            "POST",
            "/board/walk",
            Some(json!({ "path": [[5, 5]] })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "INVALID_PATH", "{}", r.body);
    let path: Vec<Value> = (1..=6).map(|x| json!([x, 5])).collect();
    let r = t
        .player(
            &t.marc,
            "POST",
            "/board/walk",
            Some(json!({ "path": path })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(
        r.body["data"]["tokens"][0]["at"],
        json!([6, 5]),
        "{}",
        r.body
    );
    // Léa watches; she walks nobody.
    let r = t
        .player(
            &t.lea,
            "POST",
            "/board/walk",
            Some(json!({ "path": [[1, 5]] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // The trapdoor is found: the GM reveals it, and it reaches the
    // phones once its cell is out of the fog — without its check.
    let r = t
        .gm(
            "POST",
            "/board/edit",
            Some(json!({ "kind": "revealThing", "id": "trappe-contrebande" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    t.gm(
        "POST",
        "/board/edit",
        Some(json!({ "kind": "revealCells", "cells": [[10, 13]] })),
    )
    .await;
    let view = t.player(&t.lea, "GET", "/board", None).await.body;
    let objects = &view["data"]["map"]["objects"];
    assert_eq!(objects[0]["id"], "trappe-contrebande", "{objects}");
    assert!(objects[0].get("check").is_none(), "{objects}");
    assert!(objects[0].get("notes").is_none(), "{objects}");

    // Invisible, Borin is a ghost on Marc's phone and gone from Léa's.
    t.gm(
        "POST",
        "/board/edit",
        Some(json!({ "kind": "tokenState", "token": tok, "invisible": true })),
    )
    .await;
    let marc = t.player(&t.marc, "GET", "/board", None).await.body;
    assert_eq!(marc["data"]["tokens"][0]["ghost"], true, "{marc}");
    let lea = t.player(&t.lea, "GET", "/board", None).await.body;
    assert_eq!(lea["data"]["tokens"], json!([]), "{lea}");

    // The GM changes the weather live: the phones see it.
    t.gm(
        "POST",
        "/board/edit",
        Some(json!({ "kind": "ambience", "weather": "rain", "time": "dawn" })),
    )
    .await;
    let lea = t.player(&t.lea, "GET", "/board", None).await.body;
    assert_eq!(lea["data"]["map"]["ambience"]["weather"], "rain");
    assert_eq!(lea["data"]["map"]["ambience"]["time"], "dawn");
}

#[tokio::test]
async fn the_quay_fight_from_initiative_to_the_loot() {
    let t = quay().await;
    let tok = format!("pc-{}", t.borin);
    let r = t
        .gm("POST", "/fight", Some(json!({ "node": "sc_quai" })))
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let gm = &r.body["data"]["encounter"];
    let order = gm["fight"]["order"].as_array().unwrap();
    // Borin, Gueule-Rouge, four sailors and Fañch.
    assert_eq!(order.len(), 7, "{order:?}");
    // No second fight while this one lasts, and no walking either.
    let again = t
        .gm("POST", "/fight", Some(json!({ "node": "sc_quai" })))
        .await;
    assert_eq!(again.body["error"]["code"], "FIGHT_IN_PROGRESS");
    let r = t
        .player(
            &t.marc,
            "POST",
            "/board/walk",
            Some(json!({ "path": [[1, 5]] })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "FIGHT_IN_PROGRESS");

    // Players never see an opponent's hit points, nor the co-GM's plan.
    let mut proposals = 0;
    let mut marc_turns = 0;
    for _ in 0..40 {
        let board = t.gm("GET", "/board", None).await.body;
        let enc = &board["data"]["encounter"];
        if enc["live"] != true {
            break;
        }
        let view = t.player(&t.marc, "GET", "/board", None).await.body;
        assert_no_gm_text("Marc's fight", &view);
        let fight = &view["data"]["fight"];
        for f in fight["order"].as_array().unwrap() {
            if f["party"] == false {
                assert!(f["hitPoints"].is_null(), "{f}");
            }
        }
        for e in fight["events"].as_array().unwrap() {
            assert_ne!(e["event"]["event"], "for_the_gm", "{e}");
        }
        let active = enc["fight"]["scene"]["active"]
            .as_str()
            .unwrap()
            .to_string();
        if active == tok {
            marc_turns += 1;
            assert_eq!(fight["myTurn"], true);
            assert!(!fight["cards"].as_array().unwrap().is_empty(), "{fight}");
            // Léa cannot play for him.
            let r = t
                .player(&t.lea, "POST", "/fight", Some(json!({ "kind": "endTurn" })))
                .await;
            assert_eq!(r.status, StatusCode::FORBIDDEN);
            let r = t
                .player(
                    &t.marc,
                    "POST",
                    "/fight",
                    Some(json!({ "kind": "endTurn" })),
                )
                .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        } else {
            // Not Marc's turn: refused.
            let r = t
                .player(
                    &t.marc,
                    "POST",
                    "/fight",
                    Some(json!({ "kind": "endTurn" })),
                )
                .await;
            assert_eq!(r.body["error"]["code"], "NOT_YOUR_TURN", "{}", r.body);
            let r = t
                .gm("POST", "/fight/command", Some(json!({ "kind": "propose" })))
                .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
            let p = &r.body["data"]["encounter"]["proposal"];
            assert_eq!(p["who"], active, "{p}");
            assert!(!p["steps"].as_array().unwrap().is_empty(), "{p}");
            let view = t.player(&t.lea, "GET", "/board", None).await.body;
            assert!(!view.to_string().contains("\"steps\""), "{view}");
            let r = t
                .gm("POST", "/fight/command", Some(json!({ "kind": "accept" })))
                .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
            // Accepted once: a second accept has no proposal.
            let r = t
                .gm("POST", "/fight/command", Some(json!({ "kind": "accept" })))
                .await;
            assert_eq!(r.body["error"]["code"], "NO_PROPOSAL");
            proposals += 1;
        }
        if proposals >= 6 && marc_turns >= 1 {
            break;
        }
    }
    assert!(
        marc_turns >= 1 && proposals >= 1,
        "{marc_turns} {proposals}"
    );

    // A condition by hand, then the GM calls the fight.
    let r = t
        .gm(
            "POST",
            "/fight/command",
            Some(json!({ "kind": "condition", "who": tok, "condition": "apeure", "turns": 1, "remove": false })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = t
        .gm("POST", "/fight/command", Some(json!({ "kind": "stop" })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let enc = &r.body["data"]["encounter"];
    assert_eq!(enc["live"], false);

    // Borin's wounds followed him into his sheet in play.
    let hp_fight = enc["fight"]["scene"]["combatants"][&tok]["hit_points"]
        .as_i64()
        .unwrap();
    let me = t.player(&t.marc, "GET", "/me", None).await.body;
    assert_eq!(
        me["data"]["character"]["play"]["hitPoints"]
            .as_i64()
            .unwrap(),
        hp_fight,
        "{me}"
    );

    // The loot: the sailors' coins to Borin, the hidden trapdoor's
    // purse stays with the GM until handed out.
    let view = t.player(&t.marc, "GET", "/board", None).await.body;
    assert_eq!(view["data"]["fight"]["loot"], json!([]));
    assert!(
        !view.to_string().contains("trappe sous l'appontement"),
        "{view}"
    );
    let gold_before = me["data"]["character"]["play"]["resources"].to_string();
    let r = t
        .gm(
            "POST",
            "/fight/loot",
            Some(json!({ "gives": [{ "index": 0, "character": t.borin }] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let again = t
        .gm(
            "POST",
            "/fight/loot",
            Some(json!({ "gives": [{ "index": 0, "character": t.borin }] })),
        )
        .await;
    assert_eq!(again.body["error"]["code"], "ALREADY_GIVEN");
    let view = t.player(&t.marc, "GET", "/board", None).await.body;
    assert_eq!(view["data"]["fight"]["loot"][0]["toMe"], true, "{view}");
    let me = t.player(&t.marc, "GET", "/me", None).await.body;
    assert_ne!(
        me["data"]["character"]["play"]["resources"].to_string(),
        gold_before
    );
    let _ = &t.pool;
}
