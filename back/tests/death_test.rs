//! engine/save-against-death and player/face-death over the API, on the
//! Corsaires' quay with the rules switched to death saves: Borin down
//! rolls on his phone; the death the engine proposes reaches nobody
//! before Romain confirms it; Marc says Borin's last words and creates
//! another character, who enters play at the group's level. The dice
//! themselves are pinned in `shared/tests/death_saves.rs`.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const KNOCKED_OUT: &str = "  rule: knocked_out\n  condition: inconscient\n  out_after_turns: 3\n  out_condition: hors_combat\n";
const DEATH_SAVES: &str = "  rule: death_saves\n  condition: inconscient\n  difficulty: 10\n  successes: 3\n  failures: 3\n";

struct Table {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
    marc: String,
    lea: String,
    anne: Uuid,
    borin: Uuid,
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

    fn tok(&self) -> String {
        format!("pc-{}", self.borin)
    }

    /// Rewrites the stored fight (what a series of dice would have done).
    async fn set_fight(&self, path: &str, value: Value) {
        sqlx::query(
            "UPDATE encounters SET fight = jsonb_set(fight, $2::text[], $3, true)
             WHERE campaign_id = $1 AND status = 'live'",
        )
        .bind(Uuid::parse_str(&self.campaign).unwrap())
        .bind(path.split('.').collect::<Vec<_>>())
        .bind(value)
        .execute(&self.pool)
        .await
        .unwrap();
    }
}

async fn validate(pool: &PgPool, player: &Reply, sheet: Value) -> Uuid {
    let id = Uuid::parse_str(player.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    sqlx::query_scalar(
        "UPDATE characters SET status = 'validated', sheet = $2 WHERE player_id = $1 RETURNING id",
    )
    .bind(id)
    .bind(sheet)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Romain's Corsaires under death saves, live on the quay fight: Marc
/// plays Borin, Camille plays Anne (level 4), Léa watches.
async fn quay_fight() -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, CORSAIRES).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let borin = validate(
        &pool,
        &marc,
        json!({ "name": "Borin", "classId": "bretteur" }),
    )
    .await;
    let camille = join(&app, &code, "Camille", "player").await;
    let anne = validate(
        &pool,
        &camille,
        json!({ "name": "Anne", "classId": "vigie" }),
    )
    .await;
    let lea = join(&app, &code, "Léa", "spectator").await;
    let t = Table {
        app,
        pool,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
        lea: lea.player_token().unwrap(),
        anne,
        borin,
    };
    let r = t
        .gm(
            "POST",
            &format!("/characters/{anne}/adjust"),
            Some(json!({ "kind": "xp", "delta": 15 })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    // Romain's next rules play death saves.
    let r = t.gm("POST", "/rules/draft", None).await;
    let yaml = r.body["data"]["draft"]["yaml"].as_str().unwrap();
    assert!(yaml.contains(KNOCKED_OUT), "{yaml}");
    let yaml = yaml.replacen(KNOCKED_OUT, DEATH_SAVES, 1);
    let r = t
        .gm("PUT", "/rules/draft", Some(json!({ "yaml": yaml })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = t.gm("POST", "/rules/draft/lock", None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    for path in ["/session", "/session/start"] {
        let r = t.gm("POST", path, None).await;
        assert!(r.status.is_success(), "{path}: {}", r.body);
    }
    let r = t
        .gm("POST", "/board", Some(json!({ "map": "quai-port-louis" })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = t
        .gm("POST", "/fight", Some(json!({ "node": "sc_quai" })))
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    t
}

fn death_saves(view: &Value, tok: &str) -> Value {
    view["data"]["fight"]["order"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == tok)
        .unwrap()["deathSaves"]
        .clone()
}

#[tokio::test]
async fn borin_falls_rolls_on_his_phone_and_dies_only_on_romains_word() {
    let t = quay_fight().await;
    let tok = t.tok();

    // The sabre took him to 0 and it is his turn.
    let order: Vec<String> = serde_json::from_value(
        t.gm("GET", "/board", None).await.body["data"]["encounter"]["fight"]["order"].clone(),
    )
    .unwrap();
    let index = order.iter().position(|id| *id == tok).unwrap();
    t.set_fight(&format!("scene.combatants.{tok}.hit_points"), json!(0))
        .await;
    let r = t
        .gm(
            "POST",
            "/fight/command",
            Some(json!({ "kind": "condition", "who": tok, "condition": "inconscient", "remove": false })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    t.set_fight("scene.active", json!(tok)).await;
    t.set_fight("turn", json!(index)).await;

    // His phone asks for the save, and only that.
    let view = t.player(&t.marc, "GET", "/board", None).await.body;
    let fight = &view["data"]["fight"];
    assert_eq!(fight["myTurn"], true);
    assert_eq!(
        fight["deathSave"],
        json!({ "die": "1d20", "difficulty": 10 })
    );
    let lea = t.player(&t.lea, "GET", "/board", None).await.body;
    assert!(lea["data"]["fight"]["deathSave"].is_null());
    assert_eq!(
        t.gm("GET", "/board", None).await.body["data"]["encounter"]["deathSaveDue"],
        true
    );
    let r = t
        .player(
            &t.marc,
            "POST",
            "/fight",
            Some(json!({ "kind": "endTurn" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"]["refusal"]["kind"], "death_save_due");
    let r = t
        .player(
            &t.lea,
            "POST",
            "/fight",
            Some(json!({ "kind": "deathSave" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = t
        .player(
            &t.marc,
            "POST",
            "/fight",
            Some(json!({ "kind": "deathSave" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let view = t.player(&t.marc, "GET", "/board", None).await.body;
    let save = view["data"]["fight"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "death_save")
        .cloned()
        .expect("the save is in the fight's events");
    let natural = save["natural"].as_u64().unwrap();
    let boxes = death_saves(&view, &tok);
    match natural {
        20 => assert!(boxes.is_null(), "back at 1 HP: {boxes}"),
        1 => assert_eq!(boxes["failures"], 2),
        10..=19 => assert_eq!(boxes["successes"], 1),
        _ => assert_eq!(boxes["failures"], 1),
    }

    // The dice bring the third failure: the engine proposes the death.
    // Players see the boxes, never the proposal.
    t.set_fight(&format!("scene.combatants.{tok}.hit_points"), json!(0))
        .await;
    t.set_fight(
        &format!("dying.{tok}"),
        json!({ "successes": 0, "failures": 3, "stable": false, "proposed": true }),
    )
    .await;
    sqlx::query(
        "INSERT INTO encounter_events (encounter_id, seq, event)
         SELECT id, 10000, $2 FROM encounters WHERE campaign_id = $1 AND status = 'live'",
    )
    .bind(Uuid::parse_str(&t.campaign).unwrap())
    .bind(json!({ "kind": "death_proposed", "who": tok }))
    .execute(&t.pool)
    .await
    .unwrap();
    for token in [&t.marc, &t.lea] {
        let view = t.player(token, "GET", "/board", None).await.body;
        assert!(!view.to_string().contains("death_proposed"), "{view}");
        assert!(!view.to_string().contains("proposed"), "{view}");
        assert_eq!(death_saves(&view, &tok)["failures"], 3);
    }
    let gm = t.gm("GET", "/board", None).await.body;
    assert_eq!(
        gm["data"]["encounter"]["fight"]["dying"][&tok]["proposed"],
        true
    );
    // Only a proposed death can be confirmed.
    let r = t
        .gm(
            "POST",
            "/fight/command",
            Some(json!({ "kind": "death", "who": format!("pc-{}", t.anne), "call": "die" })),
        )
        .await;
    assert_eq!(
        r.body["error"]["refusal"]["kind"], "no_death_proposed",
        "{}",
        r.body
    );

    // Romain confirms.
    let r = t
        .gm(
            "POST",
            "/fight/command",
            Some(json!({ "kind": "death", "who": tok, "call": "die" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(
        r.body["data"]["encounter"]["fight"]["standing"][&tok],
        "dead"
    );
    let status: String = sqlx::query_scalar("SELECT status FROM characters WHERE id = $1")
        .bind(t.borin)
        .fetch_one(&t.pool)
        .await
        .unwrap();
    assert_eq!(status, "dead");
    let view = t.player(&t.marc, "GET", "/board", None).await.body;
    assert!(
        !view["data"]["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["id"] == tok),
        "the dead leave the board"
    );
    // The fight goes on without him: Marc has no turn left to play.
    let r = t
        .player(
            &t.marc,
            "POST",
            "/fight",
            Some(json!({ "kind": "endTurn" })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NOT_YOUR_TURN");
    // The table reads it in the journal.
    let evening = t.player(&t.lea, "GET", "/evening", None).await.body;
    assert!(
        evening.to_string().contains("Borin est tombé."),
        "{evening}"
    );

    // Marc's phone: Borin fell; his last words, said once.
    let me = t.player(&t.marc, "GET", "/me", None).await.body;
    assert!(me["data"]["character"].is_null(), "{me}");
    assert_eq!(me["data"]["fallen"]["name"], "Borin");
    assert_eq!(me["data"]["fallen"]["lastWords"], "");
    let words = "Dis à Dorn que je suis descendu le chercher.";
    let r = t
        .player(&t.lea, "PUT", "/last-words", Some(json!({ "text": words })))
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = t
        .player(&t.marc, "PUT", "/last-words", Some(json!({ "text": "  " })))
        .await;
    assert_eq!(r.body["error"]["code"], "INVALID_LAST_WORDS");
    let r = t
        .player(
            &t.marc,
            "PUT",
            "/last-words",
            Some(json!({ "text": words })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["fallen"]["lastWords"], words);
    let r = t
        .player(
            &t.marc,
            "PUT",
            "/last-words",
            Some(json!({ "text": "Encore." })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "LAST_WORDS_SAID");
    let evening = t.player(&t.lea, "GET", "/evening", None).await.body;
    assert!(evening.to_string().contains(words), "{evening}");

    // Another character, at the group's level: Anne's (4).
    let r = t.player(&t.marc, "POST", "/new-character", None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let character = &r.body["data"]["character"];
    assert_eq!(character["status"], "draft");
    assert_eq!(r.body["data"]["fallen"]["name"], "Borin");
    let r = t.player(&t.marc, "POST", "/new-character", None).await;
    assert_eq!(r.body["error"]["code"], "CHARACTER_EXISTS");
    let brann = Uuid::parse_str(character["id"].as_str().unwrap()).unwrap();
    let seen: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "UPDATE characters SET status = 'submitted',
                sheet = '{\"name\":\"Brann\",\"classId\":\"bretteur\"}'
         WHERE id = $1 RETURNING updated_at",
    )
    .bind(brann)
    .fetch_one(&t.pool)
    .await
    .unwrap();
    let r = t
        .gm(
            "POST",
            &format!("/characters/{brann}/validate"),
            Some(json!({ "seen": seen })),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let me = t.player(&t.marc, "GET", "/me", None).await.body;
    assert_eq!(me["data"]["character"]["play"]["level"], 4, "{me}");
    assert_eq!(me["data"]["fallen"]["lastWords"], words);
}

#[tokio::test]
async fn romain_may_decide_another_outcome() {
    let t = quay_fight().await;
    let tok = t.tok();
    t.set_fight(&format!("scene.combatants.{tok}.hit_points"), json!(0))
        .await;
    t.set_fight(
        &format!("dying.{tok}"),
        json!({ "successes": 1, "failures": 3, "stable": false, "proposed": true }),
    )
    .await;
    let r = t
        .gm(
            "POST",
            "/fight/command",
            Some(json!({ "kind": "death", "who": tok, "call": "spare" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let fight = &r.body["data"]["encounter"]["fight"];
    assert_eq!(fight["dying"][&tok]["stable"], true);
    assert_eq!(fight["standing"][&tok], "in_fight");
    let status: String = sqlx::query_scalar("SELECT status FROM characters WHERE id = $1")
        .bind(t.borin)
        .fetch_one(&t.pool)
        .await
        .unwrap();
    assert_eq!(status, "validated");
    let me = t.player(&t.marc, "GET", "/me", None).await.body;
    assert!(me["data"]["fallen"].is_null());
    let r = t.player(&t.marc, "POST", "/new-character", None).await;
    assert_eq!(r.body["error"]["code"], "CHARACTER_EXISTS");
}
