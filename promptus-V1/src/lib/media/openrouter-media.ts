import "server-only";
import { LlmError, openRouterBaseUrl } from "@/lib/ai/llm";
import { decodeDataUrl, extFromMime, saveMedia } from "./storage";

// Génération d'images et de vidéos via OpenRouter (API chat/completions avec
// `modalities`). Les images arrivent en data URL dans `message.images`.
// La vidéo suit le même schéma (`message.videos`, ou une URL de fichier) :
// à confirmer selon les modèles disponibles au branchement.

export type MediaModality = "image" | "video";

export interface MediaRequest {
  campaignId: string;
  prompt: string;
  model: string;
  modality: MediaModality;
  /** Format souhaité (« 16:9 », « 1:1 »…), transmis aux modèles qui l'acceptent. */
  aspectRatio?: string;
}

export interface MediaResult {
  url: string;
  model: string;
  costUsd: number;
}

interface MediaPart {
  type?: string;
  image_url?: { url?: string };
  video_url?: { url?: string };
  url?: string;
}

interface Completion {
  model?: string;
  choices?: { message?: { content?: string | null; images?: MediaPart[]; videos?: MediaPart[] } }[];
  usage?: { cost?: number };
  error?: { message?: string };
}

const partUrl = (p: MediaPart | undefined) => p?.image_url?.url ?? p?.video_url?.url ?? p?.url;

export async function generateMediaOpenRouter(req: MediaRequest, apiKey: string): Promise<MediaResult> {
  const res = await fetch(`${openRouterBaseUrl()}/chat/completions`, {
    method: "POST",
    signal: AbortSignal.timeout(req.modality === "video" ? 15 * 60_000 : 3 * 60_000),
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "content-type": "application/json",
      "X-Title": "Promptus",
      ...(process.env.APP_URL ? { "HTTP-Referer": process.env.APP_URL } : {}),
    },
    body: JSON.stringify({
      model: req.model,
      messages: [{ role: "user", content: req.prompt }],
      modalities: req.modality === "image" ? ["image", "text"] : ["video", "text"],
      ...(req.aspectRatio ? { image_config: { aspect_ratio: req.aspectRatio } } : {}),
      usage: { include: true },
    }),
  }).catch((e) => {
    throw new LlmError("OpenRouter injoignable", undefined, String(e));
  });
  const body = (await res.json().catch(() => ({}))) as Completion;
  if (!res.ok || body.error) {
    throw new LlmError(`Échec de la génération (${res.status})`, res.status, body.error?.message ?? JSON.stringify(body).slice(0, 300));
  }
  const message = body.choices?.[0]?.message;
  const parts = req.modality === "image" ? message?.images : (message?.videos ?? message?.images);
  const url = partUrl(parts?.[0]) ?? message?.content?.match(/https?:\/\/\S+\.(?:mp4|webm|png|jpe?g|webp)\b/i)?.[0];
  if (!url) throw new LlmError(`Le modèle n’a pas renvoyé ${req.modality === "image" ? "d’image" : "de vidéo"}`, res.status);

  // Data URL (cas courant) ou fichier à télécharger.
  let file = decodeDataUrl(url);
  if (!file) {
    const dl = await fetch(url, { signal: AbortSignal.timeout(5 * 60_000) });
    if (!dl.ok) throw new LlmError(`Téléchargement du média impossible (${dl.status})`, dl.status);
    const ext = extFromMime(dl.headers.get("content-type") ?? "") ?? (req.modality === "video" ? "mp4" : "png");
    file = { data: Buffer.from(await dl.arrayBuffer()), ext };
  }
  return {
    url: await saveMedia(req.campaignId, file.data, file.ext),
    model: body.model ?? req.model,
    costUsd: body.usage?.cost ?? 0,
  };
}
