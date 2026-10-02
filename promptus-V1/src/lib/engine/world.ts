// État du monde d'une campagne — ce qui a changé depuis le début du jeu.
// Le contenu écrit (scénario) est dans `CampaignStory` ; ici, seulement
// l'état vivant, persisté d'une session à l'autre.

import type { CampaignStory, Scene, SceneStatus, SceneTrigger, WorldCondition } from "./story";

export interface Relation {
  value: number;
  disposition?: string;
}

export interface WorldState {
  /** Drapeaux libres posés par les effets `set_flag`. */
  flags: Record<string, unknown>;
  /** Attributs modifiés par `set_state` (ex. statut d'un PNJ). */
  entityAttributes: Record<string, Record<string, unknown>>;
  /** Relations entre fiches, clé `from->to`. */
  relations: Record<string, Relation>;
  revealedEntityIds: string[];
  /** Lieu actuel des fiches déplacées par `move_entity`. */
  entityLocations: Record<string, string>;
  currentSceneId?: string;
  sceneStatus: Record<string, SceneStatus>;
  foundClueIds: string[];
  /** Index de la dernière étape atteinte par front (absent = pas commencé). */
  frontProgress: Record<string, number>;
  /** Déclencheurs déjà tirés, clé `sceneId/triggerId`. */
  firedTriggerIds: string[];
}

export const EMPTY_WORLD: WorldState = {
  flags: {},
  entityAttributes: {},
  relations: {},
  revealedEntityIds: [],
  entityLocations: {},
  sceneStatus: {},
  foundClueIds: [],
  frontProgress: {},
  firedTriggerIds: [],
};

/** Complète un état partiel (anciennes versions, JSON incomplet). */
export function normalizeWorld(w: Partial<WorldState> | null | undefined): WorldState {
  return { ...structuredClone(EMPTY_WORLD), ...(w ?? {}) };
}

export function relationKey(fromId: string, toId: string): string {
  return `${fromId}->${toId}`;
}

export function triggerKey(sceneId: string, triggerId: string): string {
  return `${sceneId}/${triggerId}`;
}

export function frontStep(world: WorldState, frontId: string): number {
  return world.frontProgress[frontId] ?? -1;
}

export function isRevelationKnown(story: CampaignStory, world: WorldState, revelationId: string): boolean {
  return story.clues.some((c) => c.revelationId === revelationId && world.foundClueIds.includes(c.id));
}

export function evaluateCondition(c: WorldCondition, world: WorldState, story: CampaignStory): boolean {
  if ("all" in c) return c.all.every((x) => evaluateCondition(x, world, story));
  if ("any" in c) return c.any.some((x) => evaluateCondition(x, world, story));
  if ("not" in c) return !evaluateCondition(c.not, world, story);
  if ("flag" in c) {
    const v = world.flags[c.flag];
    return c.equals === undefined ? Boolean(v) : JSON.stringify(v) === JSON.stringify(c.equals);
  }
  if ("sceneStatus" in c) {
    const s = world.sceneStatus[c.sceneStatus.sceneId];
    // « visitée » est aussi vraie pour une scène résolue.
    if (c.sceneStatus.status === "visited") return s === "visited" || s === "resolved";
    return s === c.sceneStatus.status;
  }
  if ("clueFound" in c) return world.foundClueIds.includes(c.clueFound);
  if ("revelationKnown" in c) return isRevelationKnown(story, world, c.revelationKnown);
  if ("frontStepAtLeast" in c) return frontStep(world, c.frontStepAtLeast.frontId) >= c.frontStepAtLeast.step;
  const attrs = world.entityAttributes[c.entityAttribute.entityId] ?? {};
  return JSON.stringify(attrs[c.entityAttribute.attribute]) === JSON.stringify(c.entityAttribute.equals);
}

export interface TriggerStatus {
  scene: Scene;
  trigger: SceneTrigger;
  ready: boolean;
  fired: boolean;
}

/**
 * Déclencheurs de la scène en cours et leur état. Un déclencheur prêt est
 * proposé au MJ ; il n'est jamais appliqué sans sa validation.
 */
export function sceneTriggers(story: CampaignStory, world: WorldState): TriggerStatus[] {
  const scene = story.scenes.find((s) => s.id === world.currentSceneId);
  if (!scene) return [];
  return scene.triggers.map((trigger) => {
    const fired = world.firedTriggerIds.includes(triggerKey(scene.id, trigger.id));
    const oneShot = trigger.oneShot !== false;
    return {
      scene,
      trigger,
      fired,
      ready: (!fired || !oneShot) && evaluateCondition(trigger.when, world, story),
    };
  });
}
