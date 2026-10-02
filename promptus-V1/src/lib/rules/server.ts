import "server-only";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { DND5E_RULESET, type Ruleset } from "@/lib/engine/ruleset";

/** Ruleset de la campagne, ou le préréglage D&D 5e si le MJ n'en a pas défini. */
export async function getCampaignRuleset(campaignId: string): Promise<Ruleset> {
  const [row] = await db
    .select({ ruleset: campaigns.ruleset })
    .from(campaigns)
    .where(eq(campaigns.id, campaignId));
  return row?.ruleset ?? DND5E_RULESET;
}
