import "server-only";
import { and, eq, isNull, sql } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { sessions } from "@/lib/db/schema";
import { REALTIME_CHANNEL, type RealtimeKind } from "./protocol";

/**
 * Prévient les clients connectés à une session qu'une partie de son état a
 * changé (ils relisent ensuite via l'API). Ne lève jamais : le temps réel est
 * un confort, pas une condition de succès de la requête.
 */
export async function notifySession(sessionId: string, kinds: RealtimeKind[]): Promise<void> {
  try {
    await db.execute(sql`select pg_notify(${REALTIME_CHANNEL}, ${JSON.stringify({ sessionId, kinds })})`);
  } catch (e) {
    console.error("[realtime] notify failed", e);
  }
}

/** Même chose pour toutes les sessions en cours d'une campagne (état du monde partagé). */
export async function notifyCampaign(campaignId: string, kinds: RealtimeKind[]): Promise<void> {
  const rows = await db
    .select({ id: sessions.id })
    .from(sessions)
    .where(and(eq(sessions.campaignId, campaignId), isNull(sessions.endedAt)));
  await Promise.all(rows.map((r) => notifySession(r.id, kinds)));
}
