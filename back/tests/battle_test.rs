//! A ship battle at the table, over the API (engine/support-vehicle-
//! combat): the Corsaires' interception of the Greyhound and the
//! Brasier's Vorr swarm — the crew at their stations, the enemy numbers
//! hidden until scanned, the co-GM's enemy turn validated by the GM, a
//! boarding fought on the deck and handed back, the end's XP.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

struct Table {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
    marc: String,
    lea: String,
    marc_character: Uuid,
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

    async fn gm_battle(&self) -> Value {
        self.gm("GET", "/board", None).await.body["data"]["battle"].clone()
    }

    async fn command(&self, cmd: Value) -> Reply {
        self.gm("POST", "/battle/command", Some(cmd)).await
    }

    /// The GM ends turns until it is the crew's (`crew: true`) or an
    /// enemy's.
    async fn until_turn(&self, crew: bool) {
        for _ in 0..12 {
            if self.gm_battle().await["view"]["crewTurn"] == json!(crew) {
                return;
            }
            let r = self.command(json!({ "kind": "endTurn" })).await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        }
        panic!("never the turn wanted ({crew})");
    }

    /// Replace the stored battle (a test's shortcut to a position).
    async fn set_battle(&self, battle: &Value) {
        sqlx::query("UPDATE battles SET battle = $2 WHERE campaign_id = $1 AND status <> 'ended'")
            .bind(Uuid::parse_str(&self.campaign).unwrap())
            .bind(battle)
            .execute(&self.pool)
            .await
            .unwrap();
    }
}

async fn validated(pool: &PgPool, joined: &Reply, name: &str, class: &str) -> Uuid {
    let id = Uuid::parse_str(joined.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    sqlx::query_scalar(
        "UPDATE characters SET status = 'validated', sheet = $2 WHERE player_id = $1 RETURNING id",
    )
    .bind(id)
    .bind(json!({ "name": name, "classId": class }))
    .fetch_one(pool)
    .await
    .unwrap()
}

/// A world live with two players aboard, its battle `node` opened.
async fn aboard(yaml: &str, classes: [&str; 2], node: &str) -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, yaml).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let marc_character = validated(&pool, &marc, "Borin", classes[0]).await;
    let lea = join(&app, &code, "Léa", "player").await;
    validated(&pool, &lea, "Ysolde", classes[1]).await;
    let t = Table {
        app,
        pool,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
        lea: lea.player_token().unwrap(),
        marc_character,
    };
    assert_eq!(
        t.gm("POST", "/session", None).await.status,
        StatusCode::CREATED
    );
    t.gm("POST", "/session/start", None).await;
    let r = t.gm("POST", "/battle", Some(json!({ "node": node }))).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    t
}

async fn greyhound() -> Table {
    aboard(
        CORSAIRES,
        ["bretteur", "vigie"],
        "sc_interception_greyhound",
    )
    .await
}

fn ship<'a>(view: &'a Value, id: &str) -> &'a Value {
    view["ships"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
        .unwrap_or_else(|| panic!("no ship {id}: {view}"))
}

#[tokio::test]
async fn the_crew_acts_at_their_stations_and_enemy_numbers_wait_for_a_scan() {
    let t = greyhound().await;

    let gm = t.gm_battle().await;
    assert_eq!(gm["status"], "live");
    let real_hull = ship(&gm["battle"], "hms_greyhound")["hull"].clone();
    assert!(real_hull.as_i64().unwrap() > 0, "{gm}");

    // Borin the bretteur sits on the deck, Ysolde the lookout in the top;
    // the Greyhound's numbers are unknown to them.
    let view = t.player(&t.marc, "GET", "/battle", None).await.body["data"].clone();
    assert_eq!(view["me"]["station"], "pont", "{view}");
    assert!(ship(&view, "la_machoire")["hull"].is_number());
    let grey = ship(&view, "hms_greyhound");
    assert_eq!(grey["known"], false);
    assert!(grey["hull"].is_null() && grey["morale"].is_null(), "{grey}");
    assert!(!view.to_string().contains("proposal"));

    t.until_turn(true).await;
    // The helm is not Borin's station: refused, nothing changes.
    let r = t
        .player(
            &t.marc,
            "POST",
            "/battle",
            Some(json!({ "kind": "act", "action": "manoeuvrer" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["refusal"]["kind"], "not_at_station");

    // Ysolde reads the Greyhound: its numbers reach the whole crew.
    let r = t
        .player(
            &t.lea,
            "POST",
            "/battle",
            Some(json!({
                "kind": "act", "action": "lire_le_navire",
                "aim": { "target": "hms_greyhound" },
            })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let view = t.player(&t.marc, "GET", "/battle", None).await.body["data"].clone();
    let grey = ship(&view, "hms_greyhound");
    assert_eq!(grey["known"], true, "{grey}");
    assert!(grey["hull"].is_number());
}

#[tokio::test]
async fn the_co_gm_proposes_an_enemy_turn_and_a_stale_one_is_refused() {
    let t = greyhound().await;
    t.until_turn(false).await;

    let r = t.command(json!({ "kind": "propose" })).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let gm = t.gm_battle().await;
    assert!(gm["proposal"]["orders"].is_array(), "{gm}");
    // Never on a player's screen.
    let view = t.player(&t.marc, "GET", "/battle", None).await.body;
    assert!(!view.to_string().contains("orders"), "{view}");

    // The GM changes the battle: the proposal no longer applies.
    let hull = ship(&gm["battle"], "la_machoire")["hull"].clone();
    let r = t
        .command(json!({ "kind": "adjust", "ship": "la_machoire", "hull": hull }))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = t.command(json!({ "kind": "accept" })).await;
    assert_eq!(r.body["error"]["code"], "NO_PROPOSAL", "{}", r.body);

    t.command(json!({ "kind": "propose" })).await;
    let before = t.gm_battle().await;
    sqlx::query("UPDATE battles SET version = version + 1 WHERE campaign_id = $1")
        .bind(Uuid::parse_str(&t.campaign).unwrap())
        .execute(&t.pool)
        .await
        .unwrap();
    let r = t.command(json!({ "kind": "accept" })).await;
    assert_eq!(r.body["error"]["code"], "PROPOSAL_STALE", "{}", r.body);
    sqlx::query("UPDATE battles SET version = version - 1 WHERE campaign_id = $1")
        .bind(Uuid::parse_str(&t.campaign).unwrap())
        .execute(&t.pool)
        .await
        .unwrap();

    // Accepted as is, the turn the co-GM played becomes the battle.
    let r = t.command(json!({ "kind": "accept" })).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let after = t.gm_battle().await;
    assert!(after["proposal"].is_null());
    assert_ne!(after["battle"], before["battle"]);
    assert!(after["events"].as_array().unwrap().len() > before["events"].as_array().unwrap().len());
}

#[tokio::test]
async fn a_boarding_is_fought_on_the_deck_then_the_battle_resumes() {
    let t = aboard(BRASIER, ["pilote", "mecano"], "sc_essaim_vorr").await;
    // A Vorr fighter docks against the Cure-Dent.
    let mut battle = t.gm_battle().await["battle"].clone();
    let at = ship(&battle, "cure_dent")["at"].clone();
    let ships = battle["ships"].as_array_mut().unwrap();
    let fighter = ships
        .iter_mut()
        .find(|s| s["id"] == "chasseur_vorr-1")
        .unwrap();
    fighter["at"] = json!([at[0].as_i64().unwrap() + 1, at[1]]);
    t.set_battle(&battle).await;

    let r = t
        .command(json!({ "kind": "board", "attacker": "chasseur_vorr-1", "defender": "cure_dent" }))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let data = &r.body["data"];
    assert_eq!(data["battle"]["status"], "boarding");
    assert_eq!(data["encounter"]["live"], true, "{data}");
    assert_eq!(data["encounter"]["node"], "sc_essaim_vorr");
    assert_eq!(data["board"]["mapId"], "cure-dent-coursive");
    // The ship waits: nobody acts at a station during the deck fight.
    let r = t
        .player(&t.marc, "POST", "/battle", Some(json!({ "kind": "pass" })))
        .await;
    assert_eq!(r.body["error"]["refusal"]["kind"], "boarding", "{}", r.body);
    let view = t.player(&t.marc, "GET", "/battle", None).await.body["data"].clone();
    assert_eq!(view["boarding"], true);

    // The GM stops the deck fight without a winner: the boarders are
    // thrown back and the battle goes on.
    let r = t
        .gm("POST", "/fight/command", Some(json!({ "kind": "stop" })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let gm = t.gm_battle().await;
    assert_eq!(gm["status"], "live", "{gm}");
    assert!(gm["battle"]["boarding"].is_null());
    assert_eq!(ship(&gm["battle"], "chasseur_vorr-1")["standing"], "afloat");
    assert!(
        gm["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "boarding_ended" && e["attacker_won"] == false),
        "{gm}"
    );
}

#[tokio::test]
async fn the_end_grants_the_crew_xp_and_writes_the_outcome() {
    let t = greyhound().await;
    let tok = format!("pc-{}", t.marc_character);
    let mut battle = t.gm_battle().await["battle"].clone();
    battle["xp"] = json!({ tok: 2 });
    t.set_battle(&battle).await;

    for (s, standing) in [("sloop_escorte", "destroyed"), ("hms_greyhound", "struck")] {
        let r = t
            .command(json!({ "kind": "strike", "ship": s, "standing": standing }))
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }
    let gm = t.gm_battle().await;
    assert_eq!(gm["status"], "ended", "{gm}");
    let view = t.player(&t.marc, "GET", "/battle", None).await.body["data"].clone();
    assert_eq!(view["won"], true);
    assert_eq!(view["reason"], "victory");

    let xp: i32 = sqlx::query_scalar("SELECT total_xp FROM character_play WHERE character_id = $1")
        .bind(t.marc_character)
        .fetch_one(&t.pool)
        .await
        .unwrap();
    assert_eq!(xp, 2);
    let lines: Vec<(String, bool)> = sqlx::query_as(
        "SELECT text, shared FROM table_journal WHERE campaign_id = $1 AND ref = 'sc_interception_greyhound'",
    )
    .bind(Uuid::parse_str(&t.campaign).unwrap())
    .fetch_all(&t.pool)
    .await
    .unwrap();
    assert!(
        lines
            .iter()
            .any(|(text, shared)| *shared && text == "Combat de vaisseau gagné"),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|(text, shared)| !*shared && text.contains("amène son pavillon")),
        "{lines:?}"
    );
    // Over: a new battle may open, and commands are refused.
    let r = t.command(json!({ "kind": "endTurn" })).await;
    assert_eq!(r.body["error"]["code"], "NO_BATTLE");
}
