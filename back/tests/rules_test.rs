//! player/read-the-rules — done when a player who never read the rules
//! explains, after their first fight, why their attack missed.
//!
//! The rules page is read from the campaign's rule system on both
//! witness worlds: the four outcomes, every card's cost and cooldown, and
//! with a sheet, what the player needs on the die and the attack roll of
//! each card. What changed since the version a player last read stays
//! on the page until they say they read it.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::marked::FIXTURE;
use common::{Reply, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

struct Seat {
    campaign: String,
    token: String,
    player: Uuid,
}

async fn seat(app: &Router, pool: &PgPool, yaml: &str, nickname: &str, role: &str) -> Seat {
    let (_, gm) = common::signed_in_gm(pool, "Romain").await;
    let campaign = imported_campaign(app, &gm, yaml).await;
    let code = invite_code(app, &gm, &campaign).await;
    let r = join(app, &code, nickname, role).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    Seat {
        campaign,
        token: r.player_token().unwrap(),
        player: Uuid::parse_str(r.body["data"]["me"]["id"].as_str().unwrap()).unwrap(),
    }
}

async fn play(app: &Router, s: &Seat, method: &str, path: &str, body: Option<Value>) -> Reply {
    call_as_player(
        app,
        Some(&s.token),
        method,
        &format!("/api/play/{}{path}", s.campaign),
        body,
    )
    .await
}

/// Gives the seat's sheet the first class the creator offers.
async fn pick_first_class(app: &Router, s: &Seat) -> String {
    let r = play(app, s, "GET", "/creation", None).await;
    let class = r.body["data"]["rules"]["classes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = play(
        app,
        s,
        "PUT",
        "/character",
        Some(json!({ "name": "Borin", "classId": class })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    class
}

#[tokio::test]
async fn both_worlds_explain_rolls_cards_and_what_a_player_needs() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    for yaml in [FIXTURE, BRASIER] {
        let marc = seat(&app, &pool, yaml, "Marc", "player").await;

        // Before any class: the rules, without numbers of my own.
        let r = play(&app, &marc, "GET", "/rules", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let page = &r.body["data"];
        let outcomes = page["outcomes"].as_array().unwrap();
        assert_eq!(outcomes.len(), 4);
        assert_eq!(outcomes[0]["band"], "critical_failure");
        assert_eq!(outcomes[3]["band"], "critical_success");
        assert!(page["mine"].is_null());
        assert!(page["changes"].is_null(), "a new seat reads the whole page");

        pick_first_class(&app, &marc).await;
        let r = play(&app, &marc, "GET", "/rules", None).await;
        let page = &r.body["data"];
        let mine = &page["mine"];
        let cards = mine["cards"].as_array().unwrap();
        assert!(!cards.is_empty());
        for c in cards {
            assert!(c["cost"].as_u64().unwrap() >= 1, "{c}");
            assert!(c["cooldown"].is_u64(), "{c}");
        }
        // An attacking card says what its roll adds, and where from.
        let attack = cards
            .iter()
            .find(|c| !c["attackModifiers"].is_null())
            .expect("a class with an attack");
        assert_eq!(attack["attackModifiers"][0]["source"]["from"], "ability");
        // Worked rolls: the engine's breakdown, total = face + modifiers.
        let examples = mine["examples"].as_array().unwrap();
        assert!(examples.len() >= 3, "{examples:?}");
        for e in examples {
            let mods: i64 = e["modifiers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| m["value"].as_i64().unwrap())
                .sum();
            assert_eq!(
                e["total"].as_i64().unwrap(),
                e["natural"].as_i64().unwrap() + mods
            );
            assert_eq!(e["target"]["against"], "difficulty");
        }
        // Each ability says what it needs on the die, per difficulty.
        let difficulties = page["difficulties"].as_array().unwrap().len();
        for a in page["abilities"].as_array().unwrap() {
            assert_eq!(a["needs"].as_array().unwrap().len(), difficulties, "{a}");
        }
    }
}

#[tokio::test]
async fn changes_stay_until_the_player_says_they_read_them() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let marc = seat(&app, &pool, FIXTURE, "Marc", "player").await;
    let lea = seat(&app, &pool, FIXTURE, "Léa", "spectator").await;

    // The seat was given the campaign's version when it was taken.
    let seen: (String, i32) =
        sqlx::query_as("SELECT rule_system_id, version FROM rules_seen WHERE player_id = $1")
            .bind(marc.player)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(seen, ("corsaires".to_string(), 1));

    // Marc last read a version the server no longer has: the whole page
    // is to read again.
    sqlx::query("UPDATE rules_seen SET version = 0 WHERE player_id = $1")
        .bind(marc.player)
        .execute(&pool)
        .await
        .unwrap();
    let r = play(&app, &marc, "GET", "/rules", None).await;
    let changes = &r.body["data"]["changes"];
    assert_eq!(changes["fromVersion"], 0);
    assert_eq!(changes["toVersion"], 1);
    assert_eq!(changes["replaced"], true);
    // Reading the page changes nothing; saying so does.
    let r = play(&app, &marc, "GET", "/rules", None).await;
    assert!(!r.body["data"]["changes"].is_null());
    let r = play(&app, &marc, "POST", "/rules/seen", None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(r.body["data"]["changes"].is_null());
    let r = play(&app, &marc, "GET", "/rules", None).await;
    assert!(r.body["data"]["changes"].is_null());

    // Another campaign's system read before: replaced too. Léa, a
    // spectator, reads the rules as well, without numbers.
    sqlx::query("UPDATE rules_seen SET rule_system_id = 'brasier' WHERE player_id = $1")
        .bind(lea.player)
        .execute(&pool)
        .await
        .unwrap();
    let r = play(&app, &lea, "GET", "/rules", None).await;
    assert_eq!(r.body["data"]["changes"]["replaced"], true);
    assert!(r.body["data"]["mine"].is_null());
    let r = play(&app, &lea, "POST", "/rules/seen", None).await;
    assert!(r.body["data"]["changes"].is_null());
}
