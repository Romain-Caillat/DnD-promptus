//! session/validate-characters — done when moments 3 to 6 of the
//! « Inviter » board can be played in the app: the GM reads each sheet
//! with the rules' checks, validates it or returns it with a word, reads
//! only the difference once the player sends it again, and keeps secret
//! hooks drawn from the backstories.
//!
//! The player's side (saving and submitting a sheet) belongs to
//! `characters/build-character-creator`: here a submission is written
//! straight in the database, as that ticket's submit will.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::marked::FIXTURE;
use common::{call, call_as_player, imported_campaign, invite_code, join};
use promptus_back::live;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

/// Bretteur scores in the Corsaires rules.
fn bretteur(force: i32, sagesse: i32) -> Value {
    json!({ "FOR": force, "DEX": 14, "CON": 10, "INT": 8, "SAG": sagesse, "CHA": 9 })
}

struct Seat {
    character: String,
    token: String,
}

/// `nickname` joins and writes `sheet`, then sends it to the GM.
async fn seated(app: &Router, pool: &PgPool, code: &str, nickname: &str, sheet: Value) -> Seat {
    let r = join(app, code, nickname, "player").await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let character = r.body["data"]["character"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    submit(pool, &character, sheet).await;
    Seat {
        character,
        token: r.player_token().unwrap(),
    }
}

/// What the creator's « send to the GM » does: the sheet, submitted.
async fn submit(pool: &PgPool, character: &str, sheet: Value) {
    sqlx::query("UPDATE characters SET sheet = $2, status = 'submitted' WHERE id = $1")
        .bind(Uuid::parse_str(character).unwrap())
        .bind(sheet)
        .execute(pool)
        .await
        .unwrap();
}

async fn get(app: &Router, gm: &str, uri: &str) -> Value {
    let r = call(app, Some(gm), "GET", uri, None).await;
    assert_eq!(r.status, StatusCode::OK, "{uri}: {}", r.body);
    r.body["data"].clone()
}

async fn table_version(pool: &PgPool, campaign: &str) -> i64 {
    live::versions(pool, Uuid::parse_str(campaign).unwrap())
        .await
        .unwrap()
        .get("table")
        .copied()
        .unwrap_or(0)
}

#[tokio::test]
async fn romain_validates_lyra_returns_borin_then_reads_only_what_marc_changed() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let base = format!("/api/campaigns/{campaign}");

    // The table fills up: each arrival moves the table's live topic.
    let before = table_version(&pool, &campaign).await;
    let lyra = seated(
        &app,
        &pool,
        &code,
        "Camille",
        json!({ "name": "Lyra", "classId": "bretteur", "abilities": bretteur(13, 10),
                "backstory": "Les oiseaux ont quitté la colline." }),
    )
    .await;
    assert!(table_version(&pool, &campaign).await > before);

    // Moment 3: Lyra follows the rules; the GM validates her.
    let seats = get(&app, &gm, &format!("{base}/players")).await;
    assert_eq!(seats[0]["character"]["status"], "submitted");
    assert_eq!(seats[0]["character"]["className"], "Bretteur");
    let review = get(&app, &gm, &format!("{base}/characters/{}", lyra.character)).await;
    assert_eq!(review["nickname"], "Camille");
    assert_eq!(review["checks"], json!([]), "{review}");
    assert_eq!(
        review["sheet"]["backstory"],
        "Les oiseaux ont quitté la colline."
    );
    assert_eq!(review["abilities"][0]["name"], "Force");
    let before = table_version(&pool, &campaign).await;
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("{base}/characters/{}/validate", lyra.character),
        Some(json!({ "seen": review["updatedAt"] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    assert!(table_version(&pool, &campaign).await > before);
    let me = call_as_player(
        &app,
        Some(&lyra.token),
        "GET",
        &format!("/api/play/{campaign}/me"),
        None,
    )
    .await;
    assert_eq!(me.body["data"]["character"]["status"], "validated");

    // Moment 4: Borin has Force 18; the check says so, without blocking.
    let borin = seated(
        &app,
        &pool,
        &code,
        "Marc",
        json!({ "name": "Borin", "classId": "bretteur", "abilities": bretteur(18, 10),
                "backstory": "Son frère Dorn n'est jamais remonté de la mine." }),
    )
    .await;
    let uri = format!("{base}/characters/{}", borin.character);
    let review = get(&app, &gm, &uri).await;
    let codes: Vec<&str> = review["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["ABILITY_OFF_CLASS", "ABILITY_BUDGET_EXCEEDED"]);
    assert!(
        review["checks"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Force à 18"),
        "{review}"
    );

    // A word is required to return a sheet.
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("{uri}/return"),
        Some(json!({ "seen": review["updatedAt"], "note": "  " })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "NOTE_REQUIRED");

    let note =
        "Super Borin ! La Force s'arrête à 13 pour un bretteur. Tu peux mettre le point ailleurs ?";
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("{uri}/return"),
        Some(json!({ "seen": review["updatedAt"], "note": note })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);

    // Marc reads the word on his phone.
    let me = call_as_player(
        &app,
        Some(&borin.token),
        "GET",
        &format!("/api/play/{campaign}/me"),
        None,
    )
    .await;
    assert_eq!(me.body["data"]["character"]["status"], "returned");
    assert_eq!(me.body["data"]["character"]["gmNote"], note);
    // Returned, it is not waiting for a decision any more.
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("{uri}/validate"),
        Some(json!({ "seen": review["updatedAt"] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "CHARACTER_NOT_SUBMITTED");

    // Moment 5: Marc moves the point to Sagesse and sends it again. The
    // GM sees two changes and nothing else.
    submit(
        &pool,
        &borin.character,
        json!({ "name": "Borin", "classId": "bretteur", "abilities": bretteur(17, 11),
                "backstory": "Son frère Dorn n'est jamais remonté de la mine." }),
    )
    .await;
    let seats = get(&app, &gm, &format!("{base}/players")).await;
    let marc = seats
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["nickname"] == "Marc")
        .unwrap();
    assert_eq!(marc["character"]["resubmitted"], true);
    let review = get(&app, &gm, &uri).await;
    assert_eq!(
        review["changes"],
        json!([
            { "path": "abilities.FOR", "before": 18, "after": 17 },
            { "path": "abilities.SAG", "before": 10, "after": 11 },
        ])
    );
    assert_eq!(review["reviewedSheet"]["abilities"]["FOR"], 18);
    assert_eq!(review["gmNote"], note);

    // A decision on a sheet that changed since it was read is refused.
    let stale = review["updatedAt"].clone();
    submit(
        &pool,
        &borin.character,
        json!({ "name": "Borin", "classId": "bretteur", "abilities": bretteur(13, 15) }),
    )
    .await;
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("{uri}/validate"),
        Some(json!({ "seen": stale })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "CHARACTER_CHANGED");

    // The GM has the last word: validated despite the checks.
    let review = get(&app, &gm, &uri).await;
    assert!(!review["checks"].as_array().unwrap().is_empty());
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("{uri}/validate"),
        Some(json!({ "seen": review["updatedAt"] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let me = call_as_player(
        &app,
        Some(&borin.token),
        "GET",
        &format!("/api/play/{campaign}/me"),
        None,
    )
    .await;
    assert_eq!(me.body["data"]["character"]["status"], "validated");
    assert_eq!(me.body["data"]["character"]["gmNote"], Value::Null);
}

#[tokio::test]
async fn the_gm_keeps_secret_hooks_tied_to_the_story() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let borin = seated(
        &app,
        &pool,
        &code,
        "Marc",
        json!({ "name": "Borin", "backstory": "Son frère Dorn a disparu." }),
    )
    .await;
    let hooks = format!("/api/campaigns/{campaign}/hooks");

    // The story's nodes and fronts are what a hook can tie into.
    let data = get(&app, &gm, &hooks).await;
    assert_eq!(data["hooks"], json!([]));
    let targets = data["targets"].as_array().unwrap();
    assert!(
        targets
            .iter()
            .any(|t| t["id"] == "sc_crique" && t["kind"] == "node")
    );
    assert!(
        targets
            .iter()
            .any(|t| t["id"] == "front_naufrageurs" && t["kind"] == "front")
    );

    let r = call(
        &app,
        Some(&gm),
        "POST",
        &hooks,
        Some(json!({
            "characterId": borin.character,
            "title": " Dorn, le frère de Borin ",
            "body": "Il est parmi les naufrageurs.",
            "links": ["sc_crique", "front_naufrageurs", "sc_crique"],
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let hook = r.body["data"]["id"].as_str().unwrap().to_string();
    assert_eq!(r.body["data"]["title"], "Dorn, le frère de Borin");
    assert_eq!(
        r.body["data"]["links"],
        json!(["sc_crique", "front_naufrageurs"])
    );

    // Refused: no title, a link to nothing, a character of no table here.
    for (body, code) in [
        (
            json!({ "characterId": borin.character, "title": " " }),
            "HOOK_TITLE_REQUIRED",
        ),
        (
            json!({ "characterId": borin.character, "title": "X", "links": ["sc_nulle_part"] }),
            "HOOK_LINK_UNKNOWN",
        ),
    ] {
        let r = call(&app, Some(&gm), "POST", &hooks, Some(body)).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST);
        assert_eq!(r.body["error"]["code"], code);
    }
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &hooks,
        Some(json!({ "characterId": Uuid::new_v4(), "title": "X" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    let r = call(
        &app,
        Some(&gm),
        "PUT",
        &format!("{hooks}/{hook}"),
        Some(json!({ "title": "Dorn", "body": "Il guide les naufrageurs.", "links": [] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let data = get(&app, &gm, &hooks).await;
    assert_eq!(data["hooks"][0]["body"], "Il guide les naufrageurs.");
    assert_eq!(data["hooks"][0]["characterId"], borin.character.as_str());

    // Marc's phone shows nothing of it.
    let me = call_as_player(
        &app,
        Some(&borin.token),
        "GET",
        &format!("/api/play/{campaign}/me"),
        None,
    )
    .await;
    assert!(!me.body.to_string().contains("naufrageurs"), "{}", me.body);

    let r = call(&app, Some(&gm), "DELETE", &format!("{hooks}/{hook}"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let r = call(&app, Some(&gm), "DELETE", &format!("{hooks}/{hook}"), None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn another_gm_reads_and_decides_nothing() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let (_, other) = common::signed_in_gm(&pool, "Marc le MJ").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let borin = seated(&app, &pool, &code, "Marc", json!({ "name": "Borin" })).await;
    let base = format!("/api/campaigns/{campaign}");
    let review = get(&app, &gm, &format!("{base}/characters/{}", borin.character)).await;
    let r = call(
        &app,
        Some(&gm),
        "POST",
        &format!("{base}/hooks"),
        Some(json!({ "characterId": borin.character, "title": "Dorn" })),
    )
    .await;
    let hook = r.body["data"]["id"].as_str().unwrap().to_string();

    // The other GM's own campaign does not open Romain's character.
    let theirs = imported_campaign(&app, &other, FIXTURE).await;
    let seen = json!({ "seen": review["updatedAt"], "note": "Non." });
    let character = &borin.character;
    for (method, uri, body) in [
        ("GET", format!("{base}/characters/{character}"), None),
        (
            "POST",
            format!("{base}/characters/{character}/validate"),
            Some(json!({ "seen": review["updatedAt"] })),
        ),
        (
            "POST",
            format!("{base}/characters/{character}/return"),
            Some(seen.clone()),
        ),
        ("GET", format!("{base}/hooks"), None),
        (
            "POST",
            format!("{base}/hooks"),
            Some(json!({ "characterId": character, "title": "X" })),
        ),
        (
            "PUT",
            format!("{base}/hooks/{hook}"),
            Some(json!({ "title": "X" })),
        ),
        ("DELETE", format!("{base}/hooks/{hook}"), None),
        (
            "GET",
            format!("/api/campaigns/{theirs}/characters/{character}"),
            None,
        ),
        (
            "POST",
            format!("/api/campaigns/{theirs}/characters/{character}/return"),
            Some(seen.clone()),
        ),
        (
            "PUT",
            format!("/api/campaigns/{theirs}/hooks/{hook}"),
            Some(json!({ "title": "X" })),
        ),
        (
            "DELETE",
            format!("/api/campaigns/{theirs}/hooks/{hook}"),
            None,
        ),
        (
            "POST",
            format!("/api/campaigns/{theirs}/hooks"),
            Some(json!({ "characterId": character, "title": "X" })),
        ),
    ] {
        let r = call(&app, Some(&other), method, &uri, body).await;
        assert_eq!(
            r.status,
            StatusCode::NOT_FOUND,
            "{method} {uri}: {}",
            r.body
        );
    }
    let after = get(&app, &gm, &format!("{base}/characters/{character}")).await;
    assert_eq!(after["status"], "submitted");
    let hooks = get(&app, &gm, &format!("{base}/hooks")).await;
    assert_eq!(hooks["hooks"][0]["title"], "Dorn");
    assert_eq!(hooks["hooks"].as_array().unwrap().len(), 1);
}
