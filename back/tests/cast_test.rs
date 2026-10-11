//! campaign/place-cast-and-clues — the GM places NPCs, clues and fights
//! on the maps by story edits (lines of the preparation journal); the
//! atlas shows them; at the table, showing the map sets the NPCs down
//! hidden, and players see an NPC or a clue only once the GM revealed it.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const TOWN: &str = "port-louis-rues";

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

    async fn edit(&self, edits: Value) {
        let r = self
            .gm("POST", "/story/edits", Some(json!({ "edits": edits })))
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }

    async fn reveal(&self, body: Value) {
        let r = self.gm("POST", "/session/reveal", Some(body)).await;
        assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    }

    async fn gm_board(&self) -> Value {
        self.gm("GET", "/board", None).await.body["data"].clone()
    }
}

async fn corsaires() -> (Table, sqlx::PgPool) {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, CORSAIRES).await;
    (Table { app, gm, campaign }, pool)
}

fn at(map: &str, x: i32, y: i32) -> Value {
    json!({ "map": map, "at": [x, y] })
}

#[tokio::test]
async fn the_gm_places_npcs_clues_and_a_fight_and_the_atlas_shows_them() {
    let (t, _) = corsaires().await;
    let before = t.gm("GET", "/atlas", None).await.body["data"]["revision"]
        .as_i64()
        .unwrap();
    t.edit(json!([
        { "op": "set", "target": "pnj_goulven", "field": "spots", "value": [
            at(TOWN, 2, 1),
            { "map": TOWN, "at": [3, 2], "scene": "sc_anais" }
        ] },
        { "op": "set", "target": "cl_route_goulven", "field": "anchor", "value": at(TOWN, 3, 1) },
        // On a wall: reported, never refused.
        { "op": "set", "target": "cl_route_rapport", "field": "anchor", "value": at(TOWN, 0, 0) },
        { "op": "set", "target": "sc_quai", "field": "encounter.anchor", "value": at(TOWN, 10, 5) }
    ]))
    .await;

    let atlas = t.gm("GET", "/atlas", None).await.body["data"].clone();
    let town = atlas["maps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["map"]["id"] == TOWN)
        .unwrap();
    let cast = &town["cast"];
    assert_eq!(
        cast["npcs"],
        json!([
            { "npc": "pnj_goulven", "name": "Marguerite Goulven", "index": 0, "at": [2, 1] },
            { "npc": "pnj_goulven", "name": "Marguerite Goulven", "index": 1, "at": [3, 2], "scene": "sc_anais" }
        ])
    );
    let clues: Vec<(&str, &Value)> = cast["clues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["clue"].as_str().unwrap(), &c["at"]))
        .collect();
    assert_eq!(
        clues,
        [
            ("cl_route_goulven", &json!([3, 1])),
            ("cl_route_rapport", &json!([0, 0]))
        ]
    );
    // The fight opens its battle plan, which already places opponents.
    let fight = &cast["fights"][0];
    assert_eq!(fight["scene"], "sc_quai");
    assert_eq!(fight["map"], "quai-port-louis");
    let (placed, opponents) = (
        fight["placed"].as_u64().unwrap(),
        fight["opponents"].as_u64().unwrap(),
    );
    assert!(placed > 0 && placed <= opponents, "{fight}");
    let issues: Vec<(&str, &str)> = atlas["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| (i["code"].as_str().unwrap(), i["path"].as_str().unwrap()))
        .collect();
    assert_eq!(issues, [("SPOT_ON_WALL", "clues[1].anchor.at")]);
    // What the placement tool chooses from.
    assert!(
        atlas["castable"]["fights"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["scene"] == "sc_quai" && f["anchor"] == at(TOWN, 10, 5))
    );

    // One line of the GM's journal, on the entities placed.
    let r = t
        .gm("GET", &format!("/revisions?since={before}"), None)
        .await;
    let entries = r.body["data"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "{entries:#?}");
    let mut ids: Vec<&str> = entries[0]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        [
            "cl_route_goulven",
            "cl_route_rapport",
            "pnj_goulven",
            "sc_quai"
        ]
    );
}

#[tokio::test]
async fn the_evening_sets_placed_npcs_down_hidden_and_reveals_them_one_by_one() {
    let (t, pool) = corsaires().await;
    t.edit(json!([
        { "op": "set", "target": "pnj_goulven", "field": "spots", "value": [at(TOWN, 2, 1)] },
        { "op": "set", "target": "pnj_jacquot", "field": "spots", "value": [
            { "map": TOWN, "at": [3, 2], "scene": "sc_proposition" }
        ] },
        { "op": "set", "target": "cl_route_goulven", "field": "anchor", "value": at(TOWN, 3, 1) }
    ]))
    .await;
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
    let r = t.gm("POST", "/board", Some(json!({ "map": TOWN }))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = t
        .gm(
            "POST",
            "/board/edit",
            Some(json!({ "kind": "fog", "enabled": false })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    // Showing the map set Goulven down where she was placed, hidden; the
    // innkeeper's boy waits for his scene.
    let gm = t.gm_board().await;
    let npcs: Vec<(&str, &Value, bool)> = gm["board"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|tk| tk["kind"] == "npc")
        .map(|tk| {
            (
                tk["id"].as_str().unwrap(),
                &tk["at"],
                tk["hidden"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(npcs, [("prep-pnj_goulven-0", &json!([2, 1]), true)]);
    assert_eq!(gm["cast"]["clues"][0]["clue"], "cl_route_goulven");
    assert_eq!(gm["cast"]["found"], json!([]));
    // Players see neither her nor the clue.
    let seen = board().await;
    assert!(!seen.to_string().contains("Goulven"), "{seen}");
    assert_eq!(seen["clues"], json!([]));

    // Met: she comes out of hiding, on her cell.
    t.reveal(json!({ "kind": "npc", "npc": "pnj_goulven" }))
        .await;
    let seen = board().await;
    let tokens = seen["tokens"].as_array().unwrap();
    let goulven = tokens
        .iter()
        .find(|tk| tk["name"] == "Marguerite Goulven")
        .unwrap_or_else(|| panic!("{seen}"));
    assert_eq!(goulven["at"], json!([2, 1]));
    assert_eq!(seen["clues"], json!([]));

    // Found: the clue lies on its cell — its text, never its id.
    t.reveal(json!({ "kind": "clue", "clue": "cl_route_goulven" }))
        .await;
    let seen = board().await;
    assert_eq!(seen["clues"].as_array().unwrap().len(), 1);
    assert_eq!(seen["clues"][0]["at"], json!([3, 1]));
    assert!(!seen.to_string().contains("cl_route_goulven"), "{seen}");
    assert_eq!(
        t.gm_board().await["cast"]["found"],
        json!(["cl_route_goulven"])
    );

    // Off to the quay and back: the town starts fresh, and Goulven, met
    // already, stands there in the open.
    for map in ["quai-port-louis", TOWN] {
        let r = t.gm("POST", "/board", Some(json!({ "map": map }))).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }
    let gm = t.gm_board().await;
    let goulven = gm["board"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tk| tk["id"] == "prep-pnj_goulven-0")
        .unwrap_or_else(|| panic!("{}", gm["board"]));
    assert_eq!(goulven["hidden"], false);

    // Jacquot's scene opens: he joins the map shown, hidden.
    t.reveal(json!({ "kind": "scene", "node": "sc_proposition" }))
        .await;
    let gm = t.gm_board().await;
    let jacquot = gm["board"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tk| tk["id"] == "prep-pnj_jacquot-0")
        .unwrap_or_else(|| panic!("{}", gm["board"]));
    assert_eq!(jacquot["at"], json!([3, 2]));
    assert_eq!(jacquot["hidden"], true);
    assert!(!board().await.to_string().contains("Jacquot"));

    // The preparation never moves what is already on the table.
    t.edit(json!([
        { "op": "set", "target": "pnj_goulven", "field": "spots", "value": [at(TOWN, 4, 1)] }
    ]))
    .await;
    let gm = t.gm_board().await;
    let goulven = gm["board"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tk| tk["id"] == "prep-pnj_goulven-0")
        .unwrap();
    assert_eq!(goulven["at"], json!([2, 1]));
}
