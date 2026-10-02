import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { aiCalls, entities } from "@/lib/db/schema";
import { ApiError, handleApiError, notFound } from "@/lib/api/errors";
import { queueMedia, runMediaCalls } from "@/lib/media/server";

export const maxDuration = 300;

const BodySchema = z.object({
  entityId: z.string(),
  /** Prompt du MJ ; sinon déduit de la fiche et du guide de style. */
  prompt: z.string().min(3).max(4000).optional(),
});

/** Image d'une fiche, générée tout de suite (éditeur de fiche). */
export async function POST(req: NextRequest) {
  try {
    const started = Date.now();
    const input = BodySchema.parse(await req.json());
    const [entity] = await db.select().from(entities).where(eq(entities.id, input.entityId));
    if (!entity) notFound("entity", input.entityId);

    const { callIds } = await queueMedia(entity.campaignId, [{ target: { type: "entity", id: entity.id }, prompt: input.prompt }], {
      regenerate: true,
    });
    if (!callIds.length) throw new ApiError(409, "media_running", "Une image est déjà en cours pour cette fiche");
    await runMediaCalls(entity.campaignId, callIds);
    const [call] = await db.select().from(aiCalls).where(eq(aiCalls.id, callIds[0]));
    if (call?.status !== "succeeded") throw new ApiError(502, "image_generation_failed", call?.error ?? "Échec de la génération");
    return NextResponse.json({
      result: { url: String(call.output?.url), provider: call.model, latencyMs: Date.now() - started, costUsd: call.costUsd },
    });
  } catch (error) {
    return handleApiError(error);
  }
}
