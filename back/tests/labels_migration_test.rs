//! Migration 048 (`campaign/pin-scenes-on-maps`) on a database written
//! before it: the scene a label named becomes its place's anchor where
//! that pins nothing else, and every stored copy of a map loses the
//! field, so each still loads under `deny_unknown_fields`.

mod common;

use std::borrow::Cow;
use std::str::FromStr;

use promptus_shared::maps::Map;
use promptus_shared::story::{Campaign, from_yaml};
use serde_json::{Value, json};
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");
const MAP: &str = include_str!("../../content/maps/corsaires/port-louis-rues.yaml");
const MIGRATION: &str = include_str!("../migrations/048_labels_without_scenes.sql");
static ALL: Migrator = sqlx::migrate!("./migrations");

/// A pool on a fresh schema of the test database, migrated up to 047.
async fn before_048() -> PgPool {
    let url = common::test_database_url();
    let schema = format!("t_{}", Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
    let options = PgConnectOptions::from_str(&url)
        .unwrap()
        .options([("search_path", schema.as_str())]);
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .unwrap();
    let up_to_047 = Migrator {
        migrations: Cow::Owned(
            ALL.migrations
                .iter()
                .filter(|m| m.version <= 47)
                .cloned()
                .collect(),
        ),
        ..Migrator::DEFAULT
    };
    up_to_047.run(&pool).await.unwrap();
    pool
}

/// The fixture as a campaign imported before anchors: none but the
/// crique's (an anchor already there is kept), the phare's scene with no
/// place, the port inside Port-Louis, and the Corsaires' `sc_port_nuit`.
fn old_story(rules: &str) -> Value {
    let mut story = serde_json::to_value(from_yaml(FIXTURE).unwrap()).unwrap();
    story["rules"]["id"] = json!(rules);
    for place in story["locations"].as_array_mut().unwrap() {
        if place["id"] != "lieu_crique" {
            place.as_object_mut().unwrap().remove("anchor");
        }
        if place["id"] == "lieu_port" {
            place["parent"] = json!("lieu_port_louis");
        }
    }
    story["locations"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": "lieu_port_louis", "name": "Port-Louis" }));
    let nodes = story["nodes"].as_array_mut().unwrap();
    let mut nuit = nodes.iter().find(|n| n["id"] == "sc_port").unwrap().clone();
    nuit["id"] = json!("sc_port_nuit");
    nodes.push(nuit);
    for node in nodes.iter_mut() {
        if node["id"] == "sc_phare" {
            node.as_object_mut().unwrap().remove("location");
        }
    }
    story
}

/// A whole map as stored before 048, its labels naming scenes.
fn old_map() -> Value {
    let mut map = serde_json::to_value(Map::from_yaml(MAP).unwrap()).unwrap();
    map["id"] = json!("carte-du-port");
    map["labels"] = json!([
        { "text": "Le Goéland", "at": [2, 1], "scene": "sc_taverne" },
        { "text": "Le port", "at": [8, 10], "scene": "sc_port" },
        { "text": "La crique", "at": [1, 1], "scene": "sc_crique" },
        { "text": "Le phare", "at": [3, 3], "scene": "sc_phare" },
        { "text": "Ailleurs", "at": [4, 4], "scene": "sc_inconnue" },
        { "text": "Le Goéland, encore", "at": [5, 5], "scene": "sc_taverne", "layer": "secrets" },
        { "text": "Les quais", "at": [6, 6] },
    ]);
    map
}

fn anchor_of(story: &Value, place: &str) -> Value {
    story["locations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"] == place)
        .unwrap()
        .get("anchor")
        .cloned()
        .unwrap_or(Value::Null)
}

async fn stored(pool: &PgPool) -> Vec<Value> {
    sqlx::query(
        "SELECT jsonb_build_array(
           (SELECT jsonb_agg(story ORDER BY id) FROM campaigns),
           (SELECT jsonb_agg(map) FROM campaign_maps),
           (SELECT jsonb_agg(map) FROM map_states),
           (SELECT jsonb_agg(fight) FROM encounters),
           (SELECT jsonb_agg(battle) FROM battles),
           (SELECT jsonb_agg(inverse) FROM prep_revisions)) AS all",
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .get::<Value, _>("all")
    .as_array()
    .unwrap()
    .clone()
}

#[tokio::test]
async fn label_scenes_become_anchors_and_leave_every_stored_map() {
    let pool = before_048().await;
    let gm: Uuid =
        sqlx::query_scalar("INSERT INTO gms (display_name) VALUES ('Romain') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    let corsaires: Uuid =
        sqlx::query_scalar("INSERT INTO campaigns (gm_id, story) VALUES ($1, $2) RETURNING id")
            .bind(gm)
            .bind(old_story("corsaires"))
            .fetch_one(&pool)
            .await
            .unwrap();
    let elsewhere: Uuid =
        sqlx::query_scalar("INSERT INTO campaigns (gm_id, story) VALUES ($1, $2) RETURNING id")
            .bind(gm)
            .bind(old_story("brasier"))
            .fetch_one(&pool)
            .await
            .unwrap();
    let map = old_map();
    sqlx::query(
        "INSERT INTO campaign_maps (campaign_id, id, map, source)
         VALUES ($1, 'carte-du-port', $2, 'editor')",
    )
    .bind(corsaires)
    .bind(&map)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO map_states (campaign_id, map_id, map) VALUES ($1, 'carte-du-port', $2)",
    )
    .bind(corsaires)
    .bind(&map)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO encounters (campaign_id, node, fight) VALUES ($1, 'sc_crique', $2)")
        .bind(corsaires)
        .bind(json!({ "map": map, "round": 3 }))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO battles (campaign_id, node, battle) VALUES ($1, 'sc_crique', $2)")
        .bind(corsaires)
        .bind(json!({ "map": map, "round": 2 }))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO prep_revisions (campaign_id, revision, author, target, changes, inverse)
         VALUES ($1, 1, 'gm', 'map:carte-du-port', '[]', $2)",
    )
    .bind(corsaires)
    .bind(&map)
    .execute(&pool)
    .await
    .unwrap();

    ALL.run(&pool).await.unwrap();

    let story_of = |id: Uuid| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, Value>("SELECT story FROM campaigns WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    let story = story_of(corsaires).await;
    // A place with nothing inside takes the first label naming its scene.
    assert_eq!(
        anchor_of(&story, "lieu_taverne"),
        json!({ "map": "carte-du-port", "at": [2, 1] })
    );
    // An anchor already there stays; a place holding others is not
    // anchored by a label (it would pin all of them there); a scene with
    // no place stays on no map.
    assert_eq!(
        anchor_of(&story, "lieu_crique"),
        json!({ "map": "cotes-bretagne-sud", "at": [2, 5] })
    );
    assert_eq!(anchor_of(&story, "lieu_port"), Value::Null);
    assert_eq!(anchor_of(&story, "lieu_phare"), Value::Null);
    // The Corsaires' world map named `sc_port_nuit` on Port-Louis.
    assert_eq!(
        anchor_of(&story, "lieu_port_louis"),
        json!({ "map": "cotes-bretagne-sud", "at": [5, 0] })
    );
    // Another world's campaign is not touched by the Corsaires' map.
    assert_eq!(
        anchor_of(&story_of(elsewhere).await, "lieu_port_louis"),
        Value::Null
    );

    // Every stored copy loses the field and loads again.
    let all = stored(&pool).await;
    for story in all[0].as_array().unwrap() {
        serde_json::from_value::<Campaign>(story.clone()).expect("the story loads");
    }
    let maps: Vec<Value> = [
        all[1][0].clone(),
        all[2][0].clone(),
        all[3][0]["map"].clone(),
        all[4][0]["map"].clone(),
        all[5][0].clone(),
    ]
    .into();
    for m in &maps {
        let loaded: Map = serde_json::from_value(m.clone()).expect("the map loads");
        assert_eq!(loaded.labels.len(), 7);
        assert_eq!(loaded.labels[5].layer, "secrets");
    }
    assert_eq!(all[3][0]["round"], 3, "the rest of the fight is kept");
    assert_eq!(all[4][0]["round"], 2, "the rest of the battle is kept");

    // A second pass changes nothing.
    sqlx::raw_sql(MIGRATION).execute(&pool).await.unwrap();
    assert_eq!(stored(&pool).await, all);
}
