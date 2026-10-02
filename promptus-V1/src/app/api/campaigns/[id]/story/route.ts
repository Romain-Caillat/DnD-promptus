import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { ApiError, handleApiError, notFound } from "@/lib/api/errors";
import { StorySchema } from "@/lib/validation/story-schema";
import { checkStory, getCampaignStory } from "@/lib/story/server";

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const story = await getCampaignStory(id);
    if (!story) notFound("campaign", id);
    return NextResponse.json({ story, issues: await checkStory(id, story) });
  } catch (error) {
    return handleApiError(error);
  }
}

// PUT { story } : remplace l'histoire. Refusé s'il reste des erreurs
// (références cassées) ; les avertissements sont renvoyés sans bloquer.
export async function PUT(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const body = (await req.json()) as { story?: unknown };
    const story = StorySchema.parse(body.story);
    const issues = await checkStory(id, story);
    const errors = issues.filter((i) => i.severity === "error");
    if (errors.length > 0) {
      throw new ApiError(
        422,
        "story_invalid",
        `${errors.length} erreur(s) à corriger avant d’enregistrer`,
        errors.map((e) => ({ path: e.path.split("."), message: e.message })),
      );
    }
    const [row] = await db
      .update(campaigns)
      .set({ story, updatedAt: new Date() })
      .where(eq(campaigns.id, id))
      .returning({ id: campaigns.id });
    if (!row) notFound("campaign", id);
    return NextResponse.json({ story, issues });
  } catch (error) {
    return handleApiError(error);
  }
}
