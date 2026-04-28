// 5e standard conditions, declared as data.
// Modifiers are evaluated by the resolver when computing roll flags or
// when checking incapacitation.

import type {
  ActiveCondition,
  ConditionDefinition,
  ConditionModifier,
  Stat,
} from "./types";

export const CONDITIONS: Record<string, ConditionDefinition> = {
  blinded: {
    id: "blinded",
    name: "Blinded",
    description: "Can't see; auto-fails sight checks; attacks have disadvantage; attacks against you have advantage.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "incoming_attack", effect: "advantage" },
    ],
  },
  charmed: {
    id: "charmed",
    name: "Charmed",
    description: "Can't attack the charmer or target them with harmful abilities.",
    modifiers: [],
  },
  deafened: {
    id: "deafened",
    name: "Deafened",
    description: "Can't hear; auto-fails hearing checks.",
    modifiers: [],
  },
  frightened: {
    id: "frightened",
    name: "Frightened",
    description: "Disadvantage on ability checks and attacks while source is in line of sight.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "outgoing_check", effect: "disadvantage" },
    ],
  },
  grappled: {
    id: "grappled",
    name: "Grappled",
    description: "Speed becomes 0; can't benefit from speed bonuses.",
    modifiers: [],
  },
  incapacitated: {
    id: "incapacitated",
    name: "Incapacitated",
    description: "Can't take actions or reactions.",
    modifiers: [{ trigger: "incapacitated", effect: "true" }],
  },
  invisible: {
    id: "invisible",
    name: "Invisible",
    description: "Can't be seen; attacks have advantage; attacks against you have disadvantage.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "advantage" },
      { trigger: "incoming_attack", effect: "disadvantage" },
    ],
  },
  paralyzed: {
    id: "paralyzed",
    name: "Paralyzed",
    description: "Incapacitated; auto-fails STR & DEX saves; attacks against have advantage; melee hits within 5ft are critical.",
    modifiers: [
      { trigger: "incapacitated", effect: "true" },
      { trigger: "outgoing_save", saveStat: ["STR", "DEX"], effect: "auto_fail" },
      { trigger: "incoming_attack", effect: "advantage" },
      { trigger: "incoming_attack_within_5ft", effect: "auto_critical" },
    ],
  },
  petrified: {
    id: "petrified",
    name: "Petrified",
    description: "Transformed to stone; incapacitated; resists all damage; auto-fails STR/DEX saves; advantage to hit.",
    modifiers: [
      { trigger: "incapacitated", effect: "true" },
      { trigger: "outgoing_save", saveStat: ["STR", "DEX"], effect: "auto_fail" },
      { trigger: "incoming_attack", effect: "advantage" },
    ],
  },
  poisoned: {
    id: "poisoned",
    name: "Poisoned",
    description: "Disadvantage on attacks and ability checks.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "outgoing_check", effect: "disadvantage" },
    ],
  },
  prone: {
    id: "prone",
    name: "Prone",
    description: "Disadvantage on attacks; melee against have advantage; ranged against have disadvantage.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "incoming_attack_within_5ft", effect: "advantage" },
      // Ranged-against-prone disadvantage is approximated when not within 5ft;
      // the resolver only knows whether the source is within 5ft, not actual distance.
    ],
  },
  restrained: {
    id: "restrained",
    name: "Restrained",
    description: "Speed 0; disadvantage on attacks and DEX saves; attacks against have advantage.",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "outgoing_save", saveStat: ["DEX"], effect: "disadvantage" } as ConditionModifier,
      { trigger: "incoming_attack", effect: "advantage" },
    ],
  },
  stunned: {
    id: "stunned",
    name: "Stunned",
    description: "Incapacitated; auto-fails STR & DEX saves; attacks against have advantage.",
    modifiers: [
      { trigger: "incapacitated", effect: "true" },
      { trigger: "outgoing_save", saveStat: ["STR", "DEX"], effect: "auto_fail" },
      { trigger: "incoming_attack", effect: "advantage" },
    ],
  },
  unconscious: {
    id: "unconscious",
    name: "Unconscious",
    description: "Incapacitated; drops what's held; falls prone; auto-fails STR/DEX saves; advantage to hit; melee within 5ft is critical.",
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
  saveStat?: Stat;
  meleeWithin5ft?: boolean;
}

export function computeRollFlags(opts: ComputeFlagsOpts): RollFlags {
  const flags: RollFlags = { ...NEUTRAL_FLAGS };

  for (const c of opts.actorConditions) {
    const def = CONDITIONS[c.conditionId];
    if (!def) continue;
    for (const mod of def.modifiers) {
      applyModifier(mod, "outgoing", opts, flags);
    }
  }
  for (const c of opts.targetConditions ?? []) {
    const def = CONDITIONS[c.conditionId];
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

export function isIncapacitated(conds: ActiveCondition[]): boolean {
  for (const c of conds) {
    const def = CONDITIONS[c.conditionId];
    if (!def) continue;
    if (def.modifiers.some((m) => m.trigger === "incapacitated" && m.effect === "true")) {
      return true;
    }
  }
  return false;
}
