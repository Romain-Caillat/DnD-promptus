import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import {
  campaigns,
  entities,
  sessions,
  sessionState,
  sessionTimeline,
} from "@/lib/db/schema";
import { ActionSchema } from "@/lib/validation/action-schemas";
import { handleApiError, badRequest, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import {
  newContext,
  resolveAttack,
  resolveEffects,
  type CatalogEntry,
  type EntityRef,
} from "@/lib/engine/resolver";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { EMPTY_STORY } from "@/lib/engine/story";
import { normalizeWorld, triggerKey } from "@/lib/engine/world";
import type { Effect, EntityState, ResolutionRecord } from "@/lib/engine/types";
import { notifySession } from "@/lib/realtime/notify";

/** Effets qui relèvent de la cuisine du MJ : invisibles dans le journal des joueurs. */
const GM_ONLY_EFFECTS = new Set<Effect["type"]>([
  "advance_front",
  "set_flag",
  "set_state",
  "set_relation",
  "set_scene_status",
  "trigger_event",
  "play_ambience",
  "play_music",
  "play_sound",
  "display_image",
]);

function isPublicRecord(r: ResolutionRecord, isTriggerHeader: boolean): boolean {
  return !isTriggerHeader && !GM_ONLY_EFFECTS.has(r.effect.type);
}

export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id: sessionId } = await params;
    const action = ActionSchema.parse(await req.json());

    const [session] = await db.select().from(sessions).where(eq(sessions.id, sessionId));
    if (!session) notFound("session", sessionId);
    const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, session.campaignId));
    if (!campaign) notFound("campaign", session.campaignId);

    const states = await db.select().from(sessionState).where(eq(sessionState.sessionId, sessionId));
    const ents = await db.select().from(entities).where(eq(entities.campaignId, session.campaignId));
    const entById = new Map(ents.map((e) => [e.id, e]));
    const stateRowById = new Map(states.map((s) => [s.entityId, s]));

    const refs: EntityRef[] = states.map((s) => {
      const ent = entById.get(s.entityId);
      return {
        id: s.entityId,
        name: ent?.name ?? s.entityId,
        attributes: (ent?.attributes ?? {}) as Record<string, unknown>,
        state: s.currentState as EntityState,
      };
    });

    const casterId =
      action.kind === "attack"
        ? action.attackerId
        : action.kind === "fire_trigger"
          ? undefined
          : action.casterId;
    const story = campaign.story ?? EMPTY_STORY;
    const worldBefore = normalizeWorld(campaign.worldState);
    const ctx = newContext({
      ruleset: campaign.ruleset ?? DND5E_RULESET,
      story,
      world: worldBefore,
      catalog: new Map<string, CatalogEntry>(
        ents.map((e) => [e.id, { name: e.name, effects: e.effects as Effect[] }]),
      ),
      caster: casterId ? refs.find((r) => r.id === casterId) : undefined,
      entities: refs,
      initiativeOrder: session.initiativeOrder,
      explicitTargets: "targetIds" in action && action.targetIds?.length ? action.targetIds : undefined,
    });

    let records: ResolutionRecord[] = [];
    switch (action.kind) {
      case "attack":
        records = resolveAttack(
          {
            attackerId: action.attackerId,
            targetIds: action.targetIds,
            attackBonus: action.attackBonus,
            damageNotation: action.damageNotation,
            damageType: action.damageType,
            meleeWithin5ft: action.meleeWithin5ft,
          },
          ctx,
        );
        break;
      case "cast_spell": {
        const spell = entById.get(action.spellEntityId);
        if (!spell || spell.type !== "spell") badRequest(`Sort ${action.spellEntityId} introuvable`);
        records = (await resolveEffects(spell.effects as Effect[], ctx)).records;
        break;
      }
      case "apply_entity_effects": {
        const ent = entById.get(action.entityId);
        if (!ent) badRequest(`Fiche ${action.entityId} introuvable`);
        records = (await resolveEffects(ent.effects as Effect[], ctx)).records;
        break;
      }
      case "raw_effects":
        records = (await resolveEffects(action.effects, ctx)).records;
        break;
      case "fire_trigger": {
        const scene = story.scenes.find((s) => s.id === action.sceneId);
        const trigger = scene?.triggers.find((t) => t.id === action.triggerId);
        if (!scene || !trigger) badRequest(`Déclencheur ${action.sceneId}/${action.triggerId} introuvable`);
        const key = triggerKey(scene.id, trigger.id);
        if (!ctx.world.firedTriggerIds.includes(key)) ctx.world.firedTriggerIds.push(key);
        const sub = await resolveEffects(trigger.effects, ctx);
        records = [
          {
            effect: { type: "display_text", text: trigger.label },
            rolls: [],
            outcome: "success",
            applied: [],
            description: `⚡ Déclencheur : ${trigger.label}`,
            timestamp: new Date().toISOString(),
          },
          ...sub.records,
        ];
        break;
      }
    }

    // En entrant dans une scène, la session passe dans la phase de la scène.
    const entered = [...records].reverse().find((r) => r.effect.type === "enter_scene");
    const enteredScene = entered?.effect.type === "enter_scene"
      ? story.scenes.find((s) => s.id === (entered.effect as { sceneId: string }).sceneId)
      : undefined;

    await db.transaction(async (tx) => {
      for (const [entityId, nextState] of ctx.states.entries()) {
        const row = stateRowById.get(entityId);
        if (!row) continue;
        if (JSON.stringify(row.currentState) !== JSON.stringify(nextState)) {
          await tx
            .update(sessionState)
            .set({ currentState: nextState, updatedAt: new Date() })
            .where(eq(sessionState.id, row.id));
        }
      }
      if (JSON.stringify(ctx.world) !== JSON.stringify(worldBefore)) {
        await tx
          .update(campaigns)
          .set({ worldState: ctx.world, updatedAt: new Date() })
          .where(eq(campaigns.id, campaign.id));
      }
      if (enteredScene && enteredScene.phase !== session.currentPhase) {
        await tx.update(sessions).set({ currentPhase: enteredScene.phase }).where(eq(sessions.id, sessionId));
      }
      if (records.length) {
        // Horodatages croissants : le journal est trié par date de création.
        const now = Date.now();
        await tx.insert(sessionTimeline).values(
          records.map((r, i) => ({
            id: generateId("tl"),
            sessionId,
            round: session.combatRound,
            description: r.description,
            resolutionRecord: r,
            isPublic: isPublicRecord(r, action.kind === "fire_trigger" && i === 0),
            createdAt: new Date(now + i),
          })),
        );
      }
    });

    await notifySession(sessionId, ["session", "story", "timeline"]);
    return NextResponse.json({ records, world: ctx.world });
  } catch (error) {
    return handleApiError(error);
  }
}
