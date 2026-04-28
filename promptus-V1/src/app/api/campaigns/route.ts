import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { CampaignInputSchema } from "@/lib/validation/entity-schemas";
import { handleApiError } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { desc } from "drizzle-orm";

export async function GET() {
  try {
    const rows = await db.select().from(campaigns).orderBy(desc(campaigns.updatedAt));
    return NextResponse.json({ campaigns: rows });
  } catch (error) {
    return handleApiError(error);
  }
}

export async function POST(req: NextRequest) {
  try {
    const body = await req.json();
    const input = CampaignInputSchema.parse(body);
    const id = generateId("camp");
    const [row] = await db
      .insert(campaigns)
      .values({
        id,
        name: input.name,
        description: input.description,
        styleGuide: input.styleGuide ?? {},
        systemTemplate: input.systemTemplate,
      })
      .returning();
    return NextResponse.json({ campaign: row }, { status: 201 });
  } catch (error) {
    return handleApiError(error);
  }
}
