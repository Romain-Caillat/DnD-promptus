//! player/buy-and-trade over the API: the GM prepares and opens the
//! black market of Kerjean, Marc haggles and buys from his phone, a line
//! comes out from under the counter, and Marc shares with Camille.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

struct Table {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
    marc: String,
    camille: String,
    lyra: String,
    lea: String,
}

async fn seat(
    app: &Router,
    pool: &PgPool,
    code: &str,
    nickname: &str,
    name: &str,
) -> (String, String) {
    let r = join(app, code, nickname, "player").await;
    let id = Uuid::parse_str(r.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
    let character: Uuid = sqlx::query_scalar(
        "UPDATE characters SET status = 'validated', sheet = $2 WHERE player_id = $1 RETURNING id",
    )
    .bind(id)
    .bind(json!({ "name": name, "classId": "bretteur" }))
    .fetch_one(pool)
    .await
    .unwrap();
    (r.player_token().unwrap(), character.to_string())
}

/// Romain's table: Marc plays Borin, Camille plays Lyra, Léa watches.
async fn table() -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, FIXTURE).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let (marc, _) = seat(&app, &pool, &code, "Marc", "Borin").await;
    let (camille, lyra) = seat(&app, &pool, &code, "Camille", "Lyra").await;
    let lea = join(&app, &code, "Léa", "spectator").await;
    Table {
        app,
        pool,
        gm,
        campaign,
        marc,
        camille,
        lyra,
        lea: lea.player_token().unwrap(),
    }
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn ok(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        let r = self.gm(method, path, body).await;
        assert!(r.status.is_success(), "{method} {path}: {}", r.body);
        r.body["data"].clone()
    }

    async fn player(&self, token: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/play/{}{path}", self.campaign);
        call_as_player(&self.app, Some(token), method, &uri, body).await
    }

    async fn shop(&self, token: &str) -> Value {
        let r = self.player(token, "GET", "/shop", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }

    /// The play view of the character of `token`.
    async fn play(&self, token: &str) -> Value {
        let r = self.player(token, "GET", "/me", None).await;
        r.body["data"]["character"]["play"].clone()
    }

    async fn journal(&self) -> Vec<String> {
        sqlx::query_scalar(
            "SELECT text FROM table_journal WHERE campaign_id = $1 AND shared ORDER BY created_at, id",
        )
        .bind(Uuid::parse_str(&self.campaign).unwrap())
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }
}

fn gold(play: &Value) -> i64 {
    play["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "or")
        .unwrap()["amount"]
        .as_i64()
        .unwrap()
}

fn bag_line<'a>(play: &'a Value, name: &str) -> Option<&'a Value> {
    play["inventory"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == name)
}

fn market() -> Value {
    json!({
        "name": "Le marché noir de Kerjean",
        "lines": [
            { "id": "compas", "item": "compas_enchante", "price": 8, "stock": 1 },
            { "id": "vue", "name": "Longue-vue de contrebande", "description": "Volée à un officier.",
              "price": 2, "hidden": true },
            { "id": "navire", "name": "Un sloop", "price": 500 },
        ],
        "haggle": { "ability": "CHA", "difficulty": 12, "discount": 25 },
    })
}

#[tokio::test]
async fn marc_haggles_and_buys_at_the_black_market() {
    let t = table().await;
    let shop = t.ok("POST", "/shops", Some(market())).await;
    let id = shop["id"].as_str().unwrap().to_string();
    assert_eq!(shop["open"], false);

    // Closed, it is the GM's alone.
    assert_eq!(t.shop(&t.marc).await, Value::Null);
    let r = t
        .player(
            &t.marc,
            "POST",
            "/shop/buy",
            Some(json!({ "line": "compas" })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NO_SHOP");

    t.ok(
        "POST",
        &format!("/shops/{id}/open"),
        Some(json!({ "open": true })),
    )
    .await;
    let seen = t.shop(&t.marc).await;
    assert_eq!(seen["name"], "Le marché noir de Kerjean");
    let names: Vec<&str> = seen["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();
    // Under the counter: not on the phone.
    assert_eq!(names, ["Compas enchanté", "Un sloop"]);
    assert_eq!(
        seen["purse"],
        json!({ "name": "Pièces d'or", "abbr": "PO", "amount": 10 })
    );
    assert_eq!(seen["haggle"]["abilityName"], "Charisme");
    assert!(!seen.to_string().contains("Longue-vue"));
    // Léa watches: she sees the counter, without a purse, and cannot buy.
    let lea = t.shop(&t.lea).await;
    assert_eq!(lea["purse"], Value::Null);
    let r = t
        .player(
            &t.lea,
            "POST",
            "/shop/buy",
            Some(json!({ "line": "compas" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.body["error"]["code"], "SPECTATOR");

    // Marc haggles the compass, once.
    let r = t
        .player(
            &t.marc,
            "POST",
            "/shop/haggle",
            Some(json!({ "line": "compas" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let band = r.body["data"]["roll"]["band"].as_str().unwrap().to_string();
    let won = band == "success" || band == "critical_success";
    let compas = &r.body["data"]["shop"]["lines"][0];
    assert_eq!(compas["haggled"], won);
    let price = if won { 6 } else { 8 };
    assert_eq!(compas["price"], price, "{band}");
    assert_eq!(compas["listPrice"], 8);
    // Camille's price is the GM's.
    assert_eq!(t.shop(&t.camille).await["lines"][0]["price"], 8);
    let r = t
        .player(
            &t.marc,
            "POST",
            "/shop/haggle",
            Some(json!({ "line": "compas" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "ALREADY_HAGGLED");

    // Too dear, or more than the stock: nothing moves.
    let r = t
        .player(
            &t.marc,
            "POST",
            "/shop/buy",
            Some(json!({ "line": "navire" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "NOT_ENOUGH");
    let r = t
        .player(
            &t.marc,
            "POST",
            "/shop/buy",
            Some(json!({ "line": "compas", "qty": 2 })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "OUT_OF_STOCK");
    assert_eq!(gold(&t.play(&t.marc).await), 10);

    // He buys it: the purse pays his price, the bag holds it, the stock
    // is gone, and the table reads it.
    let r = t
        .player(
            &t.marc,
            "POST",
            "/shop/buy",
            Some(json!({ "line": "compas" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["lines"][0]["stock"], 0);
    assert_eq!(r.body["data"]["purse"]["amount"], 10 - price);
    let play = t.play(&t.marc).await;
    assert_eq!(gold(&play), 10 - price);
    assert!(bag_line(&play, "Compas enchanté").is_some(), "{play}");
    let r = t
        .player(
            &t.camille,
            "POST",
            "/shop/buy",
            Some(json!({ "line": "compas" })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "OUT_OF_STOCK");
    assert!(
        t.journal()
            .await
            .contains(&format!("Borin achète Compas enchanté ({price} PO)."))
    );

    // A word with the fence: the GM brings the spyglass out.
    let r = t
        .player(&t.marc, "POST", "/shop/buy", Some(json!({ "line": "vue" })))
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.body["error"]["code"], "NO_SUCH_LINE");
    t.ok(
        "POST",
        &format!("/shops/{id}/reveal"),
        Some(json!({ "line": "vue" })),
    )
    .await;
    let r = t
        .gm(
            "POST",
            &format!("/shops/{id}/reveal"),
            Some(json!({ "line": "vue" })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NOT_HIDDEN");
    assert!(
        t.journal().await.contains(
            &"Sous le comptoir de « Le marché noir de Kerjean » : Longue-vue de contrebande."
                .to_string()
        )
    );
    let r = t
        .player(&t.marc, "POST", "/shop/buy", Some(json!({ "line": "vue" })))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let play = t.play(&t.marc).await;
    assert_eq!(gold(&play), 10 - price - 2);
    assert_eq!(
        bag_line(&play, "Longue-vue de contrebande").unwrap()["description"],
        "Volée à un officier."
    );

    // The GM's shop keeps Marc's haggle; another GM never sees it.
    let shops = t.ok("GET", "/shops", None).await;
    assert_eq!(shops["shops"][0]["haggles"].as_array().unwrap().len(), 1);
    assert_eq!(shops["currency"]["abbr"], "PO");
    let (_, other) = common::signed_in_gm(&t.pool, "Autre").await;
    let uri = format!("/api/campaigns/{}/shops", t.campaign);
    let r = call(&t.app, Some(&other), "GET", &uri, None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call(
        &t.app,
        Some(&other),
        "POST",
        &format!("{uri}/{id}/open"),
        Some(json!({ "open": false })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // Closed again: gone from the phones.
    t.ok(
        "POST",
        &format!("/shops/{id}/open"),
        Some(json!({ "open": false })),
    )
    .await;
    assert_eq!(t.shop(&t.marc).await, Value::Null);
}

#[tokio::test]
async fn the_gm_writes_shops_the_rules_can_read() {
    let t = table().await;
    for (body, code) in [
        (json!({ "name": " ", "lines": [] }), "INVALID_SHOP_NAME"),
        (
            json!({ "name": "Étal", "lines": [{ "item": "inconnu", "price": 1 }] }),
            "UNKNOWN_ITEM",
        ),
        (
            json!({ "name": "Étal", "lines": [{ "price": 1 }] }),
            "INVALID_ITEM_NAME",
        ),
        (
            json!({ "name": "Étal", "lines": [{ "name": "Rhum", "price": 100001 }] }),
            "INVALID_PRICE",
        ),
        (
            json!({ "name": "Étal", "haggle": { "ability": "XYZ", "difficulty": 10, "discount": 10 } }),
            "INVALID_HAGGLE",
        ),
        (
            json!({ "name": "Étal", "haggle": { "ability": "CHA", "difficulty": 10, "discount": 95 } }),
            "INVALID_HAGGLE",
        ),
    ] {
        let r = t.gm("POST", "/shops", Some(body)).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{code}: {}", r.body);
        assert_eq!(r.body["error"]["code"], code);
    }

    // One shop open at a time: opening the second closes the first.
    let a = t
        .ok("POST", "/shops", Some(json!({ "name": "Étal A" })))
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let b = t
        .ok("POST", "/shops", Some(json!({ "name": "Étal B" })))
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    t.ok(
        "POST",
        &format!("/shops/{a}/open"),
        Some(json!({ "open": true })),
    )
    .await;
    t.ok(
        "POST",
        &format!("/shops/{b}/open"),
        Some(json!({ "open": true })),
    )
    .await;
    assert_eq!(t.shop(&t.marc).await["name"], "Étal B");
    let shops = t.ok("GET", "/shops", None).await;
    let open: Vec<bool> = shops["shops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["open"].as_bool().unwrap())
        .collect();
    assert_eq!(open, [false, true]);

    // Rewritten, a line keeps its id; deleted, the shop is gone.
    let saved = t
        .ok(
            "PUT",
            &format!("/shops/{b}"),
            Some(json!({ "name": "Étal B", "lines": [{ "id": "rhum", "name": "Rhum", "price": 1 }, { "name": "Pain", "price": 1 }] })),
        )
        .await;
    assert_eq!(saved["lines"][0]["id"], "rhum");
    assert!(!saved["lines"][1]["id"].as_str().unwrap().is_empty());
    let r = t.gm("DELETE", &format!("/shops/{b}"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(t.shop(&t.marc).await, Value::Null);
}

#[tokio::test]
async fn marc_shares_with_lyra() {
    let t = table().await;
    // Marc sees who he can share with: not himself.
    let r = t.player(&t.marc, "GET", "/party", None).await;
    assert_eq!(
        r.body["data"],
        json!([{ "id": t.lyra, "name": "Lyra", "nickname": "Camille" }])
    );

    let r = t
        .player(
            &t.marc,
            "POST",
            "/character/give",
            Some(json!({ "to": t.lyra, "coins": 3 })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(gold(&t.play(&t.marc).await), 7);
    assert_eq!(gold(&t.play(&t.camille).await), 13);
    let r = t
        .player(
            &t.marc,
            "POST",
            "/character/give",
            Some(json!({ "to": t.lyra, "coins": 8 })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "NOT_ENOUGH");
    assert_eq!(gold(&t.play(&t.camille).await), 13, "nothing moved");

    // A line of his bag.
    let play = t.play(&t.marc).await;
    let first = play["inventory"][0].clone();
    let r = t
        .player(
            &t.marc,
            "POST",
            "/character/give",
            Some(json!({ "to": t.lyra, "entry": first["key"] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let lyra = t.play(&t.camille).await;
    let given = lyra["inventory"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["name"] == first["name"])
        .map(|e| e["qty"].as_u64().unwrap())
        .sum::<u64>();
    assert!(given >= 2, "Lyra had her own and got Borin's: {lyra}");
    assert!(t.journal().await.contains(&format!(
        "Borin donne {} à Lyra.",
        first["name"].as_str().unwrap()
    )));

    // Not to himself, not to a stranger, not from a spectator, not both.
    let me: String = sqlx::query_scalar::<_, Uuid>(
        "SELECT c.id FROM characters c JOIN players p ON p.id = c.player_id WHERE p.nickname = 'Marc'
         AND c.campaign_id = $1",
    )
    .bind(Uuid::parse_str(&t.campaign).unwrap())
    .fetch_one(&t.pool)
    .await
    .unwrap()
    .to_string();
    for (to, code) in [
        (me, "NO_SUCH_CHARACTER"),
        (Uuid::new_v4().to_string(), "NO_SUCH_CHARACTER"),
    ] {
        let r = t
            .player(
                &t.marc,
                "POST",
                "/character/give",
                Some(json!({ "to": to, "coins": 1 })),
            )
            .await;
        assert_eq!(r.status, StatusCode::NOT_FOUND);
        assert_eq!(r.body["error"]["code"], code);
    }
    let r = t
        .player(
            &t.lea,
            "POST",
            "/character/give",
            Some(json!({ "to": t.lyra, "coins": 1 })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NO_CHARACTER");
    let r = t
        .player(
            &t.marc,
            "POST",
            "/character/give",
            Some(json!({ "to": t.lyra, "coins": 1, "entry": "x" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "INVALID_GIFT");
}
