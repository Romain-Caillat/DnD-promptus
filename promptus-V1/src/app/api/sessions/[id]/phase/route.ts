import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { sessions } from "@/lib/db/schema";
import { PhaseUpdateSchema } from "@/lib/validation/session-schemas";
import { handleApiError, notFound } from "@/lib/api/errors";
import { eq } from "drizzle-orm";

export async function PATCH(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const body = await req.json();
    const { phase } = PhaseUpdateSchema.parse(body);
    const [row] = await db
      .update(sessions)
      .set({ currentPhase: phase })
      .where(eq(sessions.id, id))
      .returning();
    if (!row) notFound("session", id);
    return NextResponse.json({ session: row });
  } catch (error) {
    return handleApiError(error);
  }
}
