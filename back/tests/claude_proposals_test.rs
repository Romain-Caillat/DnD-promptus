//! platform/propose-edits-by-claude — done when a write with the GM's
//! token never changes the campaign outside « Claude a la main » but
//! becomes a batch of proposals; only the GM's session keeps or discards
//! its items; keeping replays under the campaign lock and refuses what
//! no longer applies; Claude reads the GM's word back; the mode is the
//! GM's, refused while an evening is open and on what players know; and
//! nothing of a batch reaches a player or a shared screen.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::live::{next, open_player, open_screen, ready, table};
use common::{Reply, call, call_as_player, call_as_screen, call_with_token};
use promptus_back::live::LiveConfig;
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

struct Prep {
    pool: sqlx::PgPool,
    app: Router,
    gm: String,
    claude: String,
    campaign: String,
}

impl Prep {
    fn uri(&self, path: &str) -> String {
        format!("/api/campaigns/{}{path}", self.campaign)
    }

    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        call(&self.app, Some(&self.gm), method, &self.uri(path), body).await
    }

    async fn claude(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        call_with_token(&self.app, &self.claude, method, &self.uri(path), body).await
    }

    async fn story(&self) -> Value {
        self.gm("GET", "", None).await.body["data"]["story"].clone()
    }

    async fn npc_name(&self, id: &str) -> Value {
        let story = self.story().await;
        story["npcs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == id)
            .map_or(Value::Null, |n| n["name"].clone())
    }

    /// Claude's batches, as Claude reads them.
    async fn batches(&self) -> Vec<Value> {
        let r = self.claude("GET", "/proposals", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].as_array().unwrap().clone()
    }
}

async fn prep() -> Prep {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = common::imported_campaign(&app, &gm, FIXTURE).await;
    let (_, claude) = common::api_token(&app, &gm, "Claude").await;
    Prep {
        pool,
        app,
        gm,
        claude,
        campaign,
    }
}

fn rename(target: &str, name: &str) -> Value {
    json!({ "op": "set", "target": target, "field": "name", "value": name })
}

fn new_clue() -> Value {
    json!({ "op": "add", "kind": "clue", "value": {
        "id": "cl_lanterne", "revelation": "rev_crique", "node": "sc_port",
        "text": "Une lanterne cassée sur le quai."
    } })
}

#[tokio::test]
async fn claudes_edits_are_proposed_and_the_campaign_does_not_move() {
    let p = prep().await;
    let gwen = p.npc_name("pnj_gwen").await;
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({
                "title": "Peupler le port",
                "focus": { "map": "cotes-bretagne-sud", "at": [2, 4] },
                "edits": [rename("pnj_gwen", "Gwen la Rousse"), new_clue()],
            })),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "proposed");
    assert_eq!(r.body["data"]["why"], "proposals");
    let proposal = &r.body["data"]["proposal"];
    assert_eq!(proposal["author"], "token");
    assert_eq!(proposal["prompt"], "Peupler le port");
    assert_eq!(proposal["focus"]["map"], "cotes-bretagne-sud");
    assert_eq!(proposal["items"].as_array().unwrap().len(), 2);
    assert_eq!(proposal["items"][0]["change"]["after"], "Gwen la Rousse");

    // Nothing landed, no revision was taken.
    assert_eq!(p.npc_name("pnj_gwen").await, gwen);
    assert_eq!(p.gm("GET", "", None).await.body["data"]["revision"], 0);
    // An edit that does not fit is refused at once, nothing stored.
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({ "edits": [{ "op": "remove", "target": "nobody_here" }] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    assert_eq!(p.batches().await.len(), 1);
    // The co-GM's workshop does not list Claude's batches.
    let r = p.gm("GET", "/workshop", None).await;
    assert_eq!(r.body["data"], json!([]), "{}", r.body);
    // The atlas draws the batch, its place pinned virtually.
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(
                json!({ "edits": [{ "op": "set", "target": "lieu_taverne", "field": "anchor",
                "value": { "map": "cotes-bretagne-sud", "at": [2, 3] } }] }),
            ),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    let pinning = r.body["data"]["proposal"]["id"].clone();
    let atlas = p.gm("GET", "/atlas", None).await.body["data"].clone();
    assert_eq!(atlas["claude"].as_array().unwrap().len(), 2);
    let region = atlas["maps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["map"]["id"] == "cotes-bretagne-sud")
        .unwrap();
    let proposed = region["proposed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["proposal"] == pinning)
        .expect("the pin is drawn dotted");
    assert!(
        proposed["pins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pin| pin["location"] == "lieu_taverne"),
        "{proposed}"
    );
    assert!(
        region["pins"]
            .as_array()
            .unwrap()
            .iter()
            .all(|pin| pin["location"] != "lieu_taverne"),
        "the stored pins do not hold it"
    );
}

#[tokio::test]
async fn only_the_gm_keeps_and_claude_reads_the_word() {
    let p = prep().await;
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({ "edits": [rename("pnj_gwen", "Gwen la Rousse"), rename("pnj_loic", "Loïc le Muet")] })),
        )
        .await;
    let id = r.body["data"]["proposal"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let decide = format!("/proposals/{id}/decide");

    // A token never decides, nor gives itself the hand.
    let r = p
        .claude("POST", &decide, Some(json!({ "all": "keep" })))
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.body);
    let r = p
        .claude("PUT", "/claude", Some(json!({ "hand": true })))
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.body);

    // Claude adds to its own batch.
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({ "proposal": id, "edits": [new_clue()] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["data"]["why"], "batch");
    assert_eq!(
        r.body["data"]["proposal"]["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    // The GM keeps Gwen, discards Loïc with a word.
    let r = p
        .gm(
            "POST",
            &decide,
            Some(json!({ "keep": ["e0"], "discard": ["e1"], "word": "trop évident" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["proposal"]["status"], "pending");
    assert_eq!(p.npc_name("pnj_gwen").await, "Gwen la Rousse");
    assert_ne!(p.npc_name("pnj_loic").await, "Loïc le Muet");
    // Kept as a proposal the GM accepted: no « Claude écrit ».
    let r = p.gm("GET", "/revisions?since=0", None).await;
    assert_eq!(r.body["data"]["entries"][0]["author"], "copilot");
    // A decided item is not decided twice.
    let r = p.gm("POST", &decide, Some(json!({ "keep": ["e1"] }))).await;
    assert_eq!(
        r.body["error"]["code"], "ITEM_ALREADY_DECIDED",
        "{}",
        r.body
    );

    // Claude reads it back.
    let batch = p.batches().await.remove(0);
    assert_eq!(batch["items"][0]["status"], "kept");
    assert_eq!(batch["items"][1]["status"], "discarded");
    assert_eq!(batch["items"][1]["word"], "trop évident");
    assert_eq!(batch["items"][2]["status"], "pending");

    // Claude places the clue it added: kept alone, it waits on the clue.
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(
                json!({ "proposal": id, "edits": [{ "op": "set", "target": "cl_lanterne",
                "field": "anchor", "value": { "map": "cotes-bretagne-sud", "at": [2, 4] } }] }),
            ),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["data"]["proposal"]["items"][3]["stale"], false);
    let r = p.gm("POST", &decide, Some(json!({ "keep": ["e3"] }))).await;
    assert_eq!(
        r.body["error"]["code"], "PROPOSAL_ITEM_NEEDS_EARLIER",
        "{}",
        r.body
    );

    // Everything left kept, in order: the batch is settled.
    let r = p.gm("POST", &decide, Some(json!({ "all": "keep" }))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["proposal"]["status"], "accepted");
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({ "proposal": id, "edits": [rename("pnj_loic", "Loïc")] })),
        )
        .await;
    assert_eq!(
        r.body["error"]["code"], "PROPOSAL_ALREADY_DECIDED",
        "{}",
        r.body
    );
}

#[tokio::test]
async fn keeping_replays_on_the_campaign_as_it_is_and_refuses_what_no_longer_applies() {
    let p = prep().await;
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({ "edits": [rename("pnj_corentin", "Corentin le Borgne"), new_clue()] })),
        )
        .await;
    let id = r.body["data"]["proposal"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let decide = format!("/proposals/{id}/decide");
    // Meanwhile the GM removes Corentin.
    let r = p
        .gm(
            "POST",
            "/story/edits",
            Some(json!({ "edits": [{ "op": "remove", "target": "pnj_corentin" }] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let batch = p.batches().await.remove(0);
    assert_eq!(batch["items"][0]["stale"], true, "{batch}");
    assert_eq!(batch["items"][1]["stale"], false);

    // Keeping that one alone is refused, nothing written.
    let r = p
        .gm("POST", &decide, Some(json!({ "keep": ["e0", "e1"] })))
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "PROPOSAL_ITEM_STALE");
    let story = p.story().await;
    assert!(
        story["clues"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["id"] != "cl_lanterne")
    );

    // Keeping everything refuses it alone and keeps the clue.
    let r = p.gm("POST", &decide, Some(json!({ "all": "keep" }))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let items = &r.body["data"]["proposal"]["items"];
    assert_eq!(items[0]["status"], "refused");
    assert_eq!(items[0]["code"], "EDIT_UNKNOWN_TARGET");
    assert_eq!(items[1]["status"], "kept");
    let story = p.story().await;
    assert!(
        story["clues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "cl_lanterne")
    );
}

#[tokio::test]
async fn claudes_map_operations_are_proposed_then_kept() {
    let p = prep().await;
    let r = p
        .gm(
            "POST",
            "/maps",
            Some(json!({ "name": "La crypte", "width": 8, "height": 6 })),
        )
        .await;
    let map = r.body["data"]["map"]["id"].as_str().unwrap().to_string();
    let read = p.gm("GET", &format!("/maps/{map}"), None).await.body["data"].clone();
    let wall = json!({ "op": "paint", "cells": [[1, 1], [2, 1]], "kind": { "terrain": "pierre", "wall": true } });
    let r = p
        .claude(
            "POST",
            &format!("/maps/{map}/ops"),
            Some(json!({ "revision": read["revision"], "ops": [wall], "title": "Un mur" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    let proposal = r.body["data"]["proposal"].clone();
    assert_eq!(proposal["focus"]["map"], map.as_str());
    assert_eq!(proposal["items"][0]["map"]["name"], "La crypte");
    let after = p.gm("GET", &format!("/maps/{map}"), None).await.body["data"].clone();
    assert_eq!(after["map"], read["map"], "the map did not move");

    // The atlas shows the cells it touches.
    let atlas = p.gm("GET", "/atlas", None).await.body["data"].clone();
    let crypt = atlas["maps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["map"]["id"] == map.as_str())
        .unwrap();
    assert_eq!(
        crypt["proposed"][0]["cells"].as_array().unwrap().len(),
        2,
        "{crypt}"
    );

    let id = proposal["id"].as_str().unwrap();
    let r = p
        .gm(
            "POST",
            &format!("/proposals/{id}/decide"),
            Some(json!({ "keep": ["m0"] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["proposal"]["status"], "accepted");
    let kept = p.gm("GET", &format!("/maps/{map}"), None).await.body["data"].clone();
    assert_ne!(kept["map"], read["map"], "the wall is painted");
}

#[tokio::test]
async fn the_hand_is_the_gms_and_never_during_an_evening_nor_on_what_players_know() {
    let p = prep().await;
    // Without the hand, a whole file is refused.
    let r = p
        .claude("PUT", "/import", Some(json!({ "yaml": FIXTURE })))
        .await;
    assert_eq!(r.body["error"]["code"], "CLAUDE_PROPOSES", "{}", r.body);

    // With it, Claude's writes land directly, in the journal as its own.
    common::claude_hand(&p.app, &p.gm, &p.campaign).await;
    assert_eq!(p.gm("GET", "", None).await.body["data"]["claudeHand"], true);
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({ "edits": [rename("pnj_gwen", "Gwen la Rousse")] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "applied");
    assert_eq!(p.npc_name("pnj_gwen").await, "Gwen la Rousse");
    let r = p
        .claude("PUT", "/import", Some(json!({ "yaml": FIXTURE })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    // What players know is still proposed: the table met Loïc.
    sqlx::query(
        "UPDATE campaigns SET world = jsonb_set(world, '{revealed}', '[\"pnj_loic\"]') WHERE id = $1",
    )
    .bind(uuid::Uuid::parse_str(&p.campaign).unwrap())
    .execute(&p.pool)
    .await
    .unwrap();
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({ "edits": [rename("pnj_loic", "Loïc le Muet")] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["data"]["why"], "revealed");

    // The hook players read is theirs already: proposed.
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({ "edits": [{ "op": "set", "target": "bible", "field": "player_hook", "value": "Autre accroche." }] })),
        )
        .await;
    assert_eq!(r.body["data"]["why"], "revealed", "{}", r.body);

    // An evening opens: writes are proposed again, the hand cannot be
    // given, taking it back still works.
    let r = p.gm("POST", "/session", None).await;
    assert!(r.status.is_success(), "{}", r.body);
    let gwen = p.npc_name("pnj_gwen").await;
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({ "edits": [rename("pnj_gwen", "Gwen")] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["data"]["why"], "evening");
    assert_eq!(p.npc_name("pnj_gwen").await, gwen);
    let r = p.gm("PUT", "/claude", Some(json!({ "hand": false }))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = p.gm("PUT", "/claude", Some(json!({ "hand": true }))).await;
    assert_eq!(r.body["error"]["code"], "EVENING_OPEN", "{}", r.body);
}

/// Read `ws` up to the world change `version` (the sentinel), and return
/// every message type and topic seen on the way.
async fn seen_until_world(ws: &mut common::live::Socket, version: i64) -> Vec<String> {
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
async fn nothing_of_a_batch_reaches_a_player_or_a_screen() {
    let t = table(LiveConfig::default()).await;
    let campaign = t.campaign.to_string();
    let (_, secret) = common::api_token(&t.router, &t.token, "Claude").await;
    let (_, device) = t.seat("Marc").await;
    let r = call_as_screen(&t.router, None, "POST", "/api/tv", None).await;
    let tv = r.screen_token().unwrap();
    let code = r.body["data"]["code"].as_str().unwrap().to_string();
    let r = call(
        &t.router,
        Some(&t.token),
        "POST",
        &format!("/api/campaigns/{campaign}/screens"),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let view = format!("/api/play/{campaign}/view");
    let before = call_as_player(&t.router, Some(&device), "GET", &view, None).await;

    let mut gm = t.connect().await;
    let mut phone = open_player(t.addr, Some(&device), t.campaign)
        .await
        .unwrap();
    let mut screen = open_screen(t.addr, &tv).await.unwrap();
    for ws in [&mut gm, &mut phone, &mut screen] {
        ready(ws).await;
    }

    // Claude proposes, the GM discards; then the sentinel.
    let r = call_with_token(
        &t.router,
        &secret,
        "POST",
        &format!("/api/campaigns/{campaign}/story/edits"),
        Some(json!({ "edits": [rename("pnj_gwen", "Gwen la Rousse")] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    let id = r.body["data"]["proposal"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = call(
        &t.router,
        Some(&t.token),
        "POST",
        &format!("/api/campaigns/{campaign}/proposals/{id}/decide"),
        Some(json!({ "all": "discard", "word": "Non." })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    t.bump().await;

    let heard = seen_until_world(&mut gm, 1).await;
    assert!(
        heard.contains(&"changed:proposals".to_string()),
        "{heard:?}"
    );
    for ws in [&mut phone, &mut screen] {
        let seen = seen_until_world(ws, 1).await;
        assert!(
            seen.iter().all(|s| s == "presence:"),
            "a player or a screen heard of the batch: {seen:?}"
        );
    }
    let after = call_as_player(&t.router, Some(&device), "GET", &view, None).await;
    assert_eq!(after.body, before.body, "the player's view did not move");
}
