//! The deterministic provider: same request, same answer, no network.
//!
//! It answers the server's own templates the way a good model would, from
//! what the prompt contains — the co-GM's suggestions name the first clue
//! still to find, the first exit, the first front and the first NPC of
//! the context, plus one invented id that the server must drop — so the
//! whole chain (template → provider → schema → sanitising → ledger) runs
//! in tests and in development without a key (`AI_PROVIDER=fake`).
//! A campaign generation gets a small campaign drawn from the pitch, with
//! a few invented ids for the server to remove.
//! Images are small pixel patterns drawn from the prompt's hash.

use std::sync::Mutex;

use promptus_shared::sprite::Image;
use serde_json::json;

use super::{
    AiError, BoxFuture, ImageRequest, ImageResponse, LlmRequest, LlmResponse, Provider, Role,
    Usage, VideoRequest, VideoResponse,
};

/// What the fake answers with for one call.
#[derive(Debug, Default)]
pub struct FakeProvider {
    /// Every call, in order: `complete:<first words of the system>`,
    /// `generation:<step>`, `image` or `video`.
    calls: Mutex<Vec<String>>,
    /// Answer text that is not JSON (to test schema errors).
    pub broken: bool,
    /// What each call reports it cost; 0 means 1 000 µ$ (a tenth of a cent).
    pub cost_micros: i64,
    /// The very first call answers prose, not JSON (to test the retry).
    pub bad_json_first: bool,
    /// Generated scenes give each critical revelation one clue only (to
    /// test the repair).
    pub few_clues: bool,
}

impl FakeProvider {
    /// A provider whose answers are prose, not the JSON asked for.
    #[must_use]
    pub fn broken() -> Self {
        Self {
            broken: true,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn record(&self, call: String) {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(call);
    }

    fn usage(&self) -> Usage {
        Usage {
            prompt_tokens: 1_000,
            completion_tokens: 400,
            cost_micros: if self.cost_micros > 0 {
                self.cost_micros
            } else {
                1_000
            },
        }
    }
}

/// The id after `prefix` on the first line of `text` that starts with it.
fn first_id(text: &str, prefix: &str, suffix: Option<&str>) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(prefix)?;
        if let Some(s) = suffix
            && !line.contains(s)
        {
            return None;
        }
        rest.split_whitespace().next().map(str::to_string)
    })
}

fn after_line<'a>(text: &'a str, header: &str) -> Option<&'a str> {
    let at = text.find(header)?;
    text[at + header.len()..].lines().nth(1)
}

fn copilot_answer(prompt: &str) -> serde_json::Value {
    let precision = prompt
        .lines()
        .find_map(|l| l.strip_prefix("Précision du MJ : "))
        .unwrap_or("")
        .trim()
        .to_string();
    let clue = first_id(prompt, "- indice ", Some("(à trouver)"));
    let npc = first_id(prompt, "- pnj ", None);
    let front = first_id(prompt, "- front ", None);
    let exit = after_line(prompt, "Sorties :")
        .and_then(|l| l.trim_start().strip_prefix("- "))
        .and_then(|l| l.split_whitespace().next())
        .map(str::to_string);
    let mut narration =
        String::from("Le silence retombe, lourd, et chacun guette le prochain geste.");
    if !precision.is_empty() {
        narration = format!("{narration} ({precision})");
    }
    let mut suggestions = Vec::new();
    if let Some(c) = &clue {
        suggestions.push(json!({ "label": "Laisser trouver un indice", "why": "La piste s’essouffle.", "action": { "type": "reveal_clue", "clue": c } }));
    }
    if let Some(f) = &front {
        suggestions.push(json!({ "label": "Faire avancer la menace", "action": { "type": "advance_front", "front": f } }));
    }
    if let Some(n) = &exit {
        suggestions.push(json!({ "label": "Ouvrir la scène suivante", "action": { "type": "enter_scene", "node": n } }));
    }
    suggestions.push(json!({ "label": "Action fantaisiste", "action": { "type": "reveal_clue", "clue": "indice-invente" } }));
    let npc_lines = npc
        .map(|id| vec![json!({ "npc": id, "text": "Je n’ai rien vu, et vous non plus." })])
        .unwrap_or_default();
    json!({
        "narration": narration,
        "npcLines": npc_lines,
        "suggestions": suggestions,
        "gmNote": "Donnez la main à celui qui n’a pas joué depuis longtemps.",
    })
}

fn recap_answer(prompt: &str) -> serde_json::Value {
    let journal: Vec<&str> = prompt
        .split("# Ce que la table sait (journal des joueurs)")
        .nth(1)
        .and_then(|s| s.split("# Ce que le MJ sait en plus").next())
        .map(|s| {
            s.lines()
                .filter_map(|l| l.trim().strip_prefix("- "))
                .collect()
        })
        .unwrap_or_default();
    let told = if journal.is_empty() {
        "rien de notable".to_string()
    } else {
        journal.join(" ; ")
    };
    json!({
        "players": format!("Précédemment… {told}."),
        "gm": journal.iter().map(|l| format!("- {l}")).collect::<Vec<_>>().join("\n"),
    })
}

/// The workshop: reads the campaign back from the prompt's YAML and
/// proposes what a careful co-GM would — for a three-clue alert, a
/// clue of that revelation in a required scene that has none; for a
/// selected scene or a free request, a sharper motivation for the first
/// NPC and a clue in that scene — plus one edit on an invented id that
/// the server must drop.
/// The first id of each item of a `Label : id (Nom), id (Nom)` line.
fn listed<'a>(prompt: &'a str, label: &str) -> Vec<&'a str> {
    prompt
        .lines()
        .find_map(|l| l.strip_prefix(label))
        .map(|rest| {
            rest.split(", ")
                .filter_map(|item| item.split_whitespace().next())
                .collect()
        })
        .unwrap_or_default()
}

/// A walled room cut in two by a wall with a door, in the tileset of
/// the prompt: the party's starts above, one start per adversary of the
/// scene below (and one for an adversary it invents), a crate, a hidden
/// trapdoor with a check, a lantern.
fn map_answer(prompt: &str) -> serde_json::Value {
    let materials = listed(prompt, "Matériaux (terrain) : ");
    let floor = materials.first().copied().unwrap_or("sol");
    let wall = materials.get(1).copied().unwrap_or(floor);
    let prop = listed(prompt, "Décors (kind) : ")
        .first()
        .copied()
        .unwrap_or("caisse")
        .to_string();
    let stat = listed(prompt, "Caractéristiques (check.stat) : ")
        .first()
        .copied()
        .unwrap_or("SAG")
        .to_string();
    let foes: Vec<&str> = prompt
        .lines()
        .filter_map(|l| l.strip_prefix("- `"))
        .filter_map(|l| l.split('`').next())
        .chain(["adv_fantome_invente"])
        .collect();
    let mut starts: Vec<serde_json::Value> = (0..4)
        .map(|i| json!({ "id": format!("pj-{}", i + 1), "side": "party", "at": [2 + i, 2] }))
        .collect();
    starts.extend(foes.iter().enumerate().map(|(i, f)| {
        json!({ "id": format!("adv-{}", i + 1), "side": "foes", "at": [2 + i, 6], "entity": f })
    }));
    json!({
        "name": "La salle coupée",
        "ambience": { "time": "night", "weather": "clear" },
        "grid": {
            "legend": { "#": { "terrain": wall, "wall": true }, ".": { "terrain": floor } },
            "rows": [
                "##############",
                "#............#",
                "#............#",
                "#............#",
                "##############",
                "#............#",
                "#............#",
                "#............#",
                "##############"
            ]
        },
        "doors": [{ "id": "porte-milieu", "at": [6, 4], "state": "closed", "label": "Porte de la réserve" }],
        "props": [{ "id": "caisses", "kind": prop, "label": "Caisses", "at": [9, 6], "size": [2, 1], "cover": "half", "blocks_movement": true }],
        "objects": [{ "id": "trappe", "kind": "trappe", "label": "Trappe", "at": [11, 2], "layer": "secrets", "check": { "stat": stat, "dc": 13 }, "notes": "Un passage vers la cave." }],
        "lights": [{ "id": "lanterne", "at": [6, 6], "bright": 1, "dim": 4, "color": "#ffb35c" }],
        "starts": starts,
        "gm_notes": "Les adversaires attendent derrière la porte."
    })
}

fn workshop_answer(prompt: &str) -> serde_json::Value {
    use promptus_shared::story::{Importance, from_yaml};
    let yaml = prompt
        .split("```yaml\n")
        .nth(1)
        .and_then(|s| s.split("\n```").next())
        .unwrap_or("");
    let Ok(c) = from_yaml(yaml) else {
        return json!({ "reply": "Je n’ai pas pu lire la campagne.", "edits": [] });
    };
    let mut edits = Vec::new();
    let line = |head: &str| {
        prompt
            .lines()
            .find_map(|l| l.strip_prefix(head))
            .map(str::trim)
    };
    let alert_rev = line("Alerte à résoudre : THREE_CLUE_RULE revelations[")
        .and_then(|rest| rest.split(']').next())
        .and_then(|i| i.parse::<usize>().ok())
        .and_then(|i| c.revelations.get(i));
    let scene = line("Scène sélectionnée : ")
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|id| c.node(id));
    let fresh = |stem: &str| {
        let mut id = stem.to_string();
        let mut n = 2;
        while c.node(&id).is_some() || c.clue(&id).is_some() {
            id = format!("{stem}_{n}");
            n += 1;
        }
        id
    };
    let reply;
    if let Some(rev) = alert_rev {
        let places = three_clues(&c, rev, "atelier", &mut edits);
        reply = format!(
            "J’ajoute des indices vers « {} » dans {}.",
            rev.statement,
            places.join(" et ")
        );
    } else {
        if let Some(npc) = c.npcs.first() {
            edits.push(json!({ "op": "set", "target": npc.id, "field": "motivation",
                "value": format!("{} Mais il garde ses vraies raisons pour lui.", npc.motivation).trim().to_string() }));
        }
        let node = scene.or_else(|| c.nodes.first());
        let rev = c
            .revelations
            .iter()
            .find(|r| r.importance == Importance::Critical)
            .or_else(|| c.revelations.first());
        if let (Some(n), Some(r)) = (node, rev) {
            edits.push(json!({ "op": "add", "kind": "clue", "value": {
                "id": fresh("cl_atelier"),
                "revelation": r.id,
                "node": n.id,
                "text": "Un registre annoté, dont une page manque.",
            }}));
        }
        reply = "Je rends le premier PNJ plus ambigu et j’ajoute un indice qui pointe vers lui sans l’accuser.".to_string();
    }
    edits.push(
        json!({ "op": "set", "target": "pnj_invente", "field": "name", "value": "Personne" }),
    );
    json!({ "reply": reply, "edits": edits })
}

/// Enough clues of `rev`, in scenes that do not hold one yet (required
/// scenes first), to reach three scenes. Returns the scenes' titles.
fn three_clues(
    c: &promptus_shared::story::Campaign,
    rev: &promptus_shared::story::Revelation,
    stem: &str,
    edits: &mut Vec<serde_json::Value>,
) -> Vec<String> {
    let taken = |id: &str, edits: &[serde_json::Value]| {
        c.node(id).is_some() || c.clue(id).is_some() || edits.iter().any(|e| e["value"]["id"] == id)
    };
    let holding: Vec<&str> = c.clues_for(&rev.id).map(|cl| cl.node.as_str()).collect();
    let mut free: Vec<_> = c
        .nodes
        .iter()
        .filter(|n| !holding.contains(&n.id.as_str()))
        .collect();
    free.sort_by_key(|n| n.optional);
    let missing = 3usize.saturating_sub(holding.len());
    let mut places = Vec::new();
    for (k, n) in free.into_iter().take(missing).enumerate() {
        let stem = format!("cl_{}_{stem}_{}", rev.id, k + 1);
        let mut id = stem.clone();
        let mut i = 2;
        while taken(&id, edits) {
            id = format!("{stem}_{i}");
            i += 1;
        }
        edits.push(json!({ "op": "add", "kind": "clue", "value": {
            "id": id,
            "revelation": rev.id,
            "node": n.id,
            "text": format!("Un détail de plus mène à la même conclusion : {}", rev.statement),
            "discovery": "En fouillant avec soin.",
        }}));
        places.push(format!("« {} »", n.title));
    }
    places
}

/// Campaign generation, step 1: a small campaign drawn from the pitch,
/// its adversaries taken from the rule system the prompt lists. One NPC
/// names a faction nobody declared: the server must remove it.
fn generation_cast(prompt: &str) -> serde_json::Value {
    let field = |head: &str| {
        prompt
            .lines()
            .find_map(|l| l.strip_prefix(head))
            .map(str::trim)
            .unwrap_or("")
            .to_string()
    };
    let pitch = field("- Idée : ");
    let tone = field("- Ton : ");
    // The rule system's adversaries, bosses apart: a henchman is the
    // first that is not one, a lieutenant the first that is.
    let listed: Vec<(String, bool)> = prompt
        .lines()
        .find_map(|l| l.strip_prefix("Adversaires du système (id → nom) : "))
        .filter(|l| l.trim() != "aucun")
        .map(|l| {
            l.split(", ")
                .filter_map(|e| {
                    let id = e.split(" → ").next()?.trim().to_string();
                    Some((id, e.ends_with("(boss)")))
                })
                .collect()
        })
        .unwrap_or_default();
    let pick = |boss: bool| {
        listed
            .iter()
            .find(|(_, b)| *b == boss)
            .or(listed.first())
            .map(|(id, _)| id.clone())
    };
    let stats = |boss: bool| match pick(boss) {
        Some(id) => json!({ "from_rules": id }),
        None => {
            json!({ "hit_points": if boss { 30 } else { 11 }, "armor_class": if boss { 14 } else { 12 }, "attacks": [{ "name": "Lame", "damage": "1d6+1" }] })
        }
    };
    let steps = |what: &str| {
        json!([
            { "label": format!("{what} se prépare"), "description": "Des rumeurs circulent." },
            { "label": format!("{what} frappe une première fois"), "description": "Un témoin disparaît." },
            { "label": format!("{what} se rapproche"), "description": "Les alliés des personnages sont menacés." },
            { "label": format!("{what} triomphe"), "description": "La catastrophe : la ville tombe entre ses mains." },
        ])
    };
    json!({
        "bible": {
            "pitch": format!("{pitch} Tout commence par une rencontre à l’auberge du Gué."),
            "tone": if tone == "libre" || tone.is_empty() { "Tendu, mystérieux.".to_string() } else { tone },
            "themes": ["loyauté", "secrets"],
            "truths": ["Le Cercle tient la ville par la peur.", "Le port est la seule issue."],
            "secrets": ["Le commanditaire travaille pour le Cercle."],
            "player_hook": "Un inconnu vous offre une bourse pleine pour retrouver un homme disparu."
        },
        "acts": [{ "id": "acte_1", "title": "Acte I — L’appât", "summary": "Les personnages remontent la piste du Cercle.", "opening": "L’offre à l’auberge.", "closing": "Le repaire tombe." }],
        "fronts": [
            { "id": "fr_cercle", "name": "Le Cercle", "goal": "Tenir la ville", "steps": steps("Le Cercle") },
            { "id": "fr_rivaux", "name": "Les rivaux", "goal": "Doubler les personnages", "steps": steps("La bande rivale") }
        ],
        "npcs": [
            { "id": "pnj_commanditaire", "name": "Aldric", "title": "Le commanditaire", "motivation": "Retrouver ce qu’on lui a pris.", "disposition": "neutral", "hides": "Il travaille pour le Cercle.", "location": "lieu_auberge" },
            { "id": "pnj_traitre", "name": "Mirelle", "title": "La traîtresse", "motivation": "Monter dans le Cercle.", "disposition": "hostile", "location": "lieu_entrepot", "faction": "fac_cercle" },
            { "id": "pnj_temoin", "name": "Tobin", "title": "Le témoin", "motivation": "Survivre.", "disposition": "friendly", "location": "lieu_port", "faction": "fac_inventee" }
        ],
        "adversaries": [
            { "id": "adv_sbire", "name": "Sbire du Cercle", "description": "Un homme de main.", "stats": stats(false) },
            { "id": "adv_chef", "name": "Lieutenant du Cercle", "description": "Il mène les sbires.", "stats": stats(true) }
        ],
        "locations": [
            { "id": "lieu_ville", "name": "La ville basse", "description": "Ruelles et brume." },
            { "id": "lieu_auberge", "name": "L’auberge du Gué", "description": "Bruyante et chaude.", "parent": "lieu_ville" },
            { "id": "lieu_port", "name": "Le port", "description": "Des quais qui sentent le goudron.", "parent": "lieu_ville" },
            { "id": "lieu_entrepot", "name": "L’entrepôt", "description": "Des caisses, et des ombres.", "parent": "lieu_ville" },
            { "id": "lieu_repaire", "name": "Le repaire du Cercle", "description": "Sous la vieille halle." }
        ],
        "items": [
            { "id": "obj_cle", "name": "La clé de fer", "description": "Elle ouvre le repaire.", "rarity": "uncommon", "value": 20 },
            { "id": "obj_carnet", "name": "Le carnet codé", "description": "Les comptes du Cercle." }
        ],
        "factions": [
            { "id": "fac_cercle", "name": "Le Cercle", "description": "Une société secrète.", "affinity": { "start": -1, "min": -3, "max": 3 } }
        ]
    })
}

/// Campaign generation, step 2: seven scenes on the cast of step 1, two
/// critical revelations with clues in three required scenes each (one
/// scene each with `few_clues`), one optional. One NPC presence and one
/// clue point at scenes and NPCs nobody declared.
fn generation_scenes(few_clues: bool) -> serde_json::Value {
    let node = |id: &str, title: &str, location: &str, exits: serde_json::Value| {
        json!({
            "id": id, "act": "acte_1", "title": title,
            "summary": format!("{title} : la piste avance."),
            "location": location,
            "read_aloud": "La brume colle aux pavés ; quelque part, une porte claque.",
            "ambience": { "mood": "tendu", "sounds": "pas, murmures" },
            "flow": "Les personnages arrivent, observent, interrogent, puis choisissent leur route.",
            "transition": "La nuit tombe ; il faut décider où aller.",
            "exits": exits,
        })
    };
    let mut nodes = vec![
        node(
            "sc_ouverture",
            "L’offre à l’auberge",
            "lieu_auberge",
            json!([{ "to": "sc_port", "label": "Suivre la piste du port" }, { "to": "sc_marche", "label": "Interroger le marché" }]),
        ),
        node(
            "sc_port",
            "Les quais",
            "lieu_port",
            json!([{ "to": "sc_entrepot", "label": "Forcer l’entrepôt" }, { "to": "sc_marche", "label": "Revenir au marché" }]),
        ),
        node(
            "sc_marche",
            "Le marché de nuit",
            "lieu_ville",
            json!([{ "to": "sc_entrepot", "label": "Aller à l’entrepôt" }]),
        ),
        node(
            "sc_entrepot",
            "L’entrepôt",
            "lieu_entrepot",
            json!([{ "to": "sc_poursuite", "label": "Poursuivre la traîtresse" }, { "to": "sc_repaire", "label": "Descendre au repaire" }]),
        ),
        node(
            "sc_poursuite",
            "La poursuite",
            "lieu_ville",
            json!([{ "to": "sc_repaire", "label": "La suivre sous la halle" }]),
        ),
        node(
            "sc_repaire",
            "Le repaire du Cercle",
            "lieu_repaire",
            json!([{ "to": "sc_denouement", "label": "Remonter au jour" }]),
        ),
        node(
            "sc_denouement",
            "Le prix de la vérité",
            "lieu_auberge",
            json!([]),
        ),
    ];
    nodes[0]["npcs"] = json!([{ "npc": "pnj_commanditaire", "role": "Fait l’offre." }, { "npc": "pnj_fantome", "role": "N’existe pas." }]);
    nodes[1]["npcs"] = json!([{ "npc": "pnj_temoin", "role": "A tout vu." }]);
    nodes[2]["optional"] = json!(true);
    nodes[3]["encounter"] = json!({ "opponents": [{ "who": "adv_sbire", "count": 3 }], "tactics": ["Ils encerclent le plus faible."], "morale": [{ "when": "2 sbires tombent", "then": "les autres fuient" }], "on_victory": "La traîtresse s’enfuit.", "on_defeat": "Les personnages se réveillent au port." });
    nodes[3]["npcs"] = json!([{ "npc": "pnj_traitre", "role": "Surveille la marchandise." }]);
    nodes[3]["loot"] = json!([{ "item": "obj_cle", "found": "Sur le chef des sbires." }]);
    nodes[5]["requires"] = json!(["rev_repaire"]);
    nodes[5]["encounter"] = json!({ "opponents": [{ "who": "adv_chef" }, { "who": "adv_sbire", "count": 2 }], "tactics": ["Le lieutenant reste en retrait."], "on_victory": "Le Cercle est brisé.", "on_defeat": "Les personnages sont jetés au port." });
    nodes[5]["loot"] = json!([{ "item": "obj_carnet", "found": "Dans le coffre." }]);
    nodes[6].as_object_mut().map(|n| n.remove("transition"));
    let clue = |id: &str, rev: &str, node: &str, text: &str| json!({ "id": id, "revelation": rev, "node": node, "text": text, "discovery": "En questionnant ou en fouillant." });
    let mut clues = vec![
        clue(
            "cl_traitre_auberge",
            "rev_traitre",
            "sc_ouverture",
            "Aldric évite de prononcer le nom de Mirelle.",
        ),
        clue(
            "cl_repaire_port",
            "rev_repaire",
            "sc_port",
            "Tobin a vu des caisses descendre sous la halle.",
        ),
        clue(
            "cl_mobile_marche",
            "rev_mobile",
            "sc_marche",
            "On dit que Mirelle doit de l’argent au Cercle.",
        ),
        clue(
            "cl_perdu",
            "rev_mobile",
            "sc_inexistante",
            "Un indice dans une scène qui n’existe pas.",
        ),
    ];
    if !few_clues {
        clues.extend([
            clue(
                "cl_traitre_port",
                "rev_traitre",
                "sc_port",
                "Tobin décrit une femme qui ressemble à Mirelle.",
            ),
            clue(
                "cl_traitre_entrepot",
                "rev_traitre",
                "sc_entrepot",
                "Une lettre signée M. donne les ordres.",
            ),
            clue(
                "cl_repaire_entrepot",
                "rev_repaire",
                "sc_entrepot",
                "Un plan de la halle, cloué au mur.",
            ),
            clue(
                "cl_repaire_poursuite",
                "rev_repaire",
                "sc_poursuite",
                "Mirelle disparaît par une trappe sous la halle.",
            ),
        ]);
    }
    json!({
        "start_node": "sc_ouverture",
        "nodes": nodes,
        "revelations": [
            { "id": "rev_traitre", "statement": "Mirelle trahit les personnages.", "importance": "critical" },
            { "id": "rev_repaire", "statement": "Le repaire du Cercle est sous la vieille halle.", "importance": "critical" },
            { "id": "rev_mobile", "statement": "Mirelle doit tout au Cercle.", "importance": "optional" }
        ],
        "clues": clues,
    })
}

/// Campaign generation, repair: for each three-clue alert the prompt
/// lists, the clues missing; plus one edit on an invented id.
fn generation_repair(prompt: &str) -> serde_json::Value {
    use promptus_shared::story::from_yaml;
    let yaml = prompt
        .split("```yaml\n")
        .nth(1)
        .and_then(|s| s.split("\n```").next())
        .unwrap_or("");
    let Ok(c) = from_yaml(yaml) else {
        return json!({ "edits": [] });
    };
    let mut edits = Vec::new();
    for line in prompt.lines() {
        let Some(rest) = line.split("THREE_CLUE_RULE revelations[").nth(1) else {
            continue;
        };
        if let Some(rev) = rest
            .split(']')
            .next()
            .and_then(|i| i.parse::<usize>().ok())
            .and_then(|i| c.revelations.get(i))
        {
            three_clues(&c, rev, "repare", &mut edits);
        }
    }
    edits.push(
        json!({ "op": "set", "target": "pnj_invente", "field": "name", "value": "Personne" }),
    );
    json!({ "edits": edits })
}

/// A 32 × 32 pixel pattern, mirrored like a crest, from the prompt's
/// FNV-1a hash.
fn pattern(prompt: &str) -> Vec<u8> {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in prompt.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    let (w, h) = (32u32, 32u32);
    let ink = [
        (hash >> 8) as u8 | 0x40,
        (hash >> 16) as u8 | 0x40,
        (hash >> 24) as u8 | 0x40,
    ];
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let mx = if x < w / 2 { x } else { w - 1 - x };
            let bit = (hash.rotate_left((mx / 4 + (y / 4) * 4) % 64)) & 1 == 1;
            let px = if bit { ink } else { [0x14, 0x14, 0x18] };
            rgba.extend_from_slice(&[px[0], px[1], px[2], 0xff]);
        }
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
    .to_png()
}

impl Provider for FakeProvider {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn complete<'a>(&'a self, req: &'a LlmRequest) -> BoxFuture<'a, Result<LlmResponse, AiError>> {
        Box::pin(async move {
            let system = req
                .messages
                .iter()
                .find(|m| m.role == Role::System)
                .map(|m| m.content.as_str())
                .unwrap_or("");
            let prompt = req
                .messages
                .iter()
                .rev()
                .find(|m| m.role == Role::User)
                .map(|m| m.content.as_str())
                .unwrap_or("");
            let words: String = system
                .split_whitespace()
                .take(4)
                .collect::<Vec<_>>()
                .join(" ");
            // The step a generation call is at, from its first user
            // message (a retry appends the complaint after it).
            let first = req
                .messages
                .iter()
                .find(|m| m.role == Role::User)
                .map(|m| m.content.as_str())
                .unwrap_or("");
            let generation = system.contains("Génération de campagne").then(|| {
                if first.contains("Étape 1/2") {
                    "cast"
                } else if first.contains("Étape 2/2") {
                    "scenes"
                } else {
                    "repair"
                }
            });
            let first_call = self.calls().is_empty();
            match generation {
                Some(step) => self.record(format!("generation:{step}")),
                None => self.record(format!("complete:{words}")),
            }
            let text = if self.broken || (self.bad_json_first && first_call) {
                "Voici ma réponse : { pas du json".to_string()
            } else if let Some(step) = generation {
                match step {
                    "cast" => generation_cast(first),
                    "scenes" => generation_scenes(self.few_clues),
                    _ => generation_repair(first),
                }
                .to_string()
            } else if system.contains("cartographe") {
                map_answer(first).to_string()
            } else if system.contains("atelier") {
                workshop_answer(prompt).to_string()
            } else if system.contains("co-MJ") {
                copilot_answer(prompt).to_string()
            } else if system.contains("récapitulatifs") {
                recap_answer(prompt).to_string()
            } else {
                return Err(AiError::Refused {
                    status: 400,
                    message: format!("faux fournisseur : prompt inconnu ({words})"),
                });
            };
            Ok(LlmResponse {
                text,
                model: req.model.clone().unwrap_or_else(|| "fake/promptus".into()),
                usage: self.usage(),
            })
        })
    }

    fn image<'a>(&'a self, req: &'a ImageRequest) -> BoxFuture<'a, Result<ImageResponse, AiError>> {
        Box::pin(async move {
            self.record("image".into());
            Ok(ImageResponse {
                bytes: pattern(&req.prompt),
                mime: "image/png".into(),
                model: req.model.clone().unwrap_or_else(|| "fake/pixel".into()),
                usage: self.usage(),
            })
        })
    }

    fn video<'a>(&'a self, req: &'a VideoRequest) -> BoxFuture<'a, Result<VideoResponse, AiError>> {
        Box::pin(async move {
            self.record("video".into());
            if self.broken {
                return Err(AiError::Refused {
                    status: 422,
                    message: "faux fournisseur : vidéo refusée".into(),
                });
            }
            // Not a playable film: an MP4 header box followed by the
            // prompt, so two prompts give two files.
            let mut bytes = b"\0\0\0\x18ftypmp42\0\0\0\0mp42isom".to_vec();
            bytes.extend_from_slice(req.prompt.as_bytes());
            Ok(VideoResponse {
                bytes,
                mime: "video/mp4".into(),
                model: req.model.clone().unwrap_or_else(|| "fake/video".into()),
                usage: self.usage(),
            })
        })
    }
}
