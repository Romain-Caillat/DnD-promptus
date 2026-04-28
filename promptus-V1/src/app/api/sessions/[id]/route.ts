import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { sessions, sessionState, entities } from "@/lib/db/schema";
import { handleApiError, notFound } from "@/lib/api/errors";
import { eq } from "drizzle-orm";

export async function GET(
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

    const participantIds = states.map((s) => s.entityId);
    const ents = participantIds.length
      ? await db
          .select()
          .from(entities)
          .where(eq(entities.campaignId, session.campaignId))
      : [];
    const ent_by_id = new Map(ents.map((e) => [e.id, e]));

    const participants = states.map((s) => ({
      state: s,
      entity: ent_by_id.get(s.entityId),
    }));

    return NextResponse.json({ session, participants });
  } catch (error) {
    return handleApiError(error);
  }
}
