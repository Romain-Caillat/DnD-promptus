// Image generator abstraction.
// Implementations live in ./providers/. Selection via IMAGE_PROVIDER env.
// All providers must accept a prompt + options and return an ImageResult that
// already contains the locally stored URL (the provider is responsible for
// downloading the bytes to /public/generated-images and returning the path).

export interface ImageOptions {
  width?: number;
  height?: number;
  guidanceScale?: number;
  steps?: number;
  seed?: number;
  /** Used to namespace the saved file. */
  entityId?: string;
}

export interface ImageResult {
  url: string;
  provider: string;
  promptUsed: string;
  latencyMs: number;
  costEstimate?: number;
}

export interface ImageGenerator {
  readonly name: string;
  generate(prompt: string, opts: ImageOptions): Promise<ImageResult>;
}

// Lazily created singleton selected by env.
let cached: ImageGenerator | null = null;

export async function getImageGenerator(): Promise<ImageGenerator> {
  if (cached) return cached;
  const provider = (process.env.IMAGE_PROVIDER ?? "huggingface").toLowerCase();
  if (provider === "huggingface") {
    const { HuggingFaceFluxSchnell } = await import("./providers/huggingface");
    cached = new HuggingFaceFluxSchnell();
  } else {
    throw new Error(`Unknown IMAGE_PROVIDER: ${provider}`);
  }
  return cached;
}

export class ImageGenerationError extends Error {
  constructor(
    message: string,
    public status?: number,
    public providerMessage?: string,
  ) {
    super(message);
    this.name = "ImageGenerationError";
  }
}
