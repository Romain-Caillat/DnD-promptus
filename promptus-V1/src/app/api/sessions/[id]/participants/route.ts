import { NextRequest, NextResponse } from "next/server";
import { and, eq, inArray } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { entities, sessions, sessionState } from "@/lib/db/schema";
import { badRequest, handleApiError, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { deriveStateFromEntity } from "@/lib/session/derive-state";
import { notifySession } from "@/lib/realtime/notify";

const BodySchema = z.union([
  z.object({ entityIds: z.array(z.string()).min(1) }),
  /** Nouvel exemplaire d'un monstre ou PNJ (« Gobelin 2 »), ajouté à la session. */
  z.object({ copyOf: z.string() }),
]);

/** Ajoute des participants en cours de session (ceux déjà présents sont ignorés). */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const body = BodySchema.parse(await req.json());
    const [session] = await db.select().from(sessions).where(eq(sessions.id, id));
    if (!session) notFound("session", id);
    const entityIds = "copyOf" in body ? [await createCopy(session.campaignId, body.copyOf)] : body.entityIds;

    const ents = await db
      .select()
      .from(entities)
      .where(and(eq(entities.campaignId, session.campaignId), inArray(entities.id, entityIds)));
    if (ents.length !== new Set(entityIds).size) badRequest("Certaines fiches sont introuvables dans cette campagne");

    const existing = await db.select({ entityId: sessionState.entityId }).from(sessionState).where(eq(sessionState.sessionId, id));
    const present = new Set(existing.map((e) => e.entityId));
    const added = ents.filter((e) => !present.has(e.id));

    if (added.length) {
      await db.transaction(async (tx) => {
        await tx.insert(sessionState).values(
          added.map((e) => ({ id: generateId("st"), sessionId: id, entityId: e.id, currentState: deriveStateFromEntity(e) })),
        );
        await tx
          .update(sessions)
          .set({
            initiativeOrder: [
              ...session.initiativeOrder,
              ...added.map((e) => ({ entityId: e.id, initiative: 0, isPlayer: e.type === "character" })),
            ],
          })
          .where(eq(sessions.id, id));
      });
    }
    await notifySession(id, ["session"]);
    return NextResponse.json({ added: added.map((e) => e.id) });
  } catch (error) {
    return handleApiError(error);
  }
}

/** Copie d'une fiche (mêmes caractéristiques), numérotée et marquée copyOf. */
async function createCopy(campaignId: string, baseId: string): Promise<string> {
  const [base] = await db.select().from(entities).where(and(eq(entities.id, baseId), eq(entities.campaignId, campaignId)));
  if (!base) notFound("entity", baseId);
  if (base.type === "character") badRequest("Un personnage joueur ne se duplique pas");
  const rootId = typeof base.attributes.copyOf === "string" ? base.attributes.copyOf : base.id;
  const [root] = rootId === base.id ? [base] : await db.select().from(entities).where(eq(entities.id, rootId));
  const all = await db.select({ attributes: entities.attributes }).from(entities).where(eq(entities.campaignId, campaignId));
  const n = all.filter((e) => e.attributes.copyOf === rootId).length + 2;
  const copyId = generateId(`ent_${base.type}`);
  await db.insert(entities).values({
    id: copyId,
    campaignId,
    type: base.type,
    name: `${(root ?? base).name} ${n}`,
    description: base.description,
    imageUrl: base.imageUrl,
    tags: base.tags,
    attributes: { ...base.attributes, copyOf: rootId },
    effects: base.effects,
    visibility: base.visibility,
  });
  return copyId;
}
