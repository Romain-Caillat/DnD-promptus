import { NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { ApiError, handleApiError } from "@/lib/api/errors";
import { getLlmClient } from "@/lib/ai/llm";
import { COPILOT_KINDS } from "@/lib/copilot/copilot";
import { askCopilot, listCopilotCalls } from "@/lib/copilot/server";

export const maxDuration = 120;

const BodySchema = z.object({
  kind: z.enum(COPILOT_KINDS),
  npcId: z.string().optional(),
  prompt: z.string().trim().max(1000).optional(),
});

/** Historique des propositions du co-MJ pour la session. */
export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    return NextResponse.json({ configured: getLlmClient() !== null, calls: await listCopilotCalls(id) });
  } catch (error) {
    return handleApiError(error);
  }
}

/** Demande au co-MJ : narration, répliques, suggestions — à valider par le MJ. */
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const body = BodySchema.parse(await req.json());
    const llm = getLlmClient();
    if (!llm) throw new ApiError(503, "llm_not_configured", "Co-MJ indisponible : ajoutez OPENROUTER_API_KEY dans la configuration.");
    return NextResponse.json({ call: await askCopilot(id, body, llm) });
  } catch (error) {
    return handleApiError(error);
  }
}
