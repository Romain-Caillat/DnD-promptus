import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { campaigns, entities, sessions, sessionState, sessionTimeline } from "@/lib/db/schema";
import { handleApiError, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { DND5E_RULESET, rollInitiative } from "@/lib/engine/ruleset";
import { EMPTY_STORY } from "@/lib/engine/story";
import type { InitiativeEntry } from "@/lib/engine/types";
import { PARTY_TOKEN, defaultMapId, mapTokens, normalizeWorld } from "@/lib/engine/world";
import { deriveStateFromEntity } from "@/lib/session/derive-state";
import { notifySession } from "@/lib/realtime/notify";

const BodySchema = z.object({ action: z.enum(["start", "end"]) });

/**
 * Début de combat : les créatures posées sur la carte affichée rejoignent la
 * session, chacun lance l'initiative, round 1. Fin : retour à l'exploration.
 */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const { action } = BodySchema.parse(await req.json());
    const [session] = await db.select().from(sessions).where(eq(sessions.id, id));
    if (!session) notFound("session", id);
    const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, session.campaignId));
    const world = normalizeWorld(campaign?.worldState);

    if (action === "end") {
      await db.transaction(async (tx) => {
        await tx
          .update(sessions)
          .set({ currentPhase: "exploration", combatRound: 0, activeTurnIndex: 0, initiativeOrder: [] })
          .where(eq(sessions.id, id));
        await tx.update(campaigns).set({ worldState: { ...world, turnMovement: undefined } }).where(eq(campaigns.id, session.campaignId));
        await tx.insert(sessionTimeline).values({ id: generateId("tl"), sessionId: id, round: session.combatRound, description: "🏳️ Fin du combat" });
      });
      await notifySession(id, ["session", "story", "timeline"]);
      return NextResponse.json({ ok: true });
    }

    const story = campaign?.story ?? EMPTY_STORY;
    const ruleset = campaign?.ruleset ?? DND5E_RULESET;
    const map = story.maps.find((m) => m.id === defaultMapId(story, world));
    const onMap = map?.level === "local" ? mapTokens(map, world).map((t) => t.entityId).filter((e) => e !== PARTY_TOKEN) : [];

    const ents = await db.select().from(entities).where(eq(entities.campaignId, session.campaignId));
    const entById = new Map(ents.map((e) => [e.id, e]));
    const states = await db.select().from(sessionState).where(eq(sessionState.sessionId, id));
    const present = new Set(states.map((s) => s.entityId));
    const added = onMap.filter((e) => !present.has(e) && entById.has(e));

    // Combattants : les créatures sur la carte, sinon tous les participants.
    const fighters = onMap.length ? onMap.filter((e) => entById.has(e)) : states.map((s) => s.entityId);
    const order: InitiativeEntry[] = fighters
      .map((entityId) => {
        const ent = entById.get(entityId);
        const a = (ent?.attributes ?? {}) as Record<string, unknown>;
        return {
          entityId,
          initiative: rollInitiative(ruleset, {
            abilityScores: a.abilityScores as Record<string, number> | undefined,
            bonus: typeof a.initiativeBonus === "number" ? a.initiativeBonus : undefined,
          }),
          isPlayer: ent?.type === "character",
        };
      })
      .sort((a, b) => b.initiative - a.initiative);

    await db.transaction(async (tx) => {
      if (added.length) {
        await tx.insert(sessionState).values(
          added.map((e) => ({ id: generateId("st"), sessionId: id, entityId: e, currentState: deriveStateFromEntity(entById.get(e)!) })),
        );
      }
      await tx
        .update(sessions)
        .set({ currentPhase: "combat", combatRound: 1, activeTurnIndex: 0, initiativeOrder: order })
        .where(eq(sessions.id, id));
      await tx.update(campaigns).set({ worldState: { ...world, turnMovement: undefined } }).where(eq(campaigns.id, session.campaignId));
      await tx.insert(sessionTimeline).values({ id: generateId("tl"), sessionId: id, round: 1, description: "⚔️ Le combat commence ! Initiative lancée." });
    });
    await notifySession(id, ["session", "story", "timeline"]);
    return NextResponse.json({ added, order });
  } catch (error) {
    return handleApiError(error);
  }
}
