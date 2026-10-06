//! gm/adjust-sheets-fast — done when, from the GM's screen, one gesture
//! gives XP, takes or gives back hit points, gives an item or gold; the
//! player's sheet follows live, and every change lands in the history.
//! player/read-sheet-and-journal reads the same state on the player's
//! side (`me`), and lets the player carry an item.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::marked::FIXTURE;
use common::{call, call_as_player, imported_campaign, invite_code, join};
use promptus_back::live;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

struct Seat {
    character: String,
    token: String,
}

/// `nickname` joins and writes `sheet`, sent to the GM; returns the
/// character, the device token and the `updatedAt` a review reads.
async fn submitted(
    app: &Router,
    pool: &PgPool,
    code: &str,
    nickname: &str,
    sheet: Value,
) -> (Seat, chrono::DateTime<chrono::Utc>) {
    let r = join(app, code, nickname, "player").await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let character = r.body["data"]["character"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let seen = sqlx::query_scalar(
        "UPDATE characters SET sheet = $2, status = 'submitted' WHERE id = $1 RETURNING updated_at",
    )
    .bind(Uuid::parse_str(&character).unwrap())
    .bind(sheet)
    .fetch_one(pool)
    .await
    .unwrap();
    (
        Seat {
            character,
            token: r.player_token().unwrap(),
        },
        seen,
    )
}

/// [`submitted`], then validated by the GM.
async fn validated(
    app: &Router,
    pool: &PgPool,
    gm: &str,
    campaign: &str,
    code: &str,
    nickname: &str,
    sheet: Value,
) -> Seat {
    let (seat, seen) = submitted(app, pool, code, nickname, sheet).await;
    let r = call(
        app,
        Some(gm),
        "POST",
        &format!(
            "/api/campaigns/{campaign}/characters/{}/validate",
            seat.character
        ),
        Some(json!({ "seen": seen })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    seat
}

async fn adjust(app: &Router, gm: &str, campaign: &str, seat: &Seat, body: Value) -> common::Reply {
    call(
        app,
        Some(gm),
        "POST",
        &format!(
            "/api/campaigns/{campaign}/characters/{}/adjust",
            seat.character
        ),
        Some(body),
    )
    .await
}

/// The character in play, as its player reads it.
async fn my_play(app: &Router, campaign: &str, seat: &Seat) -> Value {
    let r = call_as_player(
        app,
        Some(&seat.token),
        "GET",
        &format!("/api/play/{campaign}/me"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    r.body["data"]["character"]["play"].clone()
}

async fn board(app: &Router, gm: &str, campaign: &str) -> Value {
    let r = call(
        app,
        Some(gm),
        "GET",
        &format!("/api/campaigns/{campaign}/sheets"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    r.body["data"].clone()
}

async fn topic_version(pool: &PgPool, campaign: &str, topic: &str) -> i64 {
    live::versions(pool, Uuid::parse_str(campaign).unwrap())
        .await
        .unwrap()
        .get(topic)
        .copied()
        .unwrap_or(0)
}

fn lyra() -> Value {
    json!({ "name": "Lyra", "classId": "bretteur",
            "abilities": { "FOR": 13, "DEX": 14, "CON": 10, "INT": 8, "SAG": 10, "CHA": 9 } })
}

#[tokio::test]
async fn romain_adjusts_lyra_in_one_gesture_and_her_phone_follows() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let lyra = validated(&app, &pool, &gm, &campaign, &code, "Camille", lyra()).await;

    // She starts whole, with the class kit and the rules' purse.
    let play = my_play(&app, &campaign, &lyra).await;
    assert_eq!(
        (play["hitPoints"].as_i64(), play["maxHitPoints"].as_i64()),
        (Some(10), Some(10))
    );
    assert_eq!(play["level"], 1);
    assert_eq!(play["armorClass"], 12);
    assert_eq!(play["resources"][0]["name"], "Pièces d'or");
    assert_eq!(play["resources"][0]["amount"], 10);
    assert_eq!(play["inventory"][0]["name"], "Sabre d'abordage");
    let b = board(&app, &gm, &campaign).await;
    assert_eq!(b["sheets"][0]["nickname"], "Camille");
    assert_eq!(b["sheets"][0]["play"], play, "the GM reads what she reads");
    assert!(
        b["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == "fiole_de_rhum_fortifiant")
    );
    assert!(
        b["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i.get("note").is_none()),
        "an item's GM note is not shown, even to the GM's board"
    );

    // Each gesture moves her character's topic (and the table's).
    let topic = format!("character:{}", lyra.character);
    let before = topic_version(&pool, &campaign, &topic).await;
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &lyra,
        json!({ "kind": "xp", "delta": 1 }),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["play"]["totalXp"], 1);
    assert!(topic_version(&pool, &campaign, &topic).await > before);

    adjust(
        &app,
        &gm,
        &campaign,
        &lyra,
        json!({ "kind": "hitPoints", "delta": -3 }),
    )
    .await;
    adjust(
        &app,
        &gm,
        &campaign,
        &lyra,
        json!({ "kind": "resource", "resource": "or", "delta": 5 }),
    )
    .await;
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &lyra,
        json!({ "kind": "giveItem", "item": "fiole_de_rhum_fortifiant", "qty": 2 }),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    adjust(
        &app,
        &gm,
        &campaign,
        &lyra,
        json!({ "kind": "giveItem", "name": "Lanterne du phare", "description": "Elle ne s'éteint pas." }),
    )
    .await;

    let play = my_play(&app, &campaign, &lyra).await;
    assert_eq!(play["totalXp"], 1);
    assert_eq!(play["xpBar"], 1);
    assert_eq!(play["hitPoints"], 7);
    assert_eq!(play["resources"][0]["amount"], 15);
    let names: Vec<&str> = play["inventory"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "Sabre d'abordage",
            "Dague de ceinture",
            "Fiole de rhum fortifiant",
            "Lanterne du phare"
        ]
    );
    let rum = &play["inventory"][2];
    assert_eq!(
        (rum["qty"].as_u64(), rum["consumable"].as_bool()),
        (Some(2), Some(true))
    );
    assert_eq!(play["inventory"][3]["description"], "Elle ne s'éteint pas.");

    // Taking back more than she has is refused, and changes nothing.
    let entry = rum["key"].as_str().unwrap();
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &lyra,
        json!({ "kind": "takeItem", "entry": entry, "qty": 3 }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "NOT_ENOUGH");
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &lyra,
        json!({ "kind": "takeItem", "entry": entry }),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &lyra,
        json!({ "kind": "resource", "resource": "or", "delta": -16 }),
    )
    .await;
    assert_eq!(r.body["error"]["code"], "NOT_ENOUGH");

    // Five XP: level 2 and an upgrade point, as the rules say.
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &lyra,
        json!({ "kind": "xp", "delta": 4 }),
    )
    .await;
    assert_eq!(r.body["data"]["play"]["level"], 2, "{}", r.body);
    assert_eq!(r.body["data"]["play"]["upgradePoints"], 1);
    assert_eq!(r.body["data"]["play"]["xpBar"], 0);
    assert_eq!(r.body["data"]["play"]["nextLevelXp"], 10);

    // The history keeps every change, newest first; refused ones are not in it.
    let history = board(&app, &gm, &campaign).await["history"].clone();
    let lines: Vec<(&str, Option<&str>, i64, i64)> = history
        .as_array()
        .unwrap()
        .iter()
        .map(|h| {
            assert_eq!(h["actor"], "gm");
            assert_eq!(h["characterId"], lyra.character.as_str());
            (
                h["kind"].as_str().unwrap(),
                h["label"].as_str(),
                h["before"].as_i64().unwrap(),
                h["after"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        lines,
        [
            ("xp", None, 1, 5),
            ("item", Some("Fiole de rhum fortifiant"), 2, 1),
            ("item", Some("Lanterne du phare"), 0, 1),
            ("item", Some("Fiole de rhum fortifiant"), 0, 2),
            ("resource", Some("Pièces d'or"), 10, 15),
            ("hit_points", None, 10, 7),
            ("xp", None, 0, 1),
        ]
    );
}

#[tokio::test]
async fn only_a_validated_character_of_ones_own_table_is_adjusted() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let (_, other) = common::signed_in_gm(&pool, "Hugo").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let (waiting, _) = submitted(&app, &pool, &code, "Marc", lyra()).await;

    // Still waiting for the GM: not in play, not on the board.
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &waiting,
        json!({ "kind": "xp", "delta": 1 }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "CHARACTER_NOT_VALIDATED");
    assert_eq!(board(&app, &gm, &campaign).await["sheets"], json!([]));
    let me = call_as_player(
        &app,
        Some(&waiting.token),
        "GET",
        &format!("/api/play/{campaign}/me"),
        None,
    )
    .await;
    assert_eq!(me.body["data"]["character"]["play"], Value::Null);

    let lyra = validated(&app, &pool, &gm, &campaign, &code, "Camille", lyra()).await;
    // Another GM, an unknown character: 404, like nothing there.
    let r = adjust(
        &app,
        &other,
        &campaign,
        &lyra,
        json!({ "kind": "xp", "delta": 1 }),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let ghost = Seat {
        character: Uuid::new_v4().to_string(),
        token: String::new(),
    };
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &ghost,
        json!({ "kind": "xp", "delta": 1 }),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    // A gesture the rules do not know.
    for body in [
        json!({ "kind": "xp", "delta": 0 }),
        json!({ "kind": "resource", "resource": "credits", "delta": 1 }),
        json!({ "kind": "giveItem", "item": "epee_laser" }),
        json!({ "kind": "giveItem", "name": "  " }),
        json!({ "kind": "levelUp" }),
    ] {
        let r = adjust(&app, &gm, &campaign, &lyra, body.clone()).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body}");
    }
    assert_eq!(my_play(&app, &campaign, &lyra).await["totalXp"], 0);
}

#[tokio::test]
async fn two_gestures_at_once_both_count() {
    let pool = common::test_pool_sized(6).await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let lyra = validated(&app, &pool, &gm, &campaign, &code, "Camille", lyra()).await;
    let xp = json!({ "kind": "xp", "delta": 1 });
    let hurt = json!({ "kind": "hitPoints", "delta": -1 });
    let (a, b, c, d) = tokio::join!(
        adjust(&app, &gm, &campaign, &lyra, xp.clone()),
        adjust(&app, &gm, &campaign, &lyra, xp),
        adjust(&app, &gm, &campaign, &lyra, hurt.clone()),
        adjust(&app, &gm, &campaign, &lyra, hurt),
    );
    for r in [a, b, c, d] {
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }
    let play = my_play(&app, &campaign, &lyra).await;
    assert_eq!(
        (play["totalXp"].as_i64(), play["hitPoints"].as_i64()),
        (Some(2), Some(8))
    );
}

#[tokio::test]
async fn the_player_chooses_what_she_carries_and_the_gm_sees_it() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let lyra = validated(&app, &pool, &gm, &campaign, &code, "Camille", lyra()).await;
    let uri = format!("/api/play/{campaign}/character/equip");
    let equip = |entry: &str, equipped: bool| {
        call_as_player(
            &app,
            Some(&lyra.token),
            "POST",
            &uri,
            Some(json!({ "entry": entry, "equipped": equipped })),
        )
    };
    let r = equip("sabre_d_abordage", true).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["play"]["inventory"][0]["equipped"], true);
    let r = equip("nowhere", true).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "NO_SUCH_ENTRY");

    let b = board(&app, &gm, &campaign).await;
    assert_eq!(b["sheets"][0]["play"]["inventory"][0]["equipped"], true);
    let line = &b["history"][0];
    assert_eq!(
        (line["actor"].as_str(), line["kind"].as_str()),
        (Some("player"), Some("equip"))
    );
    assert_eq!(line["label"], "Sabre d'abordage");
    // The GM's gestures keep what she carries.
    adjust(
        &app,
        &gm,
        &campaign,
        &lyra,
        json!({ "kind": "xp", "delta": 1 }),
    )
    .await;
    assert_eq!(
        my_play(&app, &campaign, &lyra).await["inventory"][0]["equipped"],
        true
    );
}

#[tokio::test]
async fn a_brasier_table_plays_the_same_without_a_purse() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, BRASIER).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let pilote = validated(
        &app,
        &pool,
        &gm,
        &campaign,
        &code,
        "Inès",
        json!({ "name": "Vex", "classId": "pilote" }),
    )
    .await;
    let play = my_play(&app, &campaign, &pilote).await;
    assert_eq!(
        play["resources"],
        json!([]),
        "the Brasier rules name no currency"
    );
    assert_eq!(play["inventory"][0]["name"], "Pistolet laser de précision");
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &pilote,
        json!({ "kind": "giveItem", "item": "stim_de_combat" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &pilote,
        json!({ "kind": "hitPoints", "delta": -2 }),
    )
    .await;
    assert_eq!(r.body["data"]["play"]["hitPoints"], 8);
    let r = adjust(
        &app,
        &gm,
        &campaign,
        &pilote,
        json!({ "kind": "resource", "resource": "or", "delta": 5 }),
    )
    .await;
    assert_eq!(r.body["error"]["code"], "UNKNOWN_RESOURCE");
}
