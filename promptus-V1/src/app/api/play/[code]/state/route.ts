import { NextRequest, NextResponse } from "next/server";
import { handleApiError } from "@/lib/api/errors";
import { loadPlayerView, requirePlayer, sessionByInvite } from "@/lib/play/server";

/** Vue joueur filtrée (jeton du joueur requis). */
export async function GET(
  req: NextRequest,
  { params }: { params: Promise<{ code: string }> },
) {
  try {
    const { code } = await params;
    const session = await sessionByInvite(code);
    const player = await requirePlayer(req, session);
    return NextResponse.json(await loadPlayerView(session, player));
  } catch (error) {
    return handleApiError(error);
  }
}
