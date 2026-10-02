import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { campaigns, entities, sessions, sessionTimeline } from "@/lib/db/schema";
import { badRequest, handleApiError, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { EMPTY_STORY } from "@/lib/engine/story";
import { PARTY_TOKEN, normalizeWorld } from "@/lib/engine/world";
import { MapOpError, applyMapOp, type MapOp } from "@/lib/engine/map-ops";

const Cells = z.array(z.tuple([z.number().int(), z.number().int()])).max(5000);
const MapOpSchema = z.discriminatedUnion("op", [
  z.object({ op: z.literal("reveal"), mapId: z.string(), cells: Cells }),
  z.object({ op: z.literal("hide"), mapId: z.string(), cells: Cells }),
  z.object({ op: z.literal("reveal_all"), mapId: z.string() }),
  z.object({ op: z.literal("hide_all"), mapId: z.string() }),
  z.object({ op: z.literal("move_token"), mapId: z.string(), entityId: z.string(), x: z.number().int(), y: z.number().int() }),
  z.object({ op: z.literal("remove_token"), mapId: z.string(), entityId: z.string() }),
  z.object({ op: z.literal("set_active"), mapId: z.string().nullable() }),
]);

/** Opérations du MJ sur les cartes (brouillard, pions, carte affichée). */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const op = MapOpSchema.parse(await req.json()) as MapOp;
    const [session] = await db.select().from(sessions).where(eq(sessions.id, id));
    if (!session) notFound("session", id);
    const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, session.campaignId));
    if (!campaign) notFound("campaign", session.campaignId);
    const ents = await db
      .select({ id: entities.id, name: entities.name })
      .from(entities)
      .where(eq(entities.campaignId, campaign.id));
    const story = campaign.story ?? EMPTY_STORY;

    let world;
    try {
      world = applyMapOp(normalizeWorld(campaign.worldState), story, op, new Set(ents.map((e) => e.id)));
    } catch (e) {
      if (e instanceof MapOpError) badRequest(e.message);
      throw e;
    }

    await db.transaction(async (tx) => {
      await tx.update(campaigns).set({ worldState: world, updatedAt: new Date() }).where(eq(campaigns.id, campaign.id));
      if (op.op === "move_token") {
        const name = op.entityId === PARTY_TOKEN ? "Le groupe" : (ents.find((e) => e.id === op.entityId)?.name ?? op.entityId);
        const mapName = story.maps.find((m) => m.id === op.mapId)?.name ?? op.mapId;
        await tx.insert(sessionTimeline).values({
          id: generateId("tl"),
          sessionId: id,
          round: session.combatRound,
          description: `📍 ${name} → (${op.x}, ${op.y}) sur « ${mapName} »`,
        });
      }
    });
    return NextResponse.json({ world });
  } catch (error) {
    return handleApiError(error);
  }
}
