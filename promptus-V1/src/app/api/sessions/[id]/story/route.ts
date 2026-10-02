import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns, sessions } from "@/lib/db/schema";
import { handleApiError, notFound } from "@/lib/api/errors";
import { EMPTY_STORY } from "@/lib/engine/story";
import { normalizeWorld } from "@/lib/engine/world";

/** Scénario et état du monde de la campagne de cette session (vue MJ). */
export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const [session] = await db.select().from(sessions).where(eq(sessions.id, id));
    if (!session) notFound("session", id);
    const [campaign] = await db
      .select({ story: campaigns.story, worldState: campaigns.worldState })
      .from(campaigns)
      .where(eq(campaigns.id, session.campaignId));
    if (!campaign) notFound("campaign", session.campaignId);
    return NextResponse.json({
      story: campaign.story ?? EMPTY_STORY,
      world: normalizeWorld(campaign.worldState),
      serverTime: Date.now(),
    });
  } catch (error) {
    return handleApiError(error);
  }
}
