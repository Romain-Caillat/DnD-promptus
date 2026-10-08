//! platform/sign-in-gm — done when every GM route refuses without the
//! GM's session, and refuses another GM where ownership applies.
//!
//! `GM_ROUTES` must list every route of the GM router in `app.rs`. Each
//! is swept with no cookie, a made-up cookie, an expired session and a
//! signed-out session (all 401), then — as a control that the 401s come
//! from the guard and not from a broken route — with the right session.
//! Routes on a resource another GM owns answer 404, exactly like a
//! resource that does not exist.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::call;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Every GM route, `{invite}` standing for an invitation of the caller,
/// `{campaign}` for one of their campaigns, `{player}` for a player at
/// that campaign's table, `{character}` for that player's character and
/// `{hook}` for a secret hook drawn from it.
const GM_ROUTES: &[(&str, &str)] = &[
    ("GET", "/api/me"),
    ("GET", "/api/gm-invites"),
    ("POST", "/api/gm-invites"),
    ("DELETE", "/api/gm-invites/{invite}"),
    ("GET", "/api/campaigns"),
    ("POST", "/api/campaigns"),
    ("POST", "/api/campaigns/import"),
    ("GET", "/api/campaigns/{campaign}"),
    ("GET", "/api/campaigns/{campaign}/export"),
    ("PUT", "/api/campaigns/{campaign}/import"),
    ("GET", "/api/campaigns/{campaign}/player-view"),
    ("PUT", "/api/campaigns/{campaign}/settings"),
    ("PUT", "/api/campaigns/{campaign}/archive"),
    ("GET", "/api/rule-systems"),
    // The rule editor: a draft, saved, compared, locked, then nothing
    // left to drop.
    ("GET", "/api/campaigns/{campaign}/rules"),
    ("POST", "/api/campaigns/{campaign}/rules/draft"),
    ("PUT", "/api/campaigns/{campaign}/rules/draft"),
    ("GET", "/api/campaigns/{campaign}/rules/compare?from=1&to=2"),
    ("POST", "/api/campaigns/{campaign}/rules/draft/lock"),
    ("DELETE", "/api/campaigns/{campaign}/rules/draft"),
    // Preparing: an edit, the campaign declared playable, then the
    // co-GM's workshop — each decision on a proposal stored just before.
    ("POST", "/api/campaigns/{campaign}/story/edits"),
    ("GET", "/api/campaigns/{campaign}/readiness"),
    ("POST", "/api/campaigns/{campaign}/story/validate"),
    ("GET", "/api/campaigns/{campaign}/workshop"),
    ("POST", "/api/campaigns/{campaign}/workshop"),
    (
        "POST",
        "/api/campaigns/{campaign}/workshop/{proposal}/accept",
    ),
    (
        "POST",
        "/api/campaigns/{campaign}/workshop/{proposal}/reject",
    ),
    // A generation, on the campaign made unvalidated again, and a draft
    // applied (validated again after).
    ("GET", "/api/campaigns/{campaign}/generation"),
    ("POST", "/api/campaigns/{campaign}/generation"),
    ("POST", "/api/campaigns/{campaign}/generation/{job}/apply"),
    ("GET", "/api/campaigns/{campaign}/invite"),
    ("POST", "/api/campaigns/{campaign}/invite"),
    ("GET", "/api/campaigns/{campaign}/players"),
    ("GET", "/api/campaigns/{campaign}/characters/{character}"),
    // Each is called on a sheet set back to "submitted" first.
    (
        "POST",
        "/api/campaigns/{campaign}/characters/{character}/note-draft",
    ),
    // On a sheet given a backstory first.
    (
        "POST",
        "/api/campaigns/{campaign}/characters/{character}/hooks/propose",
    ),
    (
        "POST",
        "/api/campaigns/{campaign}/characters/{character}/return",
    ),
    (
        "POST",
        "/api/campaigns/{campaign}/characters/{character}/validate",
    ),
    // After the validation: only a character in play is adjusted.
    ("GET", "/api/campaigns/{campaign}/sheets"),
    (
        "POST",
        "/api/campaigns/{campaign}/characters/{character}/adjust",
    ),
    ("GET", "/api/campaigns/{campaign}/hooks"),
    ("POST", "/api/campaigns/{campaign}/hooks"),
    ("PUT", "/api/campaigns/{campaign}/hooks/{hook}"),
    // The evening, in the order the GM plays it: open, start, play, end,
    // then read and write what follows.
    ("GET", "/api/campaigns/{campaign}/session"),
    ("POST", "/api/campaigns/{campaign}/session"),
    ("POST", "/api/campaigns/{campaign}/session/start"),
    ("POST", "/api/campaigns/{campaign}/session/reveal"),
    ("PUT", "/api/campaigns/{campaign}/session/music"),
    ("POST", "/api/campaigns/{campaign}/session/journal"),
    // On a reading of « Précédemment… » set under way just before.
    ("POST", "/api/campaigns/{campaign}/session/previously/next"),
    // The grid: show the quay, edit it, fight on it, hand out the loot.
    ("GET", "/api/campaigns/{campaign}/board"),
    ("POST", "/api/campaigns/{campaign}/board"),
    ("POST", "/api/campaigns/{campaign}/board/edit"),
    ("POST", "/api/campaigns/{campaign}/fight"),
    ("POST", "/api/campaigns/{campaign}/fight/command"),
    ("POST", "/api/campaigns/{campaign}/fight/loot"),
    // The ship battle: open the Greyhound's interception, stop it.
    ("POST", "/api/campaigns/{campaign}/battle"),
    ("POST", "/api/campaigns/{campaign}/battle/command"),
    // The campaign's maps: list, create, import, generate, then on the
    // map stored just before.
    ("GET", "/api/campaigns/{campaign}/maps"),
    ("POST", "/api/campaigns/{campaign}/maps"),
    ("POST", "/api/campaigns/{campaign}/maps/import"),
    ("POST", "/api/campaigns/{campaign}/maps/generate"),
    ("GET", "/api/campaigns/{campaign}/maps/{map}"),
    ("PUT", "/api/campaigns/{campaign}/maps/{map}"),
    ("POST", "/api/campaigns/{campaign}/maps/{map}/validate"),
    ("GET", "/api/campaigns/{campaign}/maps/{map}/backdrop"),
    ("DELETE", "/api/campaigns/{campaign}/maps/{map}"),
    // Images: list, draw one, then on an image stored just before.
    ("GET", "/api/campaigns/{campaign}/media"),
    ("POST", "/api/campaigns/{campaign}/media"),
    ("POST", "/api/campaigns/{campaign}/media/batch"),
    ("GET", "/api/campaigns/{campaign}/media/{asset}/image"),
    ("POST", "/api/campaigns/{campaign}/media/{asset}/decision"),
    // Each on a draft of the co-GM written just before.
    ("POST", "/api/campaigns/{campaign}/session/copilot"),
    (
        "POST",
        "/api/campaigns/{campaign}/session/copilot/{draft}/show",
    ),
    (
        "POST",
        "/api/campaigns/{campaign}/session/copilot/{draft}/dismiss",
    ),
    // On a request of the player's, sent just before.
    (
        "POST",
        "/api/campaigns/{campaign}/session/requests/{request}",
    ),
    (
        "POST",
        "/api/campaigns/{campaign}/session/spotlight/{player}",
    ),
    ("PUT", "/api/campaigns/{campaign}/hooks/{hook}/played"),
    ("GET", "/api/campaigns/{campaign}/knowledge"),
    ("POST", "/api/campaigns/{campaign}/session/end"),
    // `{session}` is the session just ended.
    ("GET", "/api/campaigns/{campaign}/sessions"),
    ("PUT", "/api/campaigns/{campaign}/sessions/{session}/recap"),
    (
        "POST",
        "/api/campaigns/{campaign}/sessions/{session}/recap-draft",
    ),
    (
        "GET",
        "/api/campaigns/{campaign}/sessions/{session}/feedback",
    ),
    (
        "PUT",
        "/api/campaigns/{campaign}/sessions/{session}/changes",
    ),
    ("GET", "/api/campaigns/{campaign}/ai"),
    // The next session's date: the channel set, a date proposed, then
    // fixed (`{date}` the one just proposed), another dropped.
    ("PUT", "/api/campaigns/{campaign}/schedule/discord"),
    ("GET", "/api/campaigns/{campaign}/schedule"),
    ("POST", "/api/campaigns/{campaign}/schedule/dates"),
    (
        "POST",
        "/api/campaigns/{campaign}/schedule/dates/{date}/choose",
    ),
    ("DELETE", "/api/campaigns/{campaign}/schedule/dates/{date}"),
    ("DELETE", "/api/campaigns/{campaign}/hooks/{hook}"),
    // After the character routes: removing the player removes them.
    ("DELETE", "/api/campaigns/{campaign}/players/{player}"),
    ("DELETE", "/api/campaigns/{campaign}/invite"),
    ("GET", "/api/campaigns/{campaign}/live"),
    // Last: it ends the session the control sweep uses.
    ("POST", "/api/auth/sign-out"),
];

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");
const CORSAIRES_RULES: &str = include_str!("../../content/rules/corsaires/v1.yaml");

/// The body a route needs to succeed, so the control sweep proves the
/// route works and the refusals come from the guard.
fn body_for(method: &str, path: &str) -> Option<Value> {
    match (method, path) {
        ("PUT", p) if p.ends_with("/schedule/discord") => Some(serde_json::json!({
            "webhook": "https://discord.com/api/webhooks/1/balayage"
        })),
        ("POST", p) if p.ends_with("/schedule/dates") => Some(serde_json::json!({
            "startsAt": chrono::Utc::now() + chrono::Duration::days(3)
        })),
        ("POST", "/api/campaigns") => Some(serde_json::json!({
            "title": "Sweep",
            "rules": { "id": "corsaires", "version": 1 }
        })),
        (_, p) if p.ends_with("/import") && !p.ends_with("/maps/import") => {
            Some(serde_json::json!({ "yaml": FIXTURE }))
        }
        (_, p) if p.ends_with("/settings") => Some(serde_json::json!({
            "title": "Sweep",
            "world": "",
            "pitch": "",
            "playerHook": "",
            "playerCount": 6,
            "aiBudgetCents": 100
        })),
        (_, p) if p.ends_with("/archive") => Some(serde_json::json!({ "archived": false })),
        (_, p) if p.ends_with("/story/edits") => Some(serde_json::json!({
            "edits": [{ "op": "set", "target": "bible", "field": "tone", "value": "Sombre." }]
        })),
        ("POST", p) if p.ends_with("/generation") => Some(serde_json::json!({
            "pitch": "Une ville tenue par une société secrète."
        })),
        ("POST", p) if p.ends_with("/workshop") => {
            Some(serde_json::json!({ "prompt": "Rends le gardien plus ambigu." }))
        }
        ("PUT", p) if p.ends_with("/rules/draft") => Some(serde_json::json!({
            "yaml": CORSAIRES_RULES.replacen("\nversion: 1\n", "\nversion: 2\n", 1),
            "note": "Facile à 12."
        })),
        // Refused before any check: the decision bodies need the sheet's
        // date, filled by the control sweep (`submitted_body`).
        (_, p) if p.ends_with("/validate") || p.ends_with("/return") => {
            Some(serde_json::json!({ "seen": "2026-10-05T00:00:00Z", "note": "Un mot." }))
        }
        ("POST", p) if p.ends_with("/hooks") => {
            Some(serde_json::json!({ "characterId": Uuid::nil(), "title": "Sweep" }))
        }
        (_, p) if p.ends_with("/adjust") => Some(serde_json::json!({ "kind": "xp", "delta": 1 })),
        ("PUT", p) if p.ends_with("/hooks/{hook}") => {
            Some(serde_json::json!({ "title": "Sweep", "body": "Réécrite." }))
        }
        (_, p) if p.ends_with("/played") => Some(serde_json::json!({ "played": true })),
        (_, p) if p.ends_with("/reveal") => {
            Some(serde_json::json!({ "kind": "scene", "node": "sc_taverne" }))
        }
        (_, p) if p.ends_with("/music") => Some(serde_json::json!({ "track": null })),
        (_, p) if p.ends_with("/journal") => {
            Some(serde_json::json!({ "kind": "note", "text": "Le phare clignote." }))
        }
        ("POST", p) if p.ends_with("/board") => {
            Some(serde_json::json!({ "map": "quai-port-louis" }))
        }
        (_, p) if p.ends_with("/board/edit") => {
            Some(serde_json::json!({ "kind": "fog", "enabled": true }))
        }
        (_, p) if p.ends_with("/fight") => Some(serde_json::json!({ "node": "sc_crique" })),
        (_, p) if p.ends_with("/fight/command") => Some(serde_json::json!({ "kind": "stop" })),
        (_, p) if p.ends_with("/battle") => {
            Some(serde_json::json!({ "node": "sc_interception_greyhound" }))
        }
        (_, p) if p.ends_with("/battle/command") => Some(serde_json::json!({ "kind": "stop" })),
        (_, p) if p.ends_with("/fight/loot") => Some(serde_json::json!({
            "gives": [{ "index": 0, "character": Uuid::nil() }]
        })),
        ("POST", p) if p.ends_with("/media") => {
            Some(serde_json::json!({ "kind": "scene", "subject": "sc_crique" }))
        }
        ("POST", p) if p.ends_with("/maps") => {
            Some(serde_json::json!({ "name": "Une salle", "width": 6, "height": 5 }))
        }
        (_, p) if p.ends_with("/maps/import") => Some(serde_json::json!({
            "kind": "image", "name": "Un plan", "image": "iVBORw0KGgpub3QgYW4gaW1hZ2U=",
            "cellPx": 64, "offsetX": 0, "offsetY": 0, "columns": 6, "rows": 5
        })),
        (_, p) if p.ends_with("/maps/generate") => Some(serde_json::json!({ "node": "sc_crique" })),
        (_, p) if p.ends_with("/media/batch") => Some(serde_json::json!({ "videos": false })),
        (_, p) if p.ends_with("/decision") => Some(serde_json::json!({ "approve": true })),
        (_, p) if p.ends_with("/copilot") => {
            Some(serde_json::json!({ "kind": "describe", "prompt": "Ils entrent." }))
        }
        (_, p) if p.ends_with("/show") => {
            Some(serde_json::json!({ "narration": "La pluie bat les carreaux." }))
        }
        (_, p) if p.ends_with("/requests/{request}") => {
            Some(serde_json::json!({ "kind": "accept", "reason": "" }))
        }
        (_, p) if p.ends_with("/end") || p.ends_with("/recap") => Some(serde_json::json!({
            "recap": "Ils ont trouvé la lanterne.",
            "previously": "La tempête approche."
        })),
        (_, p) if p.ends_with("/changes") => {
            Some(serde_json::json!({ "text": "Plus de scènes pour Marc." }))
        }
        _ => None,
    }
}

/// The ids the routes' placeholders stand for.
struct Ids {
    invite: String,
    campaign: String,
    player: String,
    character: String,
    hook: String,
    /// Filled once the control sweep has them; any id before.
    request: String,
    session: String,
    draft: String,
    asset: String,
    proposal: String,
    job: String,
    map: String,
    date: String,
}

fn route_uri(path: &str, ids: &Ids) -> String {
    path.replace("{invite}", &ids.invite)
        .replace("{campaign}", &ids.campaign)
        .replace("{player}", &ids.player)
        .replace("{character}", &ids.character)
        .replace("{hook}", &ids.hook)
        .replace("{request}", &ids.request)
        .replace("{session}", &ids.session)
        .replace("{draft}", &ids.draft)
        .replace("{asset}", &ids.asset)
        .replace("{proposal}", &ids.proposal)
        .replace("{job}", &ids.job)
        .replace("{map}", &ids.map)
        .replace("{date}", &ids.date)
}

/// Every placeholder filled for `player` of `campaign`: an invitation of
/// the GM, the player's character and a hook drawn from it.
async fn ids_of(app: &Router, pool: &PgPool, token: &str, campaign: String, player: String) -> Ids {
    // A class of the rules, so the character can be put in play.
    let character: Uuid = sqlx::query_scalar(
        "UPDATE characters SET sheet = '{\"name\":\"Borin\",\"classId\":\"bretteur\"}'
         WHERE player_id = $1 RETURNING id",
    )
    .bind(Uuid::parse_str(&player).unwrap())
    .fetch_one(pool)
    .await
    .unwrap();
    let r = call(
        app,
        Some(token),
        "POST",
        &format!("/api/campaigns/{campaign}/hooks"),
        Some(serde_json::json!({ "characterId": character, "title": "Le frère disparu" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    Ids {
        invite: invite_of(app, token).await,
        hook: r.body["data"]["id"].as_str().unwrap().to_string(),
        character: character.to_string(),
        campaign,
        player,
        request: Uuid::new_v4().to_string(),
        session: Uuid::new_v4().to_string(),
        draft: Uuid::new_v4().to_string(),
        asset: Uuid::new_v4().to_string(),
        proposal: Uuid::new_v4().to_string(),
        job: Uuid::new_v4().to_string(),
        date: Uuid::new_v4().to_string(),
        map: "carte-inconnue".to_string(),
    }
}

/// A pending request of the player in the live session.
async fn request_of(pool: &PgPool, ids: &Ids) -> String {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO player_requests (campaign_id, session_id, player_id, character_id, text)
         SELECT s.campaign_id, s.id, $2, $3, 'Je monte au phare.'
         FROM game_sessions s WHERE s.campaign_id = $1 AND s.status = 'live'
         RETURNING id",
    )
    .bind(Uuid::parse_str(&ids.campaign).unwrap())
    .bind(Uuid::parse_str(&ids.player).unwrap())
    .bind(Uuid::parse_str(&ids.character).unwrap())
    .fetch_one(pool)
    .await
    .unwrap()
    .to_string()
}

/// Put `character` back in "submitted" and return the `updatedAt` a
/// review would have read, as the body of a decision.
async fn submitted_body(pool: &PgPool, character: &str, path: &str) -> Value {
    let seen: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "UPDATE characters SET status = 'submitted' WHERE id = $1 RETURNING updated_at",
    )
    .bind(Uuid::parse_str(character).unwrap())
    .fetch_one(pool)
    .await
    .unwrap();
    if path.ends_with("/return") {
        serde_json::json!({ "seen": seen, "note": "Un mot." })
    } else {
        serde_json::json!({ "seen": seen })
    }
}

/// A player seated at `campaign`'s table: their id and device token.
async fn player_of(app: &Router, token: &str, campaign: &str) -> (String, String) {
    let code = common::invite_code(app, token, campaign).await;
    let r = common::join(app, &code, "Marc", "player").await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let id = r.body["data"]["me"]["id"].as_str().unwrap().to_string();
    (id, r.player_token().unwrap())
}

async fn campaign_of(app: &Router, token: &str) -> String {
    let r = call(
        app,
        Some(token),
        "POST",
        "/api/campaigns/import",
        body_for("POST", "/api/campaigns/import"),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["data"]["id"].as_str().unwrap().to_string()
}

async fn campaign_count(pool: &PgPool, gm: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM campaigns WHERE gm_id = $1")
        .bind(gm)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn invite_of(app: &Router, token: &str) -> String {
    let r = call(app, Some(token), "POST", "/api/gm-invites", None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["data"]["id"].as_str().unwrap().to_string()
}

async fn invite_exists(pool: &PgPool, id: &str) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM gm_invites WHERE id = $1)")
        .bind(Uuid::parse_str(id).unwrap())
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Every GM route answers 401 `UNAUTHENTICATED` with `cookie`, and
/// touches nothing.
async fn assert_all_refuse(app: &Router, pool: &PgPool, cookie: Option<&str>, ids: &Ids) {
    assert_all_refuse_raw(app, pool, cookie.map(|c| format!("promptus_gm={c}")), ids).await;
}

/// [`assert_all_refuse`] with `cookie` as the whole `Cookie` header.
async fn assert_all_refuse_raw(app: &Router, pool: &PgPool, cookie: Option<String>, ids: &Ids) {
    for (method, path) in GM_ROUTES {
        let uri = route_uri(path, ids);
        let r = common::send(app, cookie.as_deref(), method, &uri, body_for(method, path)).await;
        assert_eq!(
            r.status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} with {cookie:?}: {}",
            r.body
        );
        assert_eq!(r.body["error"]["code"], "UNAUTHENTICATED", "{method} {uri}");
    }
    assert!(
        invite_exists(pool, &ids.invite).await,
        "a refused DELETE deleted"
    );
    assert!(
        player_exists(pool, &ids.player).await,
        "a refused DELETE removed a player"
    );
    let (status, hooks): (String, i64) = sqlx::query_as(
        "SELECT c.status, (SELECT COUNT(*) FROM secret_hooks h WHERE h.character_id = c.id)
         FROM characters c WHERE c.id = $1",
    )
    .bind(Uuid::parse_str(&ids.character).unwrap())
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(status, "draft", "a refused decision changed the sheet");
    assert_eq!(hooks, 1, "a refused hook route added or removed one");
}

async fn player_exists(pool: &PgPool, id: &str) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM players WHERE id = $1)")
        .bind(Uuid::parse_str(id).unwrap())
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn every_gm_route_refuses_without_a_valid_session() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, token) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = campaign_of(&app, &token).await;
    let (player, _) = player_of(&app, &token, &campaign).await;
    let ids = ids_of(&app, &pool, &token, campaign, player).await;
    let campaigns_before = campaign_count(&pool, gm).await;
    let invites_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM gm_invites WHERE created_by = $1")
            .bind(gm)
            .fetch_one(&pool)
            .await
            .unwrap();

    // No cookie, and a cookie no session has.
    assert_all_refuse(&app, &pool, None, &ids).await;
    assert_all_refuse(&app, &pool, Some("made-up-token"), &ids).await;
    // The stored hash itself is not a session token either.
    let hash = promptus_back::auth::tokens::hash_token(&token);
    assert_all_refuse(&app, &pool, Some(&hash), &ids).await;

    // A refused POST minted nothing.
    let invites_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM gm_invites WHERE created_by = $1")
            .bind(gm)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(invites_after, invites_before);
    assert_eq!(campaign_count(&pool, gm).await, campaigns_before);

    // An expired session.
    let (_, expired) = common::signed_in_gm(&pool, "Expired").await;
    sqlx::query(
        "UPDATE gm_sessions SET expires_at = now() - interval '1 second' WHERE token_hash = $1",
    )
    .bind(promptus_back::auth::tokens::hash_token(&expired))
    .execute(&pool)
    .await
    .unwrap();
    assert_all_refuse(&app, &pool, Some(&expired), &ids).await;

    // Control: the same routes work with the right session.
    let mut ids = ids;
    for (method, path) in GM_ROUTES {
        if path.contains("{request}") {
            ids.request = request_of(&pool, &ids).await;
        }
        if path.contains("{draft}") {
            ids.draft = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO copilot_drafts (campaign_id, session_id, kind, answer)
                 SELECT campaign_id, id, 'describe', '{\"narration\":\"\",\"npcLines\":[],\"suggestions\":[],\"gmNote\":null}'
                 FROM game_sessions WHERE campaign_id = $1 AND status = 'live' RETURNING id",
            )
            .bind(Uuid::parse_str(&ids.campaign).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap()
            .to_string();
        }
        if path.contains("{proposal}") {
            ids.proposal = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO story_proposals (campaign_id, prompt, edits)
                 VALUES ($1, 'Rien.', '[]') RETURNING id",
            )
            .bind(Uuid::parse_str(&ids.campaign).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap()
            .to_string();
        }
        if path.contains("/generation") {
            sqlx::query("UPDATE campaigns SET validated_at = NULL WHERE id = $1")
                .bind(Uuid::parse_str(&ids.campaign).unwrap())
                .execute(&pool)
                .await
                .unwrap();
        }
        if path.contains("{job}") {
            ids.job = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO generation_jobs (campaign_id, status, input, steps, draft)
                 SELECT id, 'succeeded', '{\"pitch\":\"Rien de neuf.\"}', '[]', story
                 FROM campaigns WHERE id = $1 RETURNING id",
            )
            .bind(Uuid::parse_str(&ids.campaign).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap()
            .to_string();
        }
        if path.contains("{map}") && ids.map == "carte-inconnue" {
            // A copy of the world's quay, with an image behind it.
            let r = call(
                &app,
                Some(&token),
                "POST",
                &format!("/api/campaigns/{}/maps", ids.campaign),
                Some(serde_json::json!({ "name": "Carte du balayage", "copy": "quai-port-louis" })),
            )
            .await;
            ids.map = r.body["data"]["map"]["id"].as_str().unwrap().to_string();
            sqlx::query(
                "UPDATE campaign_maps SET backdrop = '\\x89504e47', backdrop_mime = 'image/png'
                 WHERE campaign_id = $1 AND id = $2",
            )
            .bind(Uuid::parse_str(&ids.campaign).unwrap())
            .bind(&ids.map)
            .execute(&pool)
            .await
            .unwrap();
        }
        if path.contains("{asset}") {
            ids.asset = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO media_assets (campaign_id, kind, subject, mime, image)
                 VALUES ($1, 'scene', 'sc_crique', 'image/png', '\\x89504e47') RETURNING id",
            )
            .bind(Uuid::parse_str(&ids.campaign).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap()
            .to_string();
        }
        if path.ends_with("/previously/next") {
            sqlx::query(
                "UPDATE game_sessions SET previously_shown = 1
                 WHERE campaign_id = $1 AND status = 'live'",
            )
            .bind(Uuid::parse_str(&ids.campaign).unwrap())
            .execute(&pool)
            .await
            .unwrap();
        }
        if path.contains("{date}") {
            ids.date = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM session_dates WHERE campaign_id = $1 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(Uuid::parse_str(&ids.campaign).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap()
            .to_string();
        }
        if path.contains("{session}") {
            ids.session = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM game_sessions WHERE campaign_id = $1 ORDER BY number DESC LIMIT 1",
            )
            .bind(Uuid::parse_str(&ids.campaign).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap()
            .to_string();
        }
        let uri = route_uri(path, &ids);
        let decision = path.contains("/characters/")
            && (path.ends_with("/validate")
                || path.ends_with("/return")
                || path.ends_with("/note-draft"));
        if path.ends_with("/hooks/propose") {
            sqlx::query(
                r#"UPDATE characters
                   SET sheet = sheet || '{"backstory":{"origin":"Le port."}}'::jsonb
                   WHERE id = $1"#,
            )
            .bind(Uuid::parse_str(&ids.character).unwrap())
            .execute(&pool)
            .await
            .unwrap();
        }
        let body = if decision {
            Some(submitted_body(&pool, &ids.character, path).await)
        } else {
            body_for(method, path)
        };
        let body = match (method, body) {
            // The editor sends the whole map back.
            (&"PUT", _) if path.ends_with("/maps/{map}") => {
                let uri = format!("/api/campaigns/{}/maps/{}", ids.campaign, ids.map);
                Some(call(&app, Some(&token), "GET", &uri, None).await.body["data"]["map"].clone())
            }
            // A hook needs a character of the table.
            (&"POST", Some(mut b)) if path.ends_with("/hooks") => {
                b["characterId"] = Value::String(ids.character.clone());
                Some(b)
            }
            (&"POST", Some(mut b)) if path.ends_with("/fight/loot") => {
                b["gives"][0]["character"] = Value::String(ids.character.clone());
                Some(b)
            }
            (_, b) => b,
        };
        let r = call(&app, Some(&token), method, &uri, body).await;
        if path.ends_with("/apply") {
            sqlx::query("UPDATE campaigns SET validated_at = now() WHERE id = $1")
                .bind(Uuid::parse_str(&ids.campaign).unwrap())
                .execute(&pool)
                .await
                .unwrap();
        }
        if path.ends_with("/live") {
            // Past the guard and the ownership check, a plain request
            // (not a WebSocket upgrade) is refused by the route itself.
            assert_eq!(r.body["error"]["code"], "WEBSOCKET_REQUIRED", "{uri}");
            continue;
        }
        assert!(
            r.status.is_success(),
            "{method} {uri} with the owner's session: {} {}",
            r.status,
            r.body
        );
    }

    // Signed out (the last control call): the session no longer opens
    // anything, even with the cookie kept.
    let (_, keeper) = common::signed_in_gm(&pool, "Keeper").await;
    let campaign = campaign_of(&app, &keeper).await;
    let player = player_of(&app, &keeper, &campaign).await.0;
    let ids = ids_of(&app, &pool, &keeper, campaign, player).await;
    assert_all_refuse(&app, &pool, Some(&token), &ids).await;
}

/// A player's device token opens no GM route, whichever cookie name it
/// is sent under; and a player cannot use a GM session either way round
/// (`player_routes_test.rs`).
#[tokio::test]
async fn a_player_token_opens_no_gm_route() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = campaign_of(&app, &token).await;
    let (player, device) = player_of(&app, &token, &campaign).await;
    let ids = ids_of(&app, &pool, &token, campaign, player).await;
    assert_all_refuse_raw(&app, &pool, Some(format!("promptus_player={device}")), &ids).await;
    assert_all_refuse(&app, &pool, Some(&device), &ids).await;
}

#[tokio::test]
async fn sign_out_clears_the_cookie_and_ends_only_that_session() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (gm, phone) = common::signed_in_gm(&pool, "Romain").await;
    // The same GM on a second device.
    let mut tx = pool.begin().await.unwrap();
    let laptop = promptus_back::auth::session::create_in_tx(&mut tx, gm)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let r = call(&app, Some(&phone), "POST", "/api/auth/sign-out", None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let cookie = r.set_cookie.unwrap();
    assert!(cookie.starts_with("promptus_gm=;"), "{cookie}");
    assert!(cookie.contains("Max-Age=0"), "{cookie}");

    assert_eq!(
        call(&app, Some(&phone), "GET", "/api/me", None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, Some(&laptop), "GET", "/api/me", None)
            .await
            .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn another_gm_cannot_reach_what_a_gm_owns() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, romain) = common::signed_in_gm(&pool, "Romain").await;
    let (marc_id, marc) = common::signed_in_gm(&pool, "Marc").await;
    let invite = invite_of(&app, &romain).await;

    // Each session is its own GM.
    let r = call(&app, Some(&marc), "GET", "/api/me", None).await;
    assert_eq!(r.body["data"]["id"], marc_id.to_string());
    assert_eq!(r.body["data"]["displayName"], "Marc");

    // Romain's invitation is invisible to Marc…
    let r = call(&app, Some(&marc), "GET", "/api/gm-invites", None).await;
    assert_eq!(r.status, StatusCode::OK);
    let listed: Vec<&Value> = r.body["data"].as_array().unwrap().iter().collect();
    assert!(
        listed.iter().all(|i| i["id"] != invite.as_str()),
        "{listed:?}"
    );

    // …and cannot be revoked by him: 404, like one that does not exist.
    let r = call(
        &app,
        Some(&marc),
        "DELETE",
        &format!("/api/gm-invites/{invite}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "NOT_FOUND");
    assert!(invite_exists(&pool, &invite).await);
    let missing = Uuid::new_v4();
    let r = call(
        &app,
        Some(&marc),
        "DELETE",
        &format!("/api/gm-invites/{missing}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call(
        &app,
        Some(&marc),
        "DELETE",
        "/api/gm-invites/not-a-uuid",
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // Its owner can.
    let r = call(&app, Some(&romain), "GET", "/api/gm-invites", None).await;
    let listed = r.body["data"].as_array().unwrap();
    assert!(listed.iter().any(|i| i["id"] == invite.as_str()));
    assert!(
        listed.iter().all(|i| i.get("code").is_none()),
        "the code is shown once"
    );
    let r = call(
        &app,
        Some(&romain),
        "DELETE",
        &format!("/api/gm-invites/{invite}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(!invite_exists(&pool, &invite).await);
}

/// The `/api/{*rest}` catch-all (a JSON 404 for unknown API paths) must
/// neither swallow a GM route nor answer an unknown path with 401, with
/// or without a session; and it must not open GM routes to a visitor.
#[tokio::test]
async fn the_api_catch_all_neither_shadows_nor_opens_gm_routes() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;

    for session in [None, Some(token.as_str())] {
        for uri in ["/api/nope", "/api/me/extra", "/api/gm-invites/a/b"] {
            let r = call(&app, session, "GET", uri, None).await;
            assert_eq!(r.status, StatusCode::NOT_FOUND, "{uri} with {session:?}");
            assert_eq!(r.body["error"]["code"], "ROUTE_NOT_FOUND", "{uri}");
        }
    }
    let r = call(&app, None, "GET", "/api/me", None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = call(&app, Some(&token), "GET", "/api/me", None).await;
    assert_eq!(r.status, StatusCode::OK);
}
