//! session/project-player-view — done when a test walks every player
//! and shared-screen route.
//!
//! The routes come from the router itself (`app::player_facing_routes`,
//! the list `app::router` mounts), so a route cannot be added without
//! being swept. The campaign is the fixture with every GM-only field
//! marked (`common::marked`), in the world that shows the most; each
//! route is called by a player and by a spectator, must succeed (so the
//! sweep is not vacuous), and must carry no marker, no hidden hit points
//! and no clue or revelation id. A player never sees another player's
//! sheet. Without the device token of a player of that campaign — or
//! with a GM session — every seated route answers 401.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::marked::{ITEM_NOTE, leaks, m, mark_review, marked, world};
use common::{call, call_as_player, imported_campaign, invite_code, join, send};
use promptus_back::app::player_facing_routes;
use promptus_shared::story::to_yaml;
use serde_json::{Value, json};
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

/// Text only Marc's own sheet holds.
const MARC_SECRET: &str = "OTHERPLAYER<Marc's sheet>";

struct Table {
    gm: String,
    campaign: String,
    code: String,
    marc: String,
    spectator: String,
    /// An approved image the table may see.
    asset: String,
    /// A date proposed for the next session.
    date: String,
}

/// Romain's marked campaign mid-scene, Marc seated as a player with a
/// written sheet the GM reviewed and drew a secret hook from, Léa as a
/// spectator.
async fn marked_table(app: &Router, pool: &PgPool) -> Table {
    let (_, gm) = common::signed_in_gm(pool, "Romain").await;
    let story = marked();
    let campaign = imported_campaign(app, &gm, &to_yaml(&story).unwrap()).await;
    // A budget, so the co-GM writing a backstory may run.
    sqlx::query("UPDATE campaigns SET world = $2, ai_budget_cents = 500 WHERE id = $1")
        .bind(Uuid::parse_str(&campaign).unwrap())
        .bind(Json(world(&story)))
        .execute(pool)
        .await
        .unwrap();
    let code = invite_code(app, &gm, &campaign).await;
    let marc = join(app, &code, "Marc", "player").await;
    let marc_id = Uuid::parse_str(marc.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    // Returned by the GM with a word, the sweep's character writes may
    // edit and resend it; what the GM keeps about it (the last review, a
    // hook) must not reach the player.
    let character: Uuid = sqlx::query_scalar(
        "UPDATE characters SET sheet = $2, status = 'returned', gm_note = 'Retouche.'
         WHERE player_id = $1 RETURNING id",
    )
    .bind(marc_id)
    .bind(json!({ "name": "Borin", "appearance": MARC_SECRET }))
    .fetch_one(pool)
    .await
    .unwrap();
    mark_review(pool, Uuid::parse_str(&campaign).unwrap(), character).await;
    let spectator = join(app, &code, "Léa", "spectator").await;
    evening(pool, Uuid::parse_str(&campaign).unwrap()).await;
    board(pool, Uuid::parse_str(&campaign).unwrap(), character).await;
    let asset = media(pool, Uuid::parse_str(&campaign).unwrap()).await;
    let date = schedule(pool, Uuid::parse_str(&campaign).unwrap()).await;
    Table {
        gm,
        campaign,
        code,
        marc: marc.player_token().unwrap(),
        spectator: spectator.player_token().unwrap(),
        asset: asset.to_string(),
        date: date.to_string(),
    }
}

/// The next session: a date fixed, another proposed, the table's Discord
/// webhook and what was sent through it — the last two GM-only, marked.
async fn schedule(pool: &PgPool, campaign: Uuid) -> Uuid {
    let chosen: Uuid = sqlx::query_scalar(
        "INSERT INTO session_dates (campaign_id, starts_at, status, chosen_at)
         VALUES ($1, now() + interval '3 days', 'chosen', now()) RETURNING id",
    )
    .bind(campaign)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO reminder_log (date_id, kind, ok, detail) VALUES ($1, 'chosen', false, $2)",
    )
    .bind(chosen)
    .bind(m("reminder_log.detail"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO campaign_reminders (campaign_id, discord_webhook) VALUES ($1, $2)")
        .bind(campaign)
        .bind(format!(
            "https://discord.com/api/webhooks/1/{}",
            m("reminders.webhook")
        ))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query_scalar(
        "INSERT INTO session_dates (campaign_id, starts_at) VALUES ($1, now() + interval '10 days')
         RETURNING id",
    )
    .bind(campaign)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// An approved tileset image (any player may see it), and images the
/// table may not see: one still pending, one of an NPC nobody has met.
/// Their subjects and the GM's words are marked.
async fn media(pool: &PgPool, campaign: Uuid) -> Uuid {
    let shown: Uuid = sqlx::query_scalar(
        "INSERT INTO media_assets (campaign_id, kind, subject, direction, status, mime, image)
         VALUES ($1, 'tileset', 'pavés', $2, 'approved', 'image/png', '\\x89504e47') RETURNING id",
    )
    .bind(campaign)
    .bind(m("media.direction"))
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO media_assets (campaign_id, kind, subject, status, mime, image)
         VALUES ($1, 'scene', $2, 'pending', 'image/png', '\\x89504e47'),
                ($1, 'npc', $3, 'approved', 'image/png', '\\x89504e47')",
    )
    .bind(campaign)
    .bind(m("media.subject (pending)"))
    .bind(m("media.subject (unmet npc)"))
    .execute(pool)
    .await
    .unwrap();
    shown
}

/// Session 1 ended with a GM recap, its « Précédemment… » published,
/// and a GM-only journal line; session 2 ended with recaps still drafts;
/// session 3 live: what the evening routes need to succeed, and what they must
/// keep from the players.
async fn evening(pool: &PgPool, campaign: Uuid) {
    let first: Uuid = sqlx::query_scalar(
        "INSERT INTO game_sessions (campaign_id, number, status, started_at, ended_at, recap, previously,
                                    chronicle_title, chronicle, published_at)
         VALUES ($1, 1, 'ended', now() - interval '1 week', now() - interval '6 days', $2,
                 'Les corsaires ont accosté.', 'Port-Louis', 'Le quai, puis la taverne.',
                 now() - interval '5 days')
         RETURNING id",
    )
    .bind(campaign)
    .bind(m("sessions.recap"))
    .fetch_one(pool)
    .await
    .unwrap();
    // Session 2's recaps are drafts the GM has not published yet.
    sqlx::query(
        "INSERT INTO game_sessions (campaign_id, number, status, started_at, ended_at, recap, previously,
                                    chronicle_title, chronicle)
         VALUES ($1, 2, 'ended', now() - interval '2 days', now() - interval '2 days', $2, $3, $4, $5)",
    )
    .bind(campaign)
    .bind(m("sessions.recap (draft)"))
    .bind(m("sessions.previously (draft)"))
    .bind(m("sessions.chronicle_title (draft)"))
    .bind(m("sessions.chronicle (draft)"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO game_sessions (campaign_id, number, status, started_at) VALUES ($1, 3, 'live', now())",
    )
    .bind(campaign)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO table_journal (campaign_id, session_id, kind, ref, text, shared)
         VALUES ($1, $2, 'note', 'cl_gwen', $3, false), ($1, $2, 'scene', 'n_crique', 'La crique.', true)",
    )
    .bind(campaign)
    .bind(first)
    .bind(m("journal.gm_line"))
    .execute(pool)
    .await
    .unwrap();
}

/// The quay shown at the table, marked where only the GM may look: the
/// map's notes, the hidden trapdoor's notes, a label under the fog, an
/// ambusher hidden by the GM and a sailor standing in the fog.
async fn board(pool: &PgPool, campaign: Uuid, character: Uuid) {
    let mut map = promptus_shared::maps::Map::from_yaml(include_str!(
        "../../content/maps/corsaires/quai-port-louis.yaml"
    ))
    .unwrap();
    map.gm_notes = Some(m("map.gm_notes"));
    map.objects[0].notes = Some(m("map.objects.notes"));
    let deck = map.labels.iter_mut().find(|l| l.at.x > 12).unwrap();
    deck.text = m("map.labels (fogged)");
    let revealed: Vec<[i32; 2]> = (0..6).flat_map(|x| (4..10).map(move |y| [x, y])).collect();
    let tokens = json!([
        { "id": format!("pc-{character}"), "kind": "character", "ref": character.to_string(),
          "name": "Borin", "at": [0, 5] },
        { "id": "gueule-rouge-1", "kind": "npc", "ref": "gueule-rouge", "name": m("tokens (hidden)"),
          "at": [2, 6], "hidden": true },
        { "id": "marin-1", "kind": "npc", "ref": "marin", "name": m("tokens (fogged)"), "at": [14, 6] },
    ]);
    sqlx::query(
        "INSERT INTO map_states (campaign_id, map_id, map, fog, revealed, tokens)
         VALUES ($1, 'quai-port-louis', $2, true, $3, $4)",
    )
    .bind(campaign)
    .bind(Json(&map))
    .bind(Json(&revealed))
    .bind(Json(&tokens))
    .execute(pool)
    .await
    .unwrap();
}

/// A request of Marc's in the live session, at the state `path` acts on.
async fn marc_request(pool: &PgPool, campaign: &str, path: &str) -> Uuid {
    let status = if path.ends_with("/roll") {
        "check"
    } else if path.ends_with("/contest") {
        "refused"
    } else {
        "pending"
    };
    sqlx::query_scalar(
        "INSERT INTO player_requests (campaign_id, session_id, player_id, character_id, text, status,
                                      check_ability, check_difficulty, gm_reason)
         SELECT s.campaign_id, s.id, p.id, c.id, 'Je grimpe au mât.', $2,
                CASE WHEN $2 = 'check' THEN 'DEX' END, CASE WHEN $2 = 'check' THEN 10 END,
                CASE WHEN $2 = 'refused' THEN 'Pas maintenant.' END
         FROM game_sessions s JOIN players p ON p.campaign_id = s.campaign_id
              JOIN characters c ON c.player_id = p.id
         WHERE s.campaign_id = $1 AND s.status = 'live' AND p.nickname = 'Marc'
         RETURNING id",
    )
    .bind(Uuid::parse_str(campaign).unwrap())
    .bind(status)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// The evening routes a spectator has no part in: they ask nothing,
/// roll nothing and answer no feedback.
fn players_only(method: &str, path: &str) -> Option<(StatusCode, &'static str)> {
    if method == "PUT" && path.ends_with("/schedule/{date}") {
        return Some((StatusCode::FORBIDDEN, "SPECTATOR"));
    }
    if method != "POST" {
        return None;
    }
    if path.contains("{request}") {
        // Marc's request: not theirs.
        return Some((StatusCode::NOT_FOUND, "NO_SUCH_REQUEST"));
    }
    (path.ends_with("/requests")
        || path.ends_with("/feedback")
        || path.ends_with("/walk")
        || path.ends_with("/fight"))
    .then_some((StatusCode::FORBIDDEN, "SPECTATOR"))
}

/// A route the sweep's table cannot make succeed for Marc, and the
/// answer it gives instead. Fighting needs a fight; the leaks of a fight
/// are swept in `board_test.rs`.
fn refused_to_marc(method: &str, path: &str) -> Option<(StatusCode, &'static str)> {
    (method == "POST" && path.ends_with("/fight")).then_some((StatusCode::CONFLICT, "NO_FIGHT"))
}

/// A route the sweep's table cannot make succeed for anyone: the quay
/// shown has no imported image behind it (`maps_test.rs` serves one).
fn refused_to_all(method: &str, path: &str) -> Option<(StatusCode, &'static str)> {
    (method == "GET" && path.ends_with("/board/backdrop"))
        .then_some((StatusCode::NOT_FOUND, "NO_SUCH_MAP"))
}

fn uri(path: &str, t: &Table) -> String {
    path.replace("{campaign}", &t.campaign)
        .replace("{code}", &t.code)
        .replace("{asset}", &t.asset)
        .replace("{date}", &t.date)
}

fn seated(path: &str) -> bool {
    path.contains("{campaign}")
}

/// The routes that write the caller's own character: a spectator has
/// none (404 `NO_CHARACTER`).
fn writes_character(method: &str, path: &str) -> bool {
    method != "GET" && path.contains("/character")
}

/// What the sweep sends to a route that takes a body.
fn sweep_body(n: usize, method: &str, path: &str) -> Option<Value> {
    match (method, path) {
        ("POST", p) if p.starts_with("/api/join/") => {
            Some(json!({ "nickname": format!("Sweep {n}"), "role": "player" }))
        }
        // A complete sheet, so the submit route that follows succeeds.
        // The line of Marc's bag `mark_review` stored.
        ("POST", p) if p.ends_with("/character/backstory") => {
            Some(json!({ "origin": "Du port.", "loss": "", "quest": "La mer." }))
        }
        ("POST", p) if p.ends_with("/equip") => Some(json!({ "entry": "k1", "equipped": true })),
        ("POST", p) if p.ends_with("/walk") => Some(json!({ "path": [[1, 5]] })),
        ("POST", p) if p.ends_with("/fight") => Some(json!({ "kind": "endTurn" })),
        ("POST", p) if p.ends_with("/lobby") => Some(json!({ "soundOk": true, "remote": true })),
        ("PUT", p) if p.ends_with("/schedule/{date}") => Some(json!({ "available": true })),
        ("POST", p) if p.ends_with("/requests") => Some(json!({
            "card": { "kind": "ability", "ability": "SAG" },
            "text": "Je scrute la crique.",
        })),
        ("POST", p) if p.ends_with("/feedback") => Some(json!({
            "rulesClear": "yes", "hadMoment": "partly", "knowsNext": "no",
            "comment": "Belle soirée.",
        })),
        ("PUT", p) if p.ends_with("/character") => Some(json!({
            "name": "Borin",
            "classId": "bretteur",
            "appearance": MARC_SECRET,
            "look": { "pack": "marins-1718", "body": "robuste", "skin": "hale",
                      "hair": { "style": "court", "colour": "roux" } },
        })),
        _ => None,
    }
}

fn assert_clean(what: &str, body: &Value) {
    let json = body.to_string();
    let found = leaks(&json);
    assert!(found.is_empty(), "{what} leaked GM-only data: {found:#?}");
    assert!(
        !json.contains("\"cl_") && !json.contains("\"rev_"),
        "{what} leaked a clue or revelation id: {json}"
    );
}

#[tokio::test]
async fn no_player_route_leaks_what_only_the_gm_may_see() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let t = marked_table(&app, &pool).await;
    let routes = player_facing_routes();
    assert!(routes.len() >= 4, "{routes:?}");

    for (n, (method, path)) in routes.iter().enumerate() {
        let mut uri = uri(path, &t);
        if path.contains("{request}") {
            let id = marc_request(&pool, &t.campaign, path).await;
            uri = uri.replace("{request}", &id.to_string());
        }
        let body = sweep_body(n, method, path);
        for (who, token) in [("Marc", &t.marc), ("Léa", &t.spectator)] {
            // Joining twice under one nickname is refused: one join is
            // enough to sweep.
            if *method == "POST" && !seated(path) && who == "Léa" {
                continue;
            }
            // Carrying an item is for a character in play: the GM
            // validates the sheet the sweep just sent.
            if path.ends_with("/equip") && who == "Marc" {
                sqlx::query(
                    "UPDATE characters SET status = 'validated'
                     WHERE player_id = (SELECT id FROM players WHERE nickname = 'Marc'
                                        AND campaign_id = $1)",
                )
                .bind(Uuid::parse_str(&t.campaign).unwrap())
                .execute(&pool)
                .await
                .unwrap();
            }
            let r = call_as_player(&app, Some(token), method, &uri, body.clone()).await;
            // The live socket carries versions and presence only
            // (`live_test.rs`); over plain HTTP, getting past the guard
            // to the upgrade check is what this sweep can see.
            if path.ends_with("/live") {
                assert_eq!(r.status, StatusCode::BAD_REQUEST, "{uri} as {who}");
                assert_eq!(r.body["error"]["code"], "WEBSOCKET_REQUIRED");
                continue;
            }
            if let Some((status, code)) = refused_to_all(method, path) {
                assert_eq!(r.status, status, "{method} {uri} as {who}: {}", r.body);
                assert_eq!(r.body["error"]["code"], code);
                continue;
            }
            if let (true, Some((status, code))) = (who == "Marc", refused_to_marc(method, path)) {
                assert_eq!(r.status, status, "{method} {uri} as Marc: {}", r.body);
                assert_eq!(r.body["error"]["code"], code);
                continue;
            }
            if let (true, Some((status, code))) = (who == "Léa", players_only(method, path)) {
                assert_eq!(r.status, status, "{method} {uri} as Léa: {}", r.body);
                assert_eq!(r.body["error"]["code"], code);
                continue;
            }
            if who == "Léa" && writes_character(method, path) {
                assert_eq!(r.status, StatusCode::NOT_FOUND, "{method} {uri} as Léa");
                assert_eq!(r.body["error"]["code"], "NO_CHARACTER");
                continue;
            }
            assert!(
                r.status.is_success(),
                "{method} {uri} as {who}: {} {}",
                r.status,
                r.body
            );
            assert_clean(&format!("{method} {uri} as {who}"), &r.body);
            // Answers that are not JSON (the calendar) are swept as text.
            assert!(
                leaks(&r.text).is_empty(),
                "{method} {uri} as {who}: {}",
                r.text
            );
            if who == "Léa" {
                assert!(
                    !r.body.to_string().contains(MARC_SECRET),
                    "{method} {uri} showed Marc's sheet to Léa"
                );
            }
        }
    }

    // The table sees the approved tileset only; the other images do not
    // leave, even by id.
    let r = call_as_player(
        &app,
        Some(&t.marc),
        "GET",
        &uri("/api/play/{campaign}/media", &t),
        None,
    )
    .await;
    let assets = r.body["data"]["assets"].as_array().unwrap();
    assert_eq!(assets.len(), 1, "{}", r.body);
    assert_eq!(assets[0]["id"], t.asset.as_str());
    assert_eq!(r.body["data"]["theme"]["id"], "corsaires");
    let pending: Uuid = sqlx::query_scalar(
        "SELECT id FROM media_assets WHERE status = 'pending' AND campaign_id = $1",
    )
    .bind(Uuid::parse_str(&t.campaign).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    let r = call_as_player(
        &app,
        Some(&t.marc),
        "GET",
        &format!("/api/play/{}/media/{pending}/image", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // What the routes are for is there: Marc reads his own sheet, the
    // view names the scene.
    let r = call_as_player(
        &app,
        Some(&t.marc),
        "GET",
        &format!("/api/play/{}/me", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["character"]["sheet"]["name"], "Borin");
    // Validated for the equip route: in play, his bag reaches him, the
    // GM's note on an item and the history of adjustments do not.
    assert_eq!(r.body["data"]["character"]["status"], "validated");
    assert_clean("Marc's character in play", &r.body);
    let play = &r.body["data"]["character"]["play"];
    assert_eq!(
        play["inventory"][0]["name"], "Épée de bonne facture",
        "{play}"
    );
    assert_eq!(play["inventory"][0]["equipped"], true, "{play}");
    assert!(!r.body.to_string().contains(ITEM_NOTE), "{play}");
    let r = call_as_player(
        &app,
        Some(&t.marc),
        "GET",
        &format!("/api/play/{}/view", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.body["data"]["scene"]["title"], "La crique aux Morts");
}

#[tokio::test]
async fn seated_routes_refuse_whoever_has_no_seat_at_that_table() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let t = marked_table(&app, &pool).await;
    // A seat at another table of the same GM.
    let other = imported_campaign(&app, &t.gm, common::marked::FIXTURE).await;
    let code = invite_code(&app, &t.gm, &other).await;
    let stranger = join(&app, &code, "Hugo", "player")
        .await
        .player_token()
        .unwrap();

    let cookies = [
        None,
        Some("promptus_player=made-up-token".to_string()),
        Some(format!("promptus_player={stranger}")),
        // The GM's own session is not a seat.
        Some(format!("promptus_gm={}", t.gm)),
        Some(format!("promptus_gm={}", t.marc)),
    ];
    for (method, path) in player_facing_routes() {
        if !seated(path) {
            continue;
        }
        let uri = uri(path, &t).replace("{request}", &Uuid::new_v4().to_string());
        for cookie in &cookies {
            let r = send(&app, cookie.as_deref(), method, &uri, None).await;
            assert_eq!(
                r.status,
                StatusCode::UNAUTHORIZED,
                "{method} {uri} with {cookie:?}"
            );
            assert_eq!(r.body["error"]["code"], "NOT_JOINED");
        }
    }
    // The GM previews the same projection on their own route.
    let r = call(
        &app,
        Some(&t.gm),
        "GET",
        &format!("/api/campaigns/{}/player-view", t.campaign),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_clean("the GM's preview", &r.body);
}
