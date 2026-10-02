import { NextRequest, NextResponse } from "next/server";
import { handleApiError } from "@/lib/api/errors";
import { applyDraft } from "@/lib/generation/server";

export async function POST(
  _req: NextRequest,
  { params }: { params: Promise<{ jobId: string }> },
) {
  try {
    const { jobId } = await params;
    const { entities } = await applyDraft(jobId);
    return NextResponse.json({ entities });
  } catch (error) {
    return handleApiError(error);
  }
}
