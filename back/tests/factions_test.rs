//! campaign/track-factions-and-goals over the API, on both witness
//! worlds: the GM moves a gauge, the rivals pay, the players see only
//! the factions they know; a goal known then ticked off; the companion
//! the GM makes speak whatever the scene.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use promptus_back::ai::Ai;
use promptus_back::live::{LiveConfig, LiveHub};
use serde_json::{Value, json};

const BRASIER: &str = include_str!("../../content/campaigns/brasier/campagne.yaml");
const CORSAIRES: &str = include_str!("../../content/campaigns/corsaires/campagne.yaml");

struct Table {
    app: Router,
    gm: String,
    campaign: String,
    marc: String,
}

async fn table(yaml: &str) -> Table {
    let pool = common::test_pool().await;
    let app = common::app_with_ai(
        pool.clone(),
        LiveHub::new(LiveConfig::default()),
        Ai::fake(),
    );
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let campaign = imported_campaign(&app, &gm, yaml).await;
    sqlx::query("UPDATE campaigns SET ai_budget_cents = 500 WHERE id = $1::uuid")
        .bind(&campaign)
        .execute(&pool)
        .await
        .unwrap();
    let code = invite_code(&app, &gm, &campaign).await;
    let marc = join(&app, &code, "Marc", "player").await;
    let t = Table {
        app,
        gm,
        campaign,
        marc: marc.player_token().unwrap(),
    };
    let r = t.gm("POST", "/session", None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let r = t.gm("POST", "/session/start", None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    t
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn reveal(&self, body: Value) -> Reply {
        self.gm("POST", "/session/reveal", Some(body)).await
    }

    /// What Marc's phone shows of the campaign.
    async fn seen(&self) -> Value {
        let uri = format!("/api/play/{}/evening", self.campaign);
        let r = call_as_player(&self.app, Some(&self.marc), "GET", &uri, None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }
}

fn gauges(view: &Value) -> Vec<(String, i64)> {
    view["campaign"]["factions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["name"].as_str().unwrap().to_string(),
                f["affinity"].as_i64().unwrap(),
            )
        })
        .collect()
}

#[tokio::test]
async fn the_brasier_gauges_move_together_and_the_table_sees_what_it_knows() {
    let t = table(BRASIER).await;
    assert!(gauges(&t.seen().await).is_empty(), "no faction met yet");

    // The crew helps the Sereth: their gauge rises, the Vorr and the
    // Céphalopodes pay for it — out of the players' sight.
    let r = t
        .reveal(json!({ "kind": "affinity", "faction": "fac_sereth", "delta": 2 }))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let view = t.seen().await;
    assert_eq!(gauges(&view), vec![("Les Sereth".to_string(), 2)]);
    let shared: Vec<&str> = view["journal"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["text"].as_str().unwrap())
        .collect();
    assert_eq!(shared, vec!["Les Sereth : +2 (2)"]);
    // The GM sees every gauge, and the hidden drops in their journal.
    let screen = t.gm("GET", "/session", None).await.body["data"].clone();
    let vorr = screen["factions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "fac_vorr")
        .unwrap()
        .clone();
    assert_eq!(
        (vorr["affinity"].as_i64(), vorr["known"].as_bool()),
        (Some(-5), Some(false))
    );
    assert!(
        screen["journal"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["text"] == "Les Vorr : -2 (-5)" && l["shared"] == false),
        "{}",
        screen["journal"]
    );

    // The Vorr attack: the players now know them, already at the floor.
    let r = t
        .reveal(json!({ "kind": "faction", "faction": "fac_vorr" }))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    assert_eq!(
        gauges(&t.seen().await),
        vec![("Les Vorr".to_string(), -5), ("Les Sereth".to_string(), 2)]
    );

    // LUMEN's assessment: four components to find; the Sereth hand over
    // the Lentille-écho.
    for goal in ["but_moelle_vive", "but_lentille_echo"] {
        let r = t
            .reveal(json!({ "kind": "goal", "goal": goal, "status": "known" }))
            .await;
        assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    }
    let r = t
        .reveal(json!({ "kind": "goal", "goal": "but_lentille_echo", "status": "done" }))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let goals = t.seen().await["campaign"]["goals"].clone();
    assert_eq!(
        goals,
        json!([
            { "title": "Obtenir la Moelle vive", "description": "Le refroidissement du moteur, sécrété par la Ruche.",
              "done": false, "heldBy": "Les Vorr" },
            { "title": "Obtenir la Lentille-écho", "description": "Le cap de la distorsion, gardé par les Sereth.",
              "done": true, "heldBy": "Les Sereth" },
        ])
    );

    // A wrong gesture is refused and changes nothing.
    for bad in [
        json!({ "kind": "affinity", "faction": "fac_sereth", "delta": 0 }),
        json!({ "kind": "affinity", "faction": "fac_inconnue", "delta": 1 }),
        json!({ "kind": "goal", "goal": "but_inconnu", "status": "done" }),
    ] {
        let r = t.reveal(bad.clone()).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bad}: {}", r.body);
    }

    // LUMEN travels with the crew: the GM makes her speak with no scene
    // shown at all, and gets a draft, nothing shown to the table.
    assert_eq!(screen["companions"][0]["id"], "pnj_lumen");
    let r = t
        .gm(
            "POST",
            "/session/copilot",
            Some(json!({ "kind": "npc", "npc": "pnj_lumen", "prompt": "Le bilan des avaries." })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["data"]["status"], "draft");
}

#[tokio::test]
async fn the_crown_and_gueule_rouge_on_the_corsaires_table() {
    let t = table(CORSAIRES).await;
    let r = t
        .reveal(json!({ "kind": "affinity", "faction": "fac_couronne", "delta": 1 }))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let r = t
        .reveal(json!({ "kind": "faction", "faction": "fac_bande_gueule_rouge" }))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let view = t.seen().await;
    assert_eq!(
        gauges(&view),
        vec![
            ("La Couronne de France".to_string(), 1),
            ("La bande de Gueule-Rouge".to_string(), -1)
        ]
    );
    // Each names the other as its rival, now that both are known.
    assert_eq!(
        view["campaign"]["factions"][0]["rivals"],
        json!(["La bande de Gueule-Rouge"])
    );
    // The Corsaires have no companion: nothing to make speak.
    let screen = t.gm("GET", "/session", None).await.body["data"].clone();
    assert_eq!(screen["companions"], json!([]));
}
