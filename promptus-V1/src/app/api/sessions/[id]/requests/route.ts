import { NextRequest, NextResponse } from "next/server";
import { desc, eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { entities, playerRequests, sessionPlayers } from "@/lib/db/schema";
import { handleApiError } from "@/lib/api/errors";

/** Vue MJ : demandes d'action des joueurs (en attente d'abord). */
export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const rows = await db
      .select({
        id: playerRequests.id,
        actionId: playerRequests.actionId,
        attackId: playerRequests.attackId,
        targetIds: playerRequests.targetIds,
        label: playerRequests.label,
        note: playerRequests.note,
        status: playerRequests.status,
        result: playerRequests.result,
        createdAt: playerRequests.createdAt,
        playerName: sessionPlayers.name,
        characterName: entities.name,
      })
      .from(playerRequests)
      .innerJoin(sessionPlayers, eq(sessionPlayers.id, playerRequests.playerId))
      .leftJoin(entities, eq(entities.id, sessionPlayers.characterEntityId))
      .where(eq(playerRequests.sessionId, id))
      .orderBy(desc(playerRequests.createdAt))
      .limit(30);
    return NextResponse.json({ requests: rows });
  } catch (error) {
    return handleApiError(error);
  }
}
