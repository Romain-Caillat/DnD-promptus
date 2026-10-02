import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { lockCampaign } from "@/lib/db/lock";
import { campaigns, sessions } from "@/lib/db/schema";
import { badRequest, handleApiError, notFound } from "@/lib/api/errors";
import { normalizeWorld } from "@/lib/engine/world";
import { musicPosition, parseYouTube, type MusicState } from "@/lib/media/youtube";
import { notifySession } from "@/lib/realtime/notify";

const BodySchema = z.discriminatedUnion("action", [
  z.object({ action: z.literal("play"), url: z.string().min(1).max(500), title: z.string().max(200).optional() }),
  z.object({ action: z.literal("pause") }),
  z.object({ action: z.literal("resume") }),
  z.object({ action: z.literal("stop") }),
]);

/** Le MJ pilote la musique d'ambiance, jouée en même temps chez chaque joueur. */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const body = BodySchema.parse(await req.json());
    const [session] = await db.select().from(sessions).where(eq(sessions.id, id));
    if (!session) notFound("session", id);
    const now = Date.now();
    const music = await db.transaction(async (tx) => {
      const campaign = await lockCampaign(tx, session.campaignId);
      const world = normalizeWorld(campaign.worldState);
      let music: MusicState | undefined = world.music;
      switch (body.action) {
        case "play": {
          const ref = parseYouTube(body.url);
          if (!ref) badRequest("Lien YouTube invalide");
          music = { ...ref, title: body.title, startedAt: now, offsetSec: 0, playing: true };
          break;
        }
        case "pause":
          if (music) music = { ...music, offsetSec: musicPosition(music, now), startedAt: now, playing: false };
          break;
        case "resume":
          if (music) music = { ...music, startedAt: now, playing: true };
          break;
        case "stop":
          music = undefined;
          break;
      }
      await tx.update(campaigns).set({ worldState: { ...world, music }, updatedAt: new Date() }).where(eq(campaigns.id, campaign.id));
      return music;
    });
    await notifySession(id, ["story"]);
    return NextResponse.json({ music: music ?? null, serverTime: now });
  } catch (error) {
    return handleApiError(error);
  }
}
