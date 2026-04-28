import { NextRequest, NextResponse } from "next/server";
import yaml from "js-yaml";
import { z } from "zod";
import { db } from "@/lib/db/client";
import { campaigns, entities } from "@/lib/db/schema";
import { EntityInputSchema } from "@/lib/validation/entity-schemas";
import { handleApiError, badRequest, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { eq } from "drizzle-orm";

// Body can be either:
// - { yaml: "<yaml string>" } — multi-document or single doc with `entities: [...]`
// - { entities: [...] } — direct JSON array
const ImportBodySchema = z.union([
  z.object({ yaml: z.string().min(1) }),
  z.object({
    entities: z.array(EntityInputSchema.omit({ campaignId: true })),
  }),
]);

export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id: campaignId } = await params;
    const [campaign] = await db
      .select()
      .from(campaigns)
      .where(eq(campaigns.id, campaignId));
    if (!campaign) notFound("campaign", campaignId);

    const body = await req.json();
    const parsed = ImportBodySchema.parse(body);

    let rawList: unknown[];
    if ("yaml" in parsed) {
      const docs = yaml.loadAll(parsed.yaml).filter((d) => d != null);
      // Each doc may be either a single entity or { entities: [...] }
      rawList = docs.flatMap((doc) => {
        if (Array.isArray(doc)) return doc;
        if (
          typeof doc === "object" &&
          doc !== null &&
          "entities" in doc &&
          Array.isArray((doc as { entities?: unknown }).entities)
        ) {
          return (doc as { entities: unknown[] }).entities;
        }
        return [doc];
      });
    } else {
      rawList = parsed.entities;
    }

    if (rawList.length === 0) {
      badRequest("No entities found in payload");
    }

    // Validate each entity individually with EntityInputSchema (without campaignId)
    const inputSchema = EntityInputSchema.omit({ campaignId: true });
    const validated = rawList.map((raw, idx) => {
      const r = inputSchema.safeParse(raw);
      if (!r.success) {
        badRequest(`Entity #${idx + 1} invalid`, r.error.issues);
      }
      return r.data!;
    });

    const inserted = [];
    for (const ent of validated) {
      const id = ent.id ?? generateId(`ent_${ent.type}`);
      const [row] = await db
        .insert(entities)
        .values({
          id,
          campaignId,
          type: ent.type,
          name: ent.name,
          description: ent.description ?? null,
          imageUrl: ent.imageUrl || null,
          tags: ent.tags,
          attributes: ent.attributes,
          effects: ent.effects,
          visibility: ent.visibility,
          version: 1,
        })
        .returning();
      inserted.push(row);
    }

    return NextResponse.json(
      { imported: inserted.length, entities: inserted },
      { status: 201 },
    );
  } catch (error) {
    return handleApiError(error);
  }
}
