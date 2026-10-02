import { NextRequest, NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { entities, sessionPlayers, sessionState } from "@/lib/db/schema";
import { badRequest, handleApiError } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { hashToken, newPlayerToken, sessionByInvite } from "@/lib/play/server";
import { notifySession } from "@/lib/realtime/notify";

const BodySchema = z.object({
  name: z.string().trim().min(1, "Choisissez un pseudo").max(40),
  characterEntityId: z.string().nullable().default(null),
});

/** Le joueur rejoint la partie : reçoit un jeton secret à conserver. */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ code: string }> },
) {
  try {
    const { code } = await params;
    const session = await sessionByInvite(code);
    const { name, characterEntityId } = BodySchema.parse(await req.json());

    if (characterEntityId) {
      const [inSession] = await db
        .select({ id: sessionState.entityId, type: entities.type })
        .from(sessionState)
        .innerJoin(entities, eq(entities.id, sessionState.entityId))
        .where(and(eq(sessionState.sessionId, session.id), eq(sessionState.entityId, characterEntityId)));
      if (!inSession || inSession.type !== "character") badRequest("Ce personnage ne participe pas à la partie");
      const [taken] = await db
        .select({ id: sessionPlayers.id })
        .from(sessionPlayers)
        .where(and(eq(sessionPlayers.sessionId, session.id), eq(sessionPlayers.characterEntityId, characterEntityId)));
      if (taken) badRequest("Ce personnage est déjà pris");
    }

    const token = newPlayerToken();
    const id = generateId("pl");
    await db.insert(sessionPlayers).values({ id, sessionId: session.id, name, characterEntityId, tokenHash: hashToken(token) });
    await notifySession(session.id, ["players"]);
    return NextResponse.json({ playerId: id, token }, { status: 201 });
  } catch (error) {
    return handleApiError(error);
  }
}
