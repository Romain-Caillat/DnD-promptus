//! player/play-between-sessions — done when the moments of the board
//! « Entre deux » are doable in the app: the end of the evening (XP, what
//! the character got), levelling up (an upgrade point spent, the new
//! cards), the published « Précédemment… » and what the table learnt,
//! the chronicle, and the sheet read outside play. On both witness
//! worlds, over the API.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

/// Text only the GM may read.
const GM_ONLY: &str = "GMONLY<the traitor is Morel>";

struct Table {
    app: Router,
    gm: String,
    campaign: String,
    character: String,
    player: String,
    spectator: String,
}

/// Romain's table on `world`: Marc plays a validated `class`, Léa watches.
async fn table(pool: &PgPool, world: &str, class: &str) -> Table {
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, world).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let character: Uuid = sqlx::query_scalar(
        "UPDATE characters SET status = 'validated', sheet = $2
         WHERE player_id = $1 RETURNING id",
    )
    .bind(Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap())
    .bind(json!({ "name": "Borin", "classId": class }))
    .fetch_one(pool)
    .await
    .unwrap();
    let lea = join(&app, &code, "Léa", "spectator").await;
    Table {
        app,
        gm,
        campaign,
        character: character.to_string(),
        player: marc.player_token().unwrap(),
        spectator: lea.player_token().unwrap(),
    }
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn as_player(&self, token: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/play/{}{path}", self.campaign);
        call_as_player(&self.app, Some(token), method, &uri, body).await
    }

    async fn between(&self, token: &str) -> Value {
        let r = self.as_player(token, "GET", "/between", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }

    async fn ok(&self, method: &str, path: &str, body: Option<Value>) {
        let r = self.gm(method, path, body).await;
        assert!(r.status.is_success(), "{method} {path}: {}", r.body);
    }

    /// One evening: Marc earns `xp`, the table finds a key item and makes a
    /// promise, the GM keeps a note; the GM ends it with « Précédemment… » and publishes it.
    async fn evening(&self, xp: i32) {
        self.ok("POST", "/session", None).await;
        self.ok("POST", "/session/start", None).await;
        self.ok(
            "POST",
            &format!("/characters/{}/adjust", self.character),
            Some(json!({ "kind": "xp", "delta": xp })),
        )
        .await;
        for (kind, text, shared) in [
            ("item", "La page arrachée nomme la porte nord.", true),
            ("promise", "Ramener la lampe de Dorn à sa veuve.", true),
            ("note", GM_ONLY, false),
        ] {
            self.ok(
                "POST",
                "/session/journal",
                Some(json!({ "kind": kind, "text": text, "shared": shared })),
            )
            .await;
        }
        let ending = json!({ "recap": GM_ONLY, "previously": "Sous l'autel, la page arrachée." });
        let r = self.gm("POST", "/session/end", Some(ending.clone())).await;
        assert!(r.status.is_success(), "POST /session/end: {}", r.body);
        // « Précédemment… » is a draft until the GM publishes it.
        let session = r.body["data"]["id"].as_str().unwrap().to_string();
        let mut publish = ending;
        publish["publish"] = json!(true);
        self.ok("PUT", &format!("/sessions/{session}/recap"), Some(publish))
            .await;
    }
}

async fn the_board_between_two_sessions(world: &str, class: &str) {
    let pool = common::test_pool().await;
    let t = table(&pool, world, class).await;

    // Before any evening: no last session, nothing to level.
    let b = t.between(&t.player).await;
    assert!(b["last"].is_null() && b["levelUp"].is_null(), "{b}");

    // Moment 1 — the evening ends: the XP it brought, the level reached.
    t.evening(10).await;
    let b = t.between(&t.player).await;
    let text = b.to_string();
    assert!(
        !text.contains("GMONLY"),
        "the GM's recap or note leaked: {text}"
    );
    let last = &b["last"];
    assert_eq!(last["number"], 1);
    assert_eq!(last["mine"]["xpGained"], 10);
    assert_eq!(last["mine"]["levelBefore"], 1);
    assert_eq!(last["mine"]["level"], 3);
    // Moment 3 — « Précédemment… », what the table learnt, what stays open.
    assert_eq!(last["previously"], "Sous l'autel, la page arrachée.");
    assert_eq!(
        last["learnt"],
        json!(["La page arrachée nomme la porte nord."])
    );
    assert_eq!(
        b["openThreads"],
        json!(["Ramener la lampe de Dorn à sa veuve."])
    );
    // Moment 4 — the chronicle has the evening.
    assert_eq!(b["chronicle"].as_array().unwrap().len(), 1);
    assert_eq!(b["chronicle"][0]["text"], "Sous l'autel, la page arrachée.");

    // Moment 2 — level up: two points (one per 5 XP), the cards levels 2
    // and 3 opened, as the rules say.
    let up = &b["levelUp"];
    assert_eq!(up["level"], 3);
    assert_eq!(up["points"], 2);
    let me = t.as_player(&t.player, "GET", "/me", None).await;
    let play = &me.body["data"]["character"]["play"];
    let opened: Vec<&Value> = play["cards"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| (2..=3).contains(&c["level"].as_u64().unwrap()))
        .map(|c| &c["id"])
        .collect();
    let shown: Vec<&Value> = up["newCards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["id"])
        .collect();
    assert_eq!(shown, opened);

    let ability = up["abilities"][0]["id"].as_str().unwrap().to_string();
    let score = up["abilities"][0]["score"].as_i64().unwrap();
    let r = t
        .as_player(
            &t.player,
            "POST",
            "/character/upgrade",
            Some(json!({ "ability": ability })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let play = &r.body["data"]["play"];
    assert_eq!(play["upgradePoints"], 1);
    let raised = play["abilities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == ability.as_str())
        .unwrap();
    assert_eq!(raised["score"].as_i64(), Some(score + 1));

    // The GM's board reads the same score and points, and the history
    // says the player spent it.
    let sheets = t.gm("GET", "/sheets", None).await;
    assert_eq!(sheets.body["data"]["sheets"][0]["play"], *play);
    let line = &sheets.body["data"]["history"][0];
    assert_eq!(
        (line["actor"].as_str(), line["kind"].as_str()),
        (Some("player"), Some("upgrade"))
    );

    let r = t
        .as_player(
            &t.player,
            "POST",
            "/character/upgrade",
            Some(json!({ "ability": "PSI" })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "UNKNOWN_ABILITY");

    // Never while a session is live: the numbers do not move mid-game.
    t.ok("POST", "/session", None).await;
    t.ok("POST", "/session/start", None).await;
    let r = t
        .as_player(
            &t.player,
            "POST",
            "/character/upgrade",
            Some(json!({ "ability": ability })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "SESSION_LIVE");
    let b = t.between(&t.player).await;
    assert_eq!(b["open"]["status"], "live");
    t.ok("POST", "/session/end", Some(json!({}))).await;

    // The last point, then none left.
    let r = t
        .as_player(
            &t.player,
            "POST",
            "/character/upgrade",
            Some(json!({ "ability": ability })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = t
        .as_player(
            &t.player,
            "POST",
            "/character/upgrade",
            Some(json!({ "ability": ability })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NO_UPGRADE_POINT");
    let b = t.between(&t.player).await;
    assert!(b["levelUp"].is_null(), "{b}");
    // An evening without a word for the players: the chronicle keeps it,
    // without text.
    assert_eq!(b["chronicle"].as_array().unwrap().len(), 2);
    assert!(b["chronicle"][1]["text"].is_null());
    assert_eq!(b["last"]["mine"]["xpGained"], 0);

    // A spectator reads the table's evening, has no character to level.
    let b = t.between(&t.spectator).await;
    assert!(b["last"]["mine"].is_null() && b["levelUp"].is_null(), "{b}");
    assert_eq!(
        b["openThreads"],
        json!(["Ramener la lampe de Dorn à sa veuve."])
    );
    let r = t
        .as_player(
            &t.spectator,
            "POST",
            "/character/upgrade",
            Some(json!({ "ability": ability })),
        )
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "NO_CHARACTER");
}

#[tokio::test]
async fn a_corsaire_between_two_sessions() {
    the_board_between_two_sessions(CORSAIRES, "bretteur").await;
}

#[tokio::test]
async fn a_brasier_crew_member_between_two_sessions() {
    the_board_between_two_sessions(BRASIER, "pilote").await;
}
