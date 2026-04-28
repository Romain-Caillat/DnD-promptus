import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db/client";
import { audioAssets } from "@/lib/db/schema";
import { handleApiError } from "@/lib/api/errors";
import { asc, eq } from "drizzle-orm";

export async function GET(req: NextRequest) {
  try {
    const url = new URL(req.url);
    const type = url.searchParams.get("type") as
      | "ambience"
      | "music"
      | "sound"
      | null;

    const rows = type
      ? await db
          .select()
          .from(audioAssets)
          .where(eq(audioAssets.type, type))
          .orderBy(asc(audioAssets.name))
      : await db.select().from(audioAssets).orderBy(asc(audioAssets.type), asc(audioAssets.name));

    return NextResponse.json({ assets: rows });
  } catch (error) {
    return handleApiError(error);
  }
}
