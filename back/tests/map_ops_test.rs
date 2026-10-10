//! maps/edit-maps-by-ops — the editor and Claude (a GM's personal token)
//! change a map by batches of operations on the revision they read: two
//! writers on different cells both keep their change, a stale batch on
//! the same cell is refused with the fresh map, never written over, and
//! each batch is a line of the campaign's one preparation journal, which
//! takes it back.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_with_token};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

struct Prep {
    app: Router,
    gm: String,
    claude: String,
    campaign: String,
    map: String,
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

    fn ops(&self) -> String {
        format!("/maps/{}/ops", self.map)
    }

    /// One batch by the GM (`gm`) or Claude on `rev`, which becomes the
    /// revision it wrote.
    async fn send(&self, gm: bool, rev: &mut Value, ops: Value) -> i64 {
        let body = batch(rev, ops);
        let r = if gm {
            self.gm("POST", &self.ops(), body).await
        } else {
            self.claude("POST", &self.ops(), body).await
        };
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        *rev = r.body["data"]["revision"].clone();
        rev.as_i64().unwrap()
    }

    async fn read(&self) -> Value {
        self.gm("GET", &format!("/maps/{}", self.map), None)
            .await
            .body["data"]
            .clone()
    }
}

async fn prep() -> Prep {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = common::imported_campaign(&app, &gm, FIXTURE).await;
    let (_, claude) = common::api_token(&app, &gm, "Claude").await;
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("/api/campaigns/{campaign}/maps"),
        Some(json!({ "name": "La crypte", "width": 8, "height": 6 })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let map = r.body["data"]["map"]["id"].as_str().unwrap().to_string();
    Prep {
        app,
        gm,
        claude,
        campaign,
        map,
    }
}

fn wall(cells: Value) -> Value {
    json!({ "op": "paint", "cells": cells, "kind": { "terrain": "pierre", "wall": true } })
}

fn npc(at: [i32; 2]) -> Value {
    json!({
        "op": "place",
        "at": at,
        "thing": { "kind": "start", "side": "neutral", "entity": "pnj_gwen" },
    })
}

fn batch(revision: &Value, ops: Value) -> Option<Value> {
    Some(json!({ "revision": revision, "ops": ops }))
}

#[tokio::test]
async fn the_gm_paints_a_wall_while_claude_places_an_npc_and_both_stay() {
    let p = prep().await;
    let read = p.read().await["revision"].clone();

    // Both read the map at the same revision.
    let r = p
        .gm(
            "POST",
            &p.ops(),
            batch(&read, json!([wall(json!([[2, 2], [3, 2]]))])),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = p
        .claude("POST", &p.ops(), batch(&read, json!([npc([5, 3])])))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    let now = p.read().await;
    let grid = &now["map"]["grid"];
    let row: Vec<char> = grid["rows"][2].as_str().unwrap().chars().collect();
    for x in [2, 3] {
        assert_eq!(grid["legend"][row[x].to_string()]["wall"], true, "{grid}");
    }
    assert_eq!(now["map"]["starts"][0]["at"], json!([5, 3]));
    assert_eq!(now["map"]["starts"][0]["entity"], "pnj_gwen");

    // Two lines of the one journal, each with its author and what it touched.
    let since = read.as_i64().unwrap();
    let r = p
        .gm("GET", &format!("/revisions?since={since}"), None)
        .await;
    let entries = r.body["data"]["entries"].as_array().unwrap().clone();
    let target = format!("map:{}", p.map);
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_eq!(entries[0]["author"], "gm");
    assert_eq!(entries[0]["target"], target.as_str());
    assert_eq!(entries[0]["changes"][0]["kind"], "cells");
    assert_eq!(entries[0]["changes"][0]["name"], "La crypte");
    assert_eq!(entries[1]["author"], "token");
    assert_eq!(entries[1]["changes"][0]["kind"], "start");
    assert_eq!(entries[1]["changes"][0]["op"], "add");
    assert_eq!(r.body["data"]["revision"], now["revision"]);
}

#[tokio::test]
async fn a_stale_batch_on_the_same_cell_is_refused_with_the_fresh_map() {
    let p = prep().await;
    let read = p.read().await["revision"].clone();
    let r = p
        .claude("POST", &p.ops(), batch(&read, json!([npc([4, 3])])))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    // The GM, still on the old revision, walls the very cell.
    let r = p
        .gm(
            "POST",
            &p.ops(),
            batch(&read, json!([wall(json!([[4, 3]]))])),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "MAP_STALE");
    let fresh = &r.body["error"]["fresh"];
    assert_eq!(fresh["map"]["starts"][0]["at"], json!([4, 3]));
    assert!(fresh["revision"].as_i64().unwrap() > read.as_i64().unwrap());
    // Nothing was written over.
    let now = p.read().await;
    assert_eq!(now["map"]["starts"][0]["at"], json!([4, 3]));
    assert_eq!(now["revision"], fresh["revision"]);

    // Replayed on the fresh revision, the batch is the GM's to make — and
    // walling a cell where someone stands is refused in the GM's words.
    let r = p
        .gm(
            "POST",
            &p.ops(),
            batch(&fresh["revision"], json!([wall(json!([[4, 3]]))])),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "MAP_INVALID", "{}", r.body);
    let r = p
        .gm(
            "POST",
            &p.ops(),
            batch(&fresh["revision"], json!([wall(json!([[1, 1]]))])),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    // A revision from the future is stale too.
    let r = p
        .gm(
            "POST",
            &p.ops(),
            batch(&json!(9999), json!([wall(json!([[2, 1]]))])),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "MAP_STALE");
}

#[tokio::test]
async fn a_map_batch_is_taken_back_through_the_journal_until_a_later_one_covers_it() {
    let p = prep().await;
    let read = p.read().await["revision"].clone();
    let r = p
        .claude("POST", &p.ops(), batch(&read, json!([npc([2, 2])])))
        .await;
    let placed = r.body["data"]["revision"].as_i64().unwrap();
    // A story edit in between neither blocks nor is blocked.
    let r = p
        .claude(
            "POST",
            "/story/edits",
            Some(json!({ "edits": [{ "op": "set", "target": "pnj_gwen", "field": "name", "value": "Gwen" }] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    // A token never takes a revision back; the GM does.
    let undo = |n: i64| format!("/revisions/{n}/undo");
    let r = p.claude("POST", &undo(placed), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = p.gm("POST", &undo(placed), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let now = p.read().await;
    assert_eq!(now["map"]["starts"], Value::Null, "{now}");
    assert_eq!(now["validatedAt"], Value::Null);

    // Two GM batches on different cells, then Claude elsewhere: the GM
    // takes back the newer, then the older; Claude's NPC stays.
    let mut rev = now["revision"].clone();
    let a = p.send(true, &mut rev, json!([wall(json!([[1, 1]]))])).await;
    let b = p.send(true, &mut rev, json!([wall(json!([[6, 4]]))])).await;
    p.send(false, &mut rev, json!([npc([4, 3])])).await;
    for n in [b, a] {
        let r = p.gm("POST", &undo(n), None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }
    let back = p.read().await;
    assert_eq!(back["map"]["grid"], now["map"]["grid"]);
    assert_eq!(back["map"]["starts"][0]["at"], json!([4, 3]));

    // A later batch on the same cell covers the GM's.
    rev = back["revision"].clone();
    let e = p.send(true, &mut rev, json!([wall(json!([[5, 1]]))])).await;
    let f = p
        .send(false, &mut rev, json!([{ "op": "door", "at": [5, 1] }]))
        .await;
    let r = p.gm("POST", &undo(e), None).await;
    assert_eq!(r.body["error"]["code"], "REVISION_COVERED");

    // The journal reads story and map lines alike, and says which undo.
    let r = p.gm("GET", "/revisions?since=0", None).await;
    let entries = r.body["data"]["entries"].as_array().unwrap();
    let line = |n: i64| entries.iter().find(|e| e["revision"] == n).unwrap();
    assert_eq!(line(placed)["undone"], true);
    assert_eq!(line(a)["undone"], true);
    assert_eq!(line(b)["undone"], true);
    assert_eq!(line(e)["undoable"], false);
    assert_eq!(line(f)["undoable"], true);
    assert!(
        entries
            .iter()
            .any(|e| e["target"] == "story" && e["changes"][0]["kind"] == "npc")
    );
    assert!(
        entries
            .iter()
            .any(|e| e["changes"][0]["kind"] == "map" && e["changes"][0]["op"] == "add")
    );
}

#[tokio::test]
async fn the_whole_map_is_no_longer_written_and_a_bad_batch_changes_nothing() {
    let p = prep().await;
    let before = p.read().await;
    let r = p
        .gm(
            "PUT",
            &format!("/maps/{}", p.map),
            Some(before["map"].clone()),
        )
        .await;
    assert_eq!(r.status, StatusCode::METHOD_NOT_ALLOWED);

    let rev = &before["revision"];
    let r = p.gm("POST", &p.ops(), batch(rev, json!([]))).await;
    assert_eq!(r.body["error"]["code"], "BAD_BATCH");
    // All or nothing: the wall lands only with the move, which fails.
    let r = p
        .gm(
            "POST",
            &p.ops(),
            batch(
                rev,
                json!([wall(json!([[3, 3]])), { "op": "move", "id": "nope", "to": [1, 1] }]),
            ),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "MAP_OP_INVALID", "{}", r.body);
    assert_eq!(p.read().await, before);
}
