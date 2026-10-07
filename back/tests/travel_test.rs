//! maps/travel-hex-world over the API, on both witness worlds: the
//! Corsaires sail from Port-Louis to Le Palais on Belle-Île, the Brasier
//! crosses its star system to the Sereth station. The seven moments of
//! the « Voyager » board — two routes, the vote, the portions, an event,
//! a group check, the night's watch, the arrival that opens the place's
//! map — played by a GM, two players and a spectator; and nothing the GM
//! alone may read reaches a phone.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use promptus_shared::travel::Guide;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

struct World {
    story: &'static str,
    guide: &'static str,
    map: &'static str,
    /// Where the demo journey goes, and the map it opens.
    to: [i32; 2],
    place_map: &'static str,
    /// The scene the place opens, when it names one.
    scene: Option<&'static str>,
    classes: [&'static str; 2],
    /// GM-only text of the world map (its notes, a secret place).
    secrets: [&'static str; 2],
}

const CORSAIRES: World = World {
    story: include_str!("../../content/campaigns/corsaires/campagne.yaml"),
    guide: include_str!("../../content/travel/corsaires/cotes-bretagne-sud.yaml"),
    map: "cotes-bretagne-sud",
    to: [9, 9],
    place_map: "le-palais",
    scene: None,
    classes: ["bretteur", "vigie"],
    secrets: ["Greyhound quitte Belle-Île", "Mouillage de la frégate"],
};

const BRASIER: World = World {
    story: include_str!("../../content/campaigns/brasier/campagne.yaml"),
    guide: include_str!("../../content/travel/brasier/systeme-brasier.yaml"),
    map: "systeme-brasier",
    to: [11, 7],
    place_map: "reliquaire-sereth",
    scene: Some("sc_carapace_sereth"),
    classes: ["pilote", "mecano"],
    secrets: ["La balise Vorr émet", "Avant-poste Vorr"],
};

struct Table {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
    marc: String,
    camille: String,
    lea: String,
    borin: Uuid,
    world: &'static World,
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn travel(&self, body: Value) -> Reply {
        self.gm("POST", "/travel", Some(body)).await
    }

    async fn player(&self, token: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/play/{}{path}", self.campaign);
        call_as_player(&self.app, Some(token), method, &uri, body).await
    }

    /// Every GM-only text a phone must never receive.
    fn gm_only(&self) -> Vec<String> {
        let guide = Guide::from_yaml(self.world.guide).unwrap();
        let mut out: Vec<String> = guide
            .terrains
            .values()
            .flat_map(|t| t.events.iter())
            .map(|e| e.gm_notes.clone())
            .filter(|n| !n.is_empty())
            .collect();
        out.extend(self.world.secrets.iter().map(|s| (*s).to_string()));
        out
    }

    /// What Marc, Camille and Léa receive on the map and the journey
    /// carries nothing in `also` nor any GM-only text.
    async fn assert_phones_clean(&self, also: &[String]) {
        let mut forbidden = self.gm_only();
        forbidden.extend(also.iter().cloned());
        for (who, token) in [
            ("Marc", &self.marc),
            ("Camille", &self.camille),
            ("Léa", &self.lea),
        ] {
            for path in ["/travel", "/board"] {
                let r = self.player(token, "GET", path, None).await;
                assert_eq!(r.status, StatusCode::OK, "{who} {path}");
                let json = r.body.to_string();
                for f in &forbidden {
                    assert!(
                        !json.contains(f.as_str()),
                        "{who} {path} leaked {f:?}: {json}"
                    );
                }
            }
        }
    }
}

/// Romain's campaign in `world`, live: Marc and Camille play, Léa
/// watches; the world map shown.
async fn table(world: &'static World) -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, world.story).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let mut seats = Vec::new();
    for (nick, (name, class)) in ["Marc", "Camille"]
        .iter()
        .zip([("Borin", world.classes[0]), ("Lyra", world.classes[1])])
    {
        let r = join(&app, &code, nick, "player").await;
        let id = Uuid::parse_str(r.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
        let character: Uuid = sqlx::query_scalar(
            "UPDATE characters SET status = 'validated', sheet = $2 WHERE player_id = $1 RETURNING id",
        )
        .bind(id)
        .bind(json!({ "name": name, "classId": class }))
        .fetch_one(&pool)
        .await
        .unwrap();
        seats.push((r.player_token().unwrap(), character));
    }
    let lea = join(&app, &code, "Léa", "spectator").await;
    let (camille, _) = seats.pop().unwrap();
    let (marc, borin) = seats.pop().unwrap();
    let t = Table {
        app,
        pool,
        gm,
        campaign,
        marc,
        camille,
        lea: lea.player_token().unwrap(),
        borin,
        world,
    };
    assert_eq!(
        t.gm("POST", "/session", None).await.status,
        StatusCode::CREATED
    );
    t.gm("POST", "/session/start", None).await;
    let r = t
        .gm("POST", "/board", Some(json!({ "map": world.map })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    t
}

async fn journey_on(world: &'static World) {
    let t = table(world).await;
    let gm_view = t.gm("GET", "/travel", None).await.body["data"].clone();
    assert_eq!(gm_view["mapId"], world.map);
    // The party starts with three days of supplies for its two members.
    assert_eq!(gm_view["travel"]["party"]["supplies"], 6, "{gm_view}");
    let place = gm_view["places"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["at"] == json!(world.to))
        .unwrap_or_else(|| panic!("{gm_view}"));
    assert_eq!(place["map"], world.place_map);

    // 1. The world map: the party is one token, nobody drags it.
    let board = t.player(&t.marc, "GET", "/board", None).await.body;
    let tokens = board["data"]["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 1, "{tokens:?}");
    assert_eq!(tokens[0]["id"], "party");
    let start = tokens[0]["at"].clone();
    let r = t
        .player(
            &t.marc,
            "POST",
            "/board/walk",
            Some(json!({ "path": [[1, 1]] })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "TRAVEL_MAP", "{}", r.body);
    let r = t
        .gm(
            "POST",
            "/board/edit",
            Some(json!({ "kind": "moveToken", "token": "party", "at": [1, 1] })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "TRAVEL_MAP", "{}", r.body);
    t.assert_phones_clean(&[]).await;

    // Two ways to the place, renamed by the GM.
    let r = t.travel(json!({ "kind": "plan", "to": world.to })).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let routes = r.body["data"]["travel"]["journey"]["routes"].clone();
    assert_eq!(routes.as_array().unwrap().len(), 2, "{routes}");
    let r = t
        .travel(json!({ "kind": "route", "index": 1, "name": "Le détour",
                        "description": "Plus long, plus sûr." }))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    // Not before the table chose.
    let r = t.travel(json!({ "kind": "advance" })).await;
    assert_eq!(r.body["error"]["code"], "NOT_CHOSEN");

    // 2. The vote: each player on their phone; a spectator has none.
    for (token, route) in [(&t.marc, 1), (&t.camille, 0), (&t.marc, 1)] {
        let r = t
            .player(
                token,
                "POST",
                "/travel",
                Some(json!({ "kind": "vote", "route": route })),
            )
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }
    let r = t
        .player(
            &t.lea,
            "POST",
            "/travel",
            Some(json!({ "kind": "vote", "route": 0 })),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let mine = t.player(&t.marc, "GET", "/travel", None).await.body["data"].clone();
    let shown = &mine["journey"]["routes"];
    assert_eq!(shown[1]["name"], "Le détour");
    assert_eq!(shown[1]["voters"], json!(["Borin"]));
    assert_eq!(shown[1]["mine"], true);
    assert_eq!(shown[0]["voters"], json!(["Lyra"]));
    assert_eq!(mine["journey"]["chosen"], Value::Null);

    // The GM decides, whatever the count.
    let r = t.travel(json!({ "kind": "choose", "index": 0 })).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let hexes = routes[0]["route"]["hexes"].as_array().unwrap().clone();
    let r = t
        .player(
            &t.marc,
            "POST",
            "/travel",
            Some(json!({ "kind": "vote", "route": 1 })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "VOTE_CLOSED");

    // 3–6. Portion by portion until the place.
    let mut kept = false;
    let mut checked = false;
    let mut watched = false;
    let mut dawns = 0;
    for _ in 0..40 {
        let r = t.travel(json!({ "kind": "advance" })).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let gm = r.body["data"]["travel"].clone();
        if gm["party"]["clock"]["portion"] == 0 && gm["party"]["clock"]["night"] == false {
            dawns += 1;
        }
        // 4. Events drawn for the GM only: none reaches a phone until one
        // is kept, and only as the GM worded it.
        if let Some(events) = gm["proposal"]["events"].as_array() {
            let titles: Vec<String> = events
                .iter()
                .map(|e| e["title"].as_str().unwrap().to_string())
                .collect();
            t.assert_phones_clean(&titles).await;
            if kept {
                let r = t.travel(json!({ "kind": "skip" })).await;
                assert_eq!(r.status, StatusCode::OK, "{}", r.body);
                let mine = t.player(&t.camille, "GET", "/travel", None).await.body["data"].clone();
                assert_eq!(mine["journey"]["events"].as_array().unwrap().len(), 1);
            } else {
                kept = true;
                let r = t
                    .travel(json!({ "kind": "keep", "event": events[0]["id"],
                                    "text": "Le MJ le raconte à sa façon." }))
                    .await;
                assert_eq!(r.status, StatusCode::OK, "{}", r.body);
                if events.len() > 1 {
                    t.assert_phones_clean(&titles[1..]).await;
                }
                let mine = t.player(&t.camille, "GET", "/travel", None).await.body["data"].clone();
                assert_eq!(mine["journey"]["events"][0]["title"], titles[0].as_str());
                assert_eq!(
                    mine["journey"]["events"][0]["text"],
                    "Le MJ le raconte à sa façon."
                );
            }
        }
        // 5. A group check once on the way: everyone rolls on their phone.
        if !checked {
            checked = true;
            let r = t
                .travel(json!({ "kind": "check", "ability": "SAG", "difficulty": 12,
                                "label": "Ne pas se perdre" }))
                .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
            let lea = t.player(&t.lea, "GET", "/travel", None).await.body["data"].clone();
            assert_eq!(lea["journey"]["check"]["canRoll"], false);
            for token in [&t.marc, &t.camille] {
                let r = t
                    .player(token, "POST", "/travel", Some(json!({ "kind": "roll" })))
                    .await;
                assert_eq!(r.status, StatusCode::OK, "{}", r.body);
                assert!(r.body["data"]["journey"]["check"]["myRoll"]["total"].is_i64());
            }
            let r = t
                .player(&t.marc, "POST", "/travel", Some(json!({ "kind": "roll" })))
                .await;
            assert_eq!(r.body["error"]["code"], "NO_CHECK");
            let c =
                t.player(&t.marc, "GET", "/travel", None).await.body["data"]["journey"]["check"]
                    .clone();
            // At least half of two: one success is enough.
            assert_eq!(c["needed"], 1);
            let successes = c["rolls"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["success"] == true)
                .count();
            assert_eq!(c["success"], successes >= 1, "{c}");
        }
        // 6. The night: the GM speaks to the one awake only.
        if gm["party"]["clock"]["night"] == true && !watched {
            watched = true;
            let lyra = t.gm("GET", "/travel", None).await.body["data"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["name"] == "Lyra")
                .unwrap()["character"]
                .clone();
            let r = t
                .travel(json!({ "kind": "watch", "slots": [t.borin, lyra, null] }))
                .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
            let words = "Des yeux brillent entre les arbres.";
            let r = t
                .travel(json!({ "kind": "watchTurn", "slot": 0, "message": words }))
                .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
            let marc = t.player(&t.marc, "GET", "/travel", None).await.body["data"].clone();
            assert_eq!(marc["journey"]["watch"]["mine"], true);
            assert_eq!(marc["journey"]["watch"]["message"], words);
            t.assert_phones_clean(&[]).await;
            for token in [&t.camille, &t.lea] {
                let other = t
                    .player(token, "GET", "/travel", None)
                    .await
                    .body
                    .to_string();
                assert!(!other.contains(words), "{other}");
            }
            let r = t
                .player(
                    &t.camille,
                    "POST",
                    "/travel",
                    Some(json!({ "kind": "wake" })),
                )
                .await;
            assert_eq!(r.body["error"]["code"], "NOT_YOUR_WATCH");
            let r = t
                .player(&t.marc, "POST", "/travel", Some(json!({ "kind": "wake" })))
                .await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
            assert_eq!(r.body["data"]["journey"]["watch"]["woken"], true);
        }
        if gm["journey"]["arrived"] == true {
            break;
        }
    }
    let gm = t.gm("GET", "/travel", None).await.body["data"]["travel"].clone();
    assert_eq!(gm["journey"]["arrived"], true, "{gm}");
    assert_eq!(gm["party"]["at"], json!(world.to));
    assert!(kept, "an event was drawn on the way");
    assert!(watched, "a night fell on the way");
    // Supplies: two eaten at each dawn.
    assert_eq!(gm["party"]["supplies"], 6 - 2 * dawns, "{gm}");
    let r = t.travel(json!({ "kind": "advance" })).await;
    assert_eq!(r.body["error"]["code"], "ALREADY_ARRIVED");
    // The phones followed the token, and the hexes crossed are out of the
    // fog.
    let board = t.player(&t.camille, "GET", "/board", None).await.body["data"].clone();
    assert_eq!(board["tokens"][0]["at"], json!(world.to));
    let rows = board["map"]["grid"]["rows"].as_array().unwrap().clone();
    let legend = board["map"]["grid"]["legend"].clone();
    for h in &hexes {
        let (x, y) = (
            h[0].as_u64().unwrap() as usize,
            h[1].as_u64().unwrap() as usize,
        );
        let glyph = rows[y]
            .as_str()
            .unwrap()
            .chars()
            .nth(x)
            .unwrap()
            .to_string();
        assert_ne!(legend[&glyph]["terrain"], "brouillard", "{h}");
    }
    t.assert_phones_clean(&[]).await;

    // 7. Entering the place opens its map — and its scene — on every
    // screen, without leaving the game.
    let r = t.travel(json!({ "kind": "enter" })).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let board = t.player(&t.marc, "GET", "/board", None).await.body["data"].clone();
    assert_eq!(board["map"]["id"], world.place_map);
    assert!(
        board["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tk| tk["id"] == format!("pc-{}", t.borin)),
        "{board}"
    );
    if let Some(scene) = world.scene {
        let node: Option<String> =
            sqlx::query_scalar("SELECT world->>'current_node' FROM campaigns WHERE id = $1")
                .bind(Uuid::parse_str(&t.campaign).unwrap())
                .fetch_one(&t.pool)
                .await
                .unwrap();
        assert_eq!(node.as_deref(), Some(scene));
    }

    // Back on the world map later: the party is where it arrived, and
    // what it saw stays seen.
    let r = t
        .gm("POST", "/board", Some(json!({ "map": world.map })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let again = t.player(&t.marc, "GET", "/board", None).await.body["data"].clone();
    assert_eq!(again["tokens"][0]["at"], json!(world.to));
    assert_ne!(again["tokens"][0]["at"], start);
    assert_eq!(again["map"]["grid"], board_grid_after(&t).await);
    let mine = t.player(&t.marc, "GET", "/travel", None).await.body["data"].clone();
    assert_eq!(mine["journey"], Value::Null);
}

/// The world map's grid as the phones last received it.
async fn board_grid_after(t: &Table) -> Value {
    t.player(&t.camille, "GET", "/board", None).await.body["data"]["map"]["grid"].clone()
}

#[tokio::test]
async fn the_corsaires_sail_from_port_louis_to_belle_ile() {
    journey_on(&CORSAIRES).await;
}

#[tokio::test]
async fn the_cure_dent_crosses_the_system_to_the_sereth_station() {
    journey_on(&BRASIER).await;
}

#[tokio::test]
async fn the_gm_holds_the_party_and_places_it_but_not_mid_journey() {
    let t = table(&CORSAIRES).await;
    // A hold spends the portion where the party stands.
    let before = t.gm("GET", "/travel", None).await.body["data"]["travel"]["party"].clone();
    let r = t.travel(json!({ "kind": "advance", "hold": true })).await;
    let after = r.body["data"]["travel"]["party"].clone();
    assert_eq!(after["at"], before["at"]);
    assert_eq!(after["clock"]["portion"], 1);
    // The land stops a ship.
    let r = t.travel(json!({ "kind": "place", "at": [10, 0] })).await;
    assert_eq!(r.body["error"]["code"], "CANNOT_STAND");
    let r = t.travel(json!({ "kind": "plan", "to": [10, 0] })).await;
    assert_eq!(r.body["error"]["code"], "NO_ROUTE");
    // Under way, the GM cannot drop the party elsewhere without
    // abandoning the journey first.
    t.travel(json!({ "kind": "plan", "to": CORSAIRES.to }))
        .await;
    t.travel(json!({ "kind": "choose", "index": 0 })).await;
    let r = t.travel(json!({ "kind": "place", "at": [3, 3] })).await;
    assert_eq!(r.body["error"]["code"], "JOURNEY_UNDER_WAY");
    let r = t.travel(json!({ "kind": "enter" })).await;
    assert_eq!(r.body["error"]["code"], "NOT_ARRIVED");
    t.travel(json!({ "kind": "abandon" })).await;
    let r = t.travel(json!({ "kind": "place", "at": [3, 3] })).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let board = t.player(&t.marc, "GET", "/board", None).await.body["data"].clone();
    assert_eq!(board["tokens"][0]["at"], json!([3, 3]));
    // Not on a place map: the journey needs the world map on the table.
    t.gm("POST", "/board", Some(json!({ "map": "quai-port-louis" })))
        .await;
    let r = t.travel(json!({ "kind": "advance" })).await;
    assert_eq!(r.body["error"]["code"], "NO_WORLD_MAP");
    let r = t.player(&t.marc, "GET", "/travel", None).await;
    assert_eq!(r.body["data"], Value::Null);
}
