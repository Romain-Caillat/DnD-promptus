import { promises as fs } from "node:fs";
import path from "node:path";
import {
  ImageGenerationError,
  type ImageGenerator,
  type ImageOptions,
  type ImageResult,
} from "../image-generator";

const ENDPOINT =
  "https://api-inference.huggingface.co/models/black-forest-labs/FLUX.1-schnell";
const PUBLIC_DIR = path.join(process.cwd(), "public", "generated-images");

export class HuggingFaceFluxSchnell implements ImageGenerator {
  readonly name = "huggingface/flux-schnell";

  async generate(prompt: string, opts: ImageOptions): Promise<ImageResult> {
    const start = performance.now();
    const token = process.env.HUGGINGFACE_TOKEN;
    if (!token) {
      throw new ImageGenerationError(
        "HUGGINGFACE_TOKEN n’est pas défini : impossible de joindre l’API de génération",
        500,
      );
    }

    const res = await fetch(ENDPOINT, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "content-type": "application/json",
        accept: "image/png",
      },
      body: JSON.stringify({
        inputs: prompt,
        parameters: {
          width: opts.width ?? 1024,
          height: opts.height ?? 1024,
          guidance_scale: opts.guidanceScale ?? 3.5,
          num_inference_steps: opts.steps ?? 4,
          ...(opts.seed !== undefined && { seed: opts.seed }),
        },
      }),
    });

    if (!res.ok) {
      const text = await res.text().catch(() => "");
      throw new ImageGenerationError(
        `Échec de la génération HuggingFace (${res.status})`,
        res.status,
        text.slice(0, 500),
      );
    }

    const buffer = Buffer.from(await res.arrayBuffer());
    const url = await saveToPublic(buffer, opts.entityId);
    return {
      url,
      provider: this.name,
      promptUsed: prompt,
      latencyMs: Math.round(performance.now() - start),
    };
  }
}

async function saveToPublic(buffer: Buffer, entityId?: string): Promise<string> {
  await fs.mkdir(PUBLIC_DIR, { recursive: true });
  const slug = entityId ? entityId.replace(/[^a-z0-9_-]/gi, "_") : "img";
  const filename = `${slug}-${Date.now()}.png`;
  const fullPath = path.join(PUBLIC_DIR, filename);
  await fs.writeFile(fullPath, buffer);
  return `/generated-images/${filename}`;
}
