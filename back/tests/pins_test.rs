//! campaign/pin-scenes-on-maps — a scene stands on a map through its
//! place's anchor: the GM pins a place by a story edit (a line of the
//! preparation journal), the atlas shows each map's pins, and at the
//! table players see only the pins of places the table went to.

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
    gm: String,
    campaign: String,
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn atlas(&self) -> Value {
        let r = self.gm("GET", "/atlas", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }
}

async fn corsaires() -> (Table, PgPool) {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, CORSAIRES).await;
    (Table { app, gm, campaign }, pool)
}

fn pins_of<'a>(atlas: &'a Value, map: &str) -> &'a Vec<Value> {
    atlas["maps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["map"]["id"] == map)
        .unwrap_or_else(|| panic!("no map {map}"))["pins"]
        .as_array()
        .unwrap()
}

/// (place, [(scene, number)]) of each pin.
fn summary(pins: &[Value]) -> Vec<(String, Vec<(String, u64)>)> {
    pins.iter()
        .map(|p| {
            (
                p["location"].as_str().unwrap().to_string(),
                p["scenes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| {
                        (
                            s["id"].as_str().unwrap().to_string(),
                            s["number"].as_u64().unwrap(),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

#[tokio::test]
async fn the_gm_pins_a_place_by_a_story_edit_and_the_atlas_shows_it() {
    let (t, _) = corsaires().await;
    let atlas = t.atlas().await;
    // The region holds Port-Louis (the act's nine scenes, through their
    // places inside it) and Belle-Île; the town map, each place of it —
    // three scenes share the tavern, numbered as in the campaign.
    let region = summary(pins_of(&atlas, "cotes-bretagne-sud"));
    assert_eq!(
        region
            .iter()
            .map(|(l, s)| (l.as_str(), s.len()))
            .collect::<Vec<_>>(),
        [("lieu_port_louis", 9), ("lieu_belle_ile", 1)]
    );
    let town = summary(pins_of(&atlas, "port-louis-rues"));
    assert_eq!(
        town[0],
        (
            "lieu_perroquet".to_string(),
            vec![
                ("sc_perroquet".to_string(), 1),
                ("sc_proposition".to_string(), 2),
                ("sc_anais".to_string(), 6),
            ]
        )
    );
    assert_eq!(atlas["unplaced"], json!([]));
    let before = atlas["revision"].as_i64().unwrap();

    // Unpinned, the interception stands nowhere — allowed, and said.
    let unpin = json!({ "edits": [
        { "op": "set", "target": "lieu_belle_ile", "field": "anchor", "value": null }
    ] });
    let r = t.gm("POST", "/story/edits", Some(unpin)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let atlas = t.atlas().await;
    assert_eq!(
        atlas["unplaced"],
        json!([{
            "id": "sc_interception_greyhound",
            "title": "L'interception du Greyhound",
            "number": 10,
            "map": "pont-du-greyhound"
        }])
    );
    assert_eq!(pins_of(&atlas, "cotes-bretagne-sud").len(), 1);

    // Pinned again elsewhere; a pin off the grid is reported, not refused.
    let pin = json!({ "edits": [
        { "op": "set", "target": "lieu_belle_ile", "field": "anchor",
          "value": { "map": "cotes-bretagne-sud", "at": [7, 10] } },
        { "op": "set", "target": "lieu_chantier", "field": "anchor",
          "value": { "map": "port-louis-rues", "at": [60, 2] } }
    ] });
    let r = t.gm("POST", "/story/edits", Some(pin)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let atlas = t.atlas().await;
    let belle_ile = &pins_of(&atlas, "cotes-bretagne-sud")[1];
    assert_eq!(belle_ile["at"], json!([7, 10]));
    let issues: Vec<(&str, &str)> = atlas["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| (i["code"].as_str().unwrap(), i["path"].as_str().unwrap()))
        .collect();
    assert_eq!(issues, [("ANCHOR_OFF_GRID", "locations[4].anchor.at")]);

    // Each pin edit is a line of the GM's journal, on the places touched.
    let r = t
        .gm("GET", &format!("/revisions?since={before}"), None)
        .await;
    let entries = r.body["data"]["entries"].as_array().unwrap().clone();
    assert_eq!(entries.len(), 2, "{entries:#?}");
    assert!(entries.iter().all(|e| e["author"] == "gm"));
    let touched: Vec<Vec<&str>> = entries
        .iter()
        .map(|e| {
            let mut ids: Vec<&str> = e["changes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["id"].as_str().unwrap())
                .collect();
            ids.sort_unstable();
            ids
        })
        .collect();
    assert_eq!(
        touched,
        [
            vec!["lieu_belle_ile"],
            vec!["lieu_belle_ile", "lieu_chantier"]
        ]
    );
}

#[tokio::test]
async fn players_see_only_the_pins_of_places_the_table_went_to() {
    let (t, pool) = corsaires().await;
    let code = invite_code(&t.app, &t.gm, &t.campaign).await;
    let marc = join(&t.app, &code, "Marc", "player").await;
    let marc_id = Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    sqlx::query(
        "UPDATE characters SET status = 'validated',
                sheet = '{\"name\":\"Borin\",\"classId\":\"bretteur\"}'
         WHERE player_id = $1",
    )
    .bind(marc_id)
    .execute(&pool)
    .await
    .unwrap();
    let marc = marc.player_token().unwrap();
    let board = || async {
        let uri = format!("/api/play/{}/board", t.campaign);
        call_as_player(&t.app, Some(&marc), "GET", &uri, None)
            .await
            .body["data"]
            .clone()
    };
    assert_eq!(
        t.gm("POST", "/session", None).await.status,
        StatusCode::CREATED
    );
    t.gm("POST", "/session/start", None).await;
    let r = t
        .gm("POST", "/board", Some(json!({ "map": "port-louis-rues" })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = t
        .gm(
            "POST",
            "/board/edit",
            Some(json!({ "kind": "fog", "enabled": false })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    // The GM's board holds every pin of the map, to launch a scene.
    let gm = t.gm("GET", "/board", None).await.body["data"].clone();
    assert_eq!(gm["pins"].as_array().unwrap().len(), 7, "{}", gm["pins"]);
    // The table has been nowhere yet.
    assert_eq!(board().await["pins"], json!([]));

    let scene = |node: &str| json!({ "kind": "scene", "node": node });
    let r = t
        .gm("POST", "/session/reveal", Some(scene("sc_perroquet")))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let pins = board().await["pins"].clone();
    // A name and a cell: nothing of the three scenes the tavern holds.
    assert_eq!(
        pins,
        json!([{ "name": "Le Perroquet Noyé", "at": [2, 1], "current": true }])
    );

    // Under the fog, a place entered but out of sight keeps its pin hidden.
    let r = t
        .gm(
            "POST",
            "/board/edit",
            Some(json!({ "kind": "fog", "enabled": true })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = t
        .gm("POST", "/session/reveal", Some(scene("sc_chantier")))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let pins = board().await["pins"].clone();
    assert!(
        !pins.to_string().contains("chantier"),
        "the shipyard is far in the fog: {pins}"
    );
}
