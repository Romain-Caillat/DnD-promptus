import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { CampaignUpdateSchema } from "@/lib/validation/entity-schemas";
import { handleApiError, notFound } from "@/lib/api/errors";
import { eq } from "drizzle-orm";

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const [row] = await db.select().from(campaigns).where(eq(campaigns.id, id));
    if (!row) notFound("campaign", id);
    return NextResponse.json({ campaign: row });
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
    const input = CampaignUpdateSchema.parse(body);
    const [row] = await db
      .update(campaigns)
      .set({ ...input, updatedAt: new Date() })
      .where(eq(campaigns.id, id))
      .returning();
    if (!row) notFound("campaign", id);
    return NextResponse.json({ campaign: row });
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
    const [row] = await db.delete(campaigns).where(eq(campaigns.id, id)).returning();
    if (!row) notFound("campaign", id);
    return NextResponse.json({ deleted: true });
  } catch (error) {
    return handleApiError(error);
  }
}
