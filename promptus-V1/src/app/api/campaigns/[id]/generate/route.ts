import { NextRequest, NextResponse, after } from "next/server";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { ApiError, handleApiError, notFound } from "@/lib/api/errors";
import { defaultModel, getLlmClient } from "@/lib/ai/llm";
import { GenerationInputSchema } from "@/lib/generation/types";
import { listJobs, runGenerationJob, spentUsd, startGenerationJob, toJobView } from "@/lib/generation/server";

// La génération tourne après la réponse : on lui laisse jusqu'à 15 minutes.
export const maxDuration = 900;

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, id));
    if (!campaign) notFound("campaign", id);
    return NextResponse.json({
      configured: getLlmClient() !== null,
      model: campaign.aiSettings.model || defaultModel(),
      budgetUsd: campaign.aiSettings.budgetUsd ?? null,
      spentUsd: await spentUsd(id),
      hasStory: (campaign.story?.scenes.length ?? 0) > 0,
      jobs: await listJobs(id),
    });
  } catch (error) {
    return handleApiError(error);
  }
}

export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const input = GenerationInputSchema.parse(await req.json());
    const llm = getLlmClient();
    if (!llm) {
      throw new ApiError(
        503,
        "llm_not_configured",
        "Génération indisponible : définissez OPENROUTER_API_KEY dans l’environnement du serveur.",
      );
    }
    const job = await startGenerationJob(id, input);
    after(() => runGenerationJob(job.id, llm));
    return NextResponse.json({ job: toJobView(job) }, { status: 202 });
  } catch (error) {
    return handleApiError(error);
  }
}
