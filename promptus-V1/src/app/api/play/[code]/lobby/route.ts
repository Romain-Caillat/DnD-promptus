import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns, entities, sessionPlayers, sessionState } from "@/lib/db/schema";
import { handleApiError } from "@/lib/api/errors";
import { sessionByInvite } from "@/lib/play/server";

/** Accueil d'un joueur : nom de la partie et personnages disponibles. */
export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ code: string }> },
) {
  try {
    const { code } = await params;
    const session = await sessionByInvite(code);
    const [campaign] = await db.select({ name: campaigns.name }).from(campaigns).where(eq(campaigns.id, session.campaignId));
    const chars = await db
      .select({ id: entities.id, name: entities.name, description: entities.description, imageUrl: entities.imageUrl, type: entities.type })
      .from(sessionState)
      .innerJoin(entities, eq(entities.id, sessionState.entityId))
      .where(eq(sessionState.sessionId, session.id));
    const taken = new Set(
      (await db.select({ c: sessionPlayers.characterEntityId }).from(sessionPlayers).where(eq(sessionPlayers.sessionId, session.id)))
        .map((r) => r.c)
        .filter(Boolean),
    );
    return NextResponse.json({
      sessionName: session.name,
      campaignName: campaign?.name ?? "",
      characters: chars
        .filter((c) => c.type === "character")
        .map((c) => ({ id: c.id, name: c.name, description: c.description, imageUrl: c.imageUrl, taken: taken.has(c.id) })),
    });
  } catch (error) {
    return handleApiError(error);
  }
}
