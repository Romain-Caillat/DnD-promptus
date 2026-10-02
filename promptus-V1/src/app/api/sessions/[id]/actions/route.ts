import { NextRequest, NextResponse } from "next/server";
import { ActionSchema } from "@/lib/validation/action-schemas";
import { handleApiError } from "@/lib/api/errors";
import { runAction } from "@/lib/session/run-action";

export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const action = ActionSchema.parse(await req.json());
    return NextResponse.json(await runAction(id, action));
  } catch (error) {
    return handleApiError(error);
  }
}
