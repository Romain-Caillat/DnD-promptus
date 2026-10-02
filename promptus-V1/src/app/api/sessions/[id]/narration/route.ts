import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { sessions, sessionTimeline } from "@/lib/db/schema";
import { handleApiError, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { notifySession } from "@/lib/realtime/notify";

const BodySchema = z.object({
  text: z.string().trim().min(1).max(4000),
  /** Nom du PNJ pour une réplique ; absent = narration. */
  speaker: z.string().trim().max(120).optional(),
});

/** Le MJ envoie une narration ou une réplique aux joueurs (journal public). */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const { text, speaker } = BodySchema.parse(await req.json());
    const [session] = await db.select().from(sessions).where(eq(sessions.id, id));
    if (!session) notFound("session", id);
    await db.insert(sessionTimeline).values({
      id: generateId("tl"),
      sessionId: id,
      round: session.combatRound,
      kind: speaker ? "npc" : "narration",
      description: speaker ? `🗣️ ${speaker} : « ${text} »` : `📜 ${text}`,
    });
    await notifySession(id, ["timeline"]);
    return NextResponse.json({ ok: true }, { status: 201 });
  } catch (error) {
    return handleApiError(error);
  }
}
