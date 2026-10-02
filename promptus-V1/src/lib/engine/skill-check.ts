// Test de compétence / caractéristique d'un personnage, selon le ruleset.

import { computeRollFlags } from "./conditions";
import type { Rng } from "./dice";
import { abilityLabel, conditionsById, defaultAbilityScore, rollCheck, skillLabel, type CheckRoll, type Ruleset } from "./ruleset";
import type { ActiveCondition } from "./types";

export interface SkillCheckInput {
  ruleset: Ruleset;
  name: string;
  attributes: Record<string, unknown>;
  conditions: ActiveCondition[];
  skill?: string;
  ability?: string;
  dc: number;
  rng?: Rng;
}

export interface SkillCheckResult extends CheckRoll {
  ability: string;
  description: string;
}

/**
 * Bonus de compétence : `skillBonuses[skill]` sur la fiche s'il existe,
 * sinon maîtrise (`proficientSkills` contient la compétence) = `proficiencyBonus`.
 */
export function resolveSkillCheck(input: SkillCheckInput): SkillCheckResult {
  const { ruleset, attributes } = input;
  const skill = input.skill ? ruleset.skills.find((s) => s.id === input.skill) : undefined;
  const ability = skill?.ability ?? input.ability ?? ruleset.abilities[0].id;
  const scores = (attributes.abilityScores as Record<string, number> | undefined) ?? {};
  const score = scores[ability] ?? defaultAbilityScore(ruleset);

  let bonus = 0;
  if (skill) {
    const explicit = (attributes.skillBonuses as Record<string, number> | undefined)?.[skill.id];
    const proficient = Array.isArray(attributes.proficientSkills) && attributes.proficientSkills.includes(skill.id);
    bonus = explicit ?? (proficient ? Number(attributes.proficiencyBonus ?? 0) : 0);
  }

  const flags = computeRollFlags({ actorConditions: input.conditions, kind: "check", conditions: conditionsById(ruleset) });
  const roll = flags.autoFail
    ? { roll: { notation: "échec automatique", result: 0, rolls: [] }, natural: 0, success: false, critical: false, fumble: false }
    : rollCheck(ruleset, { score, bonus, dc: input.dc, advantage: flags.advantage, disadvantage: flags.disadvantage, rng: input.rng });

  const what = skill ? `${skillLabel(ruleset, skill.id)} (${abilityLabel(ruleset, ability)})` : abilityLabel(ruleset, ability);
  const verdict = roll.critical ? "réussite critique" : roll.fumble ? "échec critique" : roll.success ? "réussite" : "échec";
  return {
    ...roll,
    ability,
    description: `🎲 ${input.name} — ${what} DD ${input.dc} : ${roll.roll.notation} = ${roll.roll.result} → ${verdict}`,
  };
}
