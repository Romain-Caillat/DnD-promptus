import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { handleApiError, notFound } from "@/lib/api/errors";

const BodySchema = z.object({
  model: z.string().max(200).optional(),
  imageModel: z.string().max(200).optional(),
  videoModel: z.string().max(200).optional(),
  budgetUsd: z.number().nonnegative().max(10_000).optional(),
});

export async function PUT(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const input = BodySchema.parse(await req.json());
    const aiSettings = {
      ...(input.model?.trim() ? { model: input.model.trim() } : {}),
      ...(input.imageModel?.trim() ? { imageModel: input.imageModel.trim() } : {}),
      ...(input.videoModel?.trim() ? { videoModel: input.videoModel.trim() } : {}),
      ...(input.budgetUsd !== undefined ? { budgetUsd: input.budgetUsd } : {}),
    };
    const [row] = await db
      .update(campaigns)
      .set({ aiSettings, updatedAt: new Date() })
      .where(eq(campaigns.id, id))
      .returning({ aiSettings: campaigns.aiSettings });
    if (!row) notFound("campaign", id);
    return NextResponse.json(row);
  } catch (error) {
    return handleApiError(error);
  }
}
