import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { sessions, sessionState, entities, campaigns } from "@/lib/db/schema";
import { StartSessionSchema } from "@/lib/validation/session-schemas";
import { handleApiError, badRequest, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { eq, inArray, desc } from "drizzle-orm";
import type { InitiativeEntry } from "@/lib/engine/types";
import { deriveStateFromEntity } from "@/lib/session/derive-state";

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id: campaignId } = await params;
    const rows = await db
      .select()
      .from(sessions)
      .where(eq(sessions.campaignId, campaignId))
      .orderBy(desc(sessions.startedAt));
    return NextResponse.json({ sessions: rows });
  } catch (error) {
    return handleApiError(error);
  }
}

export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id: campaignId } = await params;
    const [campaign] = await db
      .select()
      .from(campaigns)
      .where(eq(campaigns.id, campaignId));
    if (!campaign) notFound("campaign", campaignId);

    const body = await req.json();
    const input = StartSessionSchema.parse(body);

    const participants = await db
      .select()
      .from(entities)
      .where(inArray(entities.id, input.participantEntityIds));

    if (participants.length !== input.participantEntityIds.length) {
      badRequest("Certains participants sont introuvables");
    }

    const sessionId = generateId("ses");
    const initiativeOrder: InitiativeEntry[] = participants.map((p) => ({
      entityId: p.id,
      initiative: 0,
      isPlayer: p.type === "character",
    }));

    const [session] = await db
      .insert(sessions)
      .values({
        id: sessionId,
        campaignId,
        name: input.name,
        currentPhase: "exploration",
        initiativeOrder,
        combatRound: 0,
        activeTurnIndex: 0,
      })
      .returning();

    // Init session_state for each participant
    for (const p of participants) {
      await db.insert(sessionState).values({
        id: generateId("st"),
        sessionId,
        entityId: p.id,
        currentState: deriveStateFromEntity(p),
      });
    }

    return NextResponse.json({ session }, { status: 201 });
  } catch (error) {
    return handleApiError(error);
  }
}
