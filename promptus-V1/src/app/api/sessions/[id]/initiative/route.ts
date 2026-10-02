import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { sessions, sessionState, entities } from "@/lib/db/schema";
import { InitiativeUpdateSchema } from "@/lib/validation/session-schemas";
import { handleApiError, notFound } from "@/lib/api/errors";
import { eq } from "drizzle-orm";
import type { InitiativeEntry } from "@/lib/engine/types";
import { rollInitiative } from "@/lib/engine/ruleset";
import { getCampaignRuleset } from "@/lib/rules/server";

export async function PATCH(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const body = await req.json();
    const input = InitiativeUpdateSchema.parse(body);
    const [row] = await db
      .update(sessions)
      .set({
        initiativeOrder: input.initiativeOrder,
        ...(input.activeTurnIndex !== undefined && { activeTurnIndex: input.activeTurnIndex }),
        ...(input.combatRound !== undefined && { combatRound: input.combatRound }),
      })
      .where(eq(sessions.id, id))
      .returning();
    if (!row) notFound("session", id);
    return NextResponse.json({ session: row });
  } catch (error) {
    return handleApiError(error);
  }
}

// POST = roll initiative for everyone in current order
export async function POST(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const [session] = await db.select().from(sessions).where(eq(sessions.id, id));
    if (!session) notFound("session", id);

    const states = await db
      .select()
      .from(sessionState)
      .where(eq(sessionState.sessionId, id));

    const ents = await db
      .select()
      .from(entities)
      .where(eq(entities.campaignId, session.campaignId));
    const ent_by_id = new Map(ents.map((e) => [e.id, e]));

    const ruleset = await getCampaignRuleset(session.campaignId);
    const newOrder: InitiativeEntry[] = states.map((s) => {
      const ent = ent_by_id.get(s.entityId);
      const a = (ent?.attributes ?? {}) as Record<string, unknown>;
      return {
        entityId: s.entityId,
        initiative: rollInitiative(ruleset, {
          abilityScores: a.abilityScores as Record<string, number> | undefined,
          bonus: typeof a.initiativeBonus === "number" ? a.initiativeBonus : undefined,
        }),
        isPlayer: ent?.type === "character",
      };
    });

    newOrder.sort((a, b) => b.initiative - a.initiative);

    const [row] = await db
      .update(sessions)
      .set({
        initiativeOrder: newOrder,
        combatRound: 1,
        activeTurnIndex: 0,
      })
      .where(eq(sessions.id, id))
      .returning();
    return NextResponse.json({ session: row });
  } catch (error) {
    return handleApiError(error);
  }
}
