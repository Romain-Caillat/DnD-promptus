//! platform/edit-maps-by-claude — done when Claude, with the GM's
//! token, reads the atlas and a map as text, asks the engine on names,
//! and places an NPC « near » something and clues « along » a path:
//! the server picks free, dry, reachable cells — never a wall, never
//! water — and the placements are proposed to the GM like any write of
//! Claude's, the campaign unchanged until the GM keeps them.

mod common;

use std::collections::BTreeSet;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_with_token};
use promptus_shared::maps::{Cell, Map, Water};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");
const STREETS: &str = include_str!("../../content/maps/corsaires/port-louis-rues.yaml");

struct Prep {
    app: Router,
    gm: String,
    claude: String,
    campaign: String,
}

impl Prep {
    fn uri(&self, path: &str) -> String {
        format!("/api/campaigns/{}{path}", self.campaign)
    }

    async fn claude(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        call_with_token(&self.app, &self.claude, method, &self.uri(path), body).await
    }

    async fn story(&self) -> Value {
        call(&self.app, Some(&self.gm), "GET", &self.uri(""), None)
            .await
            .body["data"]["story"]
            .clone()
    }
}

async fn prep() -> Prep {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = common::imported_campaign(&app, &gm, FIXTURE).await;
    let (_, claude) = common::api_token(&app, &gm, "Claude").await;
    Prep {
        app,
        gm,
        claude,
        campaign,
    }
}

fn cell(v: &Value) -> Cell {
    serde_json::from_value(v.clone()).unwrap()
}

/// Where a creature can be put: no wall, no void, no water.
fn dry(map: &Map, at: Cell) -> bool {
    map.grid
        .kind(at)
        .is_some_and(|k| !k.wall && !k.void && k.water == Water::None)
}

#[tokio::test]
async fn claude_reads_the_atlas_and_a_world_map_as_text() {
    let p = prep().await;
    let atlas = p.claude("GET", "/atlas", None).await;
    assert_eq!(atlas.status, StatusCode::OK, "{}", atlas.body);
    assert!(
        atlas.body["data"]["maps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["map"]["id"] == "port-louis-rues")
    );

    // A map of the world, not a copy: read by its id.
    let s = p.claude("GET", "/maps/port-louis-rues/sketch", None).await;
    assert_eq!(s.status, StatusCode::OK, "{}", s.body);
    let rows = s.body["data"]["rows"].as_array().unwrap();
    assert_eq!(rows[1], "    0123456789012345678901");
    // The tavern's label is a symbol over its cell, listed in the legend.
    let label = s.body["data"]["legend"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["name"] == "Le Perroquet Noyé")
        .unwrap()
        .clone();
    let symbol = label["symbol"].as_str().unwrap();
    assert_eq!(&rows[3].as_str().unwrap()[6..7], symbol);
    // A zone cuts it.
    let z = p
        .claude(
            "GET",
            "/maps/port-louis-rues/sketch?x=10&y=10&w=5&h=2",
            None,
        )
        .await;
    assert_eq!(z.body["data"]["rows"].as_array().unwrap().len(), 4);
    assert_eq!(z.body["data"]["cut"], true);

    let missing = p.claude("GET", "/maps/valombre/sketch", None).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(missing.body["error"]["code"], "NO_SUCH_MAP");
}

#[tokio::test]
async fn the_engine_answers_on_names_and_says_what_a_name_could_be() {
    let p = prep().await;
    let r = p
        .claude(
            "POST",
            "/maps/port-louis-rues/ask",
            Some(json!({ "question": "path", "from": "pj-1", "to": "le quai de la mâchoire" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(r.body["data"]["cost"].as_u64().unwrap() >= 15);
    assert!(r.body["data"]["meters"].as_f64().unwrap() >= 75.0);

    let r = p
        .claude(
            "POST",
            "/maps/port-louis-rues/ask",
            Some(json!({ "question": "view", "from": "La cour de Kerjean" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(r.body["data"]["seen"].as_u64().unwrap() > 0);

    let r = p
        .claude(
            "POST",
            "/maps/port-louis-rues/ask",
            Some(json!({ "question": "sight", "from": "pj-1", "to": "le cimetière" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "MAP_REF_UNKNOWN");
    assert!(
        r.body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Perroquet")
    );
}

#[tokio::test]
async fn claudes_placements_land_on_free_dry_reachable_cells_and_are_proposed() {
    let p = prep().await;
    let before = p.story().await;
    let r = p
        .claude(
            "POST",
            "/maps/port-louis-rues/place",
            Some(json!({
                "title": "Peupler le quartier",
                "placements": [
                    { "what": "npc", "id": "pnj_gwen", "near": "Le Perroquet Noyé" },
                    { "what": "clue", "id": "cl_gwen", "along": { "from": "pj-1", "to": "vers-le-quai" } },
                    { "what": "clue", "id": "cl_cordages", "along": { "from": "pj-1", "to": "vers-le-quai" } },
                ],
            })),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "proposed");
    let proposal = &r.body["data"]["proposal"];
    assert_eq!(proposal["prompt"], "Peupler le quartier");
    assert_eq!(proposal["focus"]["map"], "port-louis-rues");
    // One NPC edit and two clue anchors, nothing else.
    assert_eq!(proposal["items"].as_array().unwrap().len(), 3);

    let map = Map::from_yaml(STREETS).unwrap();
    let placed = r.body["data"]["placed"].as_array().unwrap();
    let cells: BTreeSet<Cell> = placed.iter().map(|x| cell(&x["at"])).collect();
    assert_eq!(cells.len(), 3, "{placed:?}");
    for at in &cells {
        assert!(dry(&map, *at), "{at:?} is a wall or water");
    }
    // Gwen stands in the tavern, by its label.
    let gwen = cell(&placed[0]["at"]);
    assert!(gwen.x <= 4 && gwen.y <= 3, "{gwen:?}");
    // The clues are on the walk to the quay, the first nearer the start.
    assert!(cell(&placed[1]["at"]).x < cell(&placed[2]["at"]).x);

    // Nothing landed until the GM keeps it.
    assert_eq!(p.story().await, before);
    let atlas = p.claude("GET", "/atlas", None).await;
    let streets = atlas.body["data"]["maps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["map"]["id"] == "port-louis-rues")
        .unwrap()
        .clone();
    assert_eq!(
        streets["proposed"][0]["cast"]["clues"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        streets["proposed"][0]["cast"]["npcs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // A second request does not land on what is still proposed.
    let again = p
        .claude(
            "POST",
            "/maps/port-louis-rues/place",
            Some(json!({ "placements": [
                { "what": "npc", "id": "pnj_loic", "near": "Le Perroquet Noyé" }
            ] })),
        )
        .await;
    assert_eq!(again.status, StatusCode::ACCEPTED, "{}", again.body);
    assert_ne!(cell(&again.body["data"]["placed"][0]["at"]), gwen);

    let id = proposal["id"].as_str().unwrap();
    let kept = call(
        &p.app,
        Some(&p.gm),
        "POST",
        &p.uri(&format!("/proposals/{id}/decide")),
        Some(json!({ "all": "keep" })),
    )
    .await;
    assert_eq!(kept.status, StatusCode::OK, "{}", kept.body);
    let story = p.story().await;
    let spot = story["npcs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "pnj_gwen")
        .unwrap()["spots"][0]
        .clone();
    assert_eq!(spot["map"], "port-louis-rues");
    assert_eq!(cell(&spot["at"]), gwen);
}

#[tokio::test]
async fn a_placement_that_cannot_be_made_names_why_and_proposes_nothing() {
    let p = prep().await;
    let r = p
        .claude(
            "POST",
            "/maps/port-louis-rues/place",
            Some(json!({ "placements": [
                { "what": "npc", "id": "pnj_oswin", "near": "Le Perroquet Noyé" }
            ] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "PLACE_UNKNOWN_ENTITY");
    let batches = p.claude("GET", "/proposals", None).await;
    assert!(batches.body["data"].as_array().unwrap().is_empty());
}
