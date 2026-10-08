//! player/buy-and-trade — done when, at Kerjean's black market
//! (Corsaires, act 1), the GM opens Dents-de-Fer's shop from the story,
//! the players buy from their purse, haggle once with the server's roll,
//! and the compass under the counter stays out of sight until revealed;
//! when players hand items and money to each other; and when a Brasier
//! table, once the GM gave its rules a currency, trades the same way.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

struct Seat {
    character: String,
    token: String,
}

struct Table {
    app: Router,
    pool: PgPool,
    gm: String,
    campaign: String,
    code: String,
}

impl Table {
    async fn new(world: &str) -> Self {
        let pool = common::test_pool().await;
        let app = common::app(pool.clone());
        let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
        let campaign = imported_campaign(&app, &gm, world).await;
        let code = invite_code(&app, &gm, &campaign).await;
        Self {
            app,
            pool,
            gm,
            campaign,
            code,
        }
    }

    /// `nickname` joins and plays `name`, a validated `class`.
    async fn seat(&self, nickname: &str, name: &str, class: &str) -> Seat {
        let r = join(&self.app, &self.code, nickname, "player").await;
        let character: Uuid = sqlx::query_scalar(
            "UPDATE characters SET status = 'validated', sheet = $2
             WHERE player_id = $1 RETURNING id",
        )
        .bind(Uuid::parse_str(r.body["data"]["me"]["id"].as_str().unwrap()).unwrap())
        .bind(json!({ "name": name, "classId": class }))
        .fetch_one(&self.pool)
        .await
        .unwrap();
        Seat {
            character: character.to_string(),
            token: r.player_token().unwrap(),
        }
    }

    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn ok(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        let r = self.gm(method, path, body).await;
        assert!(r.status.is_success(), "{method} {path}: {}", r.body);
        r.body["data"].clone()
    }

    async fn play(&self, token: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/play/{}{path}", self.campaign);
        call_as_player(&self.app, Some(token), method, &uri, body).await
    }

    async fn trade(&self, seat: &Seat) -> Value {
        let r = self.play(&seat.token, "GET", "/trade", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }

    async fn bag(&self, seat: &Seat) -> Vec<String> {
        let r = self.play(&seat.token, "GET", "/me", None).await;
        r.body["data"]["character"]["play"]["inventory"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["name"].as_str().unwrap().to_string())
            .collect()
    }

    async fn gold(&self, seat: &Seat, delta: i32, resource: &str) {
        self.ok(
            "POST",
            &format!("/characters/{}/adjust", seat.character),
            Some(json!({ "kind": "resource", "resource": resource, "delta": delta })),
        )
        .await;
    }
}

fn price_of(shop: &Value, name: &str) -> Value {
    shop["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == name)
        .cloned()
        .unwrap_or(Value::Null)
}

#[tokio::test]
async fn dents_de_fer_sells_haggles_once_and_hides_his_compass() {
    let t = Table::new(CORSAIRES).await;
    let lyra = t.seat("Camille", "Lyra", "bretteur").await;
    let borin = t.seat("Marc", "Borin", "canonnier").await;
    let lea = join(&t.app, &t.code, "Léa", "spectator").await;
    let lea = Seat {
        character: String::new(),
        token: lea.player_token().unwrap(),
    };

    // The GM opens his shop from the story: what he sells at his prices,
    // the compass he hides, the haggling check of his scene.
    let screen = t.ok("GET", "/shops", None).await;
    assert!(
        screen["keepers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|k| k["id"] == "pnj_dents_de_fer")
    );
    let screen = t
        .ok(
            "POST",
            "/shops",
            Some(json!({ "fromNpc": "pnj_dents_de_fer" })),
        )
        .await;
    let shop = &screen["shops"][0];
    let id = shop["id"].as_str().unwrap().to_string();
    assert_eq!(shop["name"], "Le marché noir de Kerjean");
    assert_eq!(shop["keeper"], "Dents-de-Fer");
    assert_eq!(shop["open"], false);
    assert_eq!(shop["lines"].as_array().unwrap().len(), 8);
    let compass = shop["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["displayName"] == "Boussole d'Amiral")
        .unwrap();
    assert_eq!(
        (compass["hidden"].as_bool(), compass["price"].as_u64()),
        (Some(true), Some(25))
    );
    assert_eq!(shop["haggle"]["ability"], "CHA");
    assert_eq!(shop["haggle"]["difficulty"], 10);

    // Closed, it is not there for the players.
    assert_eq!(t.trade(&lyra).await["shops"], json!([]));
    let path = format!("/shops/{id}");
    let r = t
        .play(
            &lyra.token,
            "POST",
            &format!("{path}/buy"),
            Some(json!({ "line": "obj_corde" })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NO_SUCH_SHOP");

    t.ok(
        "POST",
        &format!("{path}/open"),
        Some(json!({ "open": true })),
    )
    .await;
    let view = t.trade(&lyra).await;
    assert!(
        !view.to_string().contains("Boussole") && !view.to_string().contains("pnj_"),
        "the compass under the counter or the NPC leaked: {view}"
    );
    let market = &view["shops"][0];
    assert_eq!(market["lines"].as_array().unwrap().len(), 7);
    assert_eq!(price_of(market, "Épée de bonne facture")["price"], 15);
    assert_eq!(view["purse"]["amount"], 10, "the rules' starting purse");
    assert_eq!(market["haggle"]["abilityName"], "Charisme");
    assert_eq!(market["haggle"]["label"], "Moyen");

    // Buying: from the purse, into the bag, never beyond the purse.
    let r = t
        .play(
            &lyra.token,
            "POST",
            &format!("{path}/buy"),
            Some(json!({ "line": "obj_epee_bonne_facture" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "NOT_ENOUGH");
    let r = t
        .play(
            &lyra.token,
            "POST",
            &format!("{path}/buy"),
            Some(json!({ "line": "obj_corde" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["purse"]["amount"], 8);
    assert!(t.bag(&lyra).await.contains(&"Corde de 30 m".to_string()));

    // A spectator buys nothing.
    let r = t
        .play(
            &lea.token,
            "POST",
            &format!("{path}/buy"),
            Some(json!({ "line": "obj_corde" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // One haggle: the server rolls Charisme against 10 and applies the
    // outcome; never twice.
    let r = t
        .play(&lyra.token, "POST", &format!("{path}/haggle"), None)
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let market = &r.body["data"]["shops"][0];
    let mine = &market["haggle"]["mine"];
    let band = mine["band"].as_str().unwrap().to_string();
    assert_eq!(mine["roll"]["target"]["value"], 10);
    match band.as_str() {
        "success" | "critical_success" => {
            assert_eq!(mine["discountLeft"], true);
            assert_eq!(
                price_of(market, "Épée de bonne facture")["discountedPrice"],
                8
            );
        }
        "critical_failure" => {
            assert_eq!(market["surcharge"], 2);
            assert_eq!(price_of(market, "Épée de bonne facture")["price"], 17);
        }
        _ => assert_eq!(market["surcharge"], 0),
    }
    assert_eq!(
        market["lines"].to_string().contains("Boussole"),
        band == "critical_success",
        "only a natural 20 brings the compass out"
    );
    let r = t
        .play(&lyra.token, "POST", &format!("{path}/haggle"), None)
        .await;
    assert_eq!(r.body["error"]["code"], "ALREADY_HAGGLED");

    // Borin won his haggle: one purchase of his choice at half price,
    // rounded Dents-de-Fer's way, then the full price again.
    sqlx::query(
        "INSERT INTO shop_haggles (shop_id, character_id, band, roll, discount_left)
         SELECT $1, $2, 'success', roll, true FROM shop_haggles WHERE shop_id = $1",
    )
    .bind(Uuid::parse_str(&id).unwrap())
    .bind(Uuid::parse_str(&borin.character).unwrap())
    .execute(&t.pool)
    .await
    .unwrap();
    let before = t.trade(&borin).await;
    let sword = price_of(&before["shops"][0], "Épée de bonne facture");
    let surcharge = before["shops"][0]["surcharge"].as_u64().unwrap();
    assert_eq!(
        sword["discountedPrice"].as_u64(),
        Some((15 + surcharge).div_ceil(2))
    );
    t.gold(&borin, 10, "or").await;
    let r = t
        .play(
            &borin.token,
            "POST",
            &format!("{path}/buy"),
            Some(json!({ "line": "obj_epee_bonne_facture", "discounted": true })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(
        r.body["data"]["purse"]["amount"].as_u64(),
        Some(20 - (15 + surcharge).div_ceil(2))
    );
    assert!(
        price_of(&r.body["data"]["shops"][0], "Épée de bonne facture")["discountedPrice"].is_null()
    );
    let r = t
        .play(
            &borin.token,
            "POST",
            &format!("{path}/buy"),
            Some(json!({ "line": "obj_corde", "discounted": true })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NO_DISCOUNT");

    // The GM brings the compass out after a talk: one in stock.
    t.ok(
        "POST",
        &format!("{path}/reveal"),
        Some(json!({ "line": "obj_boussole_amiral" })),
    )
    .await;
    let market = &t.trade(&borin).await["shops"][0];
    let compass = price_of(market, "Boussole d'Amiral");
    assert_eq!(
        (compass["price"].as_u64(), compass["stock"].as_u64()),
        (Some(25 + surcharge), Some(1))
    );
    t.gold(&borin, 40, "or").await;
    let buy = format!("{path}/buy");
    let buy_compass = || {
        t.play(
            &borin.token,
            "POST",
            &buy,
            Some(json!({ "line": "obj_boussole_amiral" })),
        )
    };
    assert_eq!(buy_compass().await.status, StatusCode::OK);
    assert_eq!(buy_compass().await.body["error"]["code"], "SOLD_OUT");
    assert!(
        t.bag(&borin)
            .await
            .contains(&"Boussole d'Amiral".to_string())
    );

    // The GM reads every purchase and haggle in the history and the shop.
    let screen = t.ok("GET", "/shops", None).await;
    assert_eq!(screen["shops"][0]["haggles"].as_array().unwrap().len(), 2);
    let sheets = t.ok("GET", "/sheets", None).await;
    assert!(
        sheets["history"]
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["actor"] == "player" && h["kind"] == "resource")
    );

    // Sharing: Borin hands money and the compass to Lyra.
    let lyra_gold = t.trade(&lyra).await["purse"]["amount"].as_i64().unwrap();
    let r = t
        .play(
            &borin.token,
            "POST",
            "/character/give",
            Some(json!({ "to": lyra.character, "amount": 3 })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(
        t.trade(&lyra).await["purse"]["amount"].as_i64(),
        Some(lyra_gold + 3)
    );
    let me = t.play(&borin.token, "GET", "/me", None).await;
    let key = me.body["data"]["character"]["play"]["inventory"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "Boussole d'Amiral")
        .unwrap()["key"]
        .clone();
    let r = t
        .play(
            &borin.token,
            "POST",
            "/character/give",
            Some(json!({ "to": lyra.character, "entry": key })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(
        t.bag(&lyra)
            .await
            .contains(&"Boussole d'Amiral".to_string())
    );
    assert!(
        !t.bag(&borin)
            .await
            .contains(&"Boussole d'Amiral".to_string())
    );
    let r = t
        .play(
            &borin.token,
            "POST",
            "/character/give",
            Some(json!({ "to": borin.character, "amount": 1 })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "INVALID_GIFT");
    let r = t
        .play(
            &borin.token,
            "POST",
            "/character/give",
            Some(json!({ "to": lyra.character, "amount": 999 })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NOT_ENOUGH");
    let r = t
        .play(
            &lea.token,
            "POST",
            "/character/give",
            Some(json!({ "to": lyra.character, "amount": 1 })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NO_CHARACTER");
    let companions = t.trade(&borin).await["companions"].clone();
    assert_eq!(
        companions,
        json!([{ "id": lyra.character, "name": "Lyra" }])
    );
}

#[tokio::test]
async fn a_brasier_trader_once_the_rules_have_a_currency() {
    let t = Table::new(BRASIER).await;
    let vex = t.seat("Inès", "Vex", "pilote").await;
    let kael = t.seat("Hugo", "Kael", "mecano").await;

    // The Brasier's rules name no money yet: the shop says so.
    let r = t
        .gm(
            "POST",
            "/shops",
            Some(json!({ "name": "Le comptoir céphalopode" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"]["code"], "NO_CURRENCY");

    // The GM adds the crew's credits in the rules editor; the version
    // applies from the next session.
    let draft = t.ok("POST", "/rules/draft", None).await;
    let mut doc = draft["draft"]["document"].clone();
    doc["resources"] = json!([{ "id": "credits", "name": "Crédits", "abbr": "CR", "start": 0 }]);
    t.ok(
        "PUT",
        "/rules/draft",
        Some(json!({ "document": doc, "note": "Les crédits de l'équipage." })),
    )
    .await;
    t.ok("POST", "/rules/draft/lock", None).await;
    t.ok("POST", "/session", None).await;

    let screen = t
        .ok(
            "POST",
            "/shops",
            Some(json!({ "name": "Le comptoir céphalopode", "keeper": "Ix'tli" })),
        )
        .await;
    let id = screen["shops"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(screen["currency"]["abbr"], "CR");
    let path = format!("/shops/{id}");
    let r = t
        .gm(
            "PUT",
            &path,
            Some(json!({
                "name": "Le comptoir céphalopode",
                "keeper": "Ix'tli",
                "lines": [
                    { "item": "stim_de_combat", "price": 6, "stock": 3 },
                    { "storyItem": "obj_chitine_vorr", "price": 5, "hidden": true },
                    { "name": "Carte stellaire volée", "price": 50 },
                ],
            })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let r = t
        .gm(
            "PUT",
            &path,
            Some(json!({ "name": "X", "lines": [{ "item": "epee_laser", "price": 1 }] })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "UNKNOWN_ITEM");
    t.ok(
        "POST",
        &format!("{path}/open"),
        Some(json!({ "open": true })),
    )
    .await;

    let view = t.trade(&vex).await;
    assert_eq!(view["purse"]["name"], "Crédits");
    assert_eq!(view["purse"]["amount"], 0);
    let market = &view["shops"][0];
    assert_eq!(market["abbr"], "CR");
    assert_eq!(market["lines"].as_array().unwrap().len(), 2);
    assert!(!view.to_string().contains("chitine"), "{view}");
    let stim = price_of(market, "Stim de combat");
    let key = stim["key"].as_str().unwrap().to_string();
    let r = t
        .play(
            &vex.token,
            "POST",
            &format!("{path}/buy"),
            Some(json!({ "line": key })),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "NOT_ENOUGH");
    t.gold(&vex, 10, "credits").await;
    let r = t
        .play(
            &vex.token,
            "POST",
            &format!("{path}/buy"),
            Some(json!({ "line": key })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["purse"]["amount"], 4);
    assert_eq!(
        price_of(&r.body["data"]["shops"][0], "Stim de combat")["stock"],
        2
    );
    // This keeper does not haggle.
    let r = t
        .play(&vex.token, "POST", &format!("{path}/haggle"), None)
        .await;
    assert_eq!(r.body["error"]["code"], "NO_HAGGLE");

    // The stim goes to Kael.
    let me = t.play(&vex.token, "GET", "/me", None).await;
    let entry = me.body["data"]["character"]["play"]["inventory"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "Stim de combat")
        .unwrap()["key"]
        .clone();
    let r = t
        .play(
            &vex.token,
            "POST",
            "/character/give",
            Some(json!({ "to": kael.character, "entry": entry, "qty": 1 })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(t.bag(&kael).await.contains(&"Stim de combat".to_string()));
}

/// A purse change moves at most a thousand at once (`play::DELTA_MAX`),
/// yet the GM may price a line far above that: such a line is paid in
/// full, and a purse short of it pays nothing.
#[tokio::test]
async fn a_line_dearer_than_one_purse_change_is_paid_in_full() {
    let t = Table::new(CORSAIRES).await;
    let borin = t.seat("Marc", "Borin", "canonnier").await;
    let screen = t
        .ok(
            "POST",
            "/shops",
            Some(json!({ "name": "Le chantier naval" })),
        )
        .await;
    let id = screen["shops"][0]["id"].as_str().unwrap().to_string();
    let path = format!("/shops/{id}");
    t.ok(
        "PUT",
        &path,
        Some(json!({
            "name": "Le chantier naval",
            "lines": [{ "name": "Une goélette", "price": 2500 }],
        })),
    )
    .await;
    t.ok(
        "POST",
        &format!("{path}/open"),
        Some(json!({ "open": true })),
    )
    .await;
    let start = t.trade(&borin).await["purse"]["amount"].as_i64().unwrap();
    for _ in 0..2 {
        t.gold(&borin, 1000, "or").await;
    }
    let key = price_of(&t.trade(&borin).await["shops"][0], "Une goélette")["key"]
        .as_str()
        .unwrap()
        .to_string();
    let buy_path = format!("{path}/buy");
    let buy = || {
        t.play(
            &borin.token,
            "POST",
            &buy_path,
            Some(json!({ "line": key })),
        )
    };
    if start + 2000 < 2500 {
        let r = buy().await;
        assert_eq!(r.body["error"]["code"], "NOT_ENOUGH", "{}", r.body);
        assert_eq!(t.trade(&borin).await["purse"]["amount"], start + 2000);
        assert!(!t.bag(&borin).await.contains(&"Une goélette".to_string()));
    }
    t.gold(&borin, 1000, "or").await;
    let r = buy().await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["purse"]["amount"], start + 3000 - 2500);
    assert!(t.bag(&borin).await.contains(&"Une goélette".to_string()));
}
