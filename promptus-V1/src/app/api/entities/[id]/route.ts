import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { entities } from "@/lib/db/schema";
import { EntityUpdateSchema } from "@/lib/validation/entity-schemas";
import { handleApiError, notFound } from "@/lib/api/errors";
import { eq, sql } from "drizzle-orm";

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const [row] = await db.select().from(entities).where(eq(entities.id, id));
    if (!row) notFound("entity", id);
    return NextResponse.json({ entity: row });
  } catch (error) {
    return handleApiError(error);
  }
}

export async function PATCH(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const body = await req.json();
    const input = EntityUpdateSchema.parse({ ...body, id });

    const updates: Record<string, unknown> = { updatedAt: new Date() };
    if (input.name !== undefined) updates.name = input.name;
    if (input.description !== undefined) updates.description = input.description;
    if (input.imageUrl !== undefined) updates.imageUrl = input.imageUrl || null;
    if (input.tags !== undefined) updates.tags = input.tags;
    if (input.attributes !== undefined) updates.attributes = input.attributes;
    if (input.effects !== undefined) updates.effects = input.effects;
    if (input.visibility !== undefined) updates.visibility = input.visibility;
    updates.version = sql`${entities.version} + 1`;

    const [row] = await db
      .update(entities)
      .set(updates)
      .where(eq(entities.id, id))
      .returning();
    if (!row) notFound("entity", id);
    return NextResponse.json({ entity: row });
  } catch (error) {
    return handleApiError(error);
  }
}

export async function DELETE(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const [row] = await db.delete(entities).where(eq(entities.id, id)).returning();
    if (!row) notFound("entity", id);
    return NextResponse.json({ deleted: true });
  } catch (error) {
    return handleApiError(error);
  }
}
