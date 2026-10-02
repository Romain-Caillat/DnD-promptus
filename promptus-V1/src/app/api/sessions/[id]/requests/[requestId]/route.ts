import { NextRequest, NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { campaigns, entities, playerRequests, sessionPlayers, sessionState, sessions, sessionTimeline } from "@/lib/db/schema";
import { badRequest, handleApiError, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { resolveSkillCheck } from "@/lib/engine/skill-check";
import type { EntityState } from "@/lib/engine/types";
import { notifySession } from "@/lib/realtime/notify";
import { runAction } from "@/lib/session/run-action";

const BodySchema = z.discriminatedUnion("decision", [
  z.object({ decision: z.literal("roll"), dc: z.number().int().min(-50).max(200) }),
  /** Attaque demandée par le joueur : résolue par le moteur (portée, CA, dégâts). */
  z.object({ decision: z.literal("attack"), force: z.boolean().optional() }),
  z.object({ decision: z.literal("accept"), result: z.string().trim().max(500).optional() }),
  z.object({ decision: z.literal("reject"), result: z.string().trim().max(500).optional() }),
]);

/** Le MJ tranche une demande : faire lancer le test, valider ou refuser. */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string; requestId: string }> },
) {
  try {
    const { id, requestId } = await params;
    const body = BodySchema.parse(await req.json());
    const [request] = await db
      .select()
      .from(playerRequests)
      .where(and(eq(playerRequests.id, requestId), eq(playerRequests.sessionId, id)));
    if (!request) notFound("player_request", requestId);
    if (request.status !== "pending") badRequest("Demande déjà traitée");
    const [player] = await db.select().from(sessionPlayers).where(eq(sessionPlayers.id, request.playerId));
    const [session] = await db.select().from(sessions).where(eq(sessions.id, id));

    let result: string;
    let status: "resolved" | "rejected" = "resolved";
    if (body.decision === "roll") {
      const [campaign] = await db.select({ ruleset: campaigns.ruleset }).from(campaigns).where(eq(campaigns.id, session.campaignId));
      const ruleset = campaign?.ruleset ?? DND5E_RULESET;
      const action = ruleset.actions.find((a) => a.id === request.actionId);
      if (!action?.skill && !action?.ability) badRequest("Cette action n’a pas de test associé dans les règles");
      if (!player?.characterEntityId) badRequest("Pas de personnage associé");
      const [character] = await db.select().from(entities).where(eq(entities.id, player.characterEntityId));
      const [state] = await db
        .select()
        .from(sessionState)
        .where(and(eq(sessionState.sessionId, id), eq(sessionState.entityId, character.id)));
      const check = resolveSkillCheck({
        ruleset,
        name: character.name,
        attributes: character.attributes,
        conditions: ((state?.currentState as EntityState | undefined)?.conditions) ?? [],
        skill: action.skill,
        ability: action.ability,
        dc: body.dc,
      });
      result = check.description;
    } else if (body.decision === "attack") {
      if (!player?.characterEntityId || !request.attackId || !request.targetIds.length) badRequest("Ce n’est pas une demande d’attaque");
      const { records } = await runAction(id, {
        kind: "attack",
        attackerId: player.characterEntityId,
        targetIds: request.targetIds,
        attackId: request.attackId,
        force: body.force,
      });
      result = records.map((r) => r.description).join(" · ");
    } else if (body.decision === "accept") {
      result = body.result || "Validé par le MJ";
    } else {
      status = "rejected";
      result = body.result || "Refusé par le MJ";
    }

    await db.transaction(async (tx) => {
      await tx.update(playerRequests).set({ status, result, resolvedAt: new Date() }).where(eq(playerRequests.id, requestId));
      // Une attaque est déjà au journal (résolue par le moteur).
      if (body.decision !== "attack") {
        await tx.insert(sessionTimeline).values({
          id: generateId("tl"),
          sessionId: id,
          round: session.combatRound,
          description: body.decision === "roll" ? result : `${player?.name ?? "Joueur"} — ${request.label} : ${result}`,
        });
      }
    });
    await notifySession(id, ["requests", "timeline"]);
    return NextResponse.json({ status, result });
  } catch (error) {
    return handleApiError(error);
  }
}
