import { z } from "zod";
import type { CampaignStory } from "@/lib/engine/story";
import type { StoryIssue } from "@/lib/engine/story-validator";
import type { LlmUsage } from "@/lib/ai/llm";

// ----------------------------------------------------------------------------
// Demande du MJ
// ----------------------------------------------------------------------------

export const GenerationInputSchema = z.object({
  pitch: z.string().min(10, "Décrivez l’idée en une ou deux phrases au moins").max(4000),
  tone: z.string().max(500).default(""),
  players: z.number().int().min(1).max(10).default(4),
  level: z.string().max(50).default("1"),
  length: z.enum(["one_shot", "short", "long"]).default("one_shot"),
  themes: z.string().max(500).default(""),
  constraints: z.string().max(2000).default(""),
  /** Génère des personnages prêts à jouer. */
  pregens: z.boolean().default(false),
  model: z.string().max(200).optional(),
});
export type GenerationInput = z.infer<typeof GenerationInputSchema>;

export const LENGTH_LABELS: Record<GenerationInput["length"], string> = {
  one_shot: "Partie unique (3-4 h)",
  short: "Courte (3-5 sessions)",
  long: "Longue (8+ sessions)",
};

// ----------------------------------------------------------------------------
// Brouillon produit par le LLM
// ----------------------------------------------------------------------------

/** Référence de fiche dans un brouillon, remplacée par un vrai id à l'application. */
export const ENTITY_REF = /^ent_[a-z0-9_]+$/;

export const DraftEntitySchema = z.object({
  ref: z.string().regex(ENTITY_REF, "Référence de fiche attendue : ent_… en minuscules"),
  type: z.enum(["npc", "monster", "location", "item", "character", "faction", "event"]),
  name: z.string().min(1).max(200),
  description: z.string().default(""),
  tags: z.array(z.string()).default([]),
  attributes: z.record(z.string(), z.unknown()).default({}),
  visibility: z.enum(["public", "mj_only"]).optional(),
});
export type DraftEntity = z.infer<typeof DraftEntitySchema>;

export interface CampaignDraft {
  entities: DraftEntity[];
  /** Histoire dont les ids de fiches sont des références `ent_…`. */
  story: CampaignStory;
}

// ----------------------------------------------------------------------------
// Suivi d'un job
// ----------------------------------------------------------------------------

export type StepStatus = "pending" | "running" | "done" | "failed";

export interface GenerationStep {
  id: string;
  label: string;
  status: StepStatus;
  detail?: string;
}

export interface GenerationResult {
  draft: CampaignDraft;
  issues: StoryIssue[];
}

export type JobStatus = "running" | "succeeded" | "failed" | "applied";

export interface GenerationJobView {
  id: string;
  campaignId: string;
  status: JobStatus;
  input: GenerationInput;
  steps: GenerationStep[];
  result: GenerationResult | null;
  error: string | null;
  usage: LlmUsage;
  model: string;
  createdAt: string;
  updatedAt: string;
}
