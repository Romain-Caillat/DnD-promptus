//! Importer for V1 campaigns (the frozen Next.js version), used by the
//! ported tests and to bring the V1 demo over.
//!
//! A V1 campaign came out in two parts:
//! - its **entities**, exported as YAML (`GET /api/campaigns/{id}/entities/export`:
//!   `{ campaign, exportedAt, entities: [...] }`), and imported from a
//!   single entity, a list, a `{ entities: [...] }` document, or a
//!   multi-document stream of any of these;
//! - its **story**, as JSON (`GET /api/campaigns/{id}/story`:
//!   `{ story: { bible, fronts, scenes, revelations, clues, maps } }`).
//!
//! [`parse_v1_entities`] checks entities as V1 did (entity type,
//! visibility, every effect's type, recursively). [`import_v1`] converts
//! both parts into one [`Campaign`]; what the new format has no place
//! for is listed in [`V1Import::dropped`], never silently lost. Maps are
//! converted separately by `maps::load_v1_story_maps`; a scene's battle
//! map becomes the node's `map`.
//!
//! Conversion:
//! - one act, `acte_1`: V1 had none;
//! - `character` → party slot; `npc` → NPC (faction names become
//!   factions); `monster` → adversary; `item`, `location`, `faction` →
//!   the same; `spell`, `event`, `condition` are rule-system or play
//!   data and are dropped;
//! - a scene's objective becomes its first key point, its monsters an
//!   encounter, its image prompt its art, its music link or query a
//!   track (a query alone is a track still to choose); triggers and
//!   front step effects are dropped.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use super::model::{
    Act, Adversary, Ambience, Attack, Bible, Campaign, Clue, ClueCheck, Disposition, Encounter,
    Exit, Faction, Front, FrontStep, Importance, Item, Location, MusicMood, MusicTrack, Node, Npc,
    NpcPresence, Opponents, PartyMember, Rarity, Revelation, RuleSystemRef, StatBlock,
};

/// The id of the single act an imported V1 campaign gets.
pub const V1_ACT: &str = "acte_1";

/// What V1 refused, with where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V1Error(pub String);

impl std::fmt::Display for V1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for V1Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum V1EntityType {
    Spell,
    Item,
    Npc,
    Monster,
    Character,
    Location,
    Event,
    Condition,
    Faction,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum V1Visibility {
    #[default]
    Public,
    MjOnly,
    PlayersInSession,
    SpecificUsers,
}

/// A V1 entity (`EntityInputSchema`). Unknown keys are ignored, as V1's
/// schema stripped them.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V1Entity {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: V1EntityType,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub attributes: BTreeMap<String, Value>,
    #[serde(default)]
    pub effects: Vec<Value>,
    #[serde(default)]
    pub visibility: V1Visibility,
}

/// Every effect type V1 accepted (`entity-schemas.ts`).
const V1_EFFECTS: &[&str] = &[
    "damage",
    "heal",
    "apply_condition",
    "remove_condition",
    "modify_stat",
    "consume_resource",
    "restore_resource",
    "set_state",
    "move_entity",
    "reveal_entity",
    "set_relation",
    "trigger_event",
    "add_to_inventory",
    "remove_from_inventory",
    "play_ambience",
    "play_music",
    "play_sound",
    "display_image",
    "display_text",
    "set_flag",
    "advance_front",
    "reveal_clue",
    "enter_scene",
    "set_scene_status",
    "roll_check",
];

fn check_effect(effect: &Value, path: &str) -> Result<(), V1Error> {
    let kind = effect.get("type").and_then(Value::as_str).unwrap_or("");
    if !V1_EFFECTS.contains(&kind) {
        return Err(V1Error(format!("{path}: unknown effect type `{kind}`")));
    }
    for branch in ["outcomeSuccess", "outcomeFail"] {
        if let Some(subs) = effect.get(branch).and_then(Value::as_array) {
            for (i, sub) in subs.iter().enumerate() {
                check_effect(sub, &format!("{path}.{branch}[{i}]"))?;
            }
        }
    }
    Ok(())
}

fn entity(raw: Value, n: usize) -> Result<V1Entity, V1Error> {
    let e: V1Entity =
        serde_json::from_value(raw).map_err(|err| V1Error(format!("entity {n}: {err}")))?;
    let len = e.name.chars().count();
    if len == 0 || len > 200 {
        return Err(V1Error(format!(
            "entity {n}: a name has 1 to 200 characters"
        )));
    }
    for (i, effect) in e.effects.iter().enumerate() {
        check_effect(effect, &format!("entity {n} ({}): effects[{i}]", e.name))?;
    }
    Ok(e)
}

/// Parse V1 entity YAML: one entity, a list, `{ entities: [...] }` (the
/// export), or a multi-document stream of these. Entities are numbered
/// from 1 in errors, as V1 did.
///
/// # Errors
///
/// Invalid YAML, an entity V1 would have refused, or no entity at all.
pub fn parse_v1_entities(yaml: &str) -> Result<Vec<V1Entity>, V1Error> {
    let mut raws = Vec::new();
    for doc in serde_yaml_ng::Deserializer::from_str(yaml) {
        let value = Value::deserialize(doc).map_err(|e| V1Error(e.to_string()))?;
        match value {
            Value::Null => {}
            Value::Array(list) => raws.extend(list),
            Value::Object(mut map) if map.get("entities").is_some_and(Value::is_array) => {
                if let Some(Value::Array(list)) = map.remove("entities") {
                    raws.extend(list);
                }
            }
            other => raws.push(other),
        }
    }
    if raws.is_empty() {
        return Err(V1Error("no entity in the text".into()));
    }
    raws.into_iter()
        .enumerate()
        .map(|(i, raw)| entity(raw, i + 1))
        .collect()
}

#[derive(Debug, Deserialize)]
struct V1StoryEnvelope {
    story: V1Story,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct V1Story {
    bible: V1Bible,
    fronts: Vec<V1Front>,
    scenes: Vec<V1Scene>,
    revelations: Vec<V1Revelation>,
    clues: Vec<V1Clue>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct V1Bible {
    pitch: String,
    tone: String,
    themes: Vec<String>,
    truths: Vec<String>,
    secrets: Vec<String>,
    player_hook: String,
    start_scene_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct V1Front {
    id: String,
    name: String,
    #[serde(default)]
    goal: String,
    #[serde(default)]
    description: String,
    steps: Vec<V1FrontStep>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct V1FrontStep {
    label: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    effects: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct V1Scene {
    id: String,
    title: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    objective: String,
    #[serde(default)]
    read_aloud: String,
    #[serde(default)]
    gm_notes: Option<String>,
    #[serde(default)]
    phase: Option<String>,
    #[serde(default)]
    location_entity_id: Option<String>,
    #[serde(default)]
    npc_entity_ids: Vec<String>,
    #[serde(default)]
    monster_entity_ids: Vec<String>,
    #[serde(default)]
    exits: Vec<V1Exit>,
    #[serde(default)]
    battle_map_id: Option<String>,
    #[serde(default)]
    triggers: Vec<Value>,
    #[serde(default)]
    media: Option<V1Media>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct V1Exit {
    to_scene_id: String,
    #[serde(default)]
    label: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct V1Media {
    image_prompt: Option<String>,
    music_url: Option<String>,
    music_query: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct V1Revelation {
    id: String,
    statement: String,
    #[serde(default)]
    importance: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct V1Clue {
    id: String,
    revelation_id: String,
    scene_id: String,
    text: String,
    #[serde(default)]
    discovery: String,
    #[serde(default)]
    check: Option<V1ClueCheck>,
}

#[derive(Debug, Deserialize)]
struct V1ClueCheck {
    #[serde(default)]
    skill: Option<String>,
    #[serde(default)]
    ability: Option<String>,
    #[serde(default)]
    dc: Option<u32>,
}

/// A converted V1 campaign, and what had no place in the new format.
#[derive(Debug, Clone, PartialEq)]
pub struct V1Import {
    pub campaign: Campaign,
    /// One line per thing left behind, in English, for the importer's log.
    pub dropped: Vec<String>,
}

/// A slug for an id: lowercase ASCII letters and digits, French accents
/// folded, anything else a single `-`.
#[must_use]
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars().flat_map(char::to_lowercase) {
        let folded = match ch {
            'à' | 'â' | 'ä' | 'á' => "a",
            'é' | 'è' | 'ê' | 'ë' => "e",
            'î' | 'ï' | 'í' => "i",
            'ô' | 'ö' | 'ó' => "o",
            'ù' | 'û' | 'ü' | 'ú' => "u",
            'ç' => "c",
            'œ' => "oe",
            'æ' => "ae",
            c if c.is_ascii_alphanumeric() => {
                out.push(c);
                continue;
            }
            _ => "-",
        };
        if folded == "-" {
            if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
        } else {
            out.push_str(folded);
        }
    }
    out.trim_end_matches('-').to_string()
}

fn attr_i32(e: &V1Entity, key: &str) -> Option<i32> {
    e.attributes
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|n| i32::try_from(n).ok())
}

fn attr_str(e: &V1Entity, key: &str) -> String {
    e.attributes
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn mood(phase: Option<&str>) -> MusicMood {
    match phase {
        Some("combat") => MusicMood::Combat,
        Some("dialogue" | "rest") => MusicMood::Calm,
        _ => MusicMood::Exploration,
    }
}

fn stat_block(e: &V1Entity) -> StatBlock {
    let abilities = e
        .attributes
        .get("abilityScores")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((k.clone(), i32::try_from(v.as_i64()?).ok()?)))
                .collect()
        })
        .unwrap_or_default();
    let attacks = e
        .attributes
        .get("attacks")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|a| {
                    Some(Attack {
                        name: a.get("name")?.as_str()?.to_string(),
                        damage: a.get("damage")?.as_str()?.to_string(),
                        notes: a
                            .get("bonus")
                            .and_then(Value::as_i64)
                            .map(|b| format!("{b:+} pour toucher"))
                            .unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    StatBlock {
        from_rules: None,
        abilities,
        armor_class: attr_i32(e, "ac"),
        hit_points: attr_i32(e, "hpMax").or_else(|| attr_i32(e, "hp")),
        attacks,
    }
}

/// Convert a V1 campaign: its entity export (YAML) and its story (JSON),
/// to play with `rules`.
///
/// # Errors
///
/// When either part is not what V1 produced.
pub fn import_v1(
    entities_yaml: &str,
    story_json: &str,
    rules: RuleSystemRef,
) -> Result<V1Import, V1Error> {
    let entities = parse_v1_entities(entities_yaml)?;
    let title = serde_yaml_ng::from_str::<Value>(entities_yaml)
        .ok()
        .and_then(|v| {
            v.get("campaign")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "Campagne importée".to_string());
    let story = serde_json::from_str::<V1StoryEnvelope>(story_json)
        .map(|e| e.story)
        .map_err(|e| V1Error(format!("story: {e}")))?;

    let mut dropped = Vec::new();
    let mut c = Campaign::empty(&slug(&title), &title, "", rules);
    let b = story.bible;
    c.bible = Bible {
        pitch: b.pitch,
        tone: b.tone,
        themes: b.themes,
        art_direction: String::new(),
        truths: b.truths,
        secrets: b.secrets,
        player_hook: b.player_hook,
        start_node: b.start_scene_id,
    };
    c.acts.push(Act {
        id: V1_ACT.into(),
        title: title.clone(),
        summary: String::new(),
        opening: String::new(),
        closing: String::new(),
        music: Vec::new(),
        gm_notes: "Importé du V1, qui n'avait pas d'actes.".into(),
    });

    for (n, e) in entities.into_iter().enumerate() {
        let id =
            e.id.clone()
                .unwrap_or_else(|| format!("ent_{}", slug(&e.name).replace('-', "_")));
        let description = e.description.clone().unwrap_or_default();
        if !e.effects.is_empty() && e.kind != V1EntityType::Spell {
            dropped.push(format!("{id}: {} effect(s)", e.effects.len()));
        }
        match e.kind {
            V1EntityType::Character => c.party.push(PartyMember {
                id,
                name: e.name.clone(),
                player: String::new(),
                concept: description,
                class: None,
            }),
            V1EntityType::Npc => {
                let faction = attr_str(&e, "faction");
                let faction = (!faction.is_empty()).then(|| {
                    let fid = format!("fac_{}", slug(&faction));
                    if c.factions.iter().all(|f| f.id != fid) {
                        c.factions.push(Faction {
                            id: fid.clone(),
                            name: faction.clone(),
                            description: String::new(),
                            diplomacy: String::new(),
                            affinity: super::model::Affinity::default(),
                            rivals: Vec::new(),
                            art: String::new(),
                            gm_notes: String::new(),
                        });
                    }
                    fid
                });
                let stats = stat_block(&e);
                c.npcs.push(Npc {
                    id,
                    name: e.name.clone(),
                    title: String::new(),
                    age: String::new(),
                    appearance: description,
                    portrait: String::new(),
                    roleplay: String::new(),
                    traits: e.tags.clone(),
                    flaw: String::new(),
                    motivation: attr_str(&e, "motivation"),
                    disposition: Disposition::Neutral,
                    wants: String::new(),
                    hides: attr_str(&e, "secret"),
                    stats: (stats != StatBlock::default()).then_some(stats),
                    inventory: Vec::new(),
                    sells: Vec::new(),
                    faction,
                    location: None,
                    companion: false,
                    gm_notes: String::new(),
                });
            }
            V1EntityType::Monster => c.adversaries.push(Adversary {
                stats: stat_block(&e),
                id,
                name: e.name.clone(),
                description,
                art: String::new(),
                gm_notes: e.tags.join(", "),
            }),
            V1EntityType::Item => c.items.push(Item {
                id,
                name: e.name.clone(),
                description,
                effect: String::new(),
                from_rules: None,
                value: attr_i32(&e, "value").and_then(|v| u32::try_from(v).ok()),
                rarity: match attr_str(&e, "rarity").as_str() {
                    "uncommon" => Rarity::Uncommon,
                    "rare" => Rarity::Rare,
                    "very_rare" | "epic" => Rarity::Epic,
                    "legendary" => Rarity::Legendary,
                    _ => Rarity::Common,
                },
                art: String::new(),
                gm_notes: String::new(),
            }),
            V1EntityType::Location => c.locations.push(Location {
                id,
                name: e.name.clone(),
                description,
                parent: None,
                art: String::new(),
                gm_notes: String::new(),
            }),
            V1EntityType::Faction => c.factions.push(Faction {
                id,
                name: e.name.clone(),
                description,
                diplomacy: String::new(),
                affinity: super::model::Affinity::default(),
                rivals: Vec::new(),
                art: String::new(),
                gm_notes: String::new(),
            }),
            V1EntityType::Spell | V1EntityType::Event | V1EntityType::Condition => {
                dropped.push(format!(
                    "entity {} `{}`: a {:?} belongs to the rule system or to play",
                    n + 1,
                    e.name,
                    e.kind
                ));
            }
        }
    }

    for f in story.fronts {
        let effects: usize = f.steps.iter().map(|s| s.effects.len()).sum();
        if effects > 0 {
            dropped.push(format!("{}: {effects} step effect(s)", f.id));
        }
        c.fronts.push(Front {
            id: f.id,
            name: f.name,
            goal: f.goal,
            description: f.description,
            steps: f
                .steps
                .into_iter()
                .map(|s| FrontStep {
                    label: s.label,
                    description: s.description,
                })
                .collect(),
        });
    }

    for s in story.scenes {
        if !s.triggers.is_empty() {
            dropped.push(format!("{}: {} trigger(s)", s.id, s.triggers.len()));
        }
        let media = s.media.unwrap_or_default();
        let music = match (media.music_url, media.music_query) {
            (Some(url), query) => vec![MusicTrack {
                mood: mood(s.phase.as_deref()),
                title: query.clone().unwrap_or_else(|| s.title.clone()),
                url,
                search: query.unwrap_or_default(),
            }],
            (None, Some(query)) => vec![MusicTrack {
                mood: mood(s.phase.as_deref()),
                title: query.clone(),
                url: String::new(),
                search: query,
            }],
            (None, None) => Vec::new(),
        };
        let encounter = (!s.monster_entity_ids.is_empty()).then(|| Encounter {
            opponents: s
                .monster_entity_ids
                .iter()
                .map(|who| Opponents {
                    who: who.clone(),
                    count: 1,
                })
                .collect(),
            tactics: Vec::new(),
            morale: Vec::new(),
            on_victory: String::new(),
            on_defeat: String::new(),
            vehicles: None,
        });
        c.nodes.push(Node {
            id: s.id,
            act: V1_ACT.into(),
            title: s.title,
            optional: false,
            summary: s.summary,
            location: s.location_entity_id,
            map: s.battle_map_id,
            read_aloud: s.read_aloud,
            ambience: Ambience {
                mood: String::new(),
                sounds: String::new(),
                music,
            },
            flow: String::new(),
            hook: String::new(),
            checks: Vec::new(),
            npcs: s
                .npc_entity_ids
                .into_iter()
                .map(|npc| NpcPresence {
                    npc,
                    role: String::new(),
                })
                .collect(),
            key_points: if s.objective.is_empty() {
                Vec::new()
            } else {
                vec![format!("Objectif : {}", s.objective)]
            },
            encounter,
            loot: Vec::new(),
            xp: Vec::new(),
            transition: String::new(),
            exits: s
                .exits
                .into_iter()
                .map(|x| Exit {
                    to: x.to_scene_id,
                    label: x.label,
                })
                .collect(),
            requires: Vec::new(),
            player_hooks: Vec::new(),
            if_skipped: String::new(),
            art: media.image_prompt.unwrap_or_default(),
            gm_notes: s.gm_notes.unwrap_or_default(),
        });
    }

    c.revelations = story
        .revelations
        .into_iter()
        .map(|r| Revelation {
            id: r.id,
            statement: r.statement,
            importance: if r.importance.as_deref() == Some("optional") {
                Importance::Optional
            } else {
                Importance::Critical
            },
        })
        .collect();
    c.clues = story
        .clues
        .into_iter()
        .map(|cl| Clue {
            id: cl.id,
            revelation: cl.revelation_id,
            node: cl.scene_id,
            text: cl.text,
            discovery: cl.discovery,
            source: None,
            check: cl.check.and_then(|k| {
                Some(ClueCheck {
                    stat: k.ability.or(k.skill)?,
                    difficulty: k.dc?,
                })
            }),
        })
        .collect();
    Ok(V1Import {
        campaign: c,
        dropped,
    })
}
