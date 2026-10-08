//! characters/walk-in-four-directions over the API, on both witness
//! worlds: a token carries its look, turns toward where it walks and
//! what it acts on, and keeps its last move for every screen to walk —
//! cut to the cells a player sees.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use promptus_shared::sprite::CharacterLook;
use serde_json::{Value, json};
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");
const LOOKS_CORSAIRES: &str = include_str!("../../content/sprites/looks/corsaires.yaml");
const LOOKS_BRASIER: &str = include_str!("../../content/sprites/looks/brasier.yaml");

/// What a world's walk test needs.
struct World {
    campaign: &'static str,
    looks: &'static str,
    map: &'static str,
    node: &'static str,
    class: &'static str,
    /// The character's first walk, from the first party start, and the
    /// way it then faces.
    walk: Vec<[i32; 2]>,
    facing: &'static str,
    /// An NPC set down in the fog, then moved by the GM into sight.
    npc: &'static str,
    hidden_cell: [i32; 2],
    seen_cell: [i32; 2],
    npc_facing: &'static str,
    /// Where the GM sets the character down before the fight, so the
    /// opponents reach it (the Vorr stay in their airlock otherwise).
    fight_cell: Option<[i32; 2]>,
}

fn corsaires() -> World {
    World {
        campaign: CORSAIRES,
        looks: LOOKS_CORSAIRES,
        map: "quai-port-louis",
        node: "sc_quai",
        class: "bretteur",
        walk: vec![[1, 5], [2, 5], [2, 4]],
        facing: "north",
        npc: "gueule-rouge",
        hidden_cell: [22, 5],
        seen_cell: [4, 5],
        npc_facing: "west",
        fight_cell: None,
    }
}

fn brasier() -> World {
    World {
        campaign: BRASIER,
        looks: LOOKS_BRASIER,
        map: "cure-dent-coursive",
        node: "sc_toboggan",
        class: "pilote",
        walk: vec![[6, 4], [6, 5], [6, 6]],
        facing: "south",
        npc: "abordeur-vorr",
        hidden_cell: [1, 9],
        seen_cell: [7, 6],
        npc_facing: "east",
        fight_cell: Some([4, 10]),
    }
}

/// A world's look book entry, as the API writes a look.
fn book_look(w: &World, id: &str) -> Value {
    let book: serde_yaml_ng::Value = serde_yaml_ng::from_str(w.looks).unwrap();
    let entry = ["party", "foes"]
        .iter()
        .flat_map(|k| book[*k].as_sequence().unwrap())
        .find(|e| e["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("no look {id}"));
    let look: CharacterLook = serde_yaml_ng::from_value(entry["look"].clone()).unwrap();
    serde_json::to_value(look).unwrap()
}

struct Table {
    app: Router,
    gm: String,
    campaign: String,
    marc: String,
    lea: String,
    token: String,
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

    async fn board_of(&self, token: &str) -> Value {
        let r = self.player(token, "GET", "/board", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }
}

fn token<'a>(board: &'a Value, id: &str) -> Option<&'a Value> {
    board["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == id)
}

/// The world's campaign, live, Marc's character validated with the
/// first party look of the world, Léa watching, the map shown.
async fn table(w: &World) -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, w.campaign).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let marc_id = Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    let book: serde_yaml_ng::Value = serde_yaml_ng::from_str(w.looks).unwrap();
    let first = book["party"][0]["id"].as_str().unwrap().to_string();
    let sheet = json!({ "name": "Marc", "classId": w.class, "look": book_look(w, &first) });
    let character: Uuid = sqlx::query_scalar(
        "UPDATE characters SET status = 'validated', sheet = $2 WHERE player_id = $1 RETURNING id",
    )
    .bind(marc_id)
    .bind(&sheet)
    .fetch_one(&pool)
    .await
    .unwrap();
    let lea = join(&app, &code, "Léa", "spectator").await;
    let t = Table {
        app,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
        lea: lea.player_token().unwrap(),
        token: format!("pc-{character}"),
    };
    t.gm("POST", "/session", None).await;
    t.gm("POST", "/session/start", None).await;
    let r = t.gm("POST", "/board", Some(json!({ "map": w.map }))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    t
}

async fn walks_and_turns(w: World) {
    let t = table(&w).await;
    let book: serde_yaml_ng::Value = serde_yaml_ng::from_str(w.looks).unwrap();
    let first = book["party"][0]["id"].as_str().unwrap().to_string();

    // On the map, the character is drawn from its sheet, at rest facing
    // east, with no move yet.
    let board = t.board_of(&t.marc).await;
    let me = token(&board, &t.token).unwrap();
    assert_eq!(me["look"], book_look(&w, &first), "{me}");
    assert_eq!(me["facing"], "east");
    assert_eq!(me["moves"], 0);
    assert_eq!(me["trail"], json!([]));
    let start = me["at"].clone();

    // Marc walks: every screen gets the path and the way he now faces.
    let r = t
        .player(
            &t.marc,
            "POST",
            "/board/walk",
            Some(json!({ "path": w.walk })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let mut trail = vec![start];
    trail.extend(w.walk.iter().map(|c| json!(c)));
    for who in [&t.marc, &t.lea] {
        let board = t.board_of(who).await;
        let me = token(&board, &t.token).unwrap();
        assert_eq!(me["facing"], w.facing, "{me}");
        assert_eq!(me["trail"], json!(trail), "{me}");
        assert_eq!(me["moves"], 1, "{me}");
    }
    let gm = t.gm("GET", "/board", None).await.body;
    let mine = gm["data"]["board"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tk| tk["id"] == t.token)
        .unwrap();
    assert_eq!(mine["facing"], w.facing, "{mine}");

    // An NPC set down in the fog then moved into sight: the phones see it
    // arrive, never the fogged cell it came from.
    let r = t
        .gm(
            "POST",
            "/board/edit",
            Some(json!({ "kind": "placeNpc", "npc": w.npc, "at": w.hidden_cell, "hidden": false })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let npc_id = format!("{}-1", w.npc);
    assert!(token(&t.board_of(&t.lea).await, &npc_id).is_none());
    let r = t
        .gm(
            "POST",
            "/board/edit",
            Some(json!({ "kind": "moveToken", "token": npc_id, "at": w.seen_cell })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let board = t.board_of(&t.lea).await;
    let npc = token(&board, &npc_id).unwrap_or_else(|| panic!("{board}"));
    assert_eq!(npc["look"], book_look(&w, w.npc), "{npc}");
    assert_eq!(npc["facing"], w.npc_facing, "{npc}");
    assert_eq!(npc["moves"], 1);
    assert_eq!(
        npc["trail"],
        json!([w.seen_cell]),
        "the fogged origin leaked: {npc}"
    );
    // The GM's own board keeps the whole move.
    let gm = t.gm("GET", "/board", None).await.body;
    let full = gm["data"]["board"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tk| tk["id"] == npc_id)
        .unwrap();
    assert_eq!(full["trail"], json!([w.hidden_cell, w.seen_cell]));
    t.gm(
        "POST",
        "/board/edit",
        Some(json!({ "kind": "removeToken", "token": npc_id })),
    )
    .await;

    // The fight: opponents come with their look (a numbered copy reads
    // as its kind) and walk; whoever strikes turns toward the target.
    if let Some(cell) = w.fight_cell {
        let r = t
            .gm(
                "POST",
                "/board/edit",
                Some(json!({ "kind": "moveToken", "token": t.token, "at": cell })),
            )
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }
    let r = t
        .gm("POST", "/fight", Some(json!({ "node": w.node })))
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let (mut walked, mut turned) = (false, false);
    for _ in 0..30 {
        let gm = t.gm("GET", "/board", None).await.body;
        let enc = &gm["data"]["encounter"];
        if enc["live"] != true || (walked && turned) {
            break;
        }
        let seen = enc["events"].as_array().unwrap().len();
        let active = enc["fight"]["scene"]["active"]
            .as_str()
            .unwrap()
            .to_string();
        if active == t.token {
            // Marc strikes an opponent in reach with his first card, once.
            let view = t.board_of(&t.marc).await;
            let fight = &view["fight"];
            let me = token(&view, &t.token).unwrap();
            let (mx, my) = (me["at"][0].as_i64().unwrap(), me["at"][1].as_i64().unwrap());
            let strike = fight["cards"].as_array().unwrap().iter().find_map(|c| {
                if c["locked"] == true || c["target"] != "enemy" {
                    return None;
                }
                let range = c["range"].as_i64().unwrap().max(1);
                fight["order"].as_array().unwrap().iter().find_map(|f| {
                    let (x, y) = (f["at"][0].as_i64()?, f["at"][1].as_i64()?);
                    let close = (x - mx).abs().max((y - my).abs()) <= range;
                    (f["party"] == false && f["down"] == false && close)
                        .then(|| (c["id"].clone(), f["id"].clone()))
                })
            });
            let acted = match (turned, strike) {
                (false, Some((action, target))) => {
                    let r = t
                        .player(
                            &t.marc,
                            "POST",
                            "/fight",
                            Some(json!({ "kind": "act", "action": action, "targets": [target] })),
                        )
                        .await;
                    r.status == StatusCode::OK
                }
                _ => false,
            };
            t.player(
                &t.marc,
                "POST",
                "/fight",
                Some(json!({ "kind": "endTurn" })),
            )
            .await;
            if !acted {
                continue;
            }
        } else {
            t.gm("POST", "/fight/command", Some(json!({ "kind": "propose" })))
                .await;
            let r = t
                .gm("POST", "/fight/command", Some(json!({ "kind": "accept" })))
                .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        }
        let gm = t.gm("GET", "/board", None).await.body;
        let tokens = gm["data"]["board"]["tokens"].as_array().unwrap().clone();
        let at = |id: &str| {
            tokens
                .iter()
                .find(|tk| tk["id"] == id)
                .map(|tk| (tk["at"][0].as_i64().unwrap(), tk["at"][1].as_i64().unwrap()))
        };
        let new = gm["data"]["encounter"]["events"].as_array().unwrap()[seen..].to_vec();
        for (i, e) in new.iter().enumerate() {
            let who = e["who"].as_str().unwrap_or_default();
            let me = tokens.iter().find(|tk| tk["id"] == who);
            if e["kind"] == "moved" && !who.starts_with("pc-") {
                let me = me.unwrap();
                assert_eq!(
                    me["trail"].as_array().unwrap().last(),
                    Some(&me["at"]),
                    "{me}"
                );
                let base = who.rsplit_once('-').map_or(who, |(b, n)| {
                    if n.bytes().all(|c| c.is_ascii_digit()) {
                        b
                    } else {
                        who
                    }
                });
                assert_eq!(me["look"], book_look(&w, base), "{me}");
                walked = true;
            }
            // An act on a target, not followed by a move of the same actor.
            let moved_after = new[i + 1..]
                .iter()
                .any(|n| n["kind"] == "moved" && n["who"] == e["who"]);
            if e["kind"] == "acted" && !moved_after {
                let (Some(me), Some(target)) = (at(who), e["targets"][0].as_str().and_then(at))
                else {
                    continue;
                };
                let (dx, dy) = (target.0 - me.0, target.1 - me.1);
                let want = if dx.abs() >= dy.abs() {
                    if dx > 0 { "east" } else { "west" }
                } else if dy > 0 {
                    "south"
                } else {
                    "north"
                };
                let facing = &tokens.iter().find(|tk| tk["id"] == who).unwrap()["facing"];
                assert_eq!(facing, want, "{e}");
                turned = true;
            }
        }
    }
    assert!(walked, "no opponent walked");
    assert!(turned, "nobody struck a target");
}

#[tokio::test]
async fn the_corsaires_walk_and_turn_on_every_screen() {
    walks_and_turns(corsaires()).await;
}

#[tokio::test]
async fn the_brasier_crew_walks_and_turns_on_every_screen() {
    walks_and_turns(brasier()).await;
}
