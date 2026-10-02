import { describe, it, expect } from "vitest";
import {
  DND5E_RULESET,
  abilityModifier,
  rollCheck,
  rollInitiative,
  type Ruleset,
} from "@/lib/engine/ruleset";
import { newContext, resolveEffects } from "@/lib/engine/resolver";

// rng renvoyant la séquence donnée (valeurs dans [0, 1)).
function seq(...values: number[]) {
  let i = 0;
  return () => values[i++ % values.length];
}
// Face d'un dé à N faces → valeur rng correspondante.
const face = (f: number, faces = 20) => (f - 1) / faces + 0.001;

const COC_LIKE: Ruleset = {
  ...DND5E_RULESET,
  name: "d100 sous la compétence",
  check: { dice: "1d100", mode: "roll_under", abilityModifier: "direct", criticalOn: 1, fumbleOn: 100, advantage: false },
};

describe("abilityModifier", () => {
  it("applique la formule 5e", () => {
    expect(abilityModifier(DND5E_RULESET, 16)).toBe(3);
    expect(abilityModifier(DND5E_RULESET, 9)).toBe(-1);
    expect(abilityModifier(DND5E_RULESET, undefined)).toBe(0);
  });
  it("utilise la valeur directe quand le ruleset le demande", () => {
    expect(abilityModifier({ ...DND5E_RULESET, check: { ...DND5E_RULESET.check, abilityModifier: "direct" } }, 2)).toBe(2);
  });
});

describe("rollCheck — roll_over (d20)", () => {
  it("ajoute le modificateur et compare au DD", () => {
    const r = rollCheck(DND5E_RULESET, { score: 16, dc: 15, rng: seq(face(12)) });
    expect(r.natural).toBe(12);
    expect(r.roll.result).toBe(15);
    expect(r.success).toBe(true);
  });
  it("garde le meilleur dé avec avantage, le pire avec désavantage", () => {
    expect(rollCheck(DND5E_RULESET, { dc: 10, advantage: true, rng: seq(face(3), face(17)) }).natural).toBe(17);
    expect(rollCheck(DND5E_RULESET, { dc: 10, disadvantage: true, rng: seq(face(3), face(17)) }).natural).toBe(3);
  });
  it("détecte critique et échec critique sur le naturel", () => {
    expect(rollCheck(DND5E_RULESET, { dc: 30, rng: seq(face(20)) }).critical).toBe(true);
    expect(rollCheck(DND5E_RULESET, { dc: 1, rng: seq(face(1)) }).fumble).toBe(true);
  });
});

describe("rollCheck — roll_under (d100)", () => {
  it("réussit si le dé est sous la valeur moins la difficulté", () => {
    const ok = rollCheck(COC_LIKE, { score: 60, dc: 0, rng: seq(face(42, 100)) });
    expect(ok.success).toBe(true);
    const ko = rollCheck(COC_LIKE, { score: 60, dc: 20, rng: seq(face(42, 100)) });
    expect(ko.success).toBe(false);
  });
});

describe("rollInitiative", () => {
  it("utilise le modificateur de la caractéristique du ruleset", () => {
    expect(rollInitiative(DND5E_RULESET, { abilityScores: { DEX: 14 }, rng: seq(face(10)) })).toBe(12);
  });
  it("le bonus d'initiative de la fiche prime", () => {
    expect(rollInitiative(DND5E_RULESET, { abilityScores: { DEX: 14 }, bonus: 5, rng: seq(face(10)) })).toBe(15);
  });
});

describe("résolveur + ruleset personnalisé", () => {
  it("résout un jet de sauvegarde selon un ruleset roll_under", async () => {
    const ctx = newContext({
      entities: [{ id: "a", name: "Investigateur", attributes: { abilityScores: { POW: 50 } }, state: { conditions: [] } }],
      initiativeOrder: [],
      ruleset: { ...COC_LIKE, abilities: [{ id: "POW", label: "Pouvoir", abbr: "POU" }] },
      rng: seq(face(30, 100)),
    });
    const { records } = await resolveEffects(
      [{ type: "roll_check", stat: "POW", dc: 0, target: { type: "single", entityId: "a" } }],
      ctx,
    );
    expect(records[0].outcome).toBe("success");
    expect(records[0].description).toContain("POU");
  });
});

import { RulesetSchema } from "@/lib/validation/ruleset-schema";

describe("RulesetSchema", () => {
  it("accepte le préréglage D&D 5e", () => {
    expect(RulesetSchema.safeParse(DND5E_RULESET).success).toBe(true);
  });
  it("refuse une compétence liée à une caractéristique inconnue", () => {
    const bad = { ...DND5E_RULESET, skills: [{ id: "x", label: "X", ability: "LUCK" }] };
    const r = RulesetSchema.safeParse(bad);
    expect(r.success).toBe(false);
    expect(JSON.stringify(r.error?.issues)).toContain("LUCK");
  });
  it("refuse une notation de dés invalide", () => {
    const bad = { ...DND5E_RULESET, check: { ...DND5E_RULESET.check, dice: "d20" } };
    expect(RulesetSchema.safeParse(bad).success).toBe(false);
  });
});
