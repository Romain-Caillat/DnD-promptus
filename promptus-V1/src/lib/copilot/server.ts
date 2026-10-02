import "server-only";
import { and, desc, eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import {
  aiCalls,
  campaigns,
  entities,
  playerRequests,
  sessionPlayers,
  sessionState,
  sessions,
  sessionTimeline,
  type AiCallRow,
} from "@/lib/db/schema";
import { ApiError, badRequest, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { LlmError, defaultModel, parseJsonObject, type LlmClient } from "@/lib/ai/llm";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { EMPTY_STORY } from "@/lib/engine/story";
import type { EntityState } from "@/lib/engine/types";
import { normalizeWorld } from "@/lib/engine/world";
import { spentUsd } from "@/lib/generation/server";
import { copilotMessages, sanitizeAnswer, type CopilotAnswer, type CopilotRequest } from "./copilot";

export interface CopilotCallView {
  id: string;
  request: CopilotRequest;
  status: "running" | "succeeded" | "failed";
  answer: CopilotAnswer | null;
  error: string | null;
  costUsd: number;
  createdAt: string;
}

export function toCopilotView(row: AiCallRow): CopilotCallView {
  return {
    id: row.id,
    request: row.input as unknown as CopilotRequest,
    status: row.status,
    answer: (row.output as unknown as CopilotAnswer | null) ?? null,
    error: row.error,
    costUsd: row.costUsd,
    createdAt: row.createdAt.toISOString(),
  };
}

export async function listCopilotCalls(sessionId: string): Promise<CopilotCallView[]> {
  const rows = await db
    .select()
    .from(aiCalls)
    .where(and(eq(aiCalls.sessionId, sessionId), eq(aiCalls.kind, "copilot")))
    .orderBy(desc(aiCalls.createdAt))
    .limit(20);
  return rows.map(toCopilotView);
}

/** Demande au co-MJ, dans le contexte de la session et du budget de la campagne. */
export async function askCopilot(sessionId: string, req: CopilotRequest, llm: LlmClient): Promise<CopilotCallView> {
  const [session] = await db.select().from(sessions).where(eq(sessions.id, sessionId));
  if (!session) notFound("session", sessionId);
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, session.campaignId));
  if (!campaign) notFound("campaign", session.campaignId);

  const budget = campaign.aiSettings.budgetUsd;
  if (budget !== undefined && (await spentUsd(campaign.id)) >= budget) {
    badRequest(`Budget IA épuisé (${budget.toFixed(2)} $). Augmentez-le dans les paramètres de la campagne.`);
  }

  const ents = await db.select().from(entities).where(eq(entities.campaignId, campaign.id));
  if (req.kind === "npc" && req.npcId && !ents.some((e) => e.id === req.npcId)) badRequest("PNJ inconnu");
  const states = await db.select().from(sessionState).where(eq(sessionState.sessionId, sessionId));
  const timeline = await db
    .select({ description: sessionTimeline.description })
    .from(sessionTimeline)
    .where(eq(sessionTimeline.sessionId, sessionId))
    .orderBy(desc(sessionTimeline.createdAt))
    .limit(15);
  const pending = await db
    .select({ label: playerRequests.label, note: playerRequests.note, player: sessionPlayers.name })
    .from(playerRequests)
    .innerJoin(sessionPlayers, eq(sessionPlayers.id, playerRequests.playerId))
    .where(and(eq(playerRequests.sessionId, sessionId), eq(playerRequests.status, "pending")));

  const story = campaign.story ?? EMPTY_STORY;
  const entityList = ents.map((e) => ({ id: e.id, name: e.name, type: e.type, description: e.description, attributes: e.attributes }));
  const messages = copilotMessages(req, {
    campaignName: campaign.name,
    story,
    world: normalizeWorld(campaign.worldState),
    ruleset: campaign.ruleset ?? DND5E_RULESET,
    entities: entityList,
    session: {
      phase: session.currentPhase,
      combatRound: session.combatRound,
      activeTurnIndex: session.activeTurnIndex,
      initiativeOrder: session.initiativeOrder,
    },
    participants: states.map((s) => ({ entityId: s.entityId, state: s.currentState as EntityState })),
    timeline: timeline.map((t) => t.description).reverse(),
    pendingRequests: pending.map((p) => `${p.player} : ${p.label}${p.note ? ` (« ${p.note} »)` : ""}`),
  });

  const model = campaign.aiSettings.model || defaultModel();
  const id = generateId("ai");
  await db.insert(aiCalls).values({
    id,
    campaignId: campaign.id,
    sessionId,
    kind: "copilot",
    status: "running",
    model,
    input: req as unknown as Record<string, unknown>,
  });

  let cost = 0;
  try {
    let answer: CopilotAnswer | undefined;
    // Une relance si le JSON est illisible.
    for (let attempt = 0; attempt < 2 && !answer; attempt++) {
      const res = await llm.complete({
        messages: attempt === 0 ? messages : [...messages, { role: "user", content: "Réponds uniquement avec l’objet JSON demandé." }],
        model,
        json: true,
        temperature: 0.9,
        maxTokens: 2000,
      });
      cost += res.usage.costUsd;
      try {
        answer = sanitizeAnswer(parseJsonObject(res.text), story, entityList);
      } catch (e) {
        if (attempt === 1) throw e;
      }
    }
    const [row] = await db
      .update(aiCalls)
      .set({ status: "succeeded", output: answer as unknown as Record<string, unknown>, costUsd: cost, updatedAt: new Date() })
      .where(eq(aiCalls.id, id))
      .returning();
    return toCopilotView(row);
  } catch (e) {
    const message =
      e instanceof LlmError ? `${e.message}${e.details ? ` — ${e.details.slice(0, 200)}` : ""}` : e instanceof Error ? e.message : String(e);
    await db.update(aiCalls).set({ status: "failed", error: message, costUsd: cost, updatedAt: new Date() }).where(eq(aiCalls.id, id));
    throw new ApiError(502, "copilot_failed", `Le co-MJ n’a pas pu répondre : ${message}`);
  }
}
