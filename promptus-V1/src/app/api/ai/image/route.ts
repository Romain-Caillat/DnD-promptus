import { NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { campaigns, entities } from "@/lib/db/schema";
import { eq, sql } from "drizzle-orm";
import { handleApiError, badRequest, notFound } from "@/lib/api/errors";
import { getImageGenerator, ImageGenerationError } from "@/lib/ai/image-generator";
import { buildEntityImagePrompt } from "@/lib/ai/prompt-template";
import type { StyleGuide } from "@/lib/engine/types";

const BodySchema = z.object({
  entityId: z.string(),
  /** Override prompt — if absent, we build one from style guide + entity. */
  prompt: z.string().min(3).max(2000).optional(),
  width: z.number().int().min(256).max(1536).optional(),
  height: z.number().int().min(256).max(1536).optional(),
  steps: z.number().int().min(1).max(20).optional(),
  seed: z.number().int().optional(),
  /** If true, also persist the new URL to entities.image_url. */
  persist: z.boolean().default(true),
});

// Simple in-memory rate limit (per Node process) — 30 generations / hour.
const WINDOW_MS = 60 * 60 * 1000;
const LIMIT = 30;
const calls: number[] = [];

function rateLimitOk(): boolean {
  const now = Date.now();
  while (calls.length && calls[0] < now - WINDOW_MS) calls.shift();
  if (calls.length >= LIMIT) return false;
  calls.push(now);
  return true;
}

export async function POST(req: NextRequest) {
  try {
    if (!rateLimitOk()) {
      badRequest("Image generation rate limit reached (30/hour). Try again later.");
    }

    const body = await req.json();
    const input = BodySchema.parse(body);

    const [entity] = await db.select().from(entities).where(eq(entities.id, input.entityId));
    if (!entity) notFound("entity", input.entityId);

    const [campaign] = await db
      .select()
      .from(campaigns)
      .where(eq(campaigns.id, entity.campaignId));
    if (!campaign) notFound("campaign", entity.campaignId);

    const styleGuide = (campaign.styleGuide ?? {}) as StyleGuide;
    const prompt =
      input.prompt?.trim() ||
      buildEntityImagePrompt({
        name: entity.name,
        description: entity.description,
        tags: entity.tags ?? [],
        type: entity.type,
        styleGuide,
      });

    const generator = await getImageGenerator();
    const result = await generator.generate(prompt, {
      entityId: entity.id,
      width: input.width,
      height: input.height,
      steps: input.steps,
      seed: input.seed,
    });

    if (input.persist) {
      await db
        .update(entities)
        .set({
          imageUrl: result.url,
          updatedAt: new Date(),
          version: sql`${entities.version} + 1`,
        })
        .where(eq(entities.id, entity.id));
    }

    return NextResponse.json({ result });
  } catch (error) {
    if (error instanceof ImageGenerationError) {
      return NextResponse.json(
        {
          error: {
            code: "image_generation_failed",
            message: error.message,
            details: error.providerMessage,
          },
        },
        { status: error.status ?? 502 },
      );
    }
    return handleApiError(error);
  }
}
