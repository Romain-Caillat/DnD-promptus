// Validation d'une histoire de campagne.
// - error   : référence cassée ou incohérence structurelle — bloque l'enregistrement.
// - warning : défaut de conception (règle des trois indices, scène inaccessible…)
//             — signalé au MJ et au LLM, sans bloquer.

import type { Ruleset } from "./ruleset";
import {
  EXPECTED_GRID,
  MAP_LEVEL_LABELS,
  type CampaignStory,
  type GameMap,
  type WorldCondition,
} from "./story";
import type { Effect } from "./types";

export interface StoryIssue {
  severity: "error" | "warning";
  path: string;
  message: string;
}

export interface StoryValidationContext {
  /** Ids des fiches de la campagne (PNJ, lieux, monstres, objets…). */
  entityIds: Set<string>;
  ruleset: Ruleset;
}

const MIN_CLUES_CRITICAL = 3;
const MIN_FRONT_STEPS = 3;

/** Parcourt des effets, y compris les branches imbriquées des jets. */
export function walkEffects(effects: Effect[], fn: (e: Effect, path: string) => void, path = ""): void {
  effects.forEach((e, i) => {
    const p = `${path}[${i}]`;
    fn(e, p);
    if (e.type === "roll_check") {
      walkEffects(e.outcomeSuccess ?? [], fn, `${p}.outcomeSuccess`);
      walkEffects(e.outcomeFail ?? [], fn, `${p}.outcomeFail`);
    }
  });
}

export function validateStory(story: CampaignStory, ctx: StoryValidationContext): StoryIssue[] {
  const issues: StoryIssue[] = [];
  const err = (path: string, message: string) => issues.push({ severity: "error", path, message });
  const warn = (path: string, message: string) => issues.push({ severity: "warning", path, message });

  // --- Ids uniques ---------------------------------------------------------
  const ids = (list: { id: string }[], label: string, path: string) => {
    const set = new Set<string>();
    list.forEach((x, i) => {
      if (set.has(x.id)) err(`${path}[${i}].id`, `${label} : id en double « ${x.id} »`);
      set.add(x.id);
    });
    return set;
  };
  const sceneIds = ids(story.scenes, "Scène", "scenes");
  ids(story.fronts, "Front", "fronts");
  const revelationIds = ids(story.revelations, "Révélation", "revelations");
  const clueIds = ids(story.clues, "Indice", "clues");
  ids(story.maps, "Carte", "maps");
  const mapsById = new Map(story.maps.map((m) => [m.id, m]));

  const abilityIds = new Set(ctx.ruleset.abilities.map((a) => a.id));
  const skillIds = new Set(ctx.ruleset.skills.map((s) => s.id));
  const conditionIds = new Set(ctx.ruleset.conditions.map((c) => c.id));
  const damageTypeIds = new Set(ctx.ruleset.damageTypes.map((d) => d.id));
  const resourceIds = new Set(ctx.ruleset.resources.map((r) => r.id));

  const entity = (id: string | undefined, path: string, what = "Fiche") => {
    if (id && !ctx.entityIds.has(id)) err(path, `${what} inconnue : « ${id} »`);
  };

  // --- Effets et conditions --------------------------------------------------
  const checkEffects = (effects: Effect[], path: string) =>
    walkEffects(effects, (e, p) => {
      const at = `${path}${p}`;
      switch (e.type) {
        case "roll_check":
          if (!abilityIds.has(e.stat)) err(`${at}.stat`, `Caractéristique absente des règles : « ${e.stat} »`);
          break;
        case "apply_condition":
        case "remove_condition":
          if (!conditionIds.has(e.conditionId)) err(`${at}.conditionId`, `État absent des règles : « ${e.conditionId} »`);
          break;
        case "damage":
          if (!damageTypeIds.has(e.damageType)) err(`${at}.damageType`, `Type de dégâts absent des règles : « ${e.damageType} »`);
          break;
        case "consume_resource":
        case "restore_resource":
          if (!resourceIds.has(e.resource)) err(`${at}.resource`, `Ressource absente des règles : « ${e.resource} »`);
          break;
        case "set_state":
        case "reveal_entity":
          entity(e.entityId, `${at}.entityId`);
          break;
        case "move_entity":
          entity(e.entityId, `${at}.entityId`);
          entity(e.toLocationId, `${at}.toLocationId`, "Lieu");
          break;
        case "set_relation":
          entity(e.fromId, `${at}.fromId`);
          entity(e.toId, `${at}.toId`);
          break;
        case "trigger_event":
          entity(e.eventId, `${at}.eventId`, "Événement");
          break;
        case "add_to_inventory":
        case "remove_from_inventory":
          entity(e.itemId, `${at}.itemId`, "Objet");
          break;
        default:
          break;
      }
    });

  const checkCondition = (c: WorldCondition, path: string): void => {
    if ("all" in c) return c.all.forEach((x, i) => checkCondition(x, `${path}.all[${i}]`));
    if ("any" in c) return c.any.forEach((x, i) => checkCondition(x, `${path}.any[${i}]`));
    if ("not" in c) return checkCondition(c.not, `${path}.not`);
    if ("sceneStatus" in c && !sceneIds.has(c.sceneStatus.sceneId))
      err(`${path}.sceneStatus.sceneId`, `Scène inconnue : « ${c.sceneStatus.sceneId} »`);
    if ("clueFound" in c && !clueIds.has(c.clueFound)) err(`${path}.clueFound`, `Indice inconnu : « ${c.clueFound} »`);
    if ("revelationKnown" in c && !revelationIds.has(c.revelationKnown))
      err(`${path}.revelationKnown`, `Révélation inconnue : « ${c.revelationKnown} »`);
    if ("frontStepAtLeast" in c) {
      const f = story.fronts.find((x) => x.id === c.frontStepAtLeast.frontId);
      if (!f) err(`${path}.frontStepAtLeast.frontId`, `Front inconnu : « ${c.frontStepAtLeast.frontId} »`);
      else if (c.frontStepAtLeast.step < 0 || c.frontStepAtLeast.step >= f.steps.length)
        err(`${path}.frontStepAtLeast.step`, `Étape hors de l’horloge de « ${f.name} »`);
    }
    if ("entityAttribute" in c) entity(c.entityAttribute.entityId, `${path}.entityAttribute.entityId`);
  };

  // --- Bible -----------------------------------------------------------------
  if (!story.bible.pitch.trim()) warn("bible.pitch", "Pitch vide");
  if (story.scenes.length > 0) {
    if (!story.bible.startSceneId) warn("bible.startSceneId", "Aucune scène d’ouverture définie");
    else if (!sceneIds.has(story.bible.startSceneId))
      err("bible.startSceneId", `Scène d’ouverture inconnue : « ${story.bible.startSceneId} »`);
  }

  // --- Fronts ----------------------------------------------------------------
  story.fronts.forEach((f, i) => {
    if (f.steps.length < MIN_FRONT_STEPS)
      warn(`fronts[${i}].steps`, `« ${f.name} » : horloge de ${f.steps.length} étape(s), ${MIN_FRONT_STEPS} minimum conseillées`);
    f.steps.forEach((s, j) => checkEffects(s.effects ?? [], `fronts[${i}].steps[${j}].effects`));
  });

  // --- Scènes ----------------------------------------------------------------
  story.scenes.forEach((s, i) => {
    const p = `scenes[${i}]`;
    entity(s.locationEntityId, `${p}.locationEntityId`, "Lieu");
    s.npcEntityIds.forEach((id, j) => entity(id, `${p}.npcEntityIds[${j}]`, "PNJ"));
    s.monsterEntityIds.forEach((id, j) => entity(id, `${p}.monsterEntityIds[${j}]`, "Monstre"));
    s.exits.forEach((x, j) => {
      if (!sceneIds.has(x.toSceneId)) err(`${p}.exits[${j}].toSceneId`, `Sortie vers une scène inconnue : « ${x.toSceneId} »`);
      if (x.toSceneId === s.id) warn(`${p}.exits[${j}]`, `« ${s.title} » mène vers elle-même`);
    });
    if (s.mapPlacement) {
      const m = mapsById.get(s.mapPlacement.mapId);
      if (!m) err(`${p}.mapPlacement.mapId`, `Carte inconnue : « ${s.mapPlacement.mapId} »`);
      else if (!inBounds(m, s.mapPlacement.x, s.mapPlacement.y))
        err(`${p}.mapPlacement`, `Case (${s.mapPlacement.x}, ${s.mapPlacement.y}) hors de « ${m.name} »`);
      else if (m.level === "local") warn(`${p}.mapPlacement`, "Une scène se place sur une carte de campagne ou de région");
    }
    if (s.battleMapId) {
      const m = mapsById.get(s.battleMapId);
      if (!m) err(`${p}.battleMapId`, `Carte inconnue : « ${s.battleMapId} »`);
      else if (m.level !== "local") err(`${p}.battleMapId`, `« ${m.name} » n’est pas une carte de combat / lieu`);
    }
    const triggerIds = new Set<string>();
    s.triggers.forEach((t, j) => {
      if (triggerIds.has(t.id)) err(`${p}.triggers[${j}].id`, `Déclencheur en double « ${t.id} »`);
      triggerIds.add(t.id);
      checkCondition(t.when, `${p}.triggers[${j}].when`);
      checkEffects(t.effects, `${p}.triggers[${j}].effects`);
    });
    if (!s.readAloud.trim()) warn(`${p}.readAloud`, `« ${s.title} » : pas de texte à lire`);
  });

  // Accessibilité depuis la scène d'ouverture, impasses.
  const start = story.bible.startSceneId;
  if (start && sceneIds.has(start)) {
    const reached = new Set<string>([start]);
    const queue = [start];
    const byId = new Map(story.scenes.map((s) => [s.id, s]));
    while (queue.length) {
      const cur = byId.get(queue.shift()!);
      for (const x of cur?.exits ?? []) {
        if (sceneIds.has(x.toSceneId) && !reached.has(x.toSceneId)) {
          reached.add(x.toSceneId);
          queue.push(x.toSceneId);
        }
      }
    }
    story.scenes.forEach((s, i) => {
      if (!reached.has(s.id)) warn(`scenes[${i}]`, `« ${s.title} » est inaccessible depuis la scène d’ouverture`);
    });
  }
  const deadEnds = story.scenes.filter((s) => s.exits.length === 0);
  if (deadEnds.length > 1)
    warn("scenes", `${deadEnds.length} scènes sans sortie (${deadEnds.map((s) => `« ${s.title} »`).join(", ")}) : une seule finale est conseillée`);

  // --- Révélations et indices -------------------------------------------------
  story.clues.forEach((c, i) => {
    const p = `clues[${i}]`;
    if (!revelationIds.has(c.revelationId)) err(`${p}.revelationId`, `Révélation inconnue : « ${c.revelationId} »`);
    if (!sceneIds.has(c.sceneId)) err(`${p}.sceneId`, `Scène inconnue : « ${c.sceneId} »`);
    if (c.check?.skill && !skillIds.has(c.check.skill)) err(`${p}.check.skill`, `Compétence absente des règles : « ${c.check.skill} »`);
    if (c.check?.ability && !abilityIds.has(c.check.ability))
      err(`${p}.check.ability`, `Caractéristique absente des règles : « ${c.check.ability} »`);
  });
  story.revelations.forEach((r, i) => {
    const clues = story.clues.filter((c) => c.revelationId === r.id);
    const scenes = new Set(clues.map((c) => c.sceneId));
    if (clues.length === 0) warn(`revelations[${i}]`, `« ${r.statement} » : aucun indice n’y mène`);
    else if (r.importance === "critical" && clues.length < MIN_CLUES_CRITICAL)
      warn(`revelations[${i}]`, `Règle des trois indices : « ${r.statement} » n’a que ${clues.length} indice(s)`);
    if (r.importance === "critical" && clues.length > 1 && scenes.size < 2)
      warn(`revelations[${i}]`, `« ${r.statement} » : tous les indices sont dans la même scène`);
  });

  // --- Cartes -----------------------------------------------------------------
  const referencedAsChild = new Set<string>();
  story.maps.forEach((m, i) => {
    const p = `maps[${i}]`;
    if (m.grid.cols < 1 || m.grid.rows < 1) err(`${p}.grid`, `« ${m.name} » : grille vide`);
    if (m.grid.type !== EXPECTED_GRID[m.level])
      err(`${p}.grid.type`, `« ${m.name} » : une carte ${MAP_LEVEL_LABELS[m.level].toLowerCase()} utilise une grille ${EXPECTED_GRID[m.level] === "hex" ? "hexagonale" : "carrée"}`);
    const seen = new Set<string>();
    m.cells.forEach((c, j) => {
      const cp = `${p}.cells[${j}]`;
      if (!inBounds(m, c.x, c.y)) err(cp, `Case (${c.x}, ${c.y}) hors de « ${m.name} »`);
      const key = `${c.x},${c.y}`;
      if (seen.has(key)) err(cp, `Case (${c.x}, ${c.y}) décrite deux fois dans « ${m.name} »`);
      seen.add(key);
      if (c.sceneId && !sceneIds.has(c.sceneId)) err(`${cp}.sceneId`, `Scène inconnue : « ${c.sceneId} »`);
      if (c.childMapId) {
        referencedAsChild.add(c.childMapId);
        const child = mapsById.get(c.childMapId);
        if (!child) err(`${cp}.childMapId`, `Carte inconnue : « ${c.childMapId} »`);
        else if (!isChildLevel(m.level, child.level))
          err(`${cp}.childMapId`, `« ${child.name} » (${MAP_LEVEL_LABELS[child.level]}) ne peut pas s’ouvrir depuis une carte ${MAP_LEVEL_LABELS[m.level].toLowerCase()}`);
      }
    });
    (m.tokens ?? []).forEach((t, j) => {
      entity(t.entityId, `${p}.tokens[${j}].entityId`);
      if (!inBounds(m, t.x, t.y)) err(`${p}.tokens[${j}]`, `Pion hors de « ${m.name} »`);
      else if (m.cells.some((c) => c.x === t.x && c.y === t.y && c.blocked))
        warn(`${p}.tokens[${j}]`, `Pion placé sur une case infranchissable de « ${m.name} »`);
    });
  });
  story.maps.forEach((m, i) => {
    if (m.level !== "campaign" && !referencedAsChild.has(m.id) && !story.scenes.some((s) => s.battleMapId === m.id))
      warn(`maps[${i}]`, `« ${m.name} » n’est reliée à aucune carte parente ni scène`);
  });

  return issues;
}

function inBounds(m: GameMap, x: number, y: number): boolean {
  return Number.isInteger(x) && Number.isInteger(y) && x >= 0 && y >= 0 && x < m.grid.cols && y < m.grid.rows;
}

function isChildLevel(parent: GameMap["level"], child: GameMap["level"]): boolean {
  return (parent === "campaign" && child === "region") || (parent === "region" && child === "local");
}
