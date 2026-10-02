import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { entities, sessionPlayers } from "@/lib/db/schema";
import { handleApiError } from "@/lib/api/errors";
import { ensureInviteCode } from "@/lib/play/server";

/** Vue MJ : code d'invitation et joueurs ayant rejoint. */
export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const inviteCode = await ensureInviteCode(id);
    const players = await db
      .select({
        id: sessionPlayers.id,
        name: sessionPlayers.name,
        characterEntityId: sessionPlayers.characterEntityId,
        characterName: entities.name,
        lastSeenAt: sessionPlayers.lastSeenAt,
      })
      .from(sessionPlayers)
      .leftJoin(entities, eq(entities.id, sessionPlayers.characterEntityId))
      .where(eq(sessionPlayers.sessionId, id));
    return NextResponse.json({ inviteCode, players });
  } catch (error) {
    return handleApiError(error);
  }
}
