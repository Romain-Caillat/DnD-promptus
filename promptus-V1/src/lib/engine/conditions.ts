// 5e standard conditions, declared as data.
// Modifiers are evaluated by the resolver when computing roll flags or
// when checking incapacitation.

import type {
  ActiveCondition,
  ConditionDefinition,
  ConditionModifier,
} from "./types";

export const CONDITIONS: Record<string, ConditionDefinition> = {
  blinded: {
    id: "blinded",
    name: "Aveuglé",
    description: "Ne voit pas ; rate automatiquement les tests liés à la vue ; désavantage à ses attaques ; avantage aux attaques contre lui.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "incoming_attack", effect: "advantage" },
    ],
  },
  charmed: {
    id: "charmed",
    name: "Charmé",
    description: "Ne peut pas attaquer le charmeur ni le cibler avec des capacités nuisibles.",
    modifiers: [],
  },
  deafened: {
    id: "deafened",
    name: "Assourdi",
    description: "N’entend pas ; rate automatiquement les tests liés à l’ouïe.",
    modifiers: [],
  },
  frightened: {
    id: "frightened",
    name: "Effrayé",
    description: "Désavantage aux tests et aux attaques tant que la source de la peur est en vue.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "outgoing_check", effect: "disadvantage" },
    ],
  },
  grappled: {
    id: "grappled",
    name: "Agrippé",
    description: "Vitesse réduite à 0 ; aucun bonus de vitesse.",
    modifiers: [],
  },
  incapacitated: {
    id: "incapacitated",
    name: "Neutralisé",
    description: "Ne peut effectuer ni action ni réaction.",
    modifiers: [{ trigger: "incapacitated", effect: "true" }],
  },
  invisible: {
    id: "invisible",
    name: "Invisible",
    description: "Ne peut pas être vu ; avantage à ses attaques ; désavantage aux attaques contre lui.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "advantage" },
      { trigger: "incoming_attack", effect: "disadvantage" },
    ],
  },
  paralyzed: {
    id: "paralyzed",
    name: "Paralysé",
    description: "Neutralisé ; rate automatiquement les sauvegardes de FOR et DEX ; avantage aux attaques contre lui ; toute attaque au contact à 1,5 m est un critique.",
    modifiers: [
      { trigger: "incapacitated", effect: "true" },
      { trigger: "outgoing_save", saveStat: ["STR", "DEX"], effect: "auto_fail" },
      { trigger: "incoming_attack", effect: "advantage" },
      { trigger: "incoming_attack_within_5ft", effect: "auto_critical" },
    ],
  },
  petrified: {
    id: "petrified",
    name: "Pétrifié",
    description: "Changé en pierre ; neutralisé ; résistance à tous les dégâts ; rate automatiquement les sauvegardes de FOR et DEX ; avantage pour le toucher.",
    modifiers: [
      { trigger: "incapacitated", effect: "true" },
      { trigger: "outgoing_save", saveStat: ["STR", "DEX"], effect: "auto_fail" },
      { trigger: "incoming_attack", effect: "advantage" },
    ],
  },
  poisoned: {
    id: "poisoned",
    name: "Empoisonné",
    description: "Désavantage aux attaques et aux tests de caractéristique.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "outgoing_check", effect: "disadvantage" },
    ],
  },
  prone: {
    id: "prone",
    name: "À terre",
    description: "Désavantage à ses attaques ; avantage aux attaques au contact contre lui ; désavantage aux attaques à distance contre lui.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "incoming_attack_within_5ft", effect: "advantage" },
      // Ranged-against-prone disadvantage is approximated when not within 5ft;
      // the resolver only knows whether the source is within 5ft, not actual distance.
    ],
  },
  restrained: {
    id: "restrained",
    name: "Entravé",
    description: "Vitesse 0 ; désavantage aux attaques et aux sauvegardes de DEX ; avantage aux attaques contre lui.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "outgoing_save", saveStat: ["DEX"], effect: "disadvantage" } as ConditionModifier,
      { trigger: "incoming_attack", effect: "advantage" },
    ],
  },
  stunned: {
    id: "stunned",
    name: "Étourdi",
    description: "Neutralisé ; rate automatiquement les sauvegardes de FOR et DEX ; avantage aux attaques contre lui.",
    modifiers: [
      { trigger: "incapacitated", effect: "true" },
      { trigger: "outgoing_save", saveStat: ["STR", "DEX"], effect: "auto_fail" },
      { trigger: "incoming_attack", effect: "advantage" },
    ],
  },
  unconscious: {
    id: "unconscious",
    name: "Inconscient",
    description: "Neutralisé ; lâche ce qu’il tient ; tombe à terre ; rate automatiquement les sauvegardes de FOR et DEX ; avantage pour le toucher ; toute attaque au contact à 1,5 m est un critique.",
    modifiers: [
      { trigger: "incapacitated", effect: "true" },
      { trigger: "outgoing_save", saveStat: ["STR", "DEX"], effect: "auto_fail" },
      { trigger: "incoming_attack", effect: "advantage" },
      { trigger: "incoming_attack_within_5ft", effect: "auto_critical" },
    ],
  },
};

// ----------------------------------------------------------------------------
// Roll flag computation
// ----------------------------------------------------------------------------

export type RollKind = "attack" | "save" | "check";

export interface RollFlags {
  advantage: boolean;
  disadvantage: boolean;
  autoFail: boolean;
  autoSuccess: boolean;
  autoCritical: boolean;
}

export const NEUTRAL_FLAGS: RollFlags = {
  advantage: false,
  disadvantage: false,
  autoFail: false,
  autoSuccess: false,
  autoCritical: false,
};

export interface ComputeFlagsOpts {
  actorConditions: ActiveCondition[];
  targetConditions?: ActiveCondition[];
  kind: RollKind;
  saveStat?: string;
  meleeWithin5ft?: boolean;
  /** Définitions d’états du ruleset ; par défaut les 14 états 5e. */
  conditions?: Record<string, ConditionDefinition>;
}

export function computeRollFlags(opts: ComputeFlagsOpts): RollFlags {
  const flags: RollFlags = { ...NEUTRAL_FLAGS };
  const defs = opts.conditions ?? CONDITIONS;

  for (const c of opts.actorConditions) {
    const def = defs[c.conditionId];
    if (!def) continue;
    for (const mod of def.modifiers) {
      applyModifier(mod, "outgoing", opts, flags);
    }
  }
  for (const c of opts.targetConditions ?? []) {
    const def = defs[c.conditionId];
    if (!def) continue;
    for (const mod of def.modifiers) {
      applyModifier(mod, "incoming", opts, flags);
    }
  }
  // Combined adv+disadv cancel
  if (flags.advantage && flags.disadvantage) {
    flags.advantage = false;
    flags.disadvantage = false;
  }
  return flags;
}

function applyModifier(
  mod: ConditionModifier,
  side: "outgoing" | "incoming",
  opts: ComputeFlagsOpts,
  flags: RollFlags,
): void {
  const trig = mod.trigger;
  const matchesKind =
    (opts.kind === "attack" &&
      (trig === `${side}_attack` ||
        (side === "incoming" && trig === "incoming_attack_within_5ft" && opts.meleeWithin5ft) ||
        (side === "outgoing" && trig === "outgoing_attack_within_5ft" && opts.meleeWithin5ft))) ||
    (opts.kind === "save" && trig === `${side}_save`) ||
    (opts.kind === "check" && trig === `${side}_check`);

  if (!matchesKind) return;
  if (mod.saveStat && opts.saveStat && !mod.saveStat.includes(opts.saveStat)) return;

  switch (mod.effect) {
    case "advantage":
      flags.advantage = true;
      break;
    case "disadvantage":
      flags.disadvantage = true;
      break;
    case "auto_fail":
      flags.autoFail = true;
      break;
    case "auto_success":
      flags.autoSuccess = true;
      break;
    case "auto_critical":
      flags.autoCritical = true;
      break;
    case "true":
      // not a flag; used by isIncapacitated
      break;
  }
}

export function isIncapacitated(
  conds: ActiveCondition[],
  defs: Record<string, ConditionDefinition> = CONDITIONS,
): boolean {
  for (const c of conds) {
    const def = defs[c.conditionId];
    if (!def) continue;
    if (def.modifiers.some((m) => m.trigger === "incapacitated" && m.effect === "true")) {
      return true;
    }
  }
  return false;
}
