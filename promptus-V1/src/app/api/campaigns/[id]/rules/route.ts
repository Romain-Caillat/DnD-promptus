import { NextRequest, NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { handleApiError, notFound } from "@/lib/api/errors";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { RulesetSchema } from "@/lib/validation/ruleset-schema";

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const [row] = await db
      .select({ ruleset: campaigns.ruleset })
      .from(campaigns)
      .where(eq(campaigns.id, id));
    if (!row) notFound("campaign", id);
    return NextResponse.json({
      ruleset: row.ruleset ?? DND5E_RULESET,
      isPreset: row.ruleset === null,
    });
  } catch (error) {
    return handleApiError(error);
  }
}

// PUT { ruleset } remplace les règles ; PUT { reset: true } revient au préréglage.
export async function PUT(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const body = (await req.json()) as { ruleset?: unknown; reset?: boolean };
    const ruleset = body.reset ? null : RulesetSchema.parse(body.ruleset);
    const [row] = await db
      .update(campaigns)
      .set({ ruleset, updatedAt: new Date() })
      .where(eq(campaigns.id, id))
      .returning({ ruleset: campaigns.ruleset });
    if (!row) notFound("campaign", id);
    return NextResponse.json({
      ruleset: row.ruleset ?? DND5E_RULESET,
      isPreset: row.ruleset === null,
    });
  } catch (error) {
    return handleApiError(error);
  }
}
