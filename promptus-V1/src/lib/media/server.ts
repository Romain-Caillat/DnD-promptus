import "server-only";
import { and, desc, eq, inArray } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { aiCalls, campaigns, entities, type AiCallRow, type Campaign } from "@/lib/db/schema";
import { ApiError, badRequest, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { LlmError, defaultImageModel, defaultVideoModel } from "@/lib/ai/llm";
import { getImageGenerator } from "@/lib/ai/image-generator";
import { EMPTY_STORY, type CampaignStory } from "@/lib/engine/story";
import type { StyleGuide } from "@/lib/engine/types";
import { spentUsd } from "@/lib/generation/server";
import { notifyCampaign } from "@/lib/realtime/notify";
import { generateMediaOpenRouter, type MediaResult } from "./openrouter-media";
import {
  entityImagePrompt,
  isVideo,
  mapAspectRatio,
  mapBackgroundPrompt,
  sceneImagePrompt,
  sceneVideoPrompt,
  targetKey,
  type MediaTarget,
} from "./prompts";

/** Types de fiches illustrées. */
const ILLUSTRATED = new Set(["character", "npc", "monster", "location", "item"]);
/** Coûts par défaut tant qu'aucun appel n'a servi de référence (dollars). */
const DEFAULT_COST = { image: 0.04, video: 0.5 };
/** Au-delà, un appel « en cours » est considéré interrompu (redémarrage…). */
const STALE_MS = 20 * 60_000;

export interface MediaItem {
  target: MediaTarget;
  key: string;
  group: "scenes" | "entities" | "maps";
  label: string;
  detail: string;
  url: string | null;
  prompt: string;
  status: "none" | "running" | "failed" | "done";
  error: string | null;
}

export interface MediaProviders {
  image: { available: boolean; provider: "openrouter" | "huggingface" | null; model: string | null };
  video: { available: boolean; model: string | null };
}

// ----------------------------------------------------------------------------
// Fournisseurs
// ----------------------------------------------------------------------------

function imageProvider(): "openrouter" | "huggingface" | null {
  const explicit = process.env.IMAGE_PROVIDER?.toLowerCase();
  if (explicit === "openrouter" || explicit === "huggingface") return explicit;
  if (process.env.OPENROUTER_API_KEY) return "openrouter";
  if (process.env.HUGGINGFACE_TOKEN) return "huggingface";
  return null;
}

export function mediaProviders(campaign: Campaign): MediaProviders {
  const provider = imageProvider();
  const imageReady =
    provider === "openrouter" ? !!process.env.OPENROUTER_API_KEY : provider === "huggingface" ? !!process.env.HUGGINGFACE_TOKEN : false;
  const videoModel = campaign.aiSettings.videoModel || defaultVideoModel() || null;
  return {
    image: {
      available: imageReady,
      provider,
      model: provider === "openrouter" ? campaign.aiSettings.imageModel || defaultImageModel() : provider === "huggingface" ? "flux-schnell" : null,
    },
    video: { available: !!process.env.OPENROUTER_API_KEY && !!videoModel, model: videoModel },
  };
}

async function generate(campaign: Campaign, target: MediaTarget, prompt: string, aspectRatio: string): Promise<MediaResult> {
  const providers = mediaProviders(campaign);
  const key = process.env.OPENROUTER_API_KEY;
  if (isVideo(target)) {
    if (!providers.video.available || !key) throw new ApiError(400, "no_video_model", "Choisissez un modèle vidéo dans les paramètres de la campagne.");
    return generateMediaOpenRouter({ campaignId: campaign.id, prompt, model: providers.video.model!, modality: "video", aspectRatio }, key);
  }
  if (!providers.image.available) {
    throw new ApiError(400, "no_image_provider", "Aucun fournisseur d’images : ajoutez OPENROUTER_API_KEY (ou HUGGINGFACE_TOKEN).");
  }
  if (providers.image.provider === "openrouter") {
    return generateMediaOpenRouter({ campaignId: campaign.id, prompt, model: providers.image.model!, modality: "image", aspectRatio }, key!);
  }
  const gen = await getImageGenerator();
  const [w, h] = aspectRatio.split(":").map(Number);
  const r = await gen.generate(prompt, {
    campaignId: campaign.id,
    width: w >= h ? 1024 : Math.round((1024 * w) / h / 8) * 8,
    height: h >= w ? 1024 : Math.round((1024 * h) / w / 8) * 8,
  });
  return { url: r.url, model: gen.name, costUsd: 0 };
}

// ----------------------------------------------------------------------------
// Inventaire des médias d'une campagne
// ----------------------------------------------------------------------------

interface Resolved {
  item: Omit<MediaItem, "status" | "error">;
  aspectRatio: string;
}

function resolveTargets(campaign: Campaign, ents: (typeof entities.$inferSelect)[]): Resolved[] {
  const story = campaign.story ?? EMPTY_STORY;
  const guide = (campaign.styleGuide ?? {}) as StyleGuide;
  const out: Resolved[] = [];
  for (const s of story.scenes) {
    const t1: MediaTarget = { type: "scene_image", id: s.id };
    out.push({
      item: { target: t1, key: targetKey(t1), group: "scenes", label: s.title, detail: "Image de scène", url: s.media?.imageUrl ?? null, prompt: sceneImagePrompt(s, guide, story.bible) },
      aspectRatio: "16:9",
    });
    const t2: MediaTarget = { type: "scene_video", id: s.id };
    out.push({
      item: { target: t2, key: targetKey(t2), group: "scenes", label: s.title, detail: "Vidéo d’intro", url: s.media?.videoUrl ?? null, prompt: sceneVideoPrompt(s, guide, story.bible) },
      aspectRatio: "16:9",
    });
  }
  for (const e of ents) {
    if (!ILLUSTRATED.has(e.type) || e.attributes.copyOf) continue;
    const t: MediaTarget = { type: "entity", id: e.id };
    out.push({
      item: { target: t, key: targetKey(t), group: "entities", label: e.name, detail: e.type, url: e.imageUrl, prompt: entityImagePrompt(e, guide) },
      aspectRatio: e.type === "location" ? "16:9" : "1:1",
    });
  }
  for (const m of story.maps) {
    const t: MediaTarget = { type: "map", id: m.id };
    out.push({
      item: { target: t, key: targetKey(t), group: "maps", label: m.name, detail: "Fond de carte", url: m.backgroundUrl ?? null, prompt: mapBackgroundPrompt(m, guide, story.bible) },
      aspectRatio: mapAspectRatio(m),
    });
  }
  return out;
}

function callStatus(row: AiCallRow | undefined): Pick<MediaItem, "status" | "error"> | null {
  if (!row) return null;
  if (row.status === "running" && Date.now() - row.updatedAt.getTime() > STALE_MS) {
    return { status: "failed", error: "Génération interrompue" };
  }
  if (row.status === "running") return { status: "running", error: null };
  if (row.status === "failed") return { status: "failed", error: row.error };
  return null;
}

async function latestCalls(campaignId: string): Promise<Map<string, AiCallRow>> {
  const rows = await db
    .select()
    .from(aiCalls)
    .where(and(eq(aiCalls.campaignId, campaignId), inArray(aiCalls.kind, ["image", "video"])))
    .orderBy(desc(aiCalls.createdAt))
    .limit(500);
  const latest = new Map<string, AiCallRow>();
  for (const r of rows) {
    const k = String(r.input.targetKey ?? "");
    if (k && !latest.has(k)) latest.set(k, r);
  }
  return latest;
}

/** Coût moyen observé d'un média (sinon une estimation prudente). */
async function unitCost(campaign: Campaign, kind: "image" | "video"): Promise<number> {
  const providers = mediaProviders(campaign);
  if (kind === "image" && providers.image.provider === "huggingface") return 0;
  const model = kind === "image" ? providers.image.model : providers.video.model;
  if (!model) return DEFAULT_COST[kind];
  const rows = await db
    .select({ cost: aiCalls.costUsd })
    .from(aiCalls)
    .where(and(eq(aiCalls.kind, kind), eq(aiCalls.model, model), eq(aiCalls.status, "succeeded")))
    .orderBy(desc(aiCalls.createdAt))
    .limit(10);
  const known = rows.map((r) => r.cost).filter((c) => c > 0);
  return known.length ? known.reduce((a, b) => a + b, 0) / known.length : DEFAULT_COST[kind];
}

export async function mediaOverview(campaignId: string) {
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, campaignId));
  if (!campaign) notFound("campaign", campaignId);
  const ents = await db.select().from(entities).where(eq(entities.campaignId, campaignId));
  const latest = await latestCalls(campaignId);
  const items: MediaItem[] = resolveTargets(campaign, ents).map(({ item }) => {
    const st = callStatus(latest.get(item.key));
    return { ...item, status: st?.status ?? (item.url ? "done" : "none"), error: st?.error ?? null };
  });
  const story = campaign.story ?? EMPTY_STORY;
  return {
    items,
    music: story.scenes.map((sc) => ({ sceneId: sc.id, title: sc.title, musicUrl: sc.media?.musicUrl ?? null, musicQuery: sc.media?.musicQuery ?? null })),
    providers: mediaProviders(campaign),
    unitCostUsd: { image: await unitCost(campaign, "image"), video: await unitCost(campaign, "video") },
    budgetUsd: campaign.aiSettings.budgetUsd ?? null,
    spentUsd: await spentUsd(campaignId),
  };
}

// ----------------------------------------------------------------------------
// Lancement et exécution
// ----------------------------------------------------------------------------

export interface MediaRequestItem {
  target: MediaTarget;
  /** Prompt modifié par le MJ ; il est conservé pour les prochaines fois. */
  prompt?: string;
}

/** Crée les appels (un par média) après contrôle du budget ; à exécuter ensuite. */
export async function queueMedia(
  campaignId: string,
  requests: MediaRequestItem[],
  opts: { regenerate?: boolean } = {},
): Promise<{ callIds: string[]; estimateUsd: number }> {
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, campaignId));
  if (!campaign) notFound("campaign", campaignId);
  const ents = await db.select().from(entities).where(eq(entities.campaignId, campaignId));
  const resolved = new Map(resolveTargets(campaign, ents).map((r) => [r.item.key, r]));
  const latest = await latestCalls(campaignId);
  const providers = mediaProviders(campaign);

  const todo: { r: Resolved; prompt: string }[] = [];
  for (const req of requests) {
    const r = resolved.get(targetKey(req.target));
    if (!r) badRequest(`Élément introuvable : ${targetKey(req.target)}`);
    if (callStatus(latest.get(r.item.key))?.status === "running") continue;
    if (r.item.url && !opts.regenerate) continue;
    todo.push({ r, prompt: req.prompt?.trim() || r.item.prompt });
  }
  if (!todo.length) return { callIds: [], estimateUsd: 0 };
  if (todo.some((t) => isVideo(t.r.item.target)) && !providers.video.available) {
    badRequest("Choisissez un modèle vidéo dans les paramètres de la campagne.");
  }
  if (todo.some((t) => !isVideo(t.r.item.target)) && !providers.image.available) {
    badRequest("Aucun fournisseur d’images : ajoutez OPENROUTER_API_KEY (ou HUGGINGFACE_TOKEN).");
  }

  const [img, vid] = [await unitCost(campaign, "image"), await unitCost(campaign, "video")];
  const estimate = todo.reduce((sum, t) => sum + (isVideo(t.r.item.target) ? vid : img), 0);
  const budget = campaign.aiSettings.budgetUsd;
  if (budget !== undefined) {
    const remaining = budget - (await spentUsd(campaignId));
    if (estimate > remaining) {
      badRequest(
        `Budget insuffisant : environ ${estimate.toFixed(2)} $ nécessaires, ${Math.max(0, remaining).toFixed(2)} $ restants. Augmentez le budget ou générez moins d’éléments.`,
      );
    }
  }

  const rows = todo.map(({ r, prompt }) => ({
    id: generateId("ai"),
    campaignId,
    kind: (isVideo(r.item.target) ? "video" : "image") as "image" | "video",
    status: "running" as const,
    model: (isVideo(r.item.target) ? providers.video.model : providers.image.model) ?? "?",
    input: { target: r.item.target, targetKey: r.item.key, prompt, aspectRatio: r.aspectRatio } as Record<string, unknown>,
  }));
  await db.insert(aiCalls).values(rows);
  return { callIds: rows.map((r) => r.id), estimateUsd: estimate };
}

/** Exécute les appels l'un après l'autre (en arrière-plan) et range les résultats. */
export async function runMediaCalls(campaignId: string, callIds: string[]): Promise<void> {
  for (const id of callIds) {
    const [call] = await db.select().from(aiCalls).where(eq(aiCalls.id, id));
    const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, campaignId));
    if (!call || !campaign || call.status !== "running") continue;
    const target = call.input.target as MediaTarget;
    const prompt = String(call.input.prompt ?? "");
    try {
      const budget = campaign.aiSettings.budgetUsd;
      if (budget !== undefined && (await spentUsd(campaignId)) >= budget) throw new ApiError(400, "budget", "Budget épuisé");
      const result = await generate(campaign, target, prompt, String(call.input.aspectRatio ?? "1:1"));
      await applyMedia(campaignId, target, result.url, prompt);
      await db
        .update(aiCalls)
        .set({ status: "succeeded", model: result.model, output: { url: result.url }, costUsd: result.costUsd, updatedAt: new Date() })
        .where(eq(aiCalls.id, id));
    } catch (e) {
      const message =
        e instanceof LlmError ? `${e.message}${e.details ? ` — ${e.details.slice(0, 200)}` : ""}` : e instanceof Error ? e.message : String(e);
      console.error("[media] échec", id, message);
      await db.update(aiCalls).set({ status: "failed", error: message, updatedAt: new Date() }).where(eq(aiCalls.id, id));
    }
  }
  await notifyCampaign(campaignId, ["story", "session"]);
}

/** Range l'URL du média à sa place (fiche, scène, carte) avec son prompt. */
export async function applyMedia(campaignId: string, target: MediaTarget, url: string, prompt: string): Promise<void> {
  if (target.type === "entity") {
    const [e] = await db.select().from(entities).where(and(eq(entities.id, target.id), eq(entities.campaignId, campaignId)));
    if (!e) return;
    await db
      .update(entities)
      .set({ imageUrl: url, attributes: { ...e.attributes, imagePrompt: prompt }, updatedAt: new Date() })
      .where(eq(entities.id, e.id));
    return;
  }
  // Le scénario peut être modifié en parallèle : lecture et écriture verrouillées.
  await db.transaction(async (tx) => {
    const [c] = await tx.select({ story: campaigns.story }).from(campaigns).where(eq(campaigns.id, campaignId)).for("update");
    if (!c?.story) return;
    const story: CampaignStory = structuredClone(c.story);
    if (target.type === "map") {
      const m = story.maps.find((x) => x.id === target.id);
      if (!m) return;
      // backgroundPrompt reste la description du lieu (le prompt complet en dérive).
      m.backgroundUrl = url;
    } else {
      const s = story.scenes.find((x) => x.id === target.id);
      if (!s) return;
      s.media = { ...s.media, ...(target.type === "scene_image" ? { imageUrl: url, imagePrompt: prompt } : { videoUrl: url, videoPrompt: prompt }) };
    }
    await tx.update(campaigns).set({ story, updatedAt: new Date() }).where(eq(campaigns.id, campaignId));
  });
}
