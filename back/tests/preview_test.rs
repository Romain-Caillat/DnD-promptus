//! maps/preview-as-player — the Atlas's « Vue joueurs » is the players'
//! own projection: for the same state, the GM's preview of a map equals,
//! element by element, what the shared screen receives once that map is
//! shown — on both worlds, on the region in hexes and on a place. The
//! leak sweep of the marked table runs over it in
//! `player_routes_test.rs`.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_screen, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

struct World {
    yaml: &'static str,
    region: &'static str,
    place: &'static str,
    /// A party start of the place, where the NPC is set down near.
    start: &'static str,
    npc: &'static str,
}

const WORLDS: [World; 2] = [
    World {
        yaml: CORSAIRES,
        region: "cotes-bretagne-sud",
        place: "port-louis-rues",
        start: "pj-1",
        npc: "pnj_goulven",
    },
    World {
        yaml: BRASIER,
        region: "systeme-brasier",
        place: "abords-du-toboggan",
        start: "cure-dent",
        npc: "pnj_lumen",
    },
];

struct Table {
    app: Router,
    gm: String,
    campaign: String,
    tv: String,
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn preview(&self, map: &str) -> (bool, Value) {
        let r = self.gm("GET", &format!("/maps/{map}/players"), None).await;
        assert_eq!(r.status, StatusCode::OK, "{map}: {}", r.body);
        (
            r.body["data"]["shown"].as_bool().unwrap(),
            r.body["data"]["board"].clone(),
        )
    }

    async fn screen(&self) -> Value {
        let r = call_as_screen(&self.app, Some(&self.tv), "GET", "/api/tv/show", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"]["board"].clone()
    }

    async fn show(&self, map: &str) {
        let r = self.gm("POST", "/board", Some(json!({ "map": map }))).await;
        assert_eq!(r.status, StatusCode::OK, "{map}: {}", r.body);
    }
}

/// Romain's campaign of `w`, an NPC placed near the party's start of
/// the place, Marc seated with a validated character, a TV paired, the
/// evening started.
async fn table(pool: &PgPool, w: &World) -> Table {
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, w.yaml).await;
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("/api/campaigns/{campaign}/maps/{}/place", w.place),
        Some(json!({ "placements": [{ "what": "npc", "id": w.npc, "near": w.start }] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let marc = Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    sqlx::query(
        "UPDATE characters SET status = 'validated',
                sheet = '{\"name\":\"Borin\",\"classId\":\"bretteur\"}'
         WHERE player_id = $1",
    )
    .bind(marc)
    .execute(pool)
    .await
    .unwrap();
    let r = call_as_screen(&app, None, "POST", "/api/tv", None).await;
    let tv = r.screen_token().unwrap();
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("/api/campaigns/{campaign}/screens"),
        Some(json!({ "code": r.body["data"]["code"] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let t = Table {
        app,
        gm,
        campaign,
        tv,
    };
    assert_eq!(
        t.gm("POST", "/session", None).await.status,
        StatusCode::CREATED
    );
    t.gm("POST", "/session/start", None).await;
    t
}

#[tokio::test]
async fn the_gms_preview_of_a_map_is_what_the_screen_gets_once_it_is_shown() {
    let pool = common::test_pool().await;
    for w in &WORLDS {
        let t = table(&pool, w).await;
        for map in [w.region, w.place] {
            // Not on the table yet: the board a show would lay out…
            let (shown, ahead) = t.preview(map).await;
            assert!(!shown, "{map}");
            assert!(ahead.is_object(), "{map}: {ahead}");
            // …is the one the screen receives once the GM shows it.
            t.show(map).await;
            assert_eq!(ahead, t.screen().await, "{map}: preview before the show");
            let (shown, now) = t.preview(map).await;
            assert!(shown, "{map}");
            assert_eq!(now, t.screen().await, "{map}: preview of the map shown");
        }
        // The NPC placed on the place stands hidden: nobody sees it, the
        // GM's preview no more than the screen. Met, both see it.
        let npc = |board: &Value| {
            board["tokens"]
                .as_array()
                .unwrap()
                .iter()
                .any(|tk| tk["id"] == format!("prep-{}-0", w.npc))
        };
        let (_, board) = t.preview(w.place).await;
        assert!(!npc(&board), "{board}");
        let r = t
            .gm(
                "POST",
                "/session/reveal",
                Some(json!({ "kind": "npc", "npc": w.npc })),
            )
            .await;
        assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
        let (_, board) = t.preview(w.place).await;
        assert!(npc(&board), "{board}");
        assert_eq!(board, t.screen().await);
        // The region, away from the table now, comes back as the party
        // left it — the same board a show gives again.
        let (shown, ahead) = t.preview(w.region).await;
        assert!(!shown);
        t.show(w.region).await;
        assert_eq!(ahead, t.screen().await, "{}: back to the region", w.region);
    }
}

#[tokio::test]
async fn a_draft_map_has_no_players_view_and_another_gm_none_at_all() {
    let pool = common::test_pool().await;
    let t = table(&pool, &WORLDS[0]).await;
    let r = t
        .gm(
            "POST",
            "/maps",
            Some(json!({ "name": "Quai", "copy": "quai-port-louis" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let copy = r.body["data"]["map"]["id"].as_str().unwrap().to_string();
    let r = t.gm("GET", &format!("/maps/{copy}/players"), None).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "MAP_DRAFT");
    let r = t.gm("GET", "/maps/nulle-part/players", None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "NO_SUCH_MAP");
    let (_, other) = common::signed_in_gm(&pool, "Autre").await;
    let r = call(
        &t.app,
        Some(&other),
        "GET",
        &format!("/api/campaigns/{}/maps/port-louis-rues/players", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}
