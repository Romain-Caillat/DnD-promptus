// Client LLM — OpenRouter (API compatible OpenAI) derrière une interface,
// pour pouvoir changer de fournisseur et injecter un faux client en test.

export interface LlmMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface LlmRequest {
  messages: LlmMessage[];
  model?: string;
  temperature?: number;
  maxTokens?: number;
  /** Demande une réponse JSON (objet). */
  json?: boolean;
}

export interface LlmUsage {
  promptTokens: number;
  completionTokens: number;
  /** Coût en dollars quand le fournisseur le communique. */
  costUsd: number;
}

export interface LlmResponse {
  text: string;
  model: string;
  usage: LlmUsage;
}

export interface LlmClient {
  readonly name: string;
  complete(req: LlmRequest): Promise<LlmResponse>;
}

export class LlmError extends Error {
  constructor(
    message: string,
    public status?: number,
    public details?: string,
  ) {
    super(message);
    this.name = "LlmError";
  }
}

export const EMPTY_USAGE: LlmUsage = { promptTokens: 0, completionTokens: 0, costUsd: 0 };

export function addUsage(a: LlmUsage, b: LlmUsage): LlmUsage {
  return {
    promptTokens: a.promptTokens + b.promptTokens,
    completionTokens: a.completionTokens + b.completionTokens,
    costUsd: a.costUsd + b.costUsd,
  };
}

// ----------------------------------------------------------------------------
// OpenRouter
// ----------------------------------------------------------------------------

export function openRouterBaseUrl(): string {
  return (process.env.OPENROUTER_BASE_URL ?? "https://openrouter.ai/api/v1").replace(/\/$/, "");
}

export function defaultModel(): string {
  return process.env.OPENROUTER_MODEL ?? "anthropic/claude-sonnet-4.5";
}

export function defaultImageModel(): string {
  return process.env.OPENROUTER_IMAGE_MODEL ?? "google/gemini-2.5-flash-image";
}

/** Pas de modèle vidéo par défaut : le MJ le choisit selon l'offre du moment. */
export function defaultVideoModel(): string | undefined {
  return process.env.OPENROUTER_VIDEO_MODEL || undefined;
}

interface ChatCompletionResponse {
  model?: string;
  choices?: { message?: { content?: string | null }; finish_reason?: string }[];
  usage?: { prompt_tokens?: number; completion_tokens?: number; cost?: number };
  error?: { message?: string };
}

export class OpenRouterClient implements LlmClient {
  readonly name = "openrouter";

  constructor(
    private apiKey: string,
    private opts: { baseUrl?: string; timeoutMs?: number; appUrl?: string } = {},
  ) {}

  async complete(req: LlmRequest): Promise<LlmResponse> {
    const model = req.model ?? defaultModel();
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.opts.timeoutMs ?? 5 * 60_000);
    let res: Response;
    try {
      res = await fetch(`${this.opts.baseUrl ?? openRouterBaseUrl()}/chat/completions`, {
        method: "POST",
        signal: controller.signal,
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "content-type": "application/json",
          "X-Title": "Promptus",
          ...(this.opts.appUrl ? { "HTTP-Referer": this.opts.appUrl } : {}),
        },
        body: JSON.stringify({
          model,
          messages: req.messages,
          temperature: req.temperature ?? 0.8,
          max_tokens: req.maxTokens ?? 16_000,
          ...(req.json ? { response_format: { type: "json_object" } } : {}),
          // Demande à OpenRouter de renvoyer le coût réel de l'appel.
          usage: { include: true },
        }),
      });
    } catch (e) {
      const aborted = e instanceof Error && e.name === "AbortError";
      throw new LlmError(aborted ? "Délai dépassé en attendant le modèle" : "OpenRouter injoignable", undefined, String(e));
    } finally {
      clearTimeout(timer);
    }

    const body = (await res.json().catch(() => ({}))) as ChatCompletionResponse;
    if (!res.ok || body.error) {
      throw new LlmError(
        `Échec de l’appel au modèle (${res.status})`,
        res.status,
        body.error?.message ?? JSON.stringify(body).slice(0, 500),
      );
    }
    const choice = body.choices?.[0];
    const text = choice?.message?.content ?? "";
    if (!text) throw new LlmError("Réponse vide du modèle", res.status, choice?.finish_reason);
    return {
      text,
      model: body.model ?? model,
      usage: {
        promptTokens: body.usage?.prompt_tokens ?? 0,
        completionTokens: body.usage?.completion_tokens ?? 0,
        costUsd: body.usage?.cost ?? 0,
      },
    };
  }
}

/** Client configuré par l'environnement, ou null si aucune clé n'est définie. */
export function getLlmClient(): LlmClient | null {
  const key = process.env.OPENROUTER_API_KEY;
  if (!key) return null;
  return new OpenRouterClient(key, { appUrl: process.env.APP_URL });
}

// ----------------------------------------------------------------------------
// Extraction JSON tolérante (blocs ```json, texte autour…)
// ----------------------------------------------------------------------------

export function parseJsonObject(text: string): unknown {
  const fenced = text.match(/```(?:json)?\s*([\s\S]*?)```/);
  const candidate = fenced ? fenced[1] : text;
  const start = candidate.indexOf("{");
  const end = candidate.lastIndexOf("}");
  if (start < 0 || end <= start) throw new LlmError("Le modèle n’a pas renvoyé d’objet JSON", undefined, text.slice(0, 500));
  try {
    return JSON.parse(candidate.slice(start, end + 1));
  } catch (e) {
    throw new LlmError("JSON invalide renvoyé par le modèle", undefined, `${String(e)} — ${text.slice(0, 300)}`);
  }
}
