//! engine/formalise-house-rules and engine/add-srd-preset, through the
//! API: the GM writes a house rule, the co-GM formalises it (invented
//! ids dropped, cases replayed), the GM adds it to the draft, every save
//! replays its cases, and players never read a rule that shows only its
//! effect. On both witness worlds and on a campaign started from the
//! SRD preset.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};
use uuid::Uuid;

const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");
const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");

async fn gm(app: &Router, token: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
    call(app, Some(token), method, path, body).await
}

fn all_pass(cases: &Value) -> bool {
    cases
        .as_array()
        .is_some_and(|c| !c.is_empty() && c.iter().all(|x| x["passed"] == true))
}

#[tokio::test]
async fn a_house_rule_is_formalised_added_and_judged_on_both_worlds() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;
    for (yaml, rule) in [(CORSAIRES, "pied_qui_glisse"), (BRASIER, "arme_enrayee")] {
        let campaign = imported_campaign(&app, &token, yaml).await;
        sqlx::query("UPDATE campaigns SET ai_budget_cents = 100 WHERE id = $1")
            .bind(Uuid::parse_str(&campaign).unwrap())
            .execute(&pool)
            .await
            .unwrap();
        let base = format!("/api/campaigns/{campaign}/rules");
        let input = json!({
            "id": rule,
            "name": "Sur un 1",
            "text": "Sur un 1 naturel en attaque, ça tourne mal pour l'attaquant."
        });

        // Without a draft there is nothing to formalise against.
        let r = gm(
            &app,
            &token,
            "POST",
            &format!("{base}/house-rules/formalise"),
            Some(input.clone()),
        )
        .await;
        assert_eq!(r.body["error"]["code"], "NO_DRAFT", "{}", r.body);

        let r = gm(&app, &token, "POST", &format!("{base}/draft"), None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        assert_eq!(r.body["data"]["draft"]["report"]["houseRules"], json!([]));

        // The co-GM's proposal: the invented trait and the case on an
        // invented class are taken out and named; the rest replays.
        let r = gm(
            &app,
            &token,
            "POST",
            &format!("{base}/house-rules/formalise"),
            Some(input.clone()),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let p = &r.body["data"];
        assert_eq!(p["problems"], json!([]), "{p}");
        let dropped = p["dropped"].as_array().unwrap();
        assert!(
            dropped.contains(&json!({ "kind": "trait", "id": "trait_invente" })),
            "{dropped:?}"
        );
        assert!(dropped.iter().any(|d| d["kind"] == "case"), "{dropped:?}");
        assert!(
            !p["formal"].to_string().contains("invent"),
            "{}",
            p["formal"]
        );
        assert!(all_pass(&p["cases"]), "{}", p["cases"]);
        assert!(!p["remark"].as_str().unwrap().is_empty());
        // Nothing was stored, and the call was counted.
        let r = gm(&app, &token, "GET", &base, None).await;
        assert!(r.body["data"]["draft"]["document"]["house_rules"].is_null());
        let calls: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ai_calls WHERE campaign_id = $1 AND purpose = 'rules.house_rule'",
        )
        .bind(Uuid::parse_str(&campaign).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(calls, 1);

        // The GM keeps it (the editor writes it into the document), and
        // adds a second one players only see the effect of.
        let formal = p["formal"].clone();
        let mut hidden = formal.clone();
        hidden["players"] = json!("effect");
        let mut doc = r.body["data"]["draft"]["document"].clone();
        doc["house_rules"] = json!([
            { "id": rule, "name": "Sur un 1", "text": input["text"], "formal": formal },
            { "id": "secret", "name": "La règle secrète", "text": "Le MJ la garde pour lui.", "formal": hidden }
        ]);
        let r = gm(
            &app,
            &token,
            "PUT",
            &format!("{base}/draft"),
            Some(json!({ "document": doc, "note": "" })),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let checks = r.body["data"]["draft"]["report"]["houseRules"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(checks.len(), 2);
        assert!(checks.iter().all(|c| all_pass(&c["cases"])), "{checks:?}");
        // Players will read the first rule's arrival, never the second's.
        let changes = r.body["data"]["draft"]["report"]["changes"].to_string();
        assert!(changes.contains("Sur un 1"), "{changes}");
        assert!(!changes.contains("secrète"), "{changes}");

        // A form the GM broke comes back with the loader's reason.
        let mut broken = input.clone();
        broken["formal"] = json!({
            "when": "miss",
            "effects": [{ "apply": { "condition": "condition_absente", "turns": 1, "to": "self" } }]
        });
        let r = gm(
            &app,
            &token,
            "POST",
            &format!("{base}/house-rules/try"),
            Some(broken),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        assert_eq!(
            r.body["data"]["problems"][0]["code"], "unknown_condition",
            "{}",
            r.body
        );
        let mut shapeless = input.clone();
        shapeless["formal"] = json!({ "quand": "toujours" });
        let r = gm(
            &app,
            &token,
            "POST",
            &format!("{base}/house-rules/try"),
            Some(shapeless),
        )
        .await;
        assert_eq!(r.body["error"]["code"], "FORMAL_INVALID");

        // Locked and played: the player reads the rule, not the secret one.
        let code = invite_code(&app, &token, &campaign).await;
        let device = join(&app, &code, "Marc", "player")
            .await
            .player_token()
            .unwrap();
        let r = gm(&app, &token, "POST", &format!("{base}/draft/lock"), None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        let r = gm(
            &app,
            &token,
            "POST",
            &format!("/api/campaigns/{campaign}/session"),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        let page = call_as_player(
            &app,
            Some(&device),
            "GET",
            &format!("/api/play/{campaign}/rules"),
            None,
        )
        .await;
        assert_eq!(page.status, StatusCode::OK, "{}", page.body);
        let names: Vec<_> = page.body["data"]["houseRules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["name"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(names, ["Sur un 1"]);
        assert!(!page.body.to_string().contains("secrète"));
    }
}

#[tokio::test]
async fn a_campaign_starts_from_the_srd_and_its_rule_shows_only_its_effect() {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, token) = common::signed_in_gm(&pool, "Romain").await;

    let r = gm(&app, &token, "GET", "/api/rule-systems", None).await;
    let srd = r.body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "srd")
        .expect("the SRD is offered")
        .clone();
    assert_eq!(srd["name"], "D&D 5e · SRD 5.1");
    let r = gm(
        &app,
        &token,
        "POST",
        "/api/campaigns",
        Some(json!({ "title": "Les Cendres de Valombre", "rules": { "id": "srd", "version": 1 } })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let campaign = r.body["data"]["id"].as_str().unwrap().to_string();
    let base = format!("/api/campaigns/{campaign}/rules");

    // A draft of the preset: the goblin ambush is fought on both
    // versions, and the board's undead rule passes its cases.
    let r = gm(&app, &token, "POST", &format!("{base}/draft"), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let report = &r.body["data"]["draft"]["report"];
    assert!(
        report["fights"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"] == "embuscade-des-gobelins" && !f["draft"].is_null()),
        "{}",
        report["fights"]
    );
    let rule = &report["houseRules"][0];
    assert_eq!(rule["id"], "morts_vivants_feu");
    assert!(all_pass(&rule["cases"]), "{}", rule["cases"]);
    let applied = &rule["cases"][0]["effects"][0];
    assert_eq!(applied["event"], "condition_applied");
    assert_eq!(applied["name"], "Effrayé");

    // A player of that campaign reads the SRD's rules: the proficiency,
    // their class's own armour — and not the undead rule.
    let code = invite_code(&app, &token, &campaign).await;
    let device = join(&app, &code, "Sef", "player")
        .await
        .player_token()
        .unwrap();
    let page = call_as_player(
        &app,
        Some(&device),
        "GET",
        &format!("/api/play/{campaign}/rules"),
        None,
    )
    .await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.body);
    let data = &page.body["data"];
    assert_eq!(data["attack"]["bonus"]["name"], "Maîtrise");
    assert_eq!(data["check"]["advantage"], true);
    assert_eq!(data["houseRules"], json!([]));
    assert!(!page.body.to_string().contains("morts-vivants"));
}
