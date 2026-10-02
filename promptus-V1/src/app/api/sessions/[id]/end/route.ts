import { NextRequest, NextResponse, after } from "next/server";
import { z } from "zod";
import { handleApiError } from "@/lib/api/errors";
import { getLlmClient } from "@/lib/ai/llm";
import { endSession, generateRecapWithLlm, markRecapRunning, reopenSession } from "@/lib/continuity/server";

export const maxDuration = 300;

const BodySchema = z.object({ reopen: z.boolean().optional() });

/** Fin de session : récapitulatif factuel tout de suite, réécrit par l'IA ensuite. */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const { reopen } = BodySchema.parse(await req.json().catch(() => ({})));
    if (reopen) return NextResponse.json({ session: await reopenSession(id) });
    const session = await endSession(id);
    const llm = getLlmClient();
    if (llm) {
      await markRecapRunning(id);
      after(() => generateRecapWithLlm(id, llm));
    }
    return NextResponse.json({ session });
  } catch (error) {
    return handleApiError(error);
  }
}
