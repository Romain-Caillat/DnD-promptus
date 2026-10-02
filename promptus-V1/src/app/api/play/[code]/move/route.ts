import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { campaigns, entities, sessionTimeline } from "@/lib/db/schema";
import { badRequest, handleApiError } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { speedInCells } from "@/lib/engine/grid";
import { MapOpError, applyMapOp, planPlayerMove } from "@/lib/engine/map-ops";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { EMPTY_STORY } from "@/lib/engine/story";
import { defaultMapId, normalizeWorld } from "@/lib/engine/world";
import { turnKey } from "@/lib/play/projection";
import { requirePlayer, sessionByInvite } from "@/lib/play/server";
import { notifySession } from "@/lib/realtime/notify";

const BodySchema = z.object({ x: z.number().int(), y: z.number().int() });

/** Le joueur déplace son pion sur la carte affichée, selon les règles. */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ code: string }> },
) {
  try {
    const { code } = await params;
    const session = await sessionByInvite(code);
    const player = await requirePlayer(req, session);
    if (!player.characterEntityId) badRequest("Un spectateur ne peut pas se déplacer");
    const target = BodySchema.parse(await req.json());

    const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, session.campaignId));
    const [me] = await db.select().from(entities).where(eq(entities.id, player.characterEntityId));
    const story = campaign?.story ?? EMPTY_STORY;
    const ruleset = campaign?.ruleset ?? DND5E_RULESET;
    const world = normalizeWorld(campaign?.worldState);
    const map = story.maps.find((m) => m.id === defaultMapId(story, world));
    if (!map) badRequest("Aucune carte affichée");

    const inCombat = session.currentPhase === "combat" && session.combatRound > 0 && session.initiativeOrder.length > 0;
    const isMyTurn = inCombat && session.initiativeOrder[session.activeTurnIndex]?.entityId === me.id;
    const key = turnKey(session.combatRound, session.activeTurnIndex);
    const used = inCombat && world.turnMovement?.turnKey === key ? (world.turnMovement.used[me.id] ?? 0) : 0;

    let next;
    let cost: number;
    try {
      cost = planPlayerMove({
        map,
        world,
        entityId: me.id,
        target,
        budgetCells: speedInCells(me.attributes, ruleset.movement.local.cellMeters, ruleset.movement.local.defaultSpeedCells),
        usedCells: used,
        diagonal: ruleset.movement.local.diagonal,
        inCombat,
        isMyTurn,
      });
      next = applyMapOp(world, story, { op: "move_token", mapId: map.id, entityId: me.id, x: target.x, y: target.y }, new Set([me.id]));
    } catch (e) {
      if (e instanceof MapOpError) badRequest(e.message);
      throw e;
    }
    // Le déplacement se cumule pendant un même tour de combat.
    if (inCombat) {
      const usedMap = next.turnMovement?.turnKey === key ? next.turnMovement.used : {};
      next.turnMovement = { turnKey: key, used: { ...usedMap, [me.id]: used + cost } };
    }

    await db.transaction(async (tx) => {
      await tx.update(campaigns).set({ worldState: next, updatedAt: new Date() }).where(eq(campaigns.id, session.campaignId));
      await tx.insert(sessionTimeline).values({
        id: generateId("tl"),
        sessionId: session.id,
        round: session.combatRound,
        description: `📍 ${me.name} se déplace de ${cost} case(s)`,
      });
    });
    await notifySession(session.id, ["story", "timeline"]);
    return NextResponse.json({ cost });
  } catch (error) {
    return handleApiError(error);
  }
}
