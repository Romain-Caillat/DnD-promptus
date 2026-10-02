import { NextRequest, NextResponse } from "next/server";
import * as yaml from "js-yaml";
import { db } from "@/lib/db/client";
import { entities } from "@/lib/db/schema";
import { handleApiError, notFound } from "@/lib/api/errors";
import { eq } from "drizzle-orm";

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const [row] = await db.select().from(entities).where(eq(entities.id, id));
    if (!row) notFound("entity", id);

    const stripped = {
      id: row.id,
      type: row.type,
      name: row.name,
      description: row.description ?? undefined,
      imageUrl: row.imageUrl ?? undefined,
      tags: row.tags,
      attributes: row.attributes,
      effects: row.effects,
      visibility: row.visibility,
    };

    const yamlText = yaml.dump(stripped, { lineWidth: 120, noRefs: true });
    const slug = row.name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
    return new NextResponse(yamlText, {
      headers: {
        "content-type": "application/x-yaml",
        "content-disposition": `attachment; filename="${row.type}-${slug}.yaml"`,
      },
    });
  } catch (error) {
    return handleApiError(error);
  }
}
