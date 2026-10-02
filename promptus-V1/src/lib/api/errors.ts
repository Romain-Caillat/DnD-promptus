import { ZodError } from "zod";
import { NextResponse } from "next/server";

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    public details?: unknown,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

const RESOURCE_LABELS: Record<string, string> = {
  campaign: "Campagne",
  entity: "Fiche",
  session: "Session",
  session_state: "État de session",
  generation_job: "Génération",
  player_request: "Demande",
};

export function notFound(resource: string, id?: string): never {
  const label = RESOURCE_LABELS[resource] ?? resource;
  throw new ApiError(
    404,
    "not_found",
    id ? `${label} ${id} introuvable` : `${label} introuvable`,
  );
}

export function badRequest(message: string, details?: unknown): never {
  throw new ApiError(400, "bad_request", message, details);
}

export function handleApiError(error: unknown): NextResponse {
  if (error instanceof ApiError) {
    return NextResponse.json(
      {
        error: { code: error.code, message: error.message, details: error.details },
      },
      { status: error.status },
    );
  }
  if (error instanceof ZodError) {
    return NextResponse.json(
      {
        error: {
          code: "validation_error",
          message: "Données invalides",
          details: error.issues,
        },
      },
      { status: 400 },
    );
  }
  console.error("[api] unhandled error", error);
  return NextResponse.json(
    {
      error: { code: "internal_error", message: "Erreur interne du serveur" },
    },
    { status: 500 },
  );
}
