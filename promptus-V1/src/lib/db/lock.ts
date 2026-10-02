import "server-only";
import { eq } from "drizzle-orm";
import { db } from "./client";
import { campaigns, type Campaign } from "./schema";
import { notFound } from "@/lib/api/errors";

export type Tx = Parameters<Parameters<typeof db.transaction>[0]>[0];

/**
 * Verrouille la campagne jusqu'à la fin de la transaction et la relit.
 * Toute écriture de l'état du monde ou des combattants passe par là : deux
 * requêtes simultanées (MJ et joueurs) s'exécutent l'une après l'autre au
 * lieu d'écraser mutuellement leurs changements.
 */
export async function lockCampaign(tx: Tx, campaignId: string): Promise<Campaign> {
  const [row] = await tx.select().from(campaigns).where(eq(campaigns.id, campaignId)).for("update");
  if (!row) notFound("campaign", campaignId);
  return row;
}
