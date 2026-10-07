//! campaign/track-factions-and-goals, over the API: the GM moves a
//! faction's gauge (its rivals follow), meets a faction, ticks a goal;
//! the phones see the met factions and the goals, never a rival the
//! table has not met; the party's companion speaks in every scene.

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call, call_as_player, imported_campaign, invite_code, join};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");

struct Table {
    app: Router,
    gm: String,
    campaign: String,
    lea: String,
}

/// Romain's table, Corentin going everywhere with the party; Léa
/// watches.
async fn table() -> Table {
    let pool = common::test_pool().await;
    let app = common::app(pool.clone());
    let (_, gm) = common::signed_in_gm(&pool, "Romain").await;
    let yaml = FIXTURE.replacen(
        "  - id: pnj_corentin\n",
        "  - id: pnj_corentin\n    permanent: true\n",
        1,
    );
    let campaign = imported_campaign(&app, &gm, &yaml).await;
    let code = invite_code(&app, &gm, &campaign).await;
    let lea = join(&app, &code, "Léa", "spectator").await;
    Table {
        app,
        gm,
        campaign,
        lea: lea.player_token().unwrap(),
    }
}

impl Table {
    async fn gm(&self, method: &str, path: &str, body: Option<Value>) -> Reply {
        let uri = format!("/api/campaigns/{}{path}", self.campaign);
        call(&self.app, Some(&self.gm), method, &uri, body).await
    }

    async fn reveal(&self, body: Value) -> Reply {
        self.gm("POST", "/session/reveal", Some(body)).await
    }

    async fn phone(&self) -> Value {
        let uri = format!("/api/play/{}/evening", self.campaign);
        let r = call_as_player(&self.app, Some(&self.lea), "GET", &uri, None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }

    async fn screen(&self) -> Value {
        let r = self.gm("GET", "/session", None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body["data"].clone()
    }
}

fn texts(journal: &Value) -> Vec<(String, bool)> {
    journal
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["text"].as_str().unwrap().to_string(),
                l["shared"].as_bool().unwrap_or(true),
            )
        })
        .collect()
}

#[tokio::test]
async fn winning_the_customs_over_costs_the_wreckers_and_the_table_sees_it() {
    let t = table().await;
    // Only in a live session.
    let r = t
        .reveal(json!({ "kind": "faction", "faction": "fac_douane", "delta": 2 }))
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    t.gm("POST", "/session", None).await;
    t.gm("POST", "/session/start", None).await;

    // Nothing met yet: the phones see no faction, every goal open.
    let phone = t.phone().await;
    assert_eq!(phone["campaign"]["factions"], json!([]));
    assert_eq!(
        phone["campaign"]["goals"],
        json!([{ "id": "but_lumiere", "title": "Rallumer le phare", "done": false }])
    );

    let r = t
        .reveal(json!({ "kind": "faction", "faction": "fac_douane", "delta": 2 }))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);

    // The GM sees both gauges move, and who is met.
    let screen = t.screen().await;
    let gauges: Vec<(&str, i64, bool)> = screen["factions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["id"].as_str().unwrap(),
                f["affinity"].as_i64().unwrap(),
                f["met"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        gauges,
        vec![("fac_douane", 2, true), ("fac_contrebandiers", -4, false)]
    );
    assert_eq!(screen["factions"][0]["rivals"], json!(["Les naufrageurs"]));
    // The wreckers' loss is a GM line; the customs' rise is shared.
    let gm_lines = texts(&screen["journal"]);
    assert!(gm_lines.contains(&("Rencontre : La douane royale.".into(), true)));
    assert!(gm_lines.contains(&("La douane royale : affinité 0 → 2".into(), true)));
    assert!(gm_lines.contains(&("Les naufrageurs : affinité -2 → -4".into(), false)));

    // The phones: the customs only, with their gauge; not the wreckers.
    let phone = t.phone().await;
    assert_eq!(
        phone["campaign"]["factions"],
        json!([{ "id": "fac_douane", "name": "La douane royale", "affinity": 2, "min": -5, "max": 5 }])
    );
    let seen = phone.to_string();
    assert!(!seen.contains("naufrageurs"), "{seen}");
    assert!(!seen.contains("L'ordre et la loi"), "{seen}");

    // Losing favour moves no rival; meeting the wreckers shows them as
    // they stand.
    t.reveal(json!({ "kind": "faction", "faction": "fac_douane", "delta": -1 }))
        .await;
    let r = t
        .reveal(json!({ "kind": "faction", "faction": "fac_contrebandiers", "delta": 0 }))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let phone = t.phone().await;
    let gauges: Vec<(&str, i64)> = phone["campaign"]["factions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["name"].as_str().unwrap(), f["affinity"].as_i64().unwrap()))
        .collect();
    assert_eq!(
        gauges,
        vec![("La douane royale", 1), ("Les naufrageurs", -4)]
    );
    assert!(
        texts(&phone["journal"]).contains(&("Rencontre : Les naufrageurs.".into(), true)),
        "{}",
        phone["journal"]
    );

    // The goal, ticked, then reopened; ticking twice writes one line.
    for done in [true, true] {
        let r = t
            .reveal(json!({ "kind": "goal", "goal": "but_lumiere", "done": done }))
            .await;
        assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    }
    let phone = t.phone().await;
    assert_eq!(phone["campaign"]["goals"][0]["done"], true);
    let reached = texts(&phone["journal"])
        .iter()
        .filter(|(l, _)| l == "Objectif atteint : Rallumer le phare.")
        .count();
    assert_eq!(reached, 1);
    t.reveal(json!({ "kind": "goal", "goal": "but_lumiere", "done": false }))
        .await;
    assert_eq!(t.phone().await["campaign"]["goals"][0]["done"], false);
    assert_eq!(t.screen().await["goals"][0]["done"], false);

    // What the campaign does not have is refused.
    let r = t
        .reveal(json!({ "kind": "faction", "faction": "fac_x", "delta": 1 }))
        .await;
    assert_eq!(r.body["error"]["code"], "UNKNOWN_FACTION");
    let r = t
        .reveal(json!({ "kind": "faction", "faction": "fac_douane", "delta": 11 }))
        .await;
    assert_eq!(r.body["error"]["code"], "INVALID_DELTA");
    let r = t
        .reveal(json!({ "kind": "goal", "goal": "but_x", "done": true }))
        .await;
    assert_eq!(r.body["error"]["code"], "UNKNOWN_GOAL");
}

#[tokio::test]
async fn the_companion_speaks_in_a_scene_that_does_not_list_them() {
    let t = table().await;
    t.gm("POST", "/session", None).await;
    t.gm("POST", "/session/start", None).await;
    t.reveal(json!({ "kind": "scene", "node": "sc_taverne" }))
        .await;
    let screen = t.screen().await;
    let npcs: Vec<(&str, bool)> = screen["scene"]["npcs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["id"].as_str().unwrap(), n["permanent"].as_bool().unwrap()))
        .collect();
    assert_eq!(npcs, vec![("pnj_gwen", false), ("pnj_corentin", true)]);

    // In the crique, where he is listed, he appears once.
    t.reveal(json!({ "kind": "scene", "node": "sc_crique" }))
        .await;
    let screen = t.screen().await;
    let corentin = screen["scene"]["npcs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["id"] == "pnj_corentin")
        .count();
    assert_eq!(corentin, 1);
}
