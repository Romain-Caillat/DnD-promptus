import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { lockCampaign } from "@/lib/db/lock";
import { sessions, sessionState } from "@/lib/db/schema";
import { handleApiError, notFound } from "@/lib/api/errors";
import { eq } from "drizzle-orm";
import type { EntityState } from "@/lib/engine/types";
import { notifySession } from "@/lib/realtime/notify";

export async function POST(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const [found] = await db.select({ campaignId: sessions.campaignId }).from(sessions).where(eq(sessions.id, id));
    if (!found) notFound("session", id);

    // Sous verrou : deux clics « Suivant » ne sautent pas un tour.
    const updated = await db.transaction(async (tx) => {
      await lockCampaign(tx, found.campaignId);
      const [session] = await tx.select().from(sessions).where(eq(sessions.id, id));
      const order = session.initiativeOrder;
      if (order.length === 0) return session;

      // Les adversaires à 0 PV ne jouent plus (les personnages, si : jets contre la mort).
      const states = await tx.select().from(sessionState).where(eq(sessionState.sessionId, id));
      const hpById = new Map(states.map((s) => [s.entityId, (s.currentState as EntityState).hp]));
      const out = (i: number) => !order[i].isPlayer && (hpById.get(order[i].entityId) ?? 1) <= 0;
      let nextIndex = session.activeTurnIndex;
      let wraps = false;
      for (let step = 0; step < order.length; step++) {
        nextIndex = (nextIndex + 1) % order.length;
        if (nextIndex === 0) wraps = true;
        if (!out(nextIndex)) break;
      }
      const nextRound = wraps ? session.combatRound + 1 : session.combatRound;

      const [row] = await tx
        .update(sessions)
        .set({ activeTurnIndex: nextIndex, combatRound: nextRound })
        .where(eq(sessions.id, id))
        .returning();

      // Fin de round : les durées d'états diminuent.
      if (wraps) {
        for (const s of states) {
          const cs = s.currentState as EntityState;
          const conds = (cs.conditions ?? [])
            .map((c) => ({
              ...c,
              remainingRounds: typeof c.remainingRounds === "number" ? Math.max(0, c.remainingRounds - 1) : c.remainingRounds,
            }))
            .filter((c) => c.remainingRounds === undefined || c.remainingRounds > 0);
          if (JSON.stringify(conds) !== JSON.stringify(cs.conditions ?? [])) {
            await tx
              .update(sessionState)
              .set({ currentState: { ...cs, conditions: conds }, updatedAt: new Date() })
              .where(eq(sessionState.id, s.id));
          }
        }
      }
      return row;
    });

    await notifySession(id, ["session"]);
    return NextResponse.json({ session: updated });
  } catch (error) {
    return handleApiError(error);
  }
}
