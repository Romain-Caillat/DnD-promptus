// Système de règles défini par le MJ — des données, pas du code.
// Le moteur (résolveur, initiative, états) lit ce ruleset au lieu de règles
// D&D codées en dur. Le préréglage D&D 5e (SRD) sert de point de départ.

import { CONDITIONS } from "./conditions";
import {
  DAMAGE_TYPE_LABELS,
  RESOURCE_LABELS,
  type PhaseId,
} from "./catalog";
import { rollDice, type Rng, defaultRng } from "./dice";
import type { ConditionDefinition, RollDetail } from "./types";

// ----------------------------------------------------------------------------
// Types
// ----------------------------------------------------------------------------

export interface RulesetAbility {
  /** Identifiant stable, utilisé dans `abilityScores` des fiches (ex. "STR"). */
  id: string;
  label: string;
  abbr: string;
}

export interface RulesetSkill {
  id: string;
  label: string;
  /** Caractéristique associée (id d'une `RulesetAbility`). */
  ability: string;
}

export interface RulesetCheck {
  /** Dés lancés pour un test : "1d20", "2d6", "1d100"… */
  dice: string;
  /**
   * roll_over : réussite si total (dés + modificateur) ≥ difficulté.
   * roll_under : réussite si dés ≤ valeur de caractéristique − difficulté.
   */
  mode: "roll_over" | "roll_under";
  /**
   * Comment une valeur de caractéristique devient un modificateur (roll_over) :
   * dnd = ⌊(valeur − 10) / 2⌋, direct = la valeur est le modificateur, none = 0.
   */
  abilityModifier: "dnd" | "direct" | "none";
  /** Résultat naturel des dés à partir duquel le jet est critique. */
  criticalOn?: number;
  /** Résultat naturel des dés à partir duquel le jet est un échec critique. */
  fumbleOn?: number;
  /** Avantage / désavantage : lancer deux fois et garder le meilleur / le pire. */
  advantage: boolean;
}

export interface RulesetAction {
  id: string;
  label: string;
  phases: PhaseId[];
  kind: "attack" | "spell" | "check" | "move" | "custom";
  ability?: string;
  skill?: string;
  description?: string;
}

export interface TravelPace {
  id: string;
  label: string;
  cellsPerDay: number;
}

export interface RulesetMovement {
  /** Carte de campagne (hexagones). */
  campaign: { cellKm: number; paces: TravelPace[] };
  /** Carte de région (hexagones). */
  region: { cellMeters: number; cellsPerHour: number };
  /** Carte de combat / lieu (carrés). */
  local: {
    cellMeters: number;
    defaultSpeedCells: number;
    /** chebyshev : diagonale = 1 case ; alternate : 1 puis 2 ; euclidean : √2. */
    diagonal: "chebyshev" | "alternate" | "euclidean";
  };
}

export interface Ruleset {
  name: string;
  abilities: RulesetAbility[];
  skills: RulesetSkill[];
  check: RulesetCheck;
  initiative: { dice: string; ability?: string };
  conditions: ConditionDefinition[];
  resources: { id: string; label: string }[];
  damageTypes: { id: string; label: string }[];
  movement: RulesetMovement;
  actions: RulesetAction[];
  combat: { roundSeconds: number };
}

// ----------------------------------------------------------------------------
// Préréglage D&D 5e (SRD 5.1)
// ----------------------------------------------------------------------------

export const DND5E_RULESET: Ruleset = {
  name: "D&D 5e (SRD)",
  abilities: [
    { id: "STR", label: "Force", abbr: "FOR" },
    { id: "DEX", label: "Dextérité", abbr: "DEX" },
    { id: "CON", label: "Constitution", abbr: "CON" },
    { id: "INT", label: "Intelligence", abbr: "INT" },
    { id: "WIS", label: "Sagesse", abbr: "SAG" },
    { id: "CHA", label: "Charisme", abbr: "CHA" },
  ],
  skills: [
    { id: "acrobatics", label: "Acrobaties", ability: "DEX" },
    { id: "arcana", label: "Arcanes", ability: "INT" },
    { id: "athletics", label: "Athlétisme", ability: "STR" },
    { id: "stealth", label: "Discrétion", ability: "DEX" },
    { id: "animal_handling", label: "Dressage", ability: "WIS" },
    { id: "sleight_of_hand", label: "Escamotage", ability: "DEX" },
    { id: "history", label: "Histoire", ability: "INT" },
    { id: "intimidation", label: "Intimidation", ability: "CHA" },
    { id: "investigation", label: "Investigation", ability: "INT" },
    { id: "medicine", label: "Médecine", ability: "WIS" },
    { id: "nature", label: "Nature", ability: "INT" },
    { id: "perception", label: "Perception", ability: "WIS" },
    { id: "insight", label: "Perspicacité", ability: "WIS" },
    { id: "persuasion", label: "Persuasion", ability: "CHA" },
    { id: "religion", label: "Religion", ability: "INT" },
    { id: "performance", label: "Représentation", ability: "CHA" },
    { id: "survival", label: "Survie", ability: "WIS" },
    { id: "deception", label: "Tromperie", ability: "CHA" },
  ],
  check: {
    dice: "1d20",
    mode: "roll_over",
    abilityModifier: "dnd",
    criticalOn: 20,
    fumbleOn: 1,
    advantage: true,
  },
  initiative: { dice: "1d20", ability: "DEX" },
  conditions: Object.values(CONDITIONS),
  resources: Object.entries(RESOURCE_LABELS).map(([id, label]) => ({ id, label })),
  damageTypes: Object.entries(DAMAGE_TYPE_LABELS).map(([id, label]) => ({ id, label })),
  movement: {
    campaign: {
      cellKm: 10,
      paces: [
        { id: "slow", label: "Lente (discrète)", cellsPerDay: 3 },
        { id: "normal", label: "Normale", cellsPerDay: 4 },
        { id: "fast", label: "Rapide (−5 Perception)", cellsPerDay: 5 },
      ],
    },
    region: { cellMeters: 500, cellsPerHour: 10 },
    local: { cellMeters: 1.5, defaultSpeedCells: 6, diagonal: "chebyshev" },
  },
  actions: [
    { id: "attack", label: "Attaquer", phases: ["combat"], kind: "attack" },
    { id: "cast_spell", label: "Lancer un sort", phases: ["combat", "exploration", "dialogue"], kind: "spell" },
    { id: "dash", label: "Foncer", phases: ["combat"], kind: "move", description: "Double le déplacement ce tour-ci." },
    { id: "dodge", label: "Esquiver", phases: ["combat"], kind: "custom", description: "Les attaques contre vous ont un désavantage jusqu’à votre prochain tour." },
    { id: "hide", label: "Se cacher", phases: ["combat", "exploration"], kind: "check", skill: "stealth" },
    { id: "search", label: "Fouiller", phases: ["exploration"], kind: "check", skill: "investigation" },
    { id: "perceive", label: "Observer", phases: ["exploration", "combat"], kind: "check", skill: "perception" },
    { id: "persuade", label: "Persuader", phases: ["dialogue"], kind: "check", skill: "persuasion" },
    { id: "deceive", label: "Mentir", phases: ["dialogue"], kind: "check", skill: "deception" },
    { id: "intimidate", label: "Intimider", phases: ["dialogue"], kind: "check", skill: "intimidation" },
    { id: "read_intent", label: "Deviner les intentions", phases: ["dialogue"], kind: "check", skill: "insight" },
    { id: "travel", label: "Voyager", phases: ["travel"], kind: "move" },
    { id: "short_rest", label: "Repos court", phases: ["rest"], kind: "custom", description: "1 heure : dépenser des dés de vie." },
    { id: "long_rest", label: "Repos long", phases: ["rest"], kind: "custom", description: "8 heures : récupère PV et emplacements." },
  ],
  combat: { roundSeconds: 6 },
};

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------

export function abilityLabel(ruleset: Ruleset, id: string): string {
  return ruleset.abilities.find((a) => a.id === id)?.abbr ?? id;
}

export function skillLabel(ruleset: Ruleset, id: string): string {
  return ruleset.skills.find((s) => s.id === id)?.label ?? id;
}

export function damageTypeLabel(ruleset: Ruleset, id: string): string {
  return ruleset.damageTypes.find((d) => d.id === id)?.label ?? id;
}

export function resourceLabel(ruleset: Ruleset, id: string): string {
  return ruleset.resources.find((r) => r.id === id)?.label ?? id;
}

export function conditionsById(ruleset: Ruleset): Record<string, ConditionDefinition> {
  return Object.fromEntries(ruleset.conditions.map((c) => [c.id, c]));
}

export function rulesetConditionLabel(ruleset: Ruleset, id: string): string {
  return ruleset.conditions.find((c) => c.id === id)?.name ?? id;
}

export function abilityModifier(ruleset: Ruleset, score: number | undefined): number {
  if (score === undefined) return 0;
  switch (ruleset.check.abilityModifier) {
    case "dnd":
      return Math.floor((score - 10) / 2);
    case "direct":
      return score;
    case "none":
      return 0;
  }
}

/** Valeur par défaut d'une caractéristique absente de la fiche. */
export function defaultAbilityScore(ruleset: Ruleset): number {
  return ruleset.check.abilityModifier === "dnd" ? 10 : 0;
}

export interface CheckRoll {
  roll: RollDetail;
  /** Somme des dés gardés, avant modificateur. */
  natural: number;
  success: boolean;
  critical: boolean;
  fumble: boolean;
}

/**
 * Jet de test générique selon le ruleset.
 * roll_over : total = dés + modificateur(score) + bonus ; réussite si ≥ dc.
 * roll_under : réussite si dés ≤ score − dc (dc = malus de difficulté).
 */
export function rollCheck(
  ruleset: Ruleset,
  opts: {
    score?: number;
    bonus?: number;
    dc: number;
    advantage?: boolean;
    disadvantage?: boolean;
    rng?: Rng;
  },
): CheckRoll {
  const rng = opts.rng ?? defaultRng;
  const { check } = ruleset;
  const useAdv = check.advantage && !!opts.advantage && !opts.disadvantage;
  const useDis = check.advantage && !!opts.disadvantage && !opts.advantage;

  const first = rollDice(check.dice, { rng });
  let kept = first;
  let allRolls = first.rolls ?? [];
  if (useAdv || useDis) {
    const second = rollDice(check.dice, { rng });
    allRolls = [...allRolls, ...(second.rolls ?? [])];
    // « Meilleur » dépend du sens du jet : haut en roll_over, bas en roll_under.
    const secondIsBetter =
      check.mode === "roll_over" ? second.result > first.result : second.result < first.result;
    const secondIsWorse =
      check.mode === "roll_over" ? second.result < first.result : second.result > first.result;
    if ((useAdv && secondIsBetter) || (useDis && secondIsWorse)) kept = second;
  }
  const natural = kept.result;
  const critical = check.criticalOn !== undefined
    && (check.mode === "roll_over" ? natural >= check.criticalOn : natural <= check.criticalOn);
  const fumble = check.fumbleOn !== undefined
    && (check.mode === "roll_over" ? natural <= check.fumbleOn : natural >= check.fumbleOn);

  if (check.mode === "roll_under") {
    const target = (opts.score ?? 0) + (opts.bonus ?? 0) - opts.dc;
    return {
      roll: {
        notation: `${check.dice} ≤ ${target}`,
        result: natural,
        rolls: allRolls,
        modifier: 0,
        advantage: useAdv || undefined,
        disadvantage: useDis || undefined,
      },
      natural,
      success: !fumble && (critical || natural <= target),
      critical,
      fumble,
    };
  }

  const modifier = abilityModifier(ruleset, opts.score) + (opts.bonus ?? 0);
  const total = natural + modifier;
  return {
    roll: {
      notation: `${check.dice}${modifier === 0 ? "" : modifier > 0 ? `+${modifier}` : modifier}`,
      result: total,
      rolls: allRolls,
      modifier,
      advantage: useAdv || undefined,
      disadvantage: useDis || undefined,
    },
    natural,
    success: total >= opts.dc,
    critical,
    fumble,
  };
}

/** Jet d'initiative selon le ruleset : dés + modificateur de caractéristique + bonus. */
export function rollInitiative(
  ruleset: Ruleset,
  opts: { abilityScores?: Record<string, number>; bonus?: number; rng?: Rng },
): number {
  const dice = rollDice(ruleset.initiative.dice, { rng: opts.rng });
  const ability = ruleset.initiative.ability;
  // `initiativeBonus` sur la fiche remplace le modificateur de caractéristique
  // quand il est renseigné (comportement historique des fiches V1).
  if (opts.bonus !== undefined) return dice.result + opts.bonus;
  const score = ability ? opts.abilityScores?.[ability] : undefined;
  return dice.result + (ability ? abilityModifier(ruleset, score) : 0);
}
