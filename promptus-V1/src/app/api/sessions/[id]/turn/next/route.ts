import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { sessions, sessionState } from "@/lib/db/schema";
import { handleApiError, notFound } from "@/lib/api/errors";
import { eq } from "drizzle-orm";
import type { EntityState } from "@/lib/engine/types";

export async function POST(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const [session] = await db.select().from(sessions).where(eq(sessions.id, id));
    if (!session) notFound("session", id);

    const order = session.initiativeOrder;
    if (order.length === 0) {
      return NextResponse.json({ session });
    }

    const nextIndex = (session.activeTurnIndex + 1) % order.length;
    const wraps = nextIndex === 0;
    const nextRound = wraps ? session.combatRound + 1 : session.combatRound;

    const [updated] = await db
      .update(sessions)
      .set({
        activeTurnIndex: nextIndex,
        combatRound: nextRound,
      })
      .where(eq(sessions.id, id))
      .returning();

    // Decrement condition durations on round wrap
    if (wraps) {
      const states = await db
        .select()
        .from(sessionState)
        .where(eq(sessionState.sessionId, id));
      for (const s of states) {
        const cs = s.currentState as EntityState;
        const conds = (cs.conditions ?? []).map((c) => ({
          ...c,
          remainingRounds:
            typeof c.remainingRounds === "number"
              ? Math.max(0, c.remainingRounds - 1)
              : c.remainingRounds,
        }));
        const filtered = conds.filter(
          (c) => c.remainingRounds === undefined || c.remainingRounds > 0,
        );
        if (filtered.length !== cs.conditions.length || filtered.some((f, i) => f !== cs.conditions[i])) {
          await db
            .update(sessionState)
            .set({
              currentState: { ...cs, conditions: filtered },
              updatedAt: new Date(),
            })
            .where(eq(sessionState.id, s.id));
        }
      }
    }

    return NextResponse.json({ session: updated });
  } catch (error) {
    return handleApiError(error);
  }
}
