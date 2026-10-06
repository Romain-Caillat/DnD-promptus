//! characters/build-character-creator — done when a player creates
//! their character from a phone and finds it again on their sheet.
//!
//! The creator reads what to offer from the server (the campaign's
//! sprite pack and rule system, through the projection), saves the
//! sheet as a draft while the player goes, and sends it to the GM. Going
//! over the rules' ability budget is flagged, never refused; once sent,
//! the sheet is the GM's until they return it with a note.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::marked::FIXTURE;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use promptus_back::campaigns::projection::project_creation;
use promptus_back::content;
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::model::Tag;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

struct Seat {
    campaign: String,
    token: String,
    character: Uuid,
}

/// Romain's campaign from `yaml`, and `nickname` seated at it.
async fn seat(app: &Router, pool: &PgPool, yaml: &str, nickname: &str, role: &str) -> Seat {
    let (_, gm) = common::signed_in_gm(pool, "Romain").await;
    let campaign = imported_campaign(app, &gm, yaml).await;
    let code = invite_code(app, &gm, &campaign).await;
    let r = join(app, &code, nickname, role).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let character = r.body["data"]["character"]["id"]
        .as_str()
        .map(|id| Uuid::parse_str(id).unwrap())
        .unwrap_or_default();
    Seat {
        campaign,
        token: r.player_token().unwrap(),
        character,
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

/// Borin as Marc builds him: a bretteur with 18 in FOR, over the
/// class's 64-point spread.
fn borin(look: Value) -> Value {
    json!({
        "name": "  Borin ",
        "classId": "bretteur",
        "abilities": { "FOR": 18, "DEX": 14, "CON": 10, "INT": 8, "SAG": 10, "CHA": 9 },
        "look": look,
        "backstory": {
            "origin": "De la mine d'argent de Valombre.",
            "loss": "Son frère Dorn, dans la mine.",
            "quest": "Pourquoi la mine paie encore.",
        },
    })
}

async fn live_version(pool: &PgPool, s: &Seat) -> i64 {
    sqlx::query_scalar("SELECT version FROM live_versions WHERE topic = $1")
        .bind(format!("character:{}", s.character))
        .fetch_optional(pool)
        .await
        .unwrap()
        .unwrap_or(0)
}

#[tokio::test]
async fn marc_builds_his_character_sends_it_and_finds_it_on_his_sheet() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let s = seat(&app, &pool, FIXTURE, "Marc", "player").await;

    // What the creator offers comes from the campaign: Corsaires rules,
    // the 1718 sailors' pack, a look to start from.
    let r = play(&app, &s, "GET", "/creation", None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let creation = &r.body["data"];
    assert_eq!(creation["pack"], "marins-1718");
    assert_eq!(creation["startLook"]["pack"], "marins-1718");
    let rules = &creation["rules"];
    assert_eq!(rules["abilities"].as_array().unwrap().len(), 6);
    assert_eq!(rules["peoples"], json!([]), "the Corsaires have no peoples");
    let bretteur = rules["classes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "bretteur")
        .unwrap();
    assert_eq!(bretteur["budget"], 64);
    // Its cards, as dealt at the table: Estocade rolls FOR 13, +1 (the
    // first primary; precision is not added in this system); the
    // level-3 card waits.
    let cards = bretteur["stats"]["cards"].as_array().unwrap();
    let estocade = cards.iter().find(|c| c["id"] == "estocade").unwrap();
    assert_eq!(estocade["attackBonus"], 1);
    assert_eq!(estocade["damage"], "3");
    assert!(cards.iter().any(|c| c["level"] == 3));

    // The pieces and palettes of that pack, without an account.
    let r = call(&app, None, "GET", "/api/sprites/packs/marins-1718", None).await;
    assert_eq!(r.status, StatusCode::OK);
    let slots = &r.body["data"]["slots"];
    assert!(
        slots["body"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "robuste")
    );
    assert!(
        slots["outfit"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "gilet" && p["dyed"] == true)
    );
    assert!(
        !r.body["data"]["palettes"]["cloth"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let r = call(&app, None, "GET", "/api/sprites/packs/nope", None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // Marc saves as he goes: the look, the class, 18 in Force — over
    // the budget, and kept. Whoever shows his character hears of it.
    let before = live_version(&pool, &s).await;
    let look = json!({
        "pack": "marins-1718", "body": "robuste", "skin": "hale",
        "hair": { "style": "court", "colour": "roux" }, "beard": "longue",
        "headwear": { "piece": "bonnet", "dye": "rouge" },
        "outfit": { "piece": "gilet", "dye": "bleu", "accent": "or" },
        "weapon": { "piece": "sabre" },
    });
    let r = play(&app, &s, "PUT", "/character", Some(borin(look.clone()))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "draft");
    assert_eq!(r.body["data"]["sheet"]["name"], "Borin");
    assert_eq!(r.body["data"]["sheet"]["abilities"]["FOR"], 18);
    // The numbers come from the rules, for his scores: 10 HP, AC 10 + mod(DEX).
    assert_eq!(r.body["data"]["stats"]["hitPoints"], 10);
    assert_eq!(r.body["data"]["stats"]["armorClass"], 12);
    assert_eq!(r.body["data"]["stats"]["modifiers"]["FOR"], 4);
    assert!(live_version(&pool, &s).await > before);

    // He sends it; it is the GM's now.
    let r = play(&app, &s, "POST", "/character/submit", None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "submitted");
    let r = play(&app, &s, "PUT", "/character", Some(borin(look.clone()))).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "CHARACTER_LOCKED");
    let r = play(&app, &s, "POST", "/character/submit", None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);

    // The next day, on his sheet: the same character, drawn the same.
    let r = play(&app, &s, "GET", "/me", None).await;
    let sheet = &r.body["data"]["character"]["sheet"];
    assert_eq!(sheet["name"], "Borin");
    assert_eq!(sheet["classId"], "bretteur");
    assert_eq!(sheet["look"], look);
    assert_eq!(sheet["backstory"]["loss"], "Son frère Dorn, dans la mine.");

    // The GM sends it back with a word: Marc reads it, fixes, resends.
    sqlx::query("UPDATE characters SET status = 'returned', gm_note = $2 WHERE id = $1")
        .bind(s.character)
        .bind("La Force s'arrête à 17.")
        .execute(&pool)
        .await
        .unwrap();
    let r = play(&app, &s, "GET", "/me", None).await;
    assert_eq!(r.body["data"]["character"]["status"], "returned");
    assert_eq!(
        r.body["data"]["character"]["gmNote"],
        "La Force s'arrête à 17."
    );
    let mut fixed = borin(look);
    fixed["abilities"]["FOR"] = json!(17);
    let r = play(&app, &s, "PUT", "/character", Some(fixed)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = play(&app, &s, "POST", "/character/submit", None).await;
    assert_eq!(r.body["data"]["status"], "submitted");
    assert_eq!(r.body["data"]["sheet"]["abilities"]["FOR"], 17);
}

#[tokio::test]
async fn a_sheet_names_only_what_the_campaign_has() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let s = seat(&app, &pool, FIXTURE, "Marc", "player").await;
    let look = json!({ "pack": "marins-1718", "body": "svelte", "skin": "pale", "hair": { "colour": "noir" } });

    // Not complete yet: no name, no look, no class.
    let r = play(&app, &s, "POST", "/character/submit", None).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "CHARACTER_INCOMPLETE");

    let refused = [
        (json!({ "classId": "pretre" }), "UNKNOWN_CLASS"),
        (json!({ "peopleId": "nain" }), "UNKNOWN_PEOPLE"),
        (json!({ "abilities": { "FOI": 12 } }), "UNKNOWN_ABILITY"),
        (
            json!({ "abilities": { "FOR": 99 } }),
            "ABILITY_OUT_OF_RANGE",
        ),
        (json!({ "name": "B".repeat(61) }), "INVALID_NAME"),
        (
            json!({ "backstory": { "text": "x".repeat(4001) } }),
            "TEXT_TOO_LONG",
        ),
        // The Brasier's pack is not this campaign's.
        (
            json!({ "look": { "pack": "equipage-spatial", "body": "svelte", "skin": "pale", "hair": { "colour": "noir" } } }),
            "SPRITE_WRONG_PACK",
        ),
        (
            json!({ "look": { "pack": "marins-1718", "body": "svelte", "skin": "pale", "hair": { "colour": "noir" }, "weapon": "faux" } }),
            "SPRITE_UNKNOWN_PIECE",
        ),
        (
            json!({ "look": { "pack": "marins-1718", "body": "svelte", "skin": "vert", "hair": { "colour": "noir" } } }),
            "SPRITE_UNKNOWN_COLOUR",
        ),
    ];
    for (body, code) in refused {
        let r = play(&app, &s, "PUT", "/character", Some(body.clone())).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(r.body["error"]["code"], code, "{body}");
    }

    // Name and look, but no class while the rules have classes.
    let r = play(
        &app,
        &s,
        "PUT",
        "/character",
        Some(json!({ "name": "Borin", "look": look })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = play(&app, &s, "POST", "/character/submit", None).await;
    assert_eq!(r.body["error"]["code"], "CHARACTER_INCOMPLETE");
    assert!(
        r.body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("classId")
    );

    // A spectator has no character to write.
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let code = invite_code(&app, &gm, &imported_campaign(&app, &gm, FIXTURE).await).await;
    let lea = join(&app, &code, "Léa", "spectator").await;
    let campaign = lea.body["data"]["campaign"]["campaignId"].as_str().unwrap();
    let r = call_as_player(
        &app,
        lea.player_token().as_deref(),
        "PUT",
        &format!("/api/play/{campaign}/character"),
        Some(json!({ "name": "Léa" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "NO_CHARACTER");
}

#[tokio::test]
async fn a_brasier_player_builds_from_the_space_crew_pack() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let s = seat(&app, &pool, BRASIER, "Sef", "player").await;

    let r = play(&app, &s, "GET", "/creation", None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["pack"], "equipage-spatial");
    let classes = r.body["data"]["rules"]["classes"].as_array().unwrap();
    assert!(!classes.is_empty());
    let class = classes[0]["id"].as_str().unwrap();
    let look = r.body["data"]["startLook"].clone();
    let r = play(
        &app,
        &s,
        "PUT",
        "/character",
        Some(json!({ "name": "Sef", "classId": class, "look": look })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = play(&app, &s, "POST", "/character/submit", None).await;
    assert_eq!(r.body["data"]["status"], "submitted", "{}", r.body);
}

#[test]
fn the_rules_reach_players_without_the_gm_notes() {
    let mark = "GMONLY<rules>";
    let mut rules =
        RuleSystem::from_yaml(include_str!("../../content/rules/brasier/v1.yaml")).unwrap();
    rules.description = mark.into();
    rules.sources = vec![mark.into()];
    rules.creation.note = mark.into();
    for class in &mut rules.classes {
        class.notes = vec![mark.into()];
        for action in &mut class.actions {
            action.tags.push(Tag::Note(mark.into()));
        }
    }
    let look = content::start_look(&promptus_shared::story::from_yaml(BRASIER).unwrap());
    let view = project_creation("equipage-spatial", look, Some(&rules));
    let json = serde_json::to_string(&view).unwrap();
    assert!(!json.contains(mark), "{json}");
    // Not vacuous: every class and its cards went through.
    let projected = view.rules.unwrap();
    assert_eq!(projected.classes.len(), rules.classes.len());
    assert!(projected.classes.iter().all(|c| c.stats.is_some()));
}
