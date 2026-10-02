// Effect resolver — pure functions, no DB, no React.
// The runner returns a list of ResolutionRecords plus the mutations to apply.
// The DB persistence is handled by the API route.

import {
  DND5E_RULESET,
  abilityLabel,
  conditionsById,
  damageTypeLabel,
  defaultAbilityScore,
  resourceLabel,
  rollCheck,
  rulesetConditionLabel,
  type Ruleset,
} from "./ruleset";

import type {
  ActiveCondition,
  AdvanceFrontEffect,
  AppliedMutation,
  EnterSceneEffect,
  RevealClueEffect,
  SetFlagEffect,
  SetSceneStatusEffect,
  ApplyConditionEffect,
  ConsumeResourceEffect,
  DamageEffect,
  DisplayImageEffect,
  DisplayTextEffect,
  Effect,
  EntityState,
  HealEffect,
  InitiativeEntry,
  ModifyStatEffect,
  MoveEntityEffect,
  RemoveConditionEffect,
  ResolutionRecord,
  RestoreResourceEffect,
  RevealEntityEffect,
  RollCheckEffect,
  RollDetail,
  SetRelationEffect,
  SetStateEffect,
  TargetSpec,
  TriggerEventEffect,
  AddToInventoryEffect,
  RemoveFromInventoryEffect,
} from "./types";
import { computeRollFlags } from "./conditions";
import { EMPTY_STORY, type CampaignStory } from "./story";
import {
  EMPTY_WORLD,
  frontStep,
  isRevelationKnown,
  relationKey,
  type WorldState,
} from "./world";
import { rollDice, type Rng, defaultRng } from "./dice";

// ----------------------------------------------------------------------------
// Resolution context
// ----------------------------------------------------------------------------

export interface EntityRef {
  id: string;
  name: string;
  attributes: Record<string, unknown>;
  state: EntityState;
}

export interface ResolverContext {
  caster?: EntityRef;
  /** All entities in the session, indexed for target lookup. */
  entities: Map<string, EntityRef>;
  /** Snapshot of state by entityId — mutated locally, persisted by API. */
  states: Map<string, EntityState>;
  /** Initiative order so we can resolve "all_enemies" / "all_players". */
  initiativeOrder: InitiativeEntry[];
  /** Optional explicit target list, used when caller already picked targets. */
  explicitTargets?: string[];
  /** Règles de la campagne (défaut : D&D 5e). */
  ruleset: Ruleset;
  /** Scénario (lecture seule) et état du monde (muté, persisté par l'API). */
  story: CampaignStory;
  world: WorldState;
  /** Toutes les fiches de la campagne (noms, effets des événements). */
  catalog: Map<string, CatalogEntry>;
  /** Profondeur d'imbrication des événements déclenchés. */
  depth?: number;
  rng: Rng;
}

export interface CatalogEntry {
  name: string;
  effects: Effect[];
}

// ----------------------------------------------------------------------------
// Mutation accumulator
// ----------------------------------------------------------------------------

export interface ResolveResult {
  records: ResolutionRecord[];
  /** entityId -> next state to persist. */
  finalStates: Map<string, EntityState>;
}

export function newContext(opts: {
  caster?: EntityRef;
  entities: EntityRef[];
  initiativeOrder: InitiativeEntry[];
  explicitTargets?: string[];
  ruleset?: Ruleset;
  story?: CampaignStory;
  world?: WorldState;
  catalog?: Map<string, CatalogEntry>;
  rng?: Rng;
}): ResolverContext {
  const ents = new Map(opts.entities.map((e) => [e.id, e]));
  const states = new Map(opts.entities.map((e) => [e.id, structuredClone(e.state)]));
  return {
    caster: opts.caster,
    entities: ents,
    states,
    initiativeOrder: opts.initiativeOrder,
    explicitTargets: opts.explicitTargets,
    ruleset: opts.ruleset ?? DND5E_RULESET,
    story: opts.story ?? EMPTY_STORY,
    world: structuredClone(opts.world ?? EMPTY_WORLD),
    catalog: opts.catalog ?? new Map(),
    rng: opts.rng ?? defaultRng,
  };
}

// ----------------------------------------------------------------------------
// Public entry: resolve a list of effects in cascade
// ----------------------------------------------------------------------------

export async function resolveEffects(
  effects: Effect[],
  ctx: ResolverContext,
): Promise<ResolveResult> {
  const records: ResolutionRecord[] = [];
  for (const effect of effects) {
    const sub = await resolveEffect(effect, ctx);
    records.push(...sub);
  }
  return { records, finalStates: ctx.states };
}

async function resolveEffect(
  effect: Effect,
  ctx: ResolverContext,
): Promise<ResolutionRecord[]> {
  switch (effect.type) {
    case "damage":
      return [resolveDamage(effect, ctx)];
    case "heal":
      return [resolveHeal(effect, ctx)];
    case "apply_condition":
      return [resolveApplyCondition(effect, ctx)];
    case "remove_condition":
      return [resolveRemoveCondition(effect, ctx)];
    case "modify_stat":
      return [resolveModifyStat(effect, ctx)];
    case "roll_check":
      return await resolveRollCheck(effect, ctx);
    case "consume_resource":
      return [resolveConsumeResource(effect, ctx)];
    case "restore_resource":
      return [resolveRestoreResource(effect, ctx)];
    case "set_state":
      return [resolveSetState(effect, ctx)];
    case "move_entity":
      return [resolveMoveEntity(effect, ctx)];
    case "reveal_entity":
      return [resolveRevealEntity(effect, ctx)];
    case "set_relation":
      return [resolveSetRelation(effect, ctx)];
    case "trigger_event":
      return await resolveTriggerEvent(effect, ctx);
    case "add_to_inventory":
      return [resolveAddToInventory(effect, ctx)];
    case "remove_from_inventory":
      return [resolveRemoveFromInventory(effect, ctx)];
    case "play_ambience":
      return [annotation(effect, `🔊 Ambiance : ${effect.ambienceId}`)];
    case "play_music":
      return [annotation(effect, `🎵 Musique : ${effect.musicId}`)];
    case "play_sound":
      return [annotation(effect, `🔔 Bruitage : ${effect.soundId}`)];
    case "display_image":
      return [resolveDisplayImage(effect)];
    case "display_text":
      return [resolveDisplayText(effect)];
    case "set_flag":
      return [resolveSetFlag(effect, ctx)];
    case "advance_front":
      return await resolveAdvanceFront(effect, ctx);
    case "reveal_clue":
      return [resolveRevealClue(effect, ctx)];
    case "enter_scene":
      return [resolveEnterScene(effect, ctx)];
    case "set_scene_status":
      return [resolveSetSceneStatus(effect, ctx)];
    default: {
      const _exhaustive: never = effect;
      void _exhaustive;
      return [];
    }
  }
}

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------

function resolveTargets(target: TargetSpec, ctx: ResolverContext): string[] {
  if (ctx.explicitTargets?.length) return ctx.explicitTargets;
  switch (target.type) {
    case "self":
      return ctx.caster ? [ctx.caster.id] : [];
    case "caster":
      return ctx.caster ? [ctx.caster.id] : [];
    case "single":
      return target.entityId ? [target.entityId] : [];
    case "multiple":
      return target.entityIds ?? [];
    case "all_in_area":
      return ctx.explicitTargets ?? [];
    case "all_players":
      return ctx.initiativeOrder.filter((e) => e.isPlayer).map((e) => e.entityId);
    case "all_enemies":
      return ctx.initiativeOrder.filter((e) => !e.isPlayer).map((e) => e.entityId);
    default:
      return [];
  }
}

function nameOf(ctx: ResolverContext, id: string): string {
  return ctx.entities.get(id)?.name ?? ctx.catalog.get(id)?.name ?? id;
}

function getState(ctx: ResolverContext, id: string): EntityState {
  let s = ctx.states.get(id);
  if (!s) {
    s = { conditions: [] };
    ctx.states.set(id, s);
  }
  if (!s.conditions) s.conditions = [];
  return s;
}

function annotation(effect: Effect, description: string): ResolutionRecord {
  return {
    effect,
    rolls: [],
    outcome: "none",
    applied: [],
    description,
    timestamp: new Date().toISOString(),
  };
}

// ----------------------------------------------------------------------------
// Effect handlers
// ----------------------------------------------------------------------------

function resolveDamage(effect: DamageEffect, ctx: ResolverContext): ResolutionRecord {
  const targets = resolveTargets(effect.target, ctx);
  const applied: AppliedMutation[] = [];
  const rolls: RollDetail[] = [];

  for (const tid of targets) {
    const r = rollDice(effect.amount, { rng: ctx.rng });
    rolls.push(r);
    const state = getState(ctx, tid);
    const before = state.hp ?? 0;
    const max = state.hpMax ?? before;
    const after = Math.max(0, Math.min(max, before - r.result));
    state.hp = after;
    applied.push({ entityId: tid, field: "hp", before, after });
  }

  const totalDamage = rolls.reduce((sum, r) => sum + r.result, 0);
  const description = targets.length
    ? `${ctx.caster?.name ?? "Source"} inflige ${totalDamage} dégâts de ${damageTypeLabel(ctx.ruleset, effect.damageType)} à ${targets.map((t) => nameOf(ctx, t)).join(", ")}`
    : `${ctx.caster?.name ?? "Source"} inflige des dégâts (aucune cible)`;

  return {
    effect,
    rolls,
    outcome: "success",
    applied,
    description,
    timestamp: new Date().toISOString(),
  };
}

function resolveHeal(effect: HealEffect, ctx: ResolverContext): ResolutionRecord {
  const targets = resolveTargets(effect.target, ctx);
  const applied: AppliedMutation[] = [];
  const rolls: RollDetail[] = [];

  for (const tid of targets) {
    const r = rollDice(effect.amount, { rng: ctx.rng });
    rolls.push(r);
    const state = getState(ctx, tid);
    const before = state.hp ?? 0;
    const max = state.hpMax ?? before + r.result;
    const after = Math.max(0, Math.min(max, before + r.result));
    state.hp = after;
    applied.push({ entityId: tid, field: "hp", before, after });
  }

  return {
    effect,
    rolls,
    outcome: "success",
    applied,
    description: `${ctx.caster?.name ?? "Source"} soigne ${targets.map((t) => nameOf(ctx, t)).join(", ")} de ${rolls.reduce((s, r) => s + r.result, 0)} PV`,
    timestamp: new Date().toISOString(),
  };
}

function resolveApplyCondition(
  effect: ApplyConditionEffect,
  ctx: ResolverContext,
): ResolutionRecord {
  const targets = resolveTargets(effect.target, ctx);
  const applied: AppliedMutation[] = [];

  for (const tid of targets) {
    const state = getState(ctx, tid);
    const before = state.conditions.find((c) => c.conditionId === effect.conditionId);
    const next: ActiveCondition = {
      conditionId: effect.conditionId,
      remainingRounds: effect.duration?.rounds,
      remainingMinutes: effect.duration?.minutes,
      source: ctx.caster?.id,
    };
    state.conditions = [
      ...state.conditions.filter((c) => c.conditionId !== effect.conditionId),
      next,
    ];
    applied.push({
      entityId: tid,
      field: `condition:${effect.conditionId}`,
      before: before ?? null,
      after: next,
    });
  }

  return {
    effect,
    rolls: [],
    outcome: "success",
    applied,
    description: `${ctx.caster?.name ?? "Source"} applique « ${rulesetConditionLabel(ctx.ruleset, effect.conditionId)} » à ${targets.map((t) => nameOf(ctx, t)).join(", ")}${effect.duration?.rounds ? ` pendant ${effect.duration.rounds} round(s)` : ""}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveRemoveCondition(
  effect: RemoveConditionEffect,
  ctx: ResolverContext,
): ResolutionRecord {
  const targets = resolveTargets(effect.target, ctx);
  const applied: AppliedMutation[] = [];
  for (const tid of targets) {
    const state = getState(ctx, tid);
    const before = state.conditions.find((c) => c.conditionId === effect.conditionId);
    if (before) {
      state.conditions = state.conditions.filter((c) => c.conditionId !== effect.conditionId);
      applied.push({
        entityId: tid,
        field: `condition:${effect.conditionId}`,
        before,
        after: null,
      });
    }
  }
  return {
    effect,
    rolls: [],
    outcome: applied.length ? "success" : "none",
    applied,
    description: `Retire « ${rulesetConditionLabel(ctx.ruleset, effect.conditionId)} » de ${targets.map((t) => nameOf(ctx, t)).join(", ") || "(personne)"}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveModifyStat(effect: ModifyStatEffect, ctx: ResolverContext): ResolutionRecord {
  const targets = resolveTargets(effect.target, ctx);
  const applied: AppliedMutation[] = [];
  for (const tid of targets) {
    const state = getState(ctx, tid);
    if (effect.stat === "AC" && typeof state.ac === "number") {
      const before = state.ac;
      state.ac = before + effect.modifier;
      applied.push({ entityId: tid, field: "ac", before, after: state.ac });
    }
    // For other stats we just record the intent — full stat modifier system is V1.
  }
  return {
    effect,
    rolls: [],
    outcome: "success",
    applied,
    description: `${ctx.caster?.name ?? "Source"} modifie ${abilityLabel(ctx.ruleset, effect.stat)} de ${effect.modifier > 0 ? "+" : ""}${effect.modifier} sur ${targets.map((t) => nameOf(ctx, t)).join(", ")}`,
    timestamp: new Date().toISOString(),
  };
}

async function resolveRollCheck(
  effect: RollCheckEffect,
  ctx: ResolverContext,
): Promise<ResolutionRecord[]> {
  const targets = resolveTargets(effect.target, ctx);
  const results: ResolutionRecord[] = [];

  if (targets.length === 0) {
    return [
      {
        effect,
        rolls: [],
        outcome: "none",
        applied: [],
        description: `Jet de ${abilityLabel(ctx.ruleset, effect.stat)} (aucune cible)`,
        timestamp: new Date().toISOString(),
      },
    ];
  }

  for (const tid of targets) {
    const state = getState(ctx, tid);
    const ent = ctx.entities.get(tid);
    const flags = computeRollFlags({
      actorConditions: state.conditions ?? [],
      kind: "save",
      saveStat: effect.stat,
      conditions: conditionsById(ctx.ruleset),
    });
    const dc = typeof effect.dc === "number" ? effect.dc : Number(effect.dc) || 10;
    const abilityScores = (ent?.attributes as Record<string, unknown>)?.abilityScores as
      | Record<string, number>
      | undefined;
    const score = abilityScores?.[effect.stat] ?? defaultAbilityScore(ctx.ruleset);

    let roll: RollDetail;
    let outcome: ResolutionRecord["outcome"];
    if (flags.autoFail) {
      roll = { notation: "auto-fail", result: 0, rolls: [] };
      outcome = "fail";
    } else if (flags.autoSuccess) {
      roll = { notation: "auto-success", result: dc + 1, rolls: [] };
      outcome = "success";
    } else {
      const check = rollCheck(ctx.ruleset, {
        score,
        dc,
        advantage: flags.advantage,
        disadvantage: flags.disadvantage,
        rng: ctx.rng,
      });
      roll = check.roll;
      outcome = check.success ? "success" : "fail";
    }

    const description = `${nameOf(ctx, tid)} fait un jet de sauvegarde de ${abilityLabel(ctx.ruleset, effect.stat)} DD ${dc} : ${roll.notation} = ${roll.result} → ${outcome === "success" ? "réussite" : "échec"}`;
    const subEffects = outcome === "success" ? effect.outcomeSuccess ?? [] : effect.outcomeFail ?? [];
    const sub = subEffects.length
      ? await resolveEffects(subEffects, {
          ...ctx,
          explicitTargets: [tid],
        })
      : { records: [], finalStates: ctx.states };

    results.push({
      effect,
      rolls: [roll],
      outcome,
      applied: [],
      description,
      timestamp: new Date().toISOString(),
    });
    results.push(...sub.records);
  }

  return results;
}

function resolveConsumeResource(
  effect: ConsumeResourceEffect,
  ctx: ResolverContext,
): ResolutionRecord {
  const targets = resolveTargets(effect.target, ctx);
  const applied: AppliedMutation[] = [];
  for (const tid of targets) {
    const state = getState(ctx, tid);
    const resources = state.resources ?? {};
    const key = effect.resource === "spell_slot" && effect.level
      ? `spell_slot_${effect.level}`
      : effect.resource;
    const cur = resources[key]?.current ?? 0;
    const max = resources[key]?.max ?? cur;
    const after = Math.max(0, cur - effect.amount);
    resources[key] = { current: after, max };
    state.resources = resources;
    applied.push({ entityId: tid, field: `resource:${key}`, before: cur, after });
  }
  return {
    effect,
    rolls: [],
    outcome: "success",
    applied,
    description: `${ctx.caster?.name ?? "Source"} consomme ${effect.amount} × ${resourceLabel(ctx.ruleset, effect.resource)}${effect.level ? ` niv. ${effect.level}` : ""}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveRestoreResource(
  effect: RestoreResourceEffect,
  ctx: ResolverContext,
): ResolutionRecord {
  const targets = resolveTargets(effect.target, ctx);
  const applied: AppliedMutation[] = [];
  for (const tid of targets) {
    const state = getState(ctx, tid);
    const resources = state.resources ?? {};
    const key = effect.resource === "spell_slot" && effect.level
      ? `spell_slot_${effect.level}`
      : effect.resource;
    const cur = resources[key]?.current ?? 0;
    const max = resources[key]?.max ?? cur + effect.amount;
    const after = Math.min(max, cur + effect.amount);
    resources[key] = { current: after, max };
    state.resources = resources;
    applied.push({ entityId: tid, field: `resource:${key}`, before: cur, after });
  }
  return {
    effect,
    rolls: [],
    outcome: "success",
    applied,
    description: `${ctx.caster?.name ?? "Source"} récupère ${effect.amount} × ${resourceLabel(ctx.ruleset, effect.resource)}`,
    timestamp: new Date().toISOString(),
  };
}

function record(
  effect: Effect,
  description: string,
  applied: AppliedMutation[] = [],
  outcome: ResolutionRecord["outcome"] = "success",
): ResolutionRecord {
  return { effect, rolls: [], outcome, applied, description, timestamp: new Date().toISOString() };
}

function resolveSetState(effect: SetStateEffect, ctx: ResolverContext): ResolutionRecord {
  // Persisté dans l'état du monde : l'attribut d'origine de la fiche ne change pas.
  const attrs = (ctx.world.entityAttributes[effect.entityId] ??= {});
  const before =
    effect.attribute in attrs
      ? attrs[effect.attribute]
      : (ctx.entities.get(effect.entityId)?.attributes?.[effect.attribute] ?? null);
  attrs[effect.attribute] = effect.value;
  return record(
    effect,
    `${nameOf(ctx, effect.entityId)} : ${effect.attribute} → ${String(effect.value)}`,
    [{ entityId: effect.entityId, field: `attr:${effect.attribute}`, before, after: effect.value }],
  );
}

function resolveMoveEntity(effect: MoveEntityEffect, ctx: ResolverContext): ResolutionRecord {
  const state = getState(ctx, effect.entityId);
  const before = state.position?.locationId ?? ctx.world.entityLocations[effect.entityId] ?? null;
  state.position = { ...(state.position ?? {}), locationId: effect.toLocationId };
  ctx.world.entityLocations[effect.entityId] = effect.toLocationId;
  return record(
    effect,
    `${nameOf(ctx, effect.entityId)} se déplace vers ${nameOf(ctx, effect.toLocationId)}`,
    [{ entityId: effect.entityId, field: "position.locationId", before, after: effect.toLocationId }],
  );
}

function resolveRevealEntity(effect: RevealEntityEffect, ctx: ResolverContext): ResolutionRecord {
  const state = getState(ctx, effect.entityId);
  state.visible = true;
  if (!ctx.world.revealedEntityIds.includes(effect.entityId)) ctx.world.revealedEntityIds.push(effect.entityId);
  return record(effect, `Révèle ${nameOf(ctx, effect.entityId)}`, [
    { entityId: effect.entityId, field: "visible", before: false, after: true },
  ]);
}

function resolveSetRelation(effect: SetRelationEffect, ctx: ResolverContext): ResolutionRecord {
  const key = relationKey(effect.fromId, effect.toId);
  const before = ctx.world.relations[key] ?? { value: 0 };
  const after = {
    value: before.value + effect.delta,
    disposition: effect.disposition ?? before.disposition,
  };
  ctx.world.relations[key] = after;
  return record(
    effect,
    `Relation ${nameOf(ctx, effect.fromId)} → ${nameOf(ctx, effect.toId)} : ${effect.delta > 0 ? "+" : ""}${effect.delta} (${after.value})${after.disposition ? `, ${after.disposition}` : ""}`,
    [{ entityId: effect.fromId, field: `relation:${effect.toId}`, before: before.value, after: after.value }],
  );
}

const MAX_EVENT_DEPTH = 5;

async function resolveTriggerEvent(effect: TriggerEventEffect, ctx: ResolverContext): Promise<ResolutionRecord[]> {
  const event = ctx.catalog.get(effect.eventId);
  const head = record(effect, `Déclenche l’événement ${nameOf(ctx, effect.eventId)}`);
  if (!event) return [{ ...head, outcome: "none", description: `${head.description} (introuvable)` }];
  if ((ctx.depth ?? 0) >= MAX_EVENT_DEPTH) {
    return [{ ...head, outcome: "none", description: `${head.description} (chaîne d’événements trop longue, arrêtée)` }];
  }
  const sub = await resolveEffects(event.effects, { ...ctx, depth: (ctx.depth ?? 0) + 1 });
  return [head, ...sub.records];
}

// --- Effets sur le scénario --------------------------------------------------

function resolveSetFlag(effect: SetFlagEffect, ctx: ResolverContext): ResolutionRecord {
  const before = ctx.world.flags[effect.flag] ?? null;
  ctx.world.flags[effect.flag] = effect.value;
  return record(effect, `Drapeau « ${effect.flag} » → ${JSON.stringify(effect.value)}`, [
    { entityId: "world", field: `flag:${effect.flag}`, before, after: effect.value },
  ]);
}

async function resolveAdvanceFront(effect: AdvanceFrontEffect, ctx: ResolverContext): Promise<ResolutionRecord[]> {
  const front = ctx.story.fronts.find((f) => f.id === effect.frontId);
  if (!front) return [record(effect, `Menace inconnue : ${effect.frontId}`, [], "none")];
  const before = frontStep(ctx.world, front.id);
  const after = Math.max(-1, Math.min(front.steps.length - 1, before + (effect.steps ?? 1)));
  ctx.world.frontProgress[front.id] = after;
  if (after === before) {
    return [record(effect, `« ${front.name} » : horloge inchangée`, [], "none")];
  }
  const step = front.steps[after];
  const last = after === front.steps.length - 1;
  const head = record(
    effect,
    after < 0
      ? `« ${front.name} » revient au départ`
      : `${last ? "☠ " : "⏳ "}« ${front.name} » atteint « ${step.label} » (${after + 1}/${front.steps.length})`,
    [{ entityId: "world", field: `front:${front.id}`, before, after }],
  );
  // Les effets d'étape ne s'appliquent qu'en avançant, une fois par étape franchie.
  const records = [head];
  for (let i = before + 1; i <= after; i++) {
    const effects = front.steps[i]?.effects ?? [];
    if (effects.length) records.push(...(await resolveEffects(effects, ctx)).records);
  }
  return records;
}

function resolveRevealClue(effect: RevealClueEffect, ctx: ResolverContext): ResolutionRecord {
  const clue = ctx.story.clues.find((c) => c.id === effect.clueId);
  if (!clue) return record(effect, `Indice inconnu : ${effect.clueId}`, [], "none");
  if (ctx.world.foundClueIds.includes(clue.id)) return record(effect, `🔎 Indice déjà trouvé : ${clue.text}`, [], "none");
  const wasKnown = isRevelationKnown(ctx.story, ctx.world, clue.revelationId);
  ctx.world.foundClueIds.push(clue.id);
  const revelation = ctx.story.revelations.find((r) => r.id === clue.revelationId);
  const suffix = !wasKnown && revelation ? ` — révélation : « ${revelation.statement} »` : "";
  return record(effect, `🔎 Indice trouvé : ${clue.text}${suffix}`, [
    { entityId: "world", field: `clue:${clue.id}`, before: false, after: true },
  ]);
}

function sceneTitle(ctx: ResolverContext, id: string): string {
  return ctx.story.scenes.find((s) => s.id === id)?.title ?? id;
}

function resolveEnterScene(effect: EnterSceneEffect, ctx: ResolverContext): ResolutionRecord {
  const before = ctx.world.currentSceneId ?? null;
  ctx.world.currentSceneId = effect.sceneId;
  if (ctx.world.sceneStatus[effect.sceneId] !== "resolved") ctx.world.sceneStatus[effect.sceneId] = "visited";
  return record(effect, `🎬 Scène : ${sceneTitle(ctx, effect.sceneId)}`, [
    { entityId: "world", field: "currentScene", before, after: effect.sceneId },
  ]);
}

const SCENE_STATUS_LABELS = { available: "disponible", visited: "visitée", resolved: "résolue" } as const;

function resolveSetSceneStatus(effect: SetSceneStatusEffect, ctx: ResolverContext): ResolutionRecord {
  const before = ctx.world.sceneStatus[effect.sceneId] ?? null;
  ctx.world.sceneStatus[effect.sceneId] = effect.status;
  return record(effect, `Scène « ${sceneTitle(ctx, effect.sceneId)} » ${SCENE_STATUS_LABELS[effect.status]}`, [
    { entityId: "world", field: `scene:${effect.sceneId}`, before, after: effect.status },
  ]);
}

function resolveAddToInventory(effect: AddToInventoryEffect, ctx: ResolverContext): ResolutionRecord {
  const targets = resolveTargets(effect.target, ctx);
  const applied: AppliedMutation[] = [];
  for (const tid of targets) {
    const state = getState(ctx, tid);
    const inv = state.inventory ?? [];
    const before = [...inv];
    const next = [...inv];
    const qty = effect.quantity ?? 1;
    for (let i = 0; i < qty; i++) next.push(effect.itemId);
    state.inventory = next;
    applied.push({ entityId: tid, field: "inventory", before, after: next });
  }
  return {
    effect,
    rolls: [],
    outcome: "success",
    applied,
    description: `${nameOf(ctx, effect.itemId)} → ${targets.map((t) => nameOf(ctx, t)).join(", ")}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveRemoveFromInventory(
  effect: RemoveFromInventoryEffect,
  ctx: ResolverContext,
): ResolutionRecord {
  const targets = resolveTargets(effect.target, ctx);
  const applied: AppliedMutation[] = [];
  for (const tid of targets) {
    const state = getState(ctx, tid);
    const inv = state.inventory ?? [];
    const before = [...inv];
    const qty = effect.quantity ?? 1;
    let remaining = qty;
    const next = inv.filter((id) => {
      if (id === effect.itemId && remaining > 0) {
        remaining--;
        return false;
      }
      return true;
    });
    state.inventory = next;
    applied.push({ entityId: tid, field: "inventory", before, after: next });
  }
  return {
    effect,
    rolls: [],
    outcome: "success",
    applied,
    description: `Retire ${nameOf(ctx, effect.itemId)} de ${targets.map((t) => nameOf(ctx, t)).join(", ")}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveDisplayImage(effect: DisplayImageEffect): ResolutionRecord {
  return {
    effect,
    rolls: [],
    outcome: "none",
    applied: [],
    description: `🖼 Affiche une image${effect.prompt ? ` : « ${effect.prompt} »` : effect.imageId ? ` (${effect.imageId})` : ""}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveDisplayText(effect: DisplayTextEffect): ResolutionRecord {
  return {
    effect,
    rolls: [],
    outcome: "none",
    applied: [],
    description: effect.text,
    timestamp: new Date().toISOString(),
  };
}

// ----------------------------------------------------------------------------
// Attack helper — resolves a melee/ranged attack roll vs a target's AC, then
// runs damage on hit (or critical, doubling the dice rolled).
// ----------------------------------------------------------------------------

export interface AttackInput {
  attackerId: string;
  targetIds: string[];
  attackBonus: number;
  damageNotation: string;
  damageType: DamageEffect["damageType"];
  meleeWithin5ft?: boolean;
}

export function resolveAttack(
  input: AttackInput,
  ctx: ResolverContext,
): ResolutionRecord[] {
  const records: ResolutionRecord[] = [];
  const attacker = ctx.entities.get(input.attackerId);
  const attackerState = getState(ctx, input.attackerId);

  for (const tid of input.targetIds) {
    const target = ctx.entities.get(tid);
    const targetState = getState(ctx, tid);
    const ac = (target?.attributes as Record<string, unknown>)?.ac as number | undefined ?? targetState.ac ?? 10;

    const flags = computeRollFlags({
      actorConditions: attackerState.conditions,
      targetConditions: targetState.conditions,
      kind: "attack",
      meleeWithin5ft: input.meleeWithin5ft,
      conditions: conditionsById(ctx.ruleset),
    });

    // Jet d'attaque = test du ruleset contre la CA ; le bonus d'attaque
    // s'ajoute (roll_over) ou sert de valeur cible (roll_under).
    const check = rollCheck(ctx.ruleset, {
      bonus: input.attackBonus,
      dc: ac,
      advantage: flags.advantage,
      disadvantage: flags.disadvantage,
      rng: ctx.rng,
    });
    const roll = check.roll;
    const isCritical = check.critical || flags.autoCritical;
    const hit = !check.fumble && (isCritical || check.success);

    records.push({
      effect: {
        type: "roll_check",
        stat: "STR", // attack rolls aren't a save, but we reuse the union for the timeline
        dc: ac,
        target: { type: "single", entityId: tid },
      },
      rolls: [roll],
      outcome: hit ? "success" : "fail",
      applied: [],
      description: `${attacker?.name ?? "Attaquant"} attaque ${target?.name ?? tid} : ${roll.notation} = ${roll.result} contre CA ${ac} → ${isCritical ? "CRITIQUE" : hit ? "touché" : "raté"}`,
      timestamp: new Date().toISOString(),
    });

    if (hit) {
      // Roll damage; doubled dice on critical
      const damage = isCritical
        ? doubleDamage(input.damageNotation, ctx.rng)
        : rollDice(input.damageNotation, { rng: ctx.rng });

      const before = targetState.hp ?? 0;
      const max = targetState.hpMax ?? before;
      const after = Math.max(0, Math.min(max, before - damage.result));
      targetState.hp = after;

      records.push({
        effect: {
          type: "damage",
          amount: input.damageNotation,
          damageType: input.damageType,
          target: { type: "single", entityId: tid },
        },
        rolls: [damage],
        outcome: "success",
        applied: [{ entityId: tid, field: "hp", before, after }],
        description: `${attacker?.name ?? "Attaquant"} inflige ${damage.result} dégâts de ${damageTypeLabel(ctx.ruleset, input.damageType)} à ${target?.name ?? tid} (PV ${before}→${after})`,
        timestamp: new Date().toISOString(),
      });
    }
  }

  return records;
}

function doubleDamage(notation: string, rng: Rng): RollDetail {
  // Crit: roll the dice portion twice, modifier once.
  // Approximation by rolling the same notation twice and combining.
  const a = rollDice(notation, { rng });
  const b = rollDice(notation, { rng });
  return {
    notation: `${notation} (critique)`,
    result: a.result + (b.result - (a.modifier ?? 0)),
    rolls: [...(a.rolls ?? []), ...(b.rolls ?? [])],
    modifier: a.modifier,
  };
}
