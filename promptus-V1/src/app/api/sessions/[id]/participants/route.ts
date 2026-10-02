import { NextRequest, NextResponse } from "next/server";
import { and, eq, inArray } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { entities, sessions, sessionState } from "@/lib/db/schema";
import { badRequest, handleApiError, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { deriveStateFromEntity } from "@/lib/session/derive-state";

const BodySchema = z.object({ entityIds: z.array(z.string()).min(1) });

/** Ajoute des participants en cours de session (ceux déjà présents sont ignorés). */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const { entityIds } = BodySchema.parse(await req.json());
    const [session] = await db.select().from(sessions).where(eq(sessions.id, id));
    if (!session) notFound("session", id);

    const ents = await db
      .select()
      .from(entities)
      .where(and(eq(entities.campaignId, session.campaignId), inArray(entities.id, entityIds)));
    if (ents.length !== new Set(entityIds).size) badRequest("Certaines fiches sont introuvables dans cette campagne");

    const existing = await db.select({ entityId: sessionState.entityId }).from(sessionState).where(eq(sessionState.sessionId, id));
    const present = new Set(existing.map((e) => e.entityId));
    const added = ents.filter((e) => !present.has(e.id));

    if (added.length) {
      await db.transaction(async (tx) => {
        await tx.insert(sessionState).values(
          added.map((e) => ({ id: generateId("st"), sessionId: id, entityId: e.id, currentState: deriveStateFromEntity(e) })),
        );
        await tx
          .update(sessions)
          .set({
            initiativeOrder: [
              ...session.initiativeOrder,
              ...added.map((e) => ({ entityId: e.id, initiative: 0, isPlayer: e.type === "character" })),
            ],
          })
          .where(eq(sessions.id, id));
      });
    }
    return NextResponse.json({ added: added.map((e) => e.id) });
  } catch (error) {
    return handleApiError(error);
  }
}
