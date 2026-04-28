import { z } from "zod";
import { EffectSchema } from "./entity-schemas";
import { DamageTypeSchema } from "./entity-schemas";

export const AttackActionSchema = z.object({
  kind: z.literal("attack"),
  attackerId: z.string(),
  targetIds: z.array(z.string()).min(1),
  attackBonus: z.number().int(),
  damageNotation: z.string(),
  damageType: DamageTypeSchema,
  meleeWithin5ft: z.boolean().optional(),
});

export const CastSpellActionSchema = z.object({
  kind: z.literal("cast_spell"),
  casterId: z.string(),
  spellEntityId: z.string(),
  targetIds: z.array(z.string()).default([]),
});

export const ApplyEntityEffectsActionSchema = z.object({
  kind: z.literal("apply_entity_effects"),
  entityId: z.string(),
  casterId: z.string().optional(),
  targetIds: z.array(z.string()).default([]),
});

export const RawEffectsActionSchema = z.object({
  kind: z.literal("raw_effects"),
  casterId: z.string().optional(),
  targetIds: z.array(z.string()).default([]),
  effects: z.array(EffectSchema),
});

export const ActionSchema = z.discriminatedUnion("kind", [
  AttackActionSchema,
  CastSpellActionSchema,
  ApplyEntityEffectsActionSchema,
  RawEffectsActionSchema,
]);

export type ActionInput = z.infer<typeof ActionSchema>;
