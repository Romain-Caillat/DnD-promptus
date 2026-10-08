//! engine/level-up, engine/save-against-death, player/face-death over
//! the API, on both witness worlds: a new level waiting for its player,
//! hit points taken once per level by the die the server rolls or the
//! average, a death the dice propose and the GM confirms in a fight, a
//! death the GM decides outside one, last words, and a new character at
//! the party's level.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

/// One witness world: its campaign, the class Borin plays there, the
/// scene whose fight we open, and the rules file the variants edit.
struct World {
    name: &'static str,
    campaign: &'static str,
    rules: &'static str,
    class: &'static str,
    healer: &'static str,
    node: &'static str,
    map: &'static str,
}

const WORLDS: [World; 2] = [
    World {
        name: "corsaires",
        campaign: include_str!("../../content/campaigns/corsaires/campagne.yaml"),
        rules: include_str!("../../content/rules/corsaires/v1.yaml"),
        class: "bretteur",
        healer: "chirurgien",
        node: "sc_quai",
        map: "quai-port-louis",
    },
    World {
        name: "brasier",
        campaign: include_str!("../../content/campaigns/brasier/campagne.yaml"),
        rules: include_str!("../../content/rules/brasier/v1.yaml"),
        class: "canonnier",
        healer: "toubib",
        node: "sc_toboggan",
        map: "cure-dent-coursive",
    },
];

const KNOCKED_OUT: &str = "  rule: knocked_out\n  condition: inconscient\n  out_after_turns: 3\n  out_condition: hors_combat\n";
const DEATH_SAVES: &str = "  rule: death_saves\n  condition: inconscient\n  difficulty: 10\n  successes: 3\n  failures: 3\n  failures_on_hit: 2\n  stabilize: { kind: soin, ability: SAG, difficulty: 10 }\n";
const UPGRADES: &str = "  upgrade_points: 1\n";
const HIT_DICE: &str =
    "  upgrade_points: 1\n  hit_points_per_level: { dice: 1d10, average: 6, ability: CON }\n";

struct Table {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
    marc: String,
    borin: Uuid,
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn marc(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/play/{}{path}", self.campaign);
        call_as_player(&self.app, Some(&self.marc), method, &uri, body).await
    }

    async fn me(&self) -> Value {
        let r = self.marc("GET", "/me", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }

    async fn xp(&self, delta: i32) {
        let r = self
            .gm(
                "POST",
                &format!("/characters/{}/adjust", self.borin),
                Some(json!({ "kind": "xp", "delta": delta })),
            )
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }

    /// The rules of `w` with these edits, locked as version 2, shipped by
    /// opening the session.
    async fn rules_v2(&self, w: &World, edits: &[(&str, &str)]) {
        let mut yaml = w.rules.replacen("\nversion: 1\n", "\nversion: 2\n", 1);
        for (from, to) in edits {
            assert!(yaml.contains(from), "{}: `{from}`", w.name);
            yaml = yaml.replacen(from, to, 1);
        }
        let r = self.gm("POST", "/rules/draft", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let r = self
            .gm("PUT", "/rules/draft", Some(json!({ "yaml": yaml })))
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let r = self.gm("POST", "/rules/draft/lock", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let r = self.gm("POST", "/session", None).await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    }
}

/// Romain's table for `w`, Marc playing Borin, validated.
async fn table(w: &World) -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, w.campaign).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let marc_id = Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    let borin: Uuid = sqlx::query_scalar(
        "UPDATE characters SET status = 'validated', sheet = $2 WHERE player_id = $1 RETURNING id",
    )
    .bind(marc_id)
    .bind(json!({ "name": "Borin", "classId": w.class }))
    .fetch_one(&pool)
    .await
    .unwrap();
    Table {
        app,
        pool,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
        borin,
    }
}

// ------------------------------------------------------------ engine/level-up

#[tokio::test]
async fn a_new_level_waits_for_its_player_with_the_cards_it_unlocks() {
    for w in &WORLDS {
        let t = table(w).await;
        assert!(t.me().await["character"]["play"]["levelUp"].is_null());
        t.xp(10).await;
        let play = &t.me().await["character"]["play"];
        assert_eq!(play["level"], 3, "{}", w.name);
        let up = &play["levelUp"];
        assert_eq!((up["from"].as_u64(), up["to"].as_u64()), (Some(1), Some(3)));
        assert!(
            up["hitPoints"].is_null(),
            "{}: no die in these rules",
            w.name
        );
        let cards: Vec<&Value> = up["cards"].as_array().unwrap().iter().collect();
        assert_eq!(cards.len(), 1, "{}: {up}", w.name);
        assert_eq!(cards[0]["level"], 3);

        let r = t
            .marc(
                "POST",
                "/character/level-up",
                Some(json!({ "kind": "hitPoints", "level": 2, "method": "roll" })),
            )
            .await;
        assert_eq!(
            r.body["error"]["code"], "NO_HIT_POINTS_PER_LEVEL",
            "{}",
            r.body
        );
        let r = t
            .marc(
                "POST",
                "/character/level-up",
                Some(json!({ "kind": "seen" })),
            )
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        assert!(r.body["data"]["play"]["levelUp"].is_null(), "{}", r.body);
        assert_eq!(r.body["data"]["play"]["maxHitPoints"], 10);

        // The next level opens it again, from level 3.
        t.xp(5).await;
        let up = &t.me().await["character"]["play"]["levelUp"];
        assert_eq!((up["from"].as_u64(), up["to"].as_u64()), (Some(3), Some(4)));
        assert_eq!(up["cards"], json!([]));
    }
}

#[tokio::test]
async fn each_level_s_hit_points_are_taken_once_by_the_die_or_the_average() {
    for w in &WORLDS {
        let t = table(w).await;
        t.rules_v2(w, &[(UPGRADES, HIT_DICE)]).await;
        t.xp(15).await;
        let play = t.me().await["character"]["play"].clone();
        let hp = &play["levelUp"]["hitPoints"];
        assert_eq!(hp["dice"], "1d10", "{}: {play}", w.name);
        assert_eq!(hp["average"], 6);
        assert_eq!(hp["ability"], "Constitution");
        assert_eq!(hp["due"], json!([2, 3, 4]));
        let modifier = hp["modifier"].as_i64().unwrap();
        assert_eq!(play["maxHitPoints"], 10, "{}: nothing taken yet", w.name);

        let level_up = |body: Value| t.marc("POST", "/character/level-up", Some(body));
        let r = level_up(json!({ "kind": "hitPoints", "level": 4, "method": "roll" })).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let rolled = r.body["data"]["play"]["levelUp"]["gains"][0].clone();
        assert_eq!(rolled["level"], 4);
        assert_eq!(rolled["faces"].as_array().unwrap().len(), 1, "{rolled}");
        let die = rolled["amount"].as_i64().unwrap();
        assert!((1..=10 + modifier).contains(&die), "{rolled}");
        let r = level_up(json!({ "kind": "hitPoints", "level": 4, "method": "average" })).await;
        assert_eq!(r.body["error"]["code"], "ALREADY_CHOSEN", "never rerolled");
        let r = level_up(json!({ "kind": "seen" })).await;
        assert_eq!(r.body["error"]["code"], "HIT_POINTS_TO_CHOOSE");
        for level in [2, 3] {
            let r =
                level_up(json!({ "kind": "hitPoints", "level": level, "method": "average" })).await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        }
        let r = level_up(json!({ "kind": "seen" })).await;
        let play = &r.body["data"]["play"];
        assert!(play["levelUp"].is_null(), "{play}");
        let average = 6 + modifier;
        assert_eq!(play["maxHitPoints"], 10 + 2 * average + die, "{}", w.name);

        // The GM sees each choice in the history, as the player's.
        let sheets = t.gm("GET", "/sheets", None).await.body;
        let levels: Vec<&Value> = sheets["data"]["history"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|h| h["kind"] == "level")
            .collect();
        assert_eq!(levels.len(), 3, "{sheets}");
        assert!(levels.iter().all(|h| h["actor"] == "player"));

        // XP taken back to level 3: the level-4 hit points go with it.
        t.xp(-5).await;
        let play = &t.me().await["character"]["play"];
        assert_eq!(play["maxHitPoints"], 10 + 2 * average, "{}", w.name);
    }
}

// --------------------------------------------- engine/save-against-death

/// Combatant `who`'s entry in the stored fight, set by hand: what the
/// dice would take too long to bring about.
async fn patch_fighter(pool: &PgPool, campaign: &str, who: &str, field: &str, value: Value) {
    sqlx::query(
        "UPDATE encounters
         SET fight = jsonb_set(fight, ARRAY['scene', 'combatants', $2, $3], $4)
         WHERE campaign_id = $1 AND status = 'live'",
    )
    .bind(Uuid::parse_str(campaign).unwrap())
    .bind(who)
    .bind(field)
    .bind(value)
    .execute(pool)
    .await
    .unwrap();
}

/// Camille joins with Lyra, validated: with Borin down, the party still
/// stands and the fight goes on. Returns her device token.
async fn lyra_joins(t: &Table, w: &World) -> String {
    let code = invite_code(&t.app, &t.gm, &t.campaign).await;
    let camille = join(&t.app, &code, "Camille", "player").await;
    let id = Uuid::parse_str(camille.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    sqlx::query("UPDATE characters SET status = 'validated', sheet = $2 WHERE player_id = $1")
        .bind(id)
        .bind(json!({ "name": "Lyra", "classId": w.healer }))
        .execute(&t.pool)
        .await
        .unwrap();
    camille.player_token().unwrap()
}

/// Ends turns until it is `who`'s: the GM's opponents, and Camille's.
async fn until_turn_of(t: &Table, camille: &str, who: &str) {
    for _ in 0..40 {
        let board = t.gm("GET", "/board", None).await.body;
        let fight = &board["data"]["encounter"]["fight"];
        let active = fight["scene"]["active"].as_str().unwrap().to_string();
        if active == who {
            return;
        }
        let r = if fight["scene"]["combatants"][&active]["side"] == "party" {
            let uri = format!("/api/play/{}/fight", t.campaign);
            let end = json!({ "kind": "endTurn" });
            call_as_player(&t.app, Some(camille), "POST", &uri, Some(end)).await
        } else {
            let end = json!({ "kind": "adversary", "command": { "kind": "endTurn" } });
            t.gm("POST", "/fight/command", Some(end)).await
        };
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }
    panic!("{who} never got a turn");
}

#[tokio::test]
async fn a_death_the_dice_propose_waits_for_the_gm_then_its_player_goes_on() {
    for w in &WORLDS {
        let t = table(w).await;
        t.xp(10).await;
        let camille = lyra_joins(&t, w).await;
        t.rules_v2(w, &[(KNOCKED_OUT, DEATH_SAVES)]).await;
        let r = t.gm("POST", "/session/start", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let r = t.gm("POST", "/board", Some(json!({ "map": w.map }))).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let r = t
            .gm("POST", "/fight", Some(json!({ "node": w.node })))
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        let tok = format!("pc-{}", t.borin);
        until_turn_of(&t, &camille, &tok).await;

        // Borin is down and dying: his turn is his save, nothing else.
        patch_fighter(&t.pool, &t.campaign, &tok, "hit_points", json!(0)).await;
        let saves = json!({ "successes": 0, "failures": 0, "stable": false, "death_due": false });
        patch_fighter(&t.pool, &t.campaign, &tok, "death_saves", saves).await;
        let view = t.marc("GET", "/board", None).await.body;
        let fight = &view["data"]["fight"];
        assert_eq!(fight["deathSave"], true, "{}: {fight}", w.name);
        assert_eq!(fight["stabilize"]["difficulty"], 10);
        let r = t
            .marc("POST", "/fight", Some(json!({ "kind": "endTurn" })))
            .await;
        assert_eq!(
            r.body["error"]["refusal"]["kind"], "death_save_first",
            "{}",
            r.body
        );
        let r = t
            .marc("POST", "/fight", Some(json!({ "kind": "deathSave" })))
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let gm = t.gm("GET", "/board", None).await.body;
        let me = &gm["data"]["encounter"]["fight"]["scene"]["combatants"][&tok];
        let rolled = me["hit_points"] == 1
            || me["death_saves"]["successes"].as_u64().unwrap()
                + me["death_saves"]["failures"].as_u64().unwrap()
                == 1;
        assert!(rolled, "{}: {me}", w.name);
        // A natural 20 stood him up with 1 hit point, kept in play too:
        // down again there as well, as a hit would have done.
        if me["hit_points"] == 1 {
            sqlx::query("UPDATE character_play SET damage = damage + 1 WHERE character_id = $1")
                .bind(t.borin)
                .execute(&t.pool)
                .await
                .unwrap();
        }

        // The dice propose his death: the GM sees it, the table does not.
        patch_fighter(&t.pool, &t.campaign, &tok, "hit_points", json!(0)).await;
        let due = json!({ "successes": 1, "failures": 3, "stable": false, "death_due": true });
        patch_fighter(&t.pool, &t.campaign, &tok, "death_saves", due).await;
        let gm = t.gm("GET", "/board", None).await.body;
        let me = &gm["data"]["encounter"]["fight"]["scene"]["combatants"][&tok];
        assert_eq!(me["death_saves"]["death_due"], true);
        let view = t.marc("GET", "/board", None).await.body;
        let text = view.to_string();
        assert!(
            !text.contains("death_due") && !text.contains("deathDue"),
            "{text}"
        );
        let borin = view["data"]["fight"]["order"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == tok.as_str())
            .unwrap()
            .clone();
        assert_eq!(borin["standing"], "in_fight", "{borin}");
        assert_eq!(borin["dying"]["failures"], 3, "the saves are the table's");

        // The GM confirms: Borin is dead, out of the fight and of play.
        let r = t
            .gm(
                "POST",
                "/fight/command",
                Some(json!({ "kind": "confirmDeath", "who": tok })),
            )
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let me = t.me().await;
        assert!(me["character"].is_null(), "{me}");
        assert_eq!(me["fallen"]["name"], "Borin");
        assert_eq!(me["fallen"]["level"], 3);
        assert!(me["fallen"]["lastWords"].is_null());
        let view = t.marc("GET", "/board", None).await.body;
        assert!(
            view["data"]["tokens"]
                .as_array()
                .unwrap()
                .iter()
                .all(|k| k["id"] != tok.as_str()),
            "{view}"
        );
        // The fight goes on without him, his sheet untouched by it.
        let r = t
            .gm("POST", "/fight/command", Some(json!({ "kind": "stop" })))
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let sheets = t.gm("GET", "/sheets", None).await.body["data"].clone();
        let names: Vec<&Value> = sheets["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| &c["name"])
            .collect();
        assert_eq!(names, [&json!("Lyra")], "Borin is no longer in play");
        assert_eq!(sheets["fallen"][0]["name"], "Borin");
        assert_eq!(sheets["fallen"][0]["nickname"], "Marc");
        assert!(
            sheets["history"]
                .as_array()
                .unwrap()
                .iter()
                .any(|h| h["kind"] == "death"),
            "{sheets}"
        );
        let cause: String =
            sqlx::query_scalar("SELECT cause FROM character_deaths WHERE character_id = $1")
                .bind(t.borin)
                .fetch_one(&t.pool)
                .await
                .unwrap();
        assert_eq!(cause, "rules");

        last_words_then_a_new_character(&t, 10).await;
    }
}

/// Moments 6 and 7 of « Mourir »: Marc's last words reach the table once,
/// then he chooses; a new character joins at the dead one's XP.
async fn last_words_then_a_new_character(t: &Table, xp: i64) {
    let words = "Dis à Dorn que je suis descendu le chercher.";
    let r = t
        .marc("POST", "/fate/words", Some(json!({ "text": words })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["fallen"]["lastWords"], words);
    let r = t
        .marc(
            "POST",
            "/fate/words",
            Some(json!({ "text": "Autre chose." })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "ALREADY_SAID");
    let shared: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM table_journal
                        WHERE campaign_id = $1 AND shared AND text LIKE '%' || $2 || '%')",
    )
    .bind(Uuid::parse_str(&t.campaign).unwrap())
    .bind(words)
    .fetch_one(&t.pool)
    .await
    .unwrap();
    assert!(shared, "the whole table reads them");

    let r = t
        .marc("POST", "/fate/next", Some(json!({ "next": "watch" })))
        .await;
    assert_eq!(r.body["data"]["fallen"]["next"], "watch", "{}", r.body);
    assert!(r.body["data"]["character"].is_null());
    let r = t
        .marc("POST", "/fate/next", Some(json!({ "next": "new" })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let fresh = r.body["data"]["character"].clone();
    assert_eq!(fresh["status"], "draft", "{fresh}");
    let r = t
        .marc("POST", "/fate/next", Some(json!({ "next": "hook" })))
        .await;
    assert_eq!(r.body["error"]["code"], "ALREADY_CHOSEN");

    // Brann, sent and validated: he starts at Borin's XP.
    let id = Uuid::parse_str(fresh["id"].as_str().unwrap()).unwrap();
    let class: String =
        sqlx::query_scalar("SELECT sheet->>'classId' FROM characters WHERE id = $1")
            .bind(t.borin)
            .fetch_one(&t.pool)
            .await
            .unwrap();
    let seen: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "UPDATE characters SET status = 'submitted', sheet = $2 WHERE id = $1 RETURNING updated_at",
    )
    .bind(id)
    .bind(json!({ "name": "Brann", "classId": class }))
    .fetch_one(&t.pool)
    .await
    .unwrap();
    let r = t
        .gm(
            "POST",
            &format!("/characters/{id}/validate"),
            Some(json!({ "seen": seen })),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let me = t.me().await;
    assert_eq!(me["character"]["sheet"]["name"], "Brann");
    assert_eq!(me["character"]["play"]["totalXp"], xp, "{me}");
    assert_eq!(
        me["fallen"]["name"], "Borin",
        "Borin stays in the chronicle"
    );
}

#[tokio::test]
async fn under_knocked_out_the_gm_decides_a_death_for_someone_down() {
    for w in &WORLDS {
        let t = table(w).await;
        t.xp(5).await;
        let death = format!("/characters/{}/death", t.borin);
        let r = t.gm("POST", &death, None).await;
        assert_eq!(r.body["error"]["code"], "NOT_DOWN", "{}", r.body);
        let r = t
            .gm(
                "POST",
                &format!("/characters/{}/adjust", t.borin),
                Some(json!({ "kind": "hitPoints", "delta": -10 })),
            )
            .await;
        assert_eq!(r.body["data"]["play"]["hitPoints"], 0, "{}", r.body);
        let r = t.gm("POST", &death, None).await;
        assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
        let r = t.gm("POST", &death, None).await;
        assert_eq!(r.status, StatusCode::CONFLICT, "dies once: {}", r.body);
        let cause: String =
            sqlx::query_scalar("SELECT cause FROM character_deaths WHERE character_id = $1")
                .bind(t.borin)
                .fetch_one(&t.pool)
                .await
                .unwrap();
        assert_eq!(cause, "gm", "{}", w.name);
        let me = t.me().await;
        assert_eq!(me["fallen"]["level"], 2, "{}: {me}", w.name);
        // A dead character's gestures are refused like a missing one's.
        let r = t
            .marc(
                "POST",
                "/character/level-up",
                Some(json!({ "kind": "seen" })),
            )
            .await;
        assert_eq!(r.body["error"]["code"], "NO_CHARACTER");
        last_words_then_a_new_character(&t, 5).await;
    }
}
