//! maps/generate-map-llm — the model draws a scene's encounter map in
//! the campaign's tileset (`prompts/map-generation.v1.md`).
//!
//! The answer is read as a map whose id, format version, theme, scale
//! and layers are the server's; what it names that the campaign does not
//! have is dropped (a start's adversary, a check's characteristic), and
//! a map that does not validate goes back to the model once with the
//! reasons. The map lands as a draft for the editor: nothing reaches the
//! table until the GM validates it. Each call is counted.

use std::collections::BTreeMap;

use promptus_shared::maps::Map;
use promptus_shared::story::{Campaign, Node};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{CampaignMap, Source, french, fresh_id, insert, layers, look};
use crate::ai::{self, Ai, LlmRequest, Message, Role, templates};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns;
use crate::content;
use crate::error::AppError;
use sqlx::PgPool;

/// The GM asks for the map of a scene.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ask {
    pub node: String,
}

/// The drafted map and how many invented references were dropped.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    #[serde(flatten)]
    pub map: CampaignMap,
    pub dropped: usize,
    /// The model needed a second try.
    pub retried: bool,
}

/// The scene as the model reads it.
fn scene(story: &Campaign, node: &Node) -> String {
    let mut out = format!("{}\n", node.title);
    for text in [&node.summary, &node.read_aloud] {
        if !text.trim().is_empty() {
            out.push_str(text.trim());
            out.push('\n');
        }
    }
    if let Some(place) = node.location.as_deref().and_then(|l| story.location(l)) {
        out.push_str(&format!("Lieu : {} — {}\n", place.name, place.description));
    }
    if !node.key_points.is_empty() {
        out.push_str(&format!("Points clés : {}\n", node.key_points.join(" ; ")));
    }
    if let Some(e) = &node.encounter {
        out.push_str("Adversaires de la rencontre :\n");
        for o in &e.opponents {
            let name = story
                .adversary(&o.who)
                .map_or(o.who.as_str(), |a| a.name.as_str());
            out.push_str(&format!("- `{}` ({name}) × {}\n", o.who, o.count));
        }
        if !e.tactics.is_empty() {
            out.push_str(&format!("Tactique : {}\n", e.tactics.join(" ; ")));
        }
    } else {
        out.push_str("Pas de combat prévu : une carte d’exploration.\n");
    }
    out
}

/// Read the model's map, the server's fields put back, the invented
/// references dropped. `Err` carries the GM's (and the model's) words.
fn read(
    text: &str,
    id: &str,
    theme: &str,
    story: &Campaign,
    stats: &[String],
) -> Result<(Map, usize), String> {
    let mut v: Value = ai::parse_json(text).map_err(|e| e.to_string())?;
    let obj = v.as_object_mut().ok_or("la réponse n’est pas un objet")?;
    obj.insert(
        "version".into(),
        json!(promptus_shared::maps::FORMAT_VERSION),
    );
    obj.insert("id".into(), json!(id));
    obj.insert("theme".into(), json!(theme));
    obj.insert("scale".into(), json!("encounter"));
    obj.insert(
        "layers".into(),
        serde_json::to_value(layers()).map_err(|e| e.to_string())?,
    );
    for key in ["backdrop", "exits", "cell_meters"] {
        obj.remove(key);
    }
    let mut dropped = 0;
    if let Some(starts) = obj.get_mut("starts").and_then(Value::as_array_mut) {
        for s in starts.iter_mut().filter_map(Value::as_object_mut) {
            let invented = s
                .get("entity")
                .and_then(Value::as_str)
                .is_some_and(|e| story.adversary(e).is_none() && story.npc(e).is_none());
            if invented {
                s.remove("entity");
                dropped += 1;
            }
        }
    }
    if let Some(objects) = obj.get_mut("objects").and_then(Value::as_array_mut) {
        for o in objects.iter_mut().filter_map(Value::as_object_mut) {
            let invented = o
                .get("check")
                .and_then(|c| c.get("stat"))
                .and_then(Value::as_str)
                .is_some_and(|s| !stats.iter().any(|k| k == s));
            if invented {
                o.remove("check");
                dropped += 1;
            }
            if o.get("layer")
                .and_then(Value::as_str)
                .is_some_and(|l| l != "base" && l != "secrets")
            {
                o.insert("layer".into(), json!("secrets"));
            }
        }
    }
    let map: Map = serde_json::from_value(v).map_err(|e| format!("format de carte : {e}"))?;
    map.validate().map_err(|e| match e {
        promptus_shared::maps::MapError::Invalid(issues) => {
            issues.iter().map(french).collect::<Vec<_>>().join(" ; ")
        }
        other => other.to_string(),
    })?;
    Ok((map, dropped))
}

/// # Errors
///
/// 404; 400 `UNKNOWN_NODE`; 409 `AI_BUDGET_EXCEEDED`; 502
/// `AI_OUTPUT_INVALID` when the second try still does not make a map;
/// 503 `AI_NOT_CONFIGURED`.
pub async fn generate(
    pool: &PgPool,
    ai: &Ai,
    gm: &CurrentGm,
    campaign: Uuid,
    ask: &Ask,
) -> Result<Generated, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let node = row
        .story
        .node(&ask.node)
        .ok_or(AppError::BadRequest("UNKNOWN_NODE"))?;
    let (theme, floor) = look(&row.story);
    let tileset = content::theme(&row.story).and_then(|t| t.tilesets.first());
    let materials = tileset.map_or(floor, |t| {
        t.materials
            .iter()
            .map(|(id, m)| format!("{id} ({})", m.name))
            .collect::<Vec<_>>()
            .join(", ")
    });
    let props = tileset.map_or_else(String::new, |t| {
        t.props
            .iter()
            .map(|(id, p)| format!("{id} ({})", p.name))
            .collect::<Vec<_>>()
            .join(", ")
    });
    let stats: Vec<String> = row
        .rules
        .as_ref()
        .map(|r| r.abilities.iter().map(|a| a.id.clone()).collect())
        .unwrap_or_default();
    let party = row.story.party.len().max(1);
    let vars: BTreeMap<&str, String> = [
        ("scene", scene(&row.story, node)),
        ("materials", materials),
        ("props", props),
        ("stats", stats.join(", ")),
        ("party", format!("{party} personnages joueurs.")),
    ]
    .into_iter()
    .collect();
    let mut messages = templates::MAP_GENERATION
        .render(&vars)
        .map_err(|e| AppError::internal("map template", e))?;
    let id = fresh_id(pool, campaign, &row.story, &format!("carte {}", node.title)).await?;
    for attempt in 0..2 {
        let mut req = LlmRequest::json(messages.clone());
        req.max_tokens = 6_000;
        req.temperature = 0.7;
        let reply = ai
            .complete(
                pool,
                campaign,
                "map.generate",
                &templates::MAP_GENERATION,
                &req,
            )
            .await?;
        match read(&reply.text, &id, &theme, &row.story, &stats) {
            Ok((map, dropped)) => {
                let map = insert(
                    pool,
                    campaign,
                    &map,
                    Source::Generated,
                    Some(&node.id),
                    None,
                )
                .await?;
                return Ok(Generated {
                    map,
                    dropped,
                    retried: attempt > 0,
                });
            }
            Err(why) if attempt == 0 => {
                messages.push(Message {
                    role: Role::Assistant,
                    content: reply.text,
                });
                messages.push(Message::user(format!(
                    "Cette carte est refusée : {why}. Corrige-la et renvoie l’objet JSON complet."
                )));
            }
            Err(why) => {
                return Err(AppError::Upstream {
                    code: "AI_OUTPUT_INVALID",
                    detail: why,
                });
            }
        }
    }
    unreachable!("the loop returns on its second attempt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proposed_map_keeps_the_servers_fields_and_loses_what_it_invented() {
        let story = promptus_shared::story::from_yaml(include_str!(
            "../../../content/fixtures/phare-de-kerbrume.yaml"
        ))
        .unwrap();
        let answer = json!({
            "id": "autre", "theme": "autre", "version": 9, "name": "La cale",
            "grid": { "legend": { "#": { "terrain": "bois", "wall": true }, ".": { "terrain": "bois" } },
                      "rows": ["####", "#..#", "####"] },
            "starts": [{ "id": "a", "side": "foes", "at": [1, 1], "entity": "adv_contrebandier" },
                       { "id": "b", "side": "foes", "at": [2, 1], "entity": "fantome" }],
            "objects": [{ "id": "trappe", "kind": "trappe", "at": [2, 1], "layer": "cache",
                          "check": { "stat": "CHANCE", "dc": 12 } }]
        })
        .to_string();
        let (map, dropped) = read(
            &answer,
            "carte-la-cale",
            "port-1718",
            &story,
            &["SAG".into()],
        )
        .unwrap();
        assert_eq!(
            (map.id.as_str(), map.theme.as_str(), map.version),
            ("carte-la-cale", "port-1718", 1)
        );
        assert_eq!(dropped, 2);
        assert_eq!(map.starts[0].entity.as_deref(), Some("adv_contrebandier"));
        assert_eq!(map.starts[1].entity, None);
        assert_eq!(map.objects[0].layer, "secrets");
        assert!(map.objects[0].check.is_none());

        let ragged = answer.replace("\"#..#\"", "\"#...#\"");
        let why = read(&ragged, "x", "t", &story, &[]).unwrap_err();
        assert!(why.contains("format de carte"), "{why}");
        let off = answer.replace(r#""at":[2,1],"check""#, r#""at":[9,9],"check""#);
        let why = read(&off, "x", "t", &story, &[]).unwrap_err();
        assert!(why.contains("sort de la grille"), "{why}");
    }
}
