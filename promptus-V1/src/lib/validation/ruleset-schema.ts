import { z } from "zod";
import type { Ruleset } from "@/lib/engine/ruleset";

const IdSchema = z.string().min(1).max(64);
const DiceSchema = z.string().regex(/^\d+d\d+([+-]\d+)?$/i, "Notation de dés attendue (ex. 1d20, 2d6)");
const PhaseSchema = z.enum(["exploration", "combat", "dialogue", "travel", "rest"]);

const ConditionModifierSchema = z.object({
  trigger: z.enum([
    "outgoing_attack",
    "incoming_attack",
    "outgoing_save",
    "incoming_save",
    "outgoing_check",
    "incoming_check",
    "outgoing_attack_within_5ft",
    "incoming_attack_within_5ft",
    "incapacitated",
  ]),
  saveStat: z.array(IdSchema).optional(),
  effect: z.enum(["advantage", "disadvantage", "auto_fail", "auto_success", "auto_critical", "true"]),
});

export const RulesetSchema = z
  .object({
    name: z.string().min(1).max(120),
    abilities: z.array(z.object({ id: IdSchema, label: z.string().min(1), abbr: z.string().min(1).max(8) })).min(1),
    skills: z.array(z.object({ id: IdSchema, label: z.string().min(1), ability: IdSchema })).default([]),
    check: z.object({
      dice: DiceSchema,
      mode: z.enum(["roll_over", "roll_under"]),
      abilityModifier: z.enum(["dnd", "direct", "none"]),
      criticalOn: z.number().int().optional(),
      fumbleOn: z.number().int().optional(),
      advantage: z.boolean(),
    }),
    initiative: z.object({ dice: DiceSchema, ability: IdSchema.optional() }),
    conditions: z
      .array(
        z.object({
          id: IdSchema,
          name: z.string().min(1),
          description: z.string().default(""),
          modifiers: z.array(ConditionModifierSchema).default([]),
        }),
      )
      .default([]),
    resources: z.array(z.object({ id: IdSchema, label: z.string().min(1) })).default([]),
    damageTypes: z.array(z.object({ id: IdSchema, label: z.string().min(1) })).default([]),
    movement: z.object({
      campaign: z.object({
        cellKm: z.number().positive(),
        paces: z.array(z.object({ id: IdSchema, label: z.string().min(1), cellsPerDay: z.number().positive() })).min(1),
      }),
      region: z.object({ cellMeters: z.number().positive(), cellsPerHour: z.number().positive() }),
      local: z.object({
        cellMeters: z.number().positive(),
        defaultSpeedCells: z.number().int().positive(),
        diagonal: z.enum(["chebyshev", "alternate", "euclidean"]),
      }),
    }),
    actions: z
      .array(
        z.object({
          id: IdSchema,
          label: z.string().min(1),
          phases: z.array(PhaseSchema).min(1),
          kind: z.enum(["attack", "spell", "check", "move", "custom"]),
          ability: IdSchema.optional(),
          skill: IdSchema.optional(),
          description: z.string().optional(),
        }),
      )
      .default([]),
    combat: z.object({ roundSeconds: z.number().int().positive() }),
  })
  .superRefine((r, ctx) => {
    // Cohérence interne : toutes les références pointent vers des ids déclarés.
    const abilities = new Set(r.abilities.map((a) => a.id));
    const skills = new Set(r.skills.map((s) => s.id));
    const dup = (list: { id: string }[], path: string) => {
      const seen = new Set<string>();
      list.forEach((x, i) => {
        if (seen.has(x.id)) ctx.addIssue({ code: "custom", path: [path, i, "id"], message: `Id en double : ${x.id}` });
        seen.add(x.id);
      });
    };
    dup(r.abilities, "abilities");
    dup(r.skills, "skills");
    dup(r.conditions, "conditions");
    dup(r.actions, "actions");
    r.skills.forEach((s, i) => {
      if (!abilities.has(s.ability))
        ctx.addIssue({ code: "custom", path: ["skills", i, "ability"], message: `Caractéristique inconnue : ${s.ability}` });
    });
    if (r.initiative.ability && !abilities.has(r.initiative.ability))
      ctx.addIssue({ code: "custom", path: ["initiative", "ability"], message: `Caractéristique inconnue : ${r.initiative.ability}` });
    r.actions.forEach((a, i) => {
      if (a.ability && !abilities.has(a.ability))
        ctx.addIssue({ code: "custom", path: ["actions", i, "ability"], message: `Caractéristique inconnue : ${a.ability}` });
      if (a.skill && !skills.has(a.skill))
        ctx.addIssue({ code: "custom", path: ["actions", i, "skill"], message: `Compétence inconnue : ${a.skill}` });
    });
  });

// Garde-fou : le schéma doit produire exactement le type du moteur.
export type RulesetInput = z.infer<typeof RulesetSchema>;
const _check: (r: RulesetInput) => Ruleset = (r) => r;
void _check;
