// Pipeline de génération de campagne : trois appels LLM ciblés (fiches,
// scènes, cartes) puis une boucle où le validateur d'histoire renvoie ses
// erreurs au LLM pour correction. Indépendant de la base et de Next.

import { z } from "zod";
import {
  EMPTY_USAGE,
  LlmError,
  addUsage,
  parseJsonObject,
  type LlmClient,
  type LlmMessage,
  type LlmUsage,
} from "@/lib/ai/llm";
import type { Ruleset } from "@/lib/engine/ruleset";
import type { CampaignStory } from "@/lib/engine/story";
import { validateStory, type StoryIssue } from "@/lib/engine/story-validator";
import { StorySchema } from "@/lib/validation/story-schema";
import { castPrompt, mapsPrompt, repairPrompt, scenesPrompt, systemPrompt } from "./prompts";
import {
  DraftEntitySchema,
  type CampaignDraft,
  type DraftEntity,
  type GenerationInput,
  type GenerationResult,
  type GenerationStep,
} from "./types";

export const GENERATION_STEPS: Omit<GenerationStep, "status">[] = [
  { id: "cast", label: "Bible, menaces et fiches" },
  { id: "scenes", label: "Scènes, révélations et indices" },
  { id: "maps", label: "Cartes à grille" },
  { id: "check", label: "Vérification et corrections" },
];

/** Corrections automatiques maximum après la première vérification. */
const MAX_REPAIRS = 2;

export class BudgetExceededError extends Error {
  constructor(public spentUsd: number, public budgetUsd: number) {
    super(`Budget atteint : ${spentUsd.toFixed(3)} $ dépensés sur ${budgetUsd.toFixed(2)} $ autorisés`);
    this.name = "BudgetExceededError";
  }
}

export interface PipelineOptions {
  llm: LlmClient;
  ruleset: Ruleset;
  model?: string;
  /** Plafond de dépense pour ce job (dollars). */
  budgetUsd?: number;
  onStep?: (stepId: string, status: GenerationStep["status"], detail?: string) => void | Promise<void>;
  onUsage?: (usage: LlmUsage) => void | Promise<void>;
}

const CastSchema = z.object({
  bible: StorySchema.shape.bible,
  fronts: StorySchema.shape.fronts,
  entities: z.array(DraftEntitySchema).min(1),
});

const ScenesSchema = z.object({
  startSceneId: z.string().min(1),
  scenes: StorySchema.shape.scenes,
  revelations: StorySchema.shape.revelations,
  clues: StorySchema.shape.clues,
});

const MapsSchema = z.object({
  maps: StorySchema.shape.maps,
  placements: z
    .array(z.object({ sceneId: z.string(), mapId: z.string(), x: z.number().int(), y: z.number().int() }))
    .default([]),
  battleMaps: z.array(z.object({ sceneId: z.string(), mapId: z.string() })).default([]),
});

export async function generateCampaign(
  input: GenerationInput,
  opts: PipelineOptions,
): Promise<GenerationResult & { usage: LlmUsage }> {
  let usage = EMPTY_USAGE;
  const system: LlmMessage = { role: "system", content: systemPrompt(opts.ruleset) };

  async function ask<T>(prompt: string, schema: z.ZodType<T>): Promise<T> {
    const messages: LlmMessage[] = [system, { role: "user", content: prompt }];
    // Une seconde chance si le JSON est invalide ou ne respecte pas le schéma.
    for (let attempt = 0; attempt < 2; attempt++) {
      const res = await opts.llm.complete({ messages, model: opts.model ?? input.model, json: true });
      usage = addUsage(usage, res.usage);
      await opts.onUsage?.(usage);
      if (opts.budgetUsd !== undefined && usage.costUsd > opts.budgetUsd) {
        throw new BudgetExceededError(usage.costUsd, opts.budgetUsd);
      }
      let problem: string;
      try {
        const parsed = schema.safeParse(parseJsonObject(res.text));
        if (parsed.success) return parsed.data;
        problem = parsed.error.issues
          .slice(0, 15)
          .map((i) => `${i.path.join(".")} : ${i.message}`)
          .join("\n");
      } catch (e) {
        problem = e instanceof Error ? e.message : String(e);
      }
      messages.push(
        { role: "assistant", content: res.text },
        { role: "user", content: `Ta réponse ne respecte pas le format attendu :\n${problem}\nRenvoie l’objet JSON complet corrigé.` },
      );
    }
    throw new LlmError("Le modèle n’a pas produit un JSON conforme après deux essais");
  }

  async function step<T>(id: string, run: () => Promise<T>, detail?: (r: T) => string): Promise<T> {
    await opts.onStep?.(id, "running");
    try {
      const r = await run();
      await opts.onStep?.(id, "done", detail?.(r));
      return r;
    } catch (e) {
      await opts.onStep?.(id, "failed", e instanceof Error ? e.message : String(e));
      throw e;
    }
  }

  // 1. Bible, menaces, fiches
  const cast = await step(
    "cast",
    () => ask(castPrompt(input, opts.ruleset), CastSchema),
    (r) => `${r.entities.length} fiches, ${r.fronts.length} menaces`,
  );
  const entities = dedupeEntities(cast.entities);

  // 2. Scènes, révélations, indices
  const plot = await step(
    "scenes",
    () => ask(scenesPrompt(input, cast.bible, cast.fronts, entities), ScenesSchema),
    (r) => `${r.scenes.length} scènes, ${r.clues.length} indices`,
  );

  // 3. Cartes
  const maps = await step(
    "maps",
    () => ask(mapsPrompt(plot.scenes, entities), MapsSchema),
    (r) => `${r.maps.length} cartes`,
  );

  let story: CampaignStory = {
    bible: { ...cast.bible, startSceneId: plot.startSceneId },
    fronts: cast.fronts,
    scenes: plot.scenes.map((s) => {
      const placement = maps.placements.find((p) => p.sceneId === s.id);
      const battle = maps.battleMaps.find((b) => b.sceneId === s.id);
      return {
        ...s,
        ...(placement ? { mapPlacement: { mapId: placement.mapId, x: placement.x, y: placement.y } } : {}),
        ...(battle ? { battleMapId: battle.mapId } : {}),
      };
    }),
    revelations: plot.revelations,
    clues: plot.clues,
    maps: maps.maps,
  };

  // 4. Vérification et corrections par le LLM
  const ctx = { entityIds: new Set(entities.map((e) => e.ref)), ruleset: opts.ruleset };
  let repairs = 0;
  const result = await step(
    "check",
    async () => {
      let issues = validateStory(story, ctx);
      for (let i = 0; i < MAX_REPAIRS && needsRepair(issues); i++) {
        repairs++;
        const fixed = await ask(repairPrompt(JSON.stringify(story), issues), StorySchema);
        const fixedIssues = validateStory(fixed, ctx);
        // On ne garde la correction que si elle n'aggrave pas les erreurs.
        if (countErrors(fixedIssues) <= countErrors(issues)) {
          story = fixed;
          issues = fixedIssues;
        }
      }
      return issues;
    },
    (issues) =>
      (issues.length === 0
        ? "aucun problème"
        : `${countErrors(issues)} erreur(s), ${issues.length - countErrors(issues)} avertissement(s)`) +
      (repairs ? ` après ${repairs} correction(s)` : ""),
  );

  const draft: CampaignDraft = { entities, story };
  return { draft, issues: result, usage };
}

function countErrors(issues: StoryIssue[]): number {
  return issues.filter((i) => i.severity === "error").length;
}

/** Les erreurs bloquent ; parmi les avertissements, seuls les défauts de structure justifient une correction. */
function needsRepair(issues: StoryIssue[]): boolean {
  return issues.some(
    (i) =>
      i.severity === "error" ||
      /trois indices|inaccessible|même scène|aucun indice/.test(i.message),
  );
}

/** Références en double : la première définition gagne. */
function dedupeEntities(list: DraftEntity[]): DraftEntity[] {
  const seen = new Set<string>();
  return list.filter((e) => (seen.has(e.ref) ? false : (seen.add(e.ref), true)));
}
