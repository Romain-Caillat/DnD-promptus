import { NextRequest, NextResponse } from "next/server";
import * as yaml from "js-yaml";
import { db } from "@/lib/db/client";
import { campaigns, entities } from "@/lib/db/schema";
import { handleApiError, notFound } from "@/lib/api/errors";
import { eq, asc } from "drizzle-orm";

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id: campaignId } = await params;
    const [campaign] = await db
      .select()
      .from(campaigns)
      .where(eq(campaigns.id, campaignId));
    if (!campaign) notFound("campaign", campaignId);

    const rows = await db
      .select()
      .from(entities)
      .where(eq(entities.campaignId, campaignId))
      .orderBy(asc(entities.type), asc(entities.name));

    const stripped = rows.map((r) => ({
      id: r.id,
      type: r.type,
      name: r.name,
      description: r.description ?? undefined,
      imageUrl: r.imageUrl ?? undefined,
      tags: r.tags,
      attributes: r.attributes,
      effects: r.effects,
      visibility: r.visibility,
    }));

    const yamlText = yaml.dump(
      { campaign: campaign.name, exportedAt: new Date().toISOString(), entities: stripped },
      { lineWidth: 120, noRefs: true },
    );

    const fileSlug = campaign.name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
    return new NextResponse(yamlText, {
      headers: {
        "content-type": "application/x-yaml",
        "content-disposition": `attachment; filename="promptus-${fileSlug}.yaml"`,
      },
    });
  } catch (error) {
    return handleApiError(error);
  }
}
