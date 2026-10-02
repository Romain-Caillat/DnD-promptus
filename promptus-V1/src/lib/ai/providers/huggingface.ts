import { saveMedia } from "@/lib/media/storage";
import {
  ImageGenerationError,
  type ImageGenerator,
  type ImageOptions,
  type ImageResult,
} from "../image-generator";

const ENDPOINT =
  "https://api-inference.huggingface.co/models/black-forest-labs/FLUX.1-schnell";

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
    if (!opts.campaignId) throw new ImageGenerationError("Campagne manquante pour enregistrer l’image", 500);
    const url = await saveMedia(opts.campaignId, buffer, "png");
    return {
      url,
      provider: this.name,
      promptUsed: prompt,
      latencyMs: Math.round(performance.now() - start),
    };
  }
}
