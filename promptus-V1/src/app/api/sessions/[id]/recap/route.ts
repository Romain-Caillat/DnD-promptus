import { NextRequest, NextResponse, after } from "next/server";
import { z } from "zod";
import { ApiError, handleApiError } from "@/lib/api/errors";
import { getLlmClient } from "@/lib/ai/llm";
import { generateRecapWithLlm, markRecapRunning, updateRecap } from "@/lib/continuity/server";

export const maxDuration = 300;

const BodySchema = z.object({
  gm: z.string().max(20_000).optional(),
  players: z.string().max(20_000).optional(),
  /** true : publié aux joueurs ; false : repasse en brouillon. */
  publish: z.boolean().optional(),
});

/** Le MJ corrige et publie les récapitulatifs. */
export async function PUT(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    return NextResponse.json({ session: await updateRecap(id, BodySchema.parse(await req.json())) });
  } catch (error) {
    return handleApiError(error);
  }
}

/** Réécriture par l'IA (en arrière-plan). */
export async function POST(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const llm = getLlmClient();
    if (!llm) throw new ApiError(503, "llm_not_configured", "IA indisponible : ajoutez OPENROUTER_API_KEY.");
    await markRecapRunning(id);
    after(() => generateRecapWithLlm(id, llm));
    return NextResponse.json({ ok: true }, { status: 202 });
  } catch (error) {
    return handleApiError(error);
  }
}
