import { NextRequest, NextResponse } from "next/server";
import { handleApiError } from "@/lib/api/errors";
import { applyDraft, getJob } from "@/lib/generation/server";
import { notifyCampaign } from "@/lib/realtime/notify";

export async function POST(
  _req: NextRequest,
  { params }: { params: Promise<{ jobId: string }> },
) {
  try {
    const { jobId } = await params;
    const { entities } = await applyDraft(jobId);
    const job = await getJob(jobId);
    await notifyCampaign(job.campaignId, ["story", "session"]);
    return NextResponse.json({ entities });
  } catch (error) {
    return handleApiError(error);
  }
}
