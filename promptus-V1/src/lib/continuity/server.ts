import "server-only";
import { and, asc, desc, eq, isNotNull, lt } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { lockCampaign } from "@/lib/db/lock";
import { aiCalls, campaigns, entities, sessions, sessionState, sessionTimeline, type Session } from "@/lib/db/schema";
import { badRequest, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { LlmError, defaultModel, parseJsonObject, type LlmClient } from "@/lib/ai/llm";
import { EMPTY_STORY } from "@/lib/engine/story";
import type { EntityState } from "@/lib/engine/types";
import { normalizeWorld } from "@/lib/engine/world";
import { spentUsd } from "@/lib/generation/server";
import { notifySession } from "@/lib/realtime/notify";
import { RecapAnswerSchema, factsRecap, recapMessages, sessionFacts, type SessionRecap } from "./recap";

/**
 * Termine la session : faits mesurés depuis le lancement, récapitulatif
 * factuel en brouillon, fin du combat et de la musique.
 */
export async function endSession(sessionId: string): Promise<Session> {
  const [found] = await db.select().from(sessions).where(eq(sessions.id, sessionId));
  if (!found) notFound("session", sessionId);
  if (found.endedAt) badRequest("Session déjà terminée");

  const updated = await db.transaction(async (tx) => {
    const campaign = await lockCampaign(tx, found.campaignId);
    const [session] = await tx.select().from(sessions).where(eq(sessions.id, sessionId));
    const ents = await tx.select().from(entities).where(eq(entities.campaignId, campaign.id));
    const states = await tx.select().from(sessionState).where(eq(sessionState.sessionId, sessionId));
    const timeline = await tx
      .select({ description: sessionTimeline.description })
      .from(sessionTimeline)
      .where(eq(sessionTimeline.sessionId, sessionId))
      .orderBy(asc(sessionTimeline.createdAt));
    const world = normalizeWorld(campaign.worldState);
    const endedAt = new Date();

    const facts = sessionFacts({
      story: campaign.story ?? EMPTY_STORY,
      before: normalizeWorld(session.worldAtStart),
      after: world,
      entities: ents.map((e) => ({ id: e.id, name: e.name, type: e.type, visibility: e.visibility })),
      states: states.map((s) => ({ entityId: s.entityId, state: s.currentState as EntityState })),
      timeline: timeline.map((t) => t.description),
      startedAt: session.startedAt,
      endedAt,
    });
    const recap: SessionRecap = {
      facts,
      ...factsRecap(facts),
      source: "facts",
      status: "draft",
      generatedAt: endedAt.toISOString(),
    };

    await tx
      .update(campaigns)
      .set({ worldState: { ...world, music: undefined, turnMovement: undefined, spotlightEntityId: undefined }, updatedAt: new Date() })
      .where(eq(campaigns.id, campaign.id));
    await tx.insert(sessionTimeline).values({ id: generateId("tl"), sessionId, round: session.combatRound, description: "🏁 Fin de la session" });
    const [row] = await tx
      .update(sessions)
      .set({ endedAt, recap, currentPhase: "exploration", combatRound: 0, activeTurnIndex: 0, initiativeOrder: [] })
      .where(eq(sessions.id, sessionId))
      .returning();
    return row;
  });
  await notifySession(sessionId, ["session", "story", "timeline"]);
  return updated;
}

/** Rouvre une session terminée par erreur (le récapitulatif est gardé). */
export async function reopenSession(sessionId: string): Promise<Session> {
  const [row] = await db.update(sessions).set({ endedAt: null }).where(eq(sessions.id, sessionId)).returning();
  if (!row) notFound("session", sessionId);
  await notifySession(sessionId, ["session"]);
  return row;
}

async function setRecap(sessionId: string, patch: (r: SessionRecap) => SessionRecap): Promise<Session> {
  return db.transaction(async (tx) => {
    const [s] = await tx.select().from(sessions).where(eq(sessions.id, sessionId)).for("update");
    if (!s) notFound("session", sessionId);
    if (!s.recap) badRequest("Terminez d’abord la session");
    const [row] = await tx.update(sessions).set({ recap: patch(s.recap) }).where(eq(sessions.id, sessionId)).returning();
    return row;
  });
}

/** Le MJ corrige les textes, et publie le « Précédemment… » aux joueurs. */
export async function updateRecap(
  sessionId: string,
  input: { gm?: string; players?: string; publish?: boolean },
): Promise<Session> {
  const row = await setRecap(sessionId, (r) => ({
    ...r,
    gm: input.gm ?? r.gm,
    players: input.players ?? r.players,
    ...(input.publish === true ? { status: "published" as const, publishedAt: new Date().toISOString() } : {}),
    ...(input.publish === false ? { status: "draft" as const, publishedAt: undefined } : {}),
  }));
  await notifySession(sessionId, ["session"]);
  return row;
}

/** Marque la génération IA en cours (le résultat arrive en arrière-plan). */
export async function markRecapRunning(sessionId: string): Promise<void> {
  await setRecap(sessionId, (r) => ({ ...r, llm: { status: "running" } }));
}

/** Réécrit les récapitulatifs avec le LLM (journal + faits), dans le budget. */
export async function generateRecapWithLlm(sessionId: string, llm: LlmClient): Promise<void> {
  const [session] = await db.select().from(sessions).where(eq(sessions.id, sessionId));
  if (!session?.recap) return;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, session.campaignId));
  if (!campaign) return;
  const fail = (error: string) => setRecap(sessionId, (r) => ({ ...r, llm: { status: "failed", error } }));

  const budget = campaign.aiSettings.budgetUsd;
  if (budget !== undefined && (await spentUsd(campaign.id)) >= budget) {
    await fail("Budget IA épuisé");
    return;
  }
  const timeline = await db
    .select({ description: sessionTimeline.description, isPublic: sessionTimeline.isPublic })
    .from(sessionTimeline)
    .where(eq(sessionTimeline.sessionId, sessionId))
    .orderBy(asc(sessionTimeline.createdAt));
  const [previous] = await previousRecaps(campaign.id, session.startedAt, 1);
  const model = campaign.aiSettings.model || defaultModel();
  const callId = generateId("ai");
  await db.insert(aiCalls).values({ id: callId, campaignId: campaign.id, sessionId, kind: "recap", status: "running", model, input: {} });

  let cost = 0;
  try {
    const res = await llm.complete({
      messages: recapMessages({
        campaignName: campaign.name,
        story: campaign.story ?? EMPTY_STORY,
        facts: session.recap.facts,
        publicTimeline: timeline.filter((t) => t.isPublic).map((t) => t.description),
        gmTimeline: timeline.map((t) => t.description),
        previousGmRecap: previous?.recap?.gm,
      }),
      model,
      json: true,
      temperature: 0.7,
      maxTokens: 3000,
    });
    cost = res.usage.costUsd;
    const answer = RecapAnswerSchema.parse(parseJsonObject(res.text));
    await setRecap(sessionId, (r) => ({
      ...r,
      gm: answer.gm.trim(),
      players: answer.players.trim(),
      source: "llm",
      // Nouveau texte : le MJ le relit avant de le publier.
      status: "draft",
      publishedAt: undefined,
      generatedAt: new Date().toISOString(),
      llm: undefined,
    }));
    await db.update(aiCalls).set({ status: "succeeded", output: answer, costUsd: cost, updatedAt: new Date() }).where(eq(aiCalls.id, callId));
  } catch (e) {
    const message = e instanceof LlmError ? `${e.message}${e.details ? ` — ${e.details.slice(0, 200)}` : ""}` : e instanceof Error ? e.message : String(e);
    await db.update(aiCalls).set({ status: "failed", error: message, costUsd: cost, updatedAt: new Date() }).where(eq(aiCalls.id, callId));
    await fail(message);
  }
  await notifySession(sessionId, ["session"]);
}

/** Sessions terminées d'une campagne avant une date, les plus récentes d'abord. */
export async function previousRecaps(campaignId: string, before: Date, limit: number): Promise<Session[]> {
  return db
    .select()
    .from(sessions)
    .where(and(eq(sessions.campaignId, campaignId), isNotNull(sessions.recap), lt(sessions.startedAt, before)))
    .orderBy(desc(sessions.startedAt))
    .limit(limit);
}
