import { NextRequest, NextResponse, after } from "next/server";
import { z } from "zod";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { badRequest, handleApiError, notFound } from "@/lib/api/errors";
import type { CampaignStory } from "@/lib/engine/story";
import { parseYouTube } from "@/lib/media/youtube";
import { mediaOverview, queueMedia, runMediaCalls } from "@/lib/media/server";

// Les médias sont générés après la réponse ; une vidéo peut être longue.
export const maxDuration = 900;

const TargetSchema = z.object({
  type: z.enum(["entity", "scene_image", "scene_video", "map"]),
  id: z.string().min(1),
});

const BodySchema = z.object({
  items: z.array(z.object({ target: TargetSchema, prompt: z.string().max(4000).optional() })).min(1).max(200),
  regenerate: z.boolean().optional(),
});

/** Médias de la campagne : scènes, fiches, cartes, état et budget. */
export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    return NextResponse.json(await mediaOverview(id));
  } catch (error) {
    return handleApiError(error);
  }
}

/** Lance la génération (en arrière-plan) après estimation du coût. */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const { items, regenerate } = BodySchema.parse(await req.json());
    const { callIds, estimateUsd } = await queueMedia(id, items, { regenerate });
    if (callIds.length) after(() => runMediaCalls(id, callIds));
    return NextResponse.json({ queued: callIds.length, estimateUsd }, { status: 202 });
  } catch (error) {
    return handleApiError(error);
  }
}

const MusicSchema = z.object({ sceneId: z.string(), musicUrl: z.string().max(500) });

/** Musique YouTube d'une scène (lien choisi par le MJ ; vide = aucune). */
export async function PATCH(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const { sceneId, musicUrl } = MusicSchema.parse(await req.json());
    if (musicUrl.trim() && !parseYouTube(musicUrl)) badRequest("Lien YouTube invalide");
    await db.transaction(async (tx) => {
      const [c] = await tx.select({ story: campaigns.story }).from(campaigns).where(eq(campaigns.id, id)).for("update");
      if (!c) notFound("campaign", id);
      const story: CampaignStory | null = c.story ? structuredClone(c.story) : null;
      const scene = story?.scenes.find((s) => s.id === sceneId);
      if (!story || !scene) badRequest("Scène introuvable");
      scene.media = { ...scene.media, musicUrl: musicUrl.trim() || undefined };
      await tx.update(campaigns).set({ story, updatedAt: new Date() }).where(eq(campaigns.id, id));
    });
    return NextResponse.json({ ok: true });
  } catch (error) {
    return handleApiError(error);
  }
}
