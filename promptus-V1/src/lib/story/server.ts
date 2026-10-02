import "server-only";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns, entities } from "@/lib/db/schema";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { EMPTY_STORY, type CampaignStory } from "@/lib/engine/story";
import { validateStory, type StoryIssue } from "@/lib/engine/story-validator";

/** Valide une histoire contre les fiches et les règles de la campagne. */
export async function checkStory(campaignId: string, story: CampaignStory): Promise<StoryIssue[]> {
  const [campaign] = await db
    .select({ ruleset: campaigns.ruleset })
    .from(campaigns)
    .where(eq(campaigns.id, campaignId));
  const ents = await db
    .select({ id: entities.id })
    .from(entities)
    .where(eq(entities.campaignId, campaignId));
  return validateStory(story, {
    entityIds: new Set(ents.map((e) => e.id)),
    ruleset: campaign?.ruleset ?? DND5E_RULESET,
  });
}

export async function getCampaignStory(campaignId: string): Promise<CampaignStory | null> {
  const [row] = await db
    .select({ story: campaigns.story })
    .from(campaigns)
    .where(eq(campaigns.id, campaignId));
  if (!row) return null;
  return row.story ?? EMPTY_STORY;
}
