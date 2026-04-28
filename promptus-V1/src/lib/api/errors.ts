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

export function notFound(resource: string, id?: string): never {
  throw new ApiError(
    404,
    "not_found",
    id ? `${resource} ${id} not found` : `${resource} not found`,
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
          message: "Invalid input",
          details: error.issues,
        },
      },
      { status: 400 },
    );
  }
  console.error("[api] unhandled error", error);
  return NextResponse.json(
    {
      error: { code: "internal_error", message: "Internal server error" },
    },
    { status: 500 },
  );
}
