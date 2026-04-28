import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { entities, campaigns } from "@/lib/db/schema";
import { EntityInputSchema } from "@/lib/validation/entity-schemas";
import { handleApiError, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { and, asc, eq } from "drizzle-orm";
import type { EntityType } from "@/lib/engine/types";

const VALID_TYPES: ReadonlySet<EntityType> = new Set([
  "spell",
  "item",
  "npc",
  "monster",
  "character",
  "location",
  "event",
  "condition",
  "faction",
]);

export async function GET(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id: campaignId } = await params;
    const url = new URL(req.url);
    const typeParam = url.searchParams.get("type") as EntityType | null;

    const [campaign] = await db
      .select()
      .from(campaigns)
      .where(eq(campaigns.id, campaignId));
    if (!campaign) notFound("campaign", campaignId);

    const conditions = typeParam
      ? VALID_TYPES.has(typeParam)
        ? and(eq(entities.campaignId, campaignId), eq(entities.type, typeParam))
        : eq(entities.campaignId, "__never__")
      : eq(entities.campaignId, campaignId);

    const rows = await db
      .select()
      .from(entities)
      .where(conditions)
      .orderBy(asc(entities.name));

    return NextResponse.json({ entities: rows });
  } catch (error) {
    return handleApiError(error);
  }
}

export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id: campaignId } = await params;
    const body = await req.json();
    const input = EntityInputSchema.parse({ ...body, campaignId });

    const [campaign] = await db
      .select()
      .from(campaigns)
      .where(eq(campaigns.id, campaignId));
    if (!campaign) notFound("campaign", campaignId);

    const id = input.id ?? generateId(`ent_${input.type}`);

    const [row] = await db
      .insert(entities)
      .values({
        id,
        campaignId,
        type: input.type,
        name: input.name,
        description: input.description ?? null,
        imageUrl: input.imageUrl || null,
        tags: input.tags,
        attributes: input.attributes,
        effects: input.effects,
        visibility: input.visibility,
        version: 1,
      })
      .returning();

    return NextResponse.json({ entity: row }, { status: 201 });
  } catch (error) {
    return handleApiError(error);
  }
}
