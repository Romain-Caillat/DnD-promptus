import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import {
  sessions,
  sessionState,
  sessionTimeline,
  entities,
} from "@/lib/db/schema";
import { ActionSchema } from "@/lib/validation/action-schemas";
import { handleApiError, badRequest, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { eq } from "drizzle-orm";
import {
  newContext,
  resolveAttack,
  resolveEffects,
  type EntityRef,
} from "@/lib/engine/resolver";
import type { Effect, EntityState, ResolutionRecord } from "@/lib/engine/types";

export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id: sessionId } = await params;
    const body = await req.json();
    const action = ActionSchema.parse(body);

    const [session] = await db.select().from(sessions).where(eq(sessions.id, sessionId));
    if (!session) notFound("session", sessionId);

    const states = await db
      .select()
      .from(sessionState)
      .where(eq(sessionState.sessionId, sessionId));
    const ents = await db
      .select()
      .from(entities)
      .where(eq(entities.campaignId, session.campaignId));
    const ent_by_id = new Map(ents.map((e) => [e.id, e]));
    const state_row_by_id = new Map(states.map((s) => [s.entityId, s]));

    const refs: EntityRef[] = states.map((s) => {
      const ent = ent_by_id.get(s.entityId);
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
        : action.kind === "cast_spell"
        ? action.casterId
        : "casterId" in action
        ? action.casterId
        : undefined;
    const caster = casterId ? refs.find((r) => r.id === casterId) : undefined;

    const ctx = newContext({
      caster,
      entities: refs,
      initiativeOrder: session.initiativeOrder,
      explicitTargets:
        "targetIds" in action && action.targetIds?.length
          ? action.targetIds
          : undefined,
    });

    let records: ResolutionRecord[] = [];

    if (action.kind === "attack") {
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
    } else if (action.kind === "cast_spell") {
      const spell = ent_by_id.get(action.spellEntityId);
      if (!spell || spell.type !== "spell") {
        badRequest(`Sort ${action.spellEntityId} introuvable`);
      }
      const result = await resolveEffects(spell!.effects as Effect[], ctx);
      records = result.records;
    } else if (action.kind === "apply_entity_effects") {
      const ent = ent_by_id.get(action.entityId);
      if (!ent) badRequest(`Fiche ${action.entityId} introuvable`);
      const result = await resolveEffects(ent!.effects as Effect[], ctx);
      records = result.records;
    } else if (action.kind === "raw_effects") {
      const result = await resolveEffects(action.effects, ctx);
      records = result.records;
    }

    // Persist mutations: every state in ctx.states that differs gets updated
    for (const [entityId, nextState] of ctx.states.entries()) {
      const row = state_row_by_id.get(entityId);
      if (!row) continue;
      // shallow compare via stringify — small payloads
      if (JSON.stringify(row.currentState) !== JSON.stringify(nextState)) {
        await db
          .update(sessionState)
          .set({ currentState: nextState, updatedAt: new Date() })
          .where(eq(sessionState.id, row.id));
      }
    }

    // Append timeline rows for each record
    for (const r of records) {
      await db.insert(sessionTimeline).values({
        id: generateId("tl"),
        sessionId,
        round: session.combatRound,
        description: r.description,
        resolutionRecord: r,
      });
    }

    return NextResponse.json({ records });
  } catch (error) {
    return handleApiError(error);
  }
}
