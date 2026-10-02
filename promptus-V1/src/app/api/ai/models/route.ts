import { NextResponse } from "next/server";
import { openRouterBaseUrl } from "@/lib/ai/llm";

interface OpenRouterModel {
  id: string;
  name?: string;
  context_length?: number;
  pricing?: { prompt?: string; completion?: string };
  architecture?: { output_modalities?: string[] };
}

export interface ModelOption {
  id: string;
  name: string;
  contextLength: number;
  /** Dollars par million de jetons. */
  promptPerM: number;
  completionPerM: number;
}

// Liste publique d'OpenRouter, mise en cache une heure.
let cache: { at: number; models: ModelOption[] } | null = null;

export async function GET() {
  if (cache && Date.now() - cache.at < 3_600_000) return NextResponse.json({ models: cache.models });
  try {
    const res = await fetch(`${openRouterBaseUrl()}/models`, { signal: AbortSignal.timeout(10_000) });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const body = (await res.json()) as { data?: OpenRouterModel[] };
    const models = (body.data ?? [])
      .filter((m) => (m.architecture?.output_modalities ?? ["text"]).includes("text"))
      .map((m) => ({
        id: m.id,
        name: m.name ?? m.id,
        contextLength: m.context_length ?? 0,
        promptPerM: Number(m.pricing?.prompt ?? 0) * 1e6,
        completionPerM: Number(m.pricing?.completion ?? 0) * 1e6,
      }))
      .sort((a, b) => a.id.localeCompare(b.id));
    cache = { at: Date.now(), models };
    return NextResponse.json({ models });
  } catch (e) {
    return NextResponse.json({ models: [], error: `Liste des modèles indisponible (${String(e)})` });
  }
}
