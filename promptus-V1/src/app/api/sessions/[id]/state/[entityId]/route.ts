import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { lockCampaign } from "@/lib/db/lock";
import { sessions, sessionState } from "@/lib/db/schema";
import {
  HpUpdateSchema,
  ConditionApplySchema,
} from "@/lib/validation/session-schemas";
import { handleApiError, notFound, badRequest } from "@/lib/api/errors";
import { and, eq } from "drizzle-orm";
import type { EntityState } from "@/lib/engine/types";
import { notifySession } from "@/lib/realtime/notify";

interface PatchBody {
  hpDelta?: number;
  applyCondition?: { conditionId: string; durationRounds?: number };
  removeCondition?: string;
}

export async function PATCH(
  req: NextRequest,
  { params }: { params: Promise<{ id: string; entityId: string }> },
) {
  try {
    const { id: sessionId, entityId } = await params;
    const body = (await req.json()) as PatchBody;

    const [session] = await db
      .select({ campaignId: sessions.campaignId })
      .from(sessions)
      .where(eq(sessions.id, sessionId));
    if (!session) notFound("session", sessionId);

    // Sous verrou : une attaque simultanée ne peut pas écraser ce changement.
    const updated = await db.transaction(async (tx) => {
      await lockCampaign(tx, session.campaignId);
      const [row] = await tx
        .select()
        .from(sessionState)
        .where(
          and(
            eq(sessionState.sessionId, sessionId),
            eq(sessionState.entityId, entityId),
          ),
        );
      if (!row) notFound("session_state", `${sessionId}/${entityId}`);

      const cs = row.currentState as EntityState;
      let next: EntityState = { ...cs, conditions: cs.conditions ?? [] };

      if (body.hpDelta !== undefined) {
        HpUpdateSchema.parse({ delta: body.hpDelta });
        const before = cs.hp ?? 0;
        const max = cs.hpMax ?? before;
        const after = Math.max(0, Math.min(max, before + body.hpDelta));
        next = { ...next, hp: after };
      }

      if (body.applyCondition) {
        ConditionApplySchema.parse(body.applyCondition);
        const existing = next.conditions ?? [];
        next = {
          ...next,
          conditions: [
            ...existing.filter(
              (c) => c.conditionId !== body.applyCondition!.conditionId,
            ),
            {
              conditionId: body.applyCondition.conditionId,
              remainingRounds: body.applyCondition.durationRounds,
            },
          ],
        };
      }

      if (body.removeCondition) {
        if (typeof body.removeCondition !== "string")
          badRequest("removeCondition doit être un identifiant (texte)");
        next = {
          ...next,
          conditions: (next.conditions ?? []).filter(
            (c) => c.conditionId !== body.removeCondition,
          ),
        };
      }

      const [updated] = await tx
        .update(sessionState)
        .set({ currentState: next, updatedAt: new Date() })
        .where(eq(sessionState.id, row.id))
        .returning();
      return updated;
    });

    await notifySession(sessionId, ["session"]);
    return NextResponse.json({ state: updated });
  } catch (error) {
    return handleApiError(error);
  }
}
