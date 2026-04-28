import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { sessionTimeline } from "@/lib/db/schema";
import { handleApiError } from "@/lib/api/errors";
import { eq, desc } from "drizzle-orm";

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const rows = await db
      .select()
      .from(sessionTimeline)
      .where(eq(sessionTimeline.sessionId, id))
      .orderBy(desc(sessionTimeline.createdAt))
      .limit(200);
    return NextResponse.json({ timeline: rows });
  } catch (error) {
    return handleApiError(error);
  }
}
