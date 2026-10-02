import "server-only";
import { and, desc, eq, sql } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { aiCalls, campaigns, entities, generationJobs, type GenerationJobRow, type NewEntityRow } from "@/lib/db/schema";
import { ApiError, badRequest, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { EMPTY_USAGE, LlmError, defaultModel, type LlmClient } from "@/lib/ai/llm";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import type { CampaignStory } from "@/lib/engine/story";
import { validateStory } from "@/lib/engine/story-validator";
import { BudgetExceededError, GENERATION_STEPS, generateCampaign } from "./pipeline";
import type { GenerationInput, GenerationJobView, GenerationStep } from "./types";

/** Un job « running » sans nouvelles depuis ce délai a été interrompu (redémarrage…). */
const STALE_MS = 15 * 60_000;

export function toJobView(row: GenerationJobRow): GenerationJobView {
  const stale = row.status === "running" && Date.now() - row.updatedAt.getTime() > STALE_MS;
  return {
    id: row.id,
    campaignId: row.campaignId,
    status: stale ? "failed" : row.status,
    input: row.input,
    steps: row.steps,
    result: row.result ?? null,
    error: stale ? "Génération interrompue (serveur redémarré ?)" : row.error,
    usage: row.usage,
    model: row.model,
    createdAt: row.createdAt.toISOString(),
    updatedAt: row.updatedAt.toISOString(),
  };
}

export async function getJob(jobId: string): Promise<GenerationJobRow> {
  const [row] = await db.select().from(generationJobs).where(eq(generationJobs.id, jobId));
  if (!row) notFound("generation_job", jobId);
  return row;
}

export async function listJobs(campaignId: string): Promise<GenerationJobView[]> {
  const rows = await db
    .select()
    .from(generationJobs)
    .where(eq(generationJobs.campaignId, campaignId))
    .orderBy(desc(generationJobs.createdAt))
    .limit(20);
  return rows.map(toJobView);
}

/** Dépense IA totale de la campagne (dollars) : génération, co-MJ, médias. */
export async function spentUsd(campaignId: string): Promise<number> {
  const [row] = await db
    .select({ total: sql<number>`coalesce(sum((${generationJobs.usage}->>'costUsd')::float), 0)` })
    .from(generationJobs)
    .where(eq(generationJobs.campaignId, campaignId));
  const [calls] = await db
    .select({ total: sql<number>`coalesce(sum(${aiCalls.costUsd}), 0)` })
    .from(aiCalls)
    .where(eq(aiCalls.campaignId, campaignId));
  return Number(row?.total ?? 0) + Number(calls?.total ?? 0);
}

export async function startGenerationJob(campaignId: string, input: GenerationInput): Promise<GenerationJobRow> {
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, campaignId));
  if (!campaign) notFound("campaign", campaignId);

  const running = await db
    .select()
    .from(generationJobs)
    .where(and(eq(generationJobs.campaignId, campaignId), eq(generationJobs.status, "running")));
  if (running.some((r) => toJobView(r).status === "running")) {
    throw new ApiError(409, "job_running", "Une génération est déjà en cours pour cette campagne");
  }

  const budget = campaign.aiSettings.budgetUsd;
  if (budget !== undefined && (await spentUsd(campaignId)) >= budget) {
    badRequest(`Budget de génération épuisé (${budget.toFixed(2)} $). Augmentez-le dans les paramètres de la campagne.`);
  }

  const steps: GenerationStep[] = GENERATION_STEPS.map((s) => ({ ...s, status: "pending" }));
  const [row] = await db
    .insert(generationJobs)
    .values({
      id: generateId("gen"),
      campaignId,
      status: "running",
      input,
      steps,
      usage: EMPTY_USAGE,
      model: input.model || campaign.aiSettings.model || defaultModel(),
    })
    .returning();
  return row;
}

/** Exécute le job (appelé en arrière-plan) et enregistre progression, coût et résultat. */
export async function runGenerationJob(jobId: string, llm: LlmClient): Promise<void> {
  const job = await getJob(jobId);
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, job.campaignId));
  const steps = [...job.steps];
  const touch = (set: Partial<typeof generationJobs.$inferInsert>) =>
    db.update(generationJobs).set({ ...set, updatedAt: new Date() }).where(eq(generationJobs.id, jobId));

  try {
    const budget = campaign?.aiSettings.budgetUsd;
    const remaining = budget === undefined ? undefined : Math.max(0, budget - (await spentUsd(job.campaignId)));
    const result = await generateCampaign(job.input, {
      llm,
      ruleset: campaign?.ruleset ?? DND5E_RULESET,
      model: job.model,
      budgetUsd: remaining,
      onStep: async (id, status, detail) => {
        const i = steps.findIndex((s) => s.id === id);
        if (i >= 0) steps[i] = { ...steps[i], status, detail };
        await touch({ steps });
      },
      onUsage: async (usage) => {
        await touch({ usage });
      },
    });
    await touch({ status: "succeeded", result: { draft: result.draft, issues: result.issues }, usage: result.usage, steps });
  } catch (e) {
    const message =
      e instanceof BudgetExceededError
        ? e.message
        : e instanceof LlmError
          ? `${e.message}${e.details ? ` — ${e.details.slice(0, 300)}` : ""}`
          : e instanceof Error
            ? e.message
            : String(e);
    console.error("[generation] job failed", jobId, e);
    await touch({ status: "failed", error: message, steps });
  }
}

const MJ_ONLY_TYPES = new Set(["npc", "monster", "faction", "event"]);

/**
 * Crée les fiches du brouillon et installe son scénario dans la campagne.
 * Les références `ent_…` sont remplacées par les ids créés ; l'état du monde
 * repart de zéro. Tout ou rien (transaction).
 */
export async function applyDraft(jobId: string): Promise<{ entities: number; story: CampaignStory }> {
  const job = await getJob(jobId);
  const view = toJobView(job);
  if (view.status !== "succeeded" || !job.result) {
    badRequest(view.status === "applied" ? "Ce brouillon a déjà été appliqué" : "Ce brouillon n’est pas prêt");
  }
  const { draft } = job.result;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, job.campaignId));
  if (!campaign) notFound("campaign", job.campaignId);

  const idByRef = new Map(draft.entities.map((e) => [e.ref, generateId(`ent_${e.type}`)]));
  const swap = <T>(value: T): T =>
    JSON.parse(
      JSON.stringify(value).replace(/"(ent_[a-z0-9_]+)"/g, (m, ref: string) => {
        const id = idByRef.get(ref);
        return id ? `"${id}"` : m;
      }),
    );

  const rows: NewEntityRow[] = draft.entities.map((e) => ({
    id: idByRef.get(e.ref)!,
    campaignId: campaign.id,
    type: e.type,
    name: e.name,
    description: e.description,
    tags: e.tags,
    attributes: swap(e.attributes),
    effects: [],
    visibility: e.visibility ?? (MJ_ONLY_TYPES.has(e.type) ? "mj_only" : "public"),
  }));
  const story = swap(draft.story);

  const existing = await db.select({ id: entities.id }).from(entities).where(eq(entities.campaignId, campaign.id));
  const issues = validateStory(story, {
    entityIds: new Set([...existing.map((e) => e.id), ...rows.map((r) => r.id!)]),
    ruleset: campaign.ruleset ?? DND5E_RULESET,
  });
  const errors = issues.filter((i) => i.severity === "error");
  if (errors.length) {
    throw new ApiError(
      422,
      "story_invalid",
      `${errors.length} erreur(s) à corriger avant d’appliquer`,
      errors.map((e) => ({ path: e.path.split("."), message: e.message })),
    );
  }

  await db.transaction(async (tx) => {
    if (rows.length) await tx.insert(entities).values(rows);
    await tx
      .update(campaigns)
      .set({ story, worldState: null, updatedAt: new Date() })
      .where(eq(campaigns.id, campaign.id));
    await tx.update(generationJobs).set({ status: "applied", updatedAt: new Date() }).where(eq(generationJobs.id, jobId));
  });
  return { entities: rows.length, story };
}

/** Remplace l'histoire d'un brouillon (édition YAML avant application). */
export async function updateDraftStory(jobId: string, story: CampaignStory): Promise<GenerationJobView> {
  const job = await getJob(jobId);
  if (toJobView(job).status !== "succeeded" || !job.result) badRequest("Ce brouillon n’est plus modifiable");
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, job.campaignId));
  const issues = validateStory(story, {
    entityIds: new Set(job.result.draft.entities.map((e) => e.ref)),
    ruleset: campaign?.ruleset ?? DND5E_RULESET,
  });
  const [row] = await db
    .update(generationJobs)
    .set({ result: { draft: { ...job.result.draft, story }, issues }, updatedAt: new Date() })
    .where(eq(generationJobs.id, jobId))
    .returning();
  return toJobView(row);
}
