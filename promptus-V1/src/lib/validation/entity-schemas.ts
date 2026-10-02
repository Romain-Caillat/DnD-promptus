import { z } from "zod";
import type { Effect } from "@/lib/engine/types";

// ----------------------------------------------------------------------------
// Primitives
// ----------------------------------------------------------------------------

// Ids définis par le ruleset de la campagne (validés contre lui à l’usage).
export const StatSchema = z.string().min(1);

export const DamageTypeSchema = z.string().min(1);

export const VisibilitySchema = z.enum([
  "public",
  "mj_only",
  "players_in_session",
  "specific_users",
]);

export const EntityTypeSchema = z.enum([
  "spell",
  "item",
  "npc",
  "monster",
  "character",
  "location",
  "event",
  "condition",
  "faction",
]);

export const TargetSpecSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("self") }),
  z.object({ type: z.literal("caster") }),
  z.object({ type: z.literal("single"), entityId: z.string() }),
  z.object({ type: z.literal("multiple"), entityIds: z.array(z.string()) }),
  z.object({ type: z.literal("all_in_area") }),
  z.object({ type: z.literal("all_players") }),
  z.object({ type: z.literal("all_enemies") }),
]);

export const ResourceKindSchema = z.string().min(1);

// ----------------------------------------------------------------------------
// Effects (recursive: roll_check has sub-effects)
// ----------------------------------------------------------------------------

const DurationSchema = z
  .object({
    rounds: z.number().int().nonnegative().optional(),
    minutes: z.number().int().nonnegative().optional(),
  })
  .partial();

// We define non-recursive effect first, then patch in roll_check with z.lazy
const baseEffectSchemas = [
  z.object({
    type: z.literal("damage"),
    amount: z.string(),
    damageType: DamageTypeSchema,
    target: TargetSpecSchema,
  }),
  z.object({
    type: z.literal("heal"),
    amount: z.string(),
    target: TargetSpecSchema,
  }),
  z.object({
    type: z.literal("apply_condition"),
    conditionId: z.string(),
    duration: DurationSchema.optional(),
    target: TargetSpecSchema,
    save: z
      .object({
        stat: StatSchema,
        dc: z.union([z.string(), z.number()]),
      })
      .optional(),
  }),
  z.object({
    type: z.literal("remove_condition"),
    conditionId: z.string(),
    target: TargetSpecSchema,
  }),
  z.object({
    type: z.literal("modify_stat"),
    stat: z.string(),
    modifier: z.number(),
    duration: DurationSchema.optional(),
    target: TargetSpecSchema,
  }),
  z.object({
    type: z.literal("consume_resource"),
    resource: ResourceKindSchema,
    amount: z.number().int().nonnegative(),
    level: z.number().int().optional(),
    target: TargetSpecSchema,
  }),
  z.object({
    type: z.literal("restore_resource"),
    resource: ResourceKindSchema,
    amount: z.number().int().nonnegative(),
    level: z.number().int().optional(),
    target: TargetSpecSchema,
  }),
  z.object({
    type: z.literal("set_state"),
    entityId: z.string(),
    attribute: z.string(),
    value: z.unknown(),
  }),
  z.object({
    type: z.literal("move_entity"),
    entityId: z.string(),
    toLocationId: z.string(),
  }),
  z.object({
    type: z.literal("reveal_entity"),
    entityId: z.string(),
    toUsers: z.union([
      z.literal("all_players"),
      z.literal("mj_only"),
      z.array(z.string()),
    ]),
  }),
  z.object({
    type: z.literal("set_relation"),
    fromId: z.string(),
    toId: z.string(),
    disposition: z.string().optional(),
    delta: z.number(),
  }),
  z.object({
    type: z.literal("trigger_event"),
    eventId: z.string(),
  }),
  z.object({
    type: z.literal("add_to_inventory"),
    itemId: z.string(),
    target: TargetSpecSchema,
    quantity: z.number().int().positive().optional(),
  }),
  z.object({
    type: z.literal("remove_from_inventory"),
    itemId: z.string(),
    target: TargetSpecSchema,
    quantity: z.number().int().positive().optional(),
  }),
  z.object({
    type: z.literal("play_ambience"),
    ambienceId: z.string(),
  }),
  z.object({
    type: z.literal("play_music"),
    musicId: z.string(),
  }),
  z.object({
    type: z.literal("play_sound"),
    soundId: z.string(),
  }),
  z.object({
    type: z.literal("display_image"),
    imageId: z.string().optional(),
    prompt: z.string().optional(),
  }),
  z.object({
    type: z.literal("display_text"),
    text: z.string(),
  }),
];

// Recursive effect schema using z.lazy for roll_check
export const EffectSchema: z.ZodType<Effect> = z.lazy(() =>
  z.union([
    ...baseEffectSchemas,
    z.object({
      type: z.literal("roll_check"),
      stat: StatSchema,
      dc: z.union([z.string(), z.number()]),
      target: TargetSpecSchema,
      outcomeSuccess: z.array(EffectSchema).optional(),
      outcomeFail: z.array(EffectSchema).optional(),
    }),
  ]) as unknown as z.ZodType<Effect>,
);

// ----------------------------------------------------------------------------
// Entity (input shape for create/update)
// ----------------------------------------------------------------------------

export const EntityInputSchema = z.object({
  id: z.string().optional(), // server-generated if absent
  campaignId: z.string(),
  type: EntityTypeSchema,
  name: z.string().min(1).max(200),
  description: z.string().optional(),
  imageUrl: z.string().url().optional().or(z.literal("")),
  tags: z.array(z.string()).default([]),
  attributes: z.record(z.string(), z.unknown()).default({}),
  effects: z.array(EffectSchema).default([]),
  visibility: VisibilitySchema.default("public"),
});

// Update schema: every field truly optional, no defaults to avoid wiping
// existing values when the client sends a partial payload.
export const EntityUpdateSchema = z.object({
  id: z.string(),
  type: EntityTypeSchema.optional(),
  name: z.string().min(1).max(200).optional(),
  description: z.string().optional(),
  imageUrl: z.union([z.string().url(), z.literal("")]).optional(),
  tags: z.array(z.string()).optional(),
  attributes: z.record(z.string(), z.unknown()).optional(),
  effects: z.array(EffectSchema).optional(),
  visibility: VisibilitySchema.optional(),
});

// ----------------------------------------------------------------------------
// Bulk YAML import
// ----------------------------------------------------------------------------

export const EntityBulkImportSchema = z.object({
  entities: z.array(EntityInputSchema.omit({ campaignId: true })),
});

// ----------------------------------------------------------------------------
// Style guide
// ----------------------------------------------------------------------------

export const StyleGuideSchema = z.object({
  artStyle: z.string().optional(),
  palette: z.string().optional(),
  mood: z.string().optional(),
  promptPrefix: z.string().optional(),
  promptSuffix: z.string().optional(),
  negativePrompt: z.string().optional(),
});

// ----------------------------------------------------------------------------
// Campaign
// ----------------------------------------------------------------------------

export const CampaignInputSchema = z.object({
  name: z.string().min(1).max(200),
  description: z.string().optional(),
  styleGuide: StyleGuideSchema.optional(),
  systemTemplate: z.string().default("dnd5e"),
});

export const CampaignUpdateSchema = CampaignInputSchema.partial();
