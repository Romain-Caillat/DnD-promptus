import { z } from "zod";
import { EffectSchema } from "./entity-schemas";
import { DamageTypeSchema } from "./entity-schemas";

export const AttackActionSchema = z.object({
  kind: z.literal("attack"),
  attackerId: z.string(),
  targetIds: z.array(z.string()).min(1),
  /** Attaque de la fiche (attributes.attacks) : bonus, dégâts et portée en découlent. */
  attackId: z.string().optional(),
  attackBonus: z.number().int().optional(),
  damageNotation: z.string().optional(),
  damageType: DamageTypeSchema.optional(),
  meleeWithin5ft: z.boolean().optional(),
  /** Le MJ passe outre la portée et la ligne de vue. */
  force: z.boolean().optional(),
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

/** Le MJ valide un déclencheur de scène : ses effets sont appliqués. */
export const FireTriggerActionSchema = z.object({
  kind: z.literal("fire_trigger"),
  sceneId: z.string(),
  triggerId: z.string(),
});

export const ActionSchema = z.discriminatedUnion("kind", [
  AttackActionSchema,
  CastSpellActionSchema,
  ApplyEntityEffectsActionSchema,
  RawEffectsActionSchema,
  FireTriggerActionSchema,
]);

export type ActionInput = z.infer<typeof ActionSchema>;
