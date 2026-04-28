// Effect resolver — pure functions, no DB, no React.
// The runner returns a list of ResolutionRecords plus the mutations to apply.
// The DB persistence is handled by the API route.

import type {
  ActiveCondition,
  AppliedMutation,
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
  PlayAmbienceEffect,
  PlayMusicEffect,
  PlaySoundEffect,
  RemoveConditionEffect,
  ResolutionRecord,
  RestoreResourceEffect,
  RevealEntityEffect,
  RollCheckEffect,
  RollDetail,
  SetRelationEffect,
  SetStateEffect,
  Stat,
  TargetSpec,
  TriggerEventEffect,
  AddToInventoryEffect,
  RemoveFromInventoryEffect,
} from "./types";
import { computeRollFlags } from "./conditions";
import { rollD20, rollDice, type Rng, defaultRng } from "./dice";

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
  rng: Rng;
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
      return [resolveTriggerEvent(effect, ctx)];
    case "add_to_inventory":
      return [resolveAddToInventory(effect, ctx)];
    case "remove_from_inventory":
      return [resolveRemoveFromInventory(effect, ctx)];
    case "play_ambience":
      return [annotation(effect, `🔊 Ambience: ${effect.ambienceId}`)];
    case "play_music":
      return [annotation(effect, `🎵 Music: ${effect.musicId}`)];
    case "play_sound":
      return [annotation(effect, `🔔 Sound: ${effect.soundId}`)];
    case "display_image":
      return [resolveDisplayImage(effect)];
    case "display_text":
      return [resolveDisplayText(effect)];
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
  return ctx.entities.get(id)?.name ?? id;
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
    ? `${ctx.caster?.name ?? "Source"} deals ${totalDamage} ${effect.damageType} damage to ${targets.map((t) => nameOf(ctx, t)).join(", ")}`
    : `${ctx.caster?.name ?? "Source"} deals damage (no targets)`;

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
    description: `${ctx.caster?.name ?? "Source"} heals ${targets.map((t) => nameOf(ctx, t)).join(", ")} for ${rolls.reduce((s, r) => s + r.result, 0)}`,
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
    description: `${ctx.caster?.name ?? "Source"} applies ${effect.conditionId} to ${targets.map((t) => nameOf(ctx, t)).join(", ")}${effect.duration?.rounds ? ` for ${effect.duration.rounds}r` : ""}`,
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
    description: `Remove ${effect.conditionId} from ${targets.map((t) => nameOf(ctx, t)).join(", ") || "(no one)"}`,
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
    description: `${ctx.caster?.name ?? "Source"} modifies ${effect.stat} by ${effect.modifier} on ${targets.map((t) => nameOf(ctx, t)).join(", ")}`,
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
        description: `Roll ${effect.stat} check (no targets)`,
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
      saveStat: effect.stat as Stat,
    });
    const dc = typeof effect.dc === "number" ? effect.dc : Number(effect.dc) || 10;
    const abilityScore = (ent?.attributes as Record<string, unknown>)?.abilityScores as
      | Record<Stat, number>
      | undefined;
    const score = abilityScore?.[effect.stat as Stat] ?? 10;
    const modifier = Math.floor((score - 10) / 2);

    let roll: RollDetail;
    let outcome: ResolutionRecord["outcome"];
    if (flags.autoFail) {
      roll = { notation: "auto-fail", result: 0, rolls: [] };
      outcome = "fail";
    } else if (flags.autoSuccess) {
      roll = { notation: "auto-success", result: dc + 1, rolls: [] };
      outcome = "success";
    } else {
      roll = rollD20(modifier, {
        advantage: flags.advantage,
        disadvantage: flags.disadvantage,
        rng: ctx.rng,
      });
      outcome = roll.result >= dc ? "success" : "fail";
    }

    const description = `${nameOf(ctx, tid)} rolls ${effect.stat} save vs DC ${dc}: ${roll.notation} = ${roll.result} → ${outcome}`;
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
    description: `${ctx.caster?.name ?? "Source"} consumes ${effect.amount} ${effect.resource}${effect.level ? ` lvl ${effect.level}` : ""}`,
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
    description: `${ctx.caster?.name ?? "Source"} restores ${effect.amount} ${effect.resource}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveSetState(effect: SetStateEffect, ctx: ResolverContext): ResolutionRecord {
  // Generic free-form state mutation — only common attributes get persisted via
  // session_state; arbitrary attributes are recorded in the timeline only.
  const ent = ctx.entities.get(effect.entityId);
  const before = ent?.attributes?.[effect.attribute] ?? null;
  return {
    effect,
    rolls: [],
    outcome: "success",
    applied: [{ entityId: effect.entityId, field: `attr:${effect.attribute}`, before, after: effect.value }],
    description: `${nameOf(ctx, effect.entityId)} ${effect.attribute} → ${String(effect.value)}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveMoveEntity(effect: MoveEntityEffect, ctx: ResolverContext): ResolutionRecord {
  const state = getState(ctx, effect.entityId);
  const before = state.position?.locationId ?? null;
  state.position = { ...(state.position ?? {}), locationId: effect.toLocationId };
  return {
    effect,
    rolls: [],
    outcome: "success",
    applied: [{ entityId: effect.entityId, field: "position.locationId", before, after: effect.toLocationId }],
    description: `${nameOf(ctx, effect.entityId)} moved to ${nameOf(ctx, effect.toLocationId)}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveRevealEntity(effect: RevealEntityEffect, ctx: ResolverContext): ResolutionRecord {
  const state = getState(ctx, effect.entityId);
  state.visible = true;
  return {
    effect,
    rolls: [],
    outcome: "success",
    applied: [{ entityId: effect.entityId, field: "visible", before: false, after: true }],
    description: `Reveal ${nameOf(ctx, effect.entityId)}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveSetRelation(effect: SetRelationEffect, _ctx: ResolverContext): ResolutionRecord {
  return {
    effect,
    rolls: [],
    outcome: "success",
    applied: [],
    description: `Relation ${effect.fromId} → ${effect.toId}: ${effect.delta > 0 ? "+" : ""}${effect.delta}${effect.disposition ? ` (${effect.disposition})` : ""}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveTriggerEvent(effect: TriggerEventEffect, _ctx: ResolverContext): ResolutionRecord {
  return {
    effect,
    rolls: [],
    outcome: "success",
    applied: [],
    description: `Trigger event ${effect.eventId}`,
    timestamp: new Date().toISOString(),
  };
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
    description: `${effect.itemId} → ${targets.map((t) => nameOf(ctx, t)).join(", ")}`,
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
    description: `Remove ${effect.itemId} from ${targets.map((t) => nameOf(ctx, t)).join(", ")}`,
    timestamp: new Date().toISOString(),
  };
}

function resolveDisplayImage(effect: DisplayImageEffect): ResolutionRecord {
  return {
    effect,
    rolls: [],
    outcome: "none",
    applied: [],
    description: `🖼 Display image${effect.prompt ? `: "${effect.prompt}"` : effect.imageId ? ` (${effect.imageId})` : ""}`,
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
    });

    const roll = rollD20(input.attackBonus, {
      advantage: flags.advantage,
      disadvantage: flags.disadvantage,
      rng: ctx.rng,
    });
    const rolledFaces = roll.rolls ?? [];
    const isNat20 = rolledFaces[0] === 20 || (flags.advantage && rolledFaces.includes(20));
    const isNat1 = rolledFaces[0] === 1 || (flags.disadvantage && rolledFaces.includes(1));
    const isCritical = isNat20 || flags.autoCritical;
    const hit = !isNat1 && (isCritical || roll.result >= ac);

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
      description: `${attacker?.name ?? "Attacker"} attacks ${target?.name ?? tid}: ${roll.notation} = ${roll.result} vs AC ${ac} → ${isCritical ? "CRITICAL" : hit ? "hit" : "miss"}`,
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
        description: `${attacker?.name ?? "Attacker"} deals ${damage.result} ${input.damageType} damage to ${target?.name ?? tid} (HP ${before}→${after})`,
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
    notation: `${notation} (crit)`,
    result: a.result + (b.result - (a.modifier ?? 0)),
    rolls: [...(a.rolls ?? []), ...(b.rolls ?? [])],
    modifier: a.modifier,
  };
}
