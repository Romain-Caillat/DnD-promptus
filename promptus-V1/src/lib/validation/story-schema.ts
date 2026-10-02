import { z } from "zod";
import type { CampaignStory, WorldCondition } from "@/lib/engine/story";
import { EffectSchema } from "./entity-schemas";

const IdSchema = z.string().min(1).max(80);
const Text = z.string().default("");
const PhaseSchema = z.enum(["exploration", "combat", "dialogue", "travel", "rest"]);
const Coord = z.number().int().nonnegative();

export const WorldConditionSchema: z.ZodType<WorldCondition> = z.lazy(() =>
  z.union([
    z.object({ all: z.array(WorldConditionSchema) }).strict(),
    z.object({ any: z.array(WorldConditionSchema) }).strict(),
    z.object({ not: WorldConditionSchema }).strict(),
    z.object({ flag: z.string().min(1), equals: z.unknown().optional() }).strict(),
    z.object({ sceneStatus: z.object({ sceneId: IdSchema, status: z.enum(["available", "visited", "resolved"]) }) }).strict(),
    z.object({ clueFound: IdSchema }).strict(),
    z.object({ revelationKnown: IdSchema }).strict(),
    z.object({ frontStepAtLeast: z.object({ frontId: IdSchema, step: z.number().int().nonnegative() }) }).strict(),
    z.object({ entityAttribute: z.object({ entityId: IdSchema, attribute: z.string().min(1), equals: z.unknown() }) }).strict(),
  ]),
) as z.ZodType<WorldCondition>;

export const StorySchema = z.object({
  bible: z.object({
    pitch: Text,
    tone: Text,
    themes: z.array(z.string()).default([]),
    truths: z.array(z.string()).default([]),
    secrets: z.array(z.string()).default([]),
    playerHook: Text,
    startSceneId: IdSchema.optional(),
  }),
  fronts: z
    .array(
      z.object({
        id: IdSchema,
        name: z.string().min(1),
        goal: Text,
        description: Text,
        steps: z.array(z.object({ label: z.string().min(1), description: Text, effects: z.array(EffectSchema).optional() })),
      }),
    )
    .default([]),
  scenes: z
    .array(
      z.object({
        id: IdSchema,
        title: z.string().min(1),
        summary: Text,
        objective: Text,
        readAloud: Text,
        gmNotes: z.string().optional(),
        phase: PhaseSchema.default("exploration"),
        locationEntityId: IdSchema.optional(),
        npcEntityIds: z.array(IdSchema).default([]),
        monsterEntityIds: z.array(IdSchema).default([]),
        exits: z.array(z.object({ toSceneId: IdSchema, label: Text })).default([]),
        mapPlacement: z.object({ mapId: IdSchema, x: Coord, y: Coord }).optional(),
        battleMapId: IdSchema.optional(),
        triggers: z
          .array(
            z.object({
              id: IdSchema,
              label: z.string().min(1),
              when: WorldConditionSchema,
              effects: z.array(EffectSchema).default([]),
              oneShot: z.boolean().optional(),
            }),
          )
          .default([]),
        media: z
          .object({
            imagePrompt: z.string().optional(),
            imageUrl: z.string().optional(),
            videoPrompt: z.string().optional(),
            videoUrl: z.string().optional(),
            musicUrl: z.string().optional(),
            musicQuery: z.string().optional(),
          })
          .optional(),
      }),
    )
    .default([]),
  revelations: z
    .array(z.object({ id: IdSchema, statement: z.string().min(1), importance: z.enum(["critical", "optional"]).default("critical") }))
    .default([]),
  clues: z
    .array(
      z.object({
        id: IdSchema,
        revelationId: IdSchema,
        sceneId: IdSchema,
        text: z.string().min(1),
        discovery: Text,
        check: z
          .object({ skill: IdSchema.optional(), ability: IdSchema.optional(), dc: z.number().int().optional() })
          .optional(),
      }),
    )
    .default([]),
  maps: z
    .array(
      z.object({
        id: IdSchema,
        name: z.string().min(1),
        level: z.enum(["campaign", "region", "local"]),
        grid: z.object({
          type: z.enum(["hex", "square"]),
          cols: z.number().int().min(1).max(200),
          rows: z.number().int().min(1).max(200),
        }),
        backgroundPrompt: z.string().optional(),
        backgroundUrl: z.string().optional(),
        cells: z
          .array(
            z.object({
              x: Coord,
              y: Coord,
              terrain: z.string().optional(),
              blocked: z.boolean().optional(),
              label: z.string().optional(),
              sceneId: IdSchema.optional(),
              childMapId: IdSchema.optional(),
            }),
          )
          .default([]),
        tokens: z.array(z.object({ entityId: IdSchema, x: Coord, y: Coord })).optional(),
      }),
    )
    .default([]),
});

export type StoryInput = z.infer<typeof StorySchema>;
const _check: (s: StoryInput) => CampaignStory = (s) => s;
void _check;
