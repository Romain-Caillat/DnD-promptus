// Prompts de génération de campagne. Le LLM produit du JSON strict, validé
// ensuite par Zod puis par le validateur d'histoire (qui sert aussi à lui
// renvoyer ses propres erreurs pour correction).

import type { Ruleset } from "@/lib/engine/ruleset";
import type { StoryIssue } from "@/lib/engine/story-validator";
import type { DraftEntity, GenerationInput } from "./types";
import { LENGTH_LABELS } from "./types";

const SIZE: Record<GenerationInput["length"], { scenes: string; npcs: string; monsters: string; locations: string; items: string; revelations: string }> = {
  one_shot: { scenes: "5 à 8", npcs: "3 à 5", monsters: "3 à 6", locations: "3 à 5", items: "2 à 4", revelations: "2 à 3" },
  short: { scenes: "9 à 14", npcs: "5 à 8", monsters: "5 à 8", locations: "5 à 8", items: "3 à 6", revelations: "3 à 5" },
  long: { scenes: "14 à 20", npcs: "8 à 12", monsters: "6 à 10", locations: "7 à 10", items: "4 à 8", revelations: "4 à 6" },
};

export function systemPrompt(ruleset: Ruleset): string {
  return `Tu es le co-MJ de Promptus, une application qui aide un maître du jeu humain à préparer et mener des campagnes de jeu de rôle.
Tu prépares une campagne complète que le MJ relira et modifiera avant de jouer. Tu écris tout en français, avec un style évocateur mais concis.

Règles de sortie :
- Réponds UNIQUEMENT par un objet JSON valide, sans texte avant ni après, sans bloc de code.
- Identifiants en snake_case ASCII avec préfixe : fiches « ent_ », scènes « sc_ », menaces « fr_ », révélations « rv_ », indices « cl_ », cartes « map_ ».
- N’utilise que les identifiants que tu as toi-même déclarés.

Principes de conception (obligatoires) :
- Le scénario est un GRAPHE de scènes, pas une ligne droite : chaque scène a 1 à 3 sorties, une seule scène finale sans sortie, toutes les scènes sont accessibles depuis la scène d’ouverture.
- Règle des trois indices : chaque révélation essentielle a au moins 3 indices, répartis dans au moins 2 scènes différentes.
- Les menaces (fronts) avancent si les joueurs n’agissent pas : 4 étapes, la dernière est la catastrophe.
- Les PNJ ont une motivation claire et, souvent, un secret.

Système de règles de la campagne : « ${ruleset.name} »
- Caractéristiques (id → nom) : ${ruleset.abilities.map((a) => `${a.id} → ${a.label}`).join(", ")}
- Compétences (id) : ${ruleset.skills.map((s) => s.id).join(", ") || "aucune"}
- Test : ${ruleset.check.dice}, ${ruleset.check.mode === "roll_over" ? "réussite si total ≥ difficulté (DD)" : "réussite si dés ≤ valeur − difficulté"}
- États (id) : ${ruleset.conditions.map((c) => c.id).join(", ")}
- Types de dégâts (id) : ${ruleset.damageTypes.map((d) => d.id).join(", ")}`;
}

function brief(input: GenerationInput): string {
  return `Demande du MJ :
- Idée : ${input.pitch}
- Ton : ${input.tone || "libre"}
- Thèmes : ${input.themes || "libres"}
- Joueurs : ${input.players}, niveau ${input.level}
- Format : ${LENGTH_LABELS[input.length]}
- Contraintes : ${input.constraints || "aucune"}`;
}

export function castPrompt(input: GenerationInput, ruleset: Ruleset): string {
  const n = SIZE[input.length];
  const abilities = ruleset.abilities.map((a) => a.id).join(", ");
  return `${brief(input)}

Étape 1/3 — Bible, menaces et fiches.
Produis ce JSON :
{
  "bible": {
    "pitch": "2-3 phrases",
    "tone": "ambiance",
    "themes": ["…"],
    "truths": ["vérités du monde, connues ou non des joueurs (3-5)"],
    "secrets": ["secrets réservés au MJ (2-4)"],
    "playerHook": "accroche lue aux joueurs"
  },
  "fronts": [{ "id": "fr_…", "name": "…", "goal": "ce que veut la menace", "description": "…",
               "steps": [{ "label": "…", "description": "…" }] }],
  "entities": [{ "ref": "ent_…", "type": "npc|monster|location|item|faction|event${input.pregens ? "|character" : ""}",
                 "name": "…", "description": "…", "tags": ["…"], "attributes": { … } }]
}
Quantités : 2 menaces de 4 étapes ; PNJ ${n.npcs} ; lieux ${n.locations} ; monstres ${n.monsters} ; objets ${n.items}${input.pregens ? ` ; ${input.players} personnages prêts à jouer (type character)` : ""}.
Attributs selon le type :
- npc : { "hp", "hpMax", "ac", "faction", "status": "alive", "motivation", "secret", "voiceActorRecommended": "conseil d’interprétation" }
- monster${input.pregens ? " / character" : ""} : { "hp", "hpMax", "ac", "speed", "challengeRating", "size", "abilityScores": { ${abilities} }, "initiativeBonus" }
- location : { "parentLocation": "ent_… éventuel" }
- item : { "rarity": "common|uncommon|rare|very_rare|legendary", "value" }
- faction : { "reputation": 0 }
Équilibre les monstres pour ${input.players} personnages de niveau ${input.level}.`;
}

export function scenesPrompt(
  input: GenerationInput,
  bible: unknown,
  fronts: { id: string; name: string; steps: { label: string }[] }[],
  entities: DraftEntity[],
): string {
  const n = SIZE[input.length];
  const list = entities.map((e) => `${e.ref} (${e.type}) ${e.name}`).join("\n");
  return `${brief(input)}

Bible déjà écrite : ${JSON.stringify(bible)}
Menaces : ${fronts.map((f) => `${f.id} « ${f.name} » (${f.steps.length} étapes, index 0 à ${f.steps.length - 1})`).join(" ; ")}
Fiches disponibles (utilise exactement ces ref) :
${list}

Étape 2/3 — Scènes, révélations et indices.
Produis ce JSON :
{
  "startSceneId": "sc_…",
  "scenes": [{
    "id": "sc_…", "title": "…", "summary": "pour le MJ, 1-2 phrases", "objective": "ce que les joueurs peuvent accomplir",
    "readAloud": "texte à lire aux joueurs, 2-4 phrases immersives", "gmNotes": "conseils de jeu",
    "phase": "exploration|combat|dialogue|travel|rest",
    "locationEntityId": "ent_… (un lieu)", "npcEntityIds": ["ent_…"], "monsterEntityIds": ["ent_…"],
    "exits": [{ "toSceneId": "sc_…", "label": "ce qui y mène" }],
    "triggers": [{ "id": "tg_…", "label": "…", "when": <condition>, "effects": [<effet>], "oneShot": true }],
    "media": { "imagePrompt": "description visuelle de la scène", "musicQuery": "recherche YouTube pour l’ambiance" }
  }],
  "revelations": [{ "id": "rv_…", "statement": "conclusion à atteindre", "importance": "critical|optional" }],
  "clues": [{ "id": "cl_…", "revelationId": "rv_…", "sceneId": "sc_…", "text": "ce qu’on découvre",
              "discovery": "comment", "check": { "skill": "id de compétence", "dc": 12 } }]
}
Quantités : scènes ${n.scenes} ; révélations ${n.revelations} (au moins 2 essentielles).
Conditions possibles (when) : { "clueFound": "cl_…" }, { "revelationKnown": "rv_…" }, { "frontStepAtLeast": { "frontId": "fr_…", "step": 1 } },
  { "sceneStatus": { "sceneId": "sc_…", "status": "visited|resolved" } }, { "flag": "nom" }, { "all": [...] }, { "any": [...] }, { "not": {...} }.
Effets possibles : { "type": "display_text", "text": "…" }, { "type": "set_flag", "flag": "…", "value": true },
  { "type": "advance_front", "frontId": "fr_…", "steps": 1 }, { "type": "reveal_entity", "entityId": "ent_…", "toUsers": "all_players" },
  { "type": "set_state", "entityId": "ent_…", "attribute": "status", "value": "dead" }.
Ajoute 1 à 2 déclencheurs pertinents dans les scènes clés (pas partout).`;
}

export function mapsPrompt(
  scenes: { id: string; title: string; phase: string; summary: string }[],
  entities: DraftEntity[],
): string {
  const monsters = entities.filter((e) => e.type === "monster").map((e) => `${e.ref} ${e.name}`).join(", ");
  return `Scènes de la campagne :
${scenes.map((s) => `${s.id} [${s.phase}] ${s.title} — ${s.summary}`).join("\n")}
Monstres : ${monsters || "aucun"}

Étape 3/3 — Cartes à grille, sur trois niveaux :
- 1 carte de campagne (level "campaign", grille "hex", environ 12×8) : grandes régions, villes, reliefs.
- 1 ou 2 cartes de région (level "region", grille "hex", environ 10×8) : chaque scène y est placée sur une case.
- 1 carte de combat (level "local", grille "square", entre 12×10 et 24×16) pour chaque scène de phase "combat" (au plus 4), avec murs infranchissables et pions de départ des monstres.
Chaque carte de région doit être ouverte depuis une case de la carte de campagne (childMapId), chaque carte de combat depuis une case d’une carte de région.
Coordonnées : x de 0 à cols−1, y de 0 à rows−1. Ne décris que les cases notables (pas toutes les cases), au plus 60 par carte ; une même case n’apparaît qu’une fois.

Produis ce JSON :
{
  "maps": [{
    "id": "map_…", "name": "…", "level": "campaign|region|local",
    "grid": { "type": "hex|square", "cols": 12, "rows": 8 },
    "backgroundPrompt": "description visuelle de la carte vue de dessus",
    "cells": [{ "x": 0, "y": 0, "terrain": "forêt|montagne|mur|eau|…", "blocked": false, "label": "…", "childMapId": "map_…" }],
    "tokens": [{ "entityId": "ent_…", "x": 0, "y": 0 }]
  }],
  "placements": [{ "sceneId": "sc_…", "mapId": "map_… (région)", "x": 0, "y": 0 }],
  "battleMaps": [{ "sceneId": "sc_…", "mapId": "map_… (combat)" }]
}`;
}

export function repairPrompt(storyJson: string, issues: StoryIssue[]): string {
  return `Voici le scénario actuel (JSON) :
${storyJson}

Le validateur a trouvé ces problèmes :
${issues.map((i) => `- [${i.severity === "error" ? "ERREUR" : "avertissement"}] ${i.path} : ${i.message}`).join("\n")}

Corrige TOUTES les erreurs et autant d’avertissements que possible (ajoute des indices, des sorties, corrige les références) sans réécrire ce qui fonctionne.
Réponds avec l’objet complet corrigé, même structure : { "bible", "fronts", "scenes", "revelations", "clues", "maps" }.`;
}
