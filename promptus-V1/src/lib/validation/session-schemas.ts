import { z } from "zod";

export const PhaseSchema = z.enum([
  "exploration",
  "combat",
  "dialogue",
  "travel",
  "rest",
]);

export const StartSessionSchema = z.object({
  name: z.string().min(1).max(200),
  participantEntityIds: z.array(z.string()).min(1),
});

export const InitiativeEntrySchema = z.object({
  entityId: z.string(),
  initiative: z.number().int(),
  isPlayer: z.boolean(),
});

export const PhaseUpdateSchema = z.object({
  phase: PhaseSchema,
});

export const InitiativeUpdateSchema = z.object({
  initiativeOrder: z.array(InitiativeEntrySchema),
  activeTurnIndex: z.number().int().nonnegative().optional(),
  combatRound: z.number().int().nonnegative().optional(),
});

export const HpUpdateSchema = z.object({
  delta: z.number().int(),
});

export const ConditionApplySchema = z.object({
  conditionId: z.string(),
  durationRounds: z.number().int().nonnegative().optional(),
});
