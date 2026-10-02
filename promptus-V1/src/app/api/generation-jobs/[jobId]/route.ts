import { NextRequest, NextResponse } from "next/server";
import { handleApiError } from "@/lib/api/errors";
import { StorySchema } from "@/lib/validation/story-schema";
import { getJob, toJobView, updateDraftStory } from "@/lib/generation/server";

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ jobId: string }> },
) {
  try {
    const { jobId } = await params;
    return NextResponse.json({ job: toJobView(await getJob(jobId)) });
  } catch (error) {
    return handleApiError(error);
  }
}

// PUT { story } : le MJ corrige le brouillon avant de l'appliquer.
export async function PUT(
  req: NextRequest,
  { params }: { params: Promise<{ jobId: string }> },
) {
  try {
    const { jobId } = await params;
    const body = (await req.json()) as { story?: unknown };
    return NextResponse.json({ job: await updateDraftStory(jobId, StorySchema.parse(body.story)) });
  } catch (error) {
    return handleApiError(error);
  }
}
