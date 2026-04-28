import { describe, it, expect } from "vitest";
import { newContext, resolveAttack, resolveEffects, type EntityRef } from "@/lib/engine/resolver";
import type { Effect, EntityState } from "@/lib/engine/types";

function makeEntity(id: string, name: string, attrs: Record<string, unknown>, state?: Partial<EntityState>): EntityRef {
  return {
    id,
    name,
    attributes: attrs,
    state: { conditions: [], ...state },
  };
}

describe("resolver — damage", () => {
  it("applies damage to target's HP", async () => {
    const bobby = makeEntity("bob", "Bobby", {}, { hp: 10, hpMax: 10 });
    const goblin = makeEntity("gob", "Goblin", {}, { hp: 7, hpMax: 7 });
    const ctx = newContext({
      caster: bobby,
      entities: [bobby, goblin],
      initiativeOrder: [],
      explicitTargets: ["gob"],
      rng: () => 0, // min roll → 1d6+0 = 1
    });
    const effects: Effect[] = [
      {
        type: "damage",
        amount: "1d6",
        damageType: "slashing",
        target: { type: "single", entityId: "gob" },
      },
    ];
    const res = await resolveEffects(effects, ctx);
    expect(res.records).toHaveLength(1);
    expect(res.finalStates.get("gob")!.hp).toBe(6);
  });

  it("clamps HP at 0", async () => {
    const gob = makeEntity("gob", "Goblin", {}, { hp: 2, hpMax: 7 });
    const ctx = newContext({
      caster: gob,
      entities: [gob],
      initiativeOrder: [],
      explicitTargets: ["gob"],
      rng: () => 0.99, // 1d20 max
    });
    const effects: Effect[] = [
      {
        type: "damage",
        amount: "1d20",
        damageType: "fire",
        target: { type: "single", entityId: "gob" },
      },
    ];
    const res = await resolveEffects(effects, ctx);
    expect(res.finalStates.get("gob")!.hp).toBe(0);
  });
});

describe("resolver — heal", () => {
  it("heals up to hpMax", async () => {
    const bobby = makeEntity("bob", "Bobby", {}, { hp: 3, hpMax: 10 });
    const ctx = newContext({
      caster: bobby,
      entities: [bobby],
      initiativeOrder: [],
      rng: () => 0.999,
    });
    const effects: Effect[] = [
      {
        type: "heal",
        amount: "1d20",
        target: { type: "self" },
      },
    ];
    const res = await resolveEffects(effects, ctx);
    expect(res.finalStates.get("bob")!.hp).toBe(10); // capped
  });
});

describe("resolver — apply_condition", () => {
  it("adds an active condition with duration", async () => {
    const druid = makeEntity("druid", "Druid", {});
    const gob = makeEntity("gob", "Goblin", {}, { conditions: [] });
    const ctx = newContext({
      caster: druid,
      entities: [druid, gob],
      initiativeOrder: [],
      rng: () => 0,
    });
    const effects: Effect[] = [
      {
        type: "apply_condition",
        conditionId: "poisoned",
        duration: { rounds: 3 },
        target: { type: "single", entityId: "gob" },
      },
    ];
    const res = await resolveEffects(effects, ctx);
    expect(res.records).toHaveLength(1);
    const conditions = res.finalStates.get("gob")!.conditions;
    expect(conditions).toHaveLength(1);
    expect(conditions[0]).toMatchObject({
      conditionId: "poisoned",
      remainingRounds: 3,
      source: "druid",
    });
  });
});

describe("resolver — roll_check cascade", () => {
  it("runs outcomeFail effects when save fails", async () => {
    const wiz = makeEntity("wiz", "Wizard", {});
    const gob = makeEntity(
      "gob",
      "Goblin",
      { abilityScores: { STR: 10, DEX: 10, CON: 10, INT: 10, WIS: 10, CHA: 10 } },
      { hp: 30, hpMax: 30 },
    );
    const ctx = newContext({
      caster: wiz,
      entities: [wiz, gob],
      initiativeOrder: [],
      explicitTargets: ["gob"],
      rng: () => 0, // d20 → 1, will fail save
    });
    const effects: Effect[] = [
      {
        type: "roll_check",
        stat: "DEX",
        dc: 15,
        target: { type: "single", entityId: "gob" },
        outcomeFail: [
          {
            type: "damage",
            amount: "8d6",
            damageType: "fire",
            target: { type: "single", entityId: "gob" },
          },
        ],
        outcomeSuccess: [],
      },
    ];
    const res = await resolveEffects(effects, ctx);
    expect(res.records.length).toBeGreaterThanOrEqual(2);
    // Save check + damage cascade
    expect(res.records[0].outcome).toBe("fail");
    const finalHp = res.finalStates.get("gob")!.hp!;
    expect(finalHp).toBeLessThan(30);
  });

  it("runs outcomeSuccess effects when save passes", async () => {
    const wiz = makeEntity("wiz", "Wizard", {});
    const gob = makeEntity(
      "gob",
      "Goblin",
      { abilityScores: { DEX: 18 } },
      { hp: 30, hpMax: 30 },
    );
    const ctx = newContext({
      caster: wiz,
      entities: [wiz, gob],
      initiativeOrder: [],
      explicitTargets: ["gob"],
      rng: () => 0.999, // d20 → 20, passes save easily
    });
    const effects: Effect[] = [
      {
        type: "roll_check",
        stat: "DEX",
        dc: 15,
        target: { type: "single", entityId: "gob" },
        outcomeFail: [
          {
            type: "damage",
            amount: "8d6",
            damageType: "fire",
            target: { type: "single", entityId: "gob" },
          },
        ],
        outcomeSuccess: [
          {
            type: "damage",
            amount: "8d6/2",
            damageType: "fire",
            target: { type: "single", entityId: "gob" },
          },
        ],
      },
    ];
    const res = await resolveEffects(effects, ctx);
    expect(res.records[0].outcome).toBe("success");
  });
});

describe("resolveAttack", () => {
  it("hits when total >= AC and applies damage", () => {
    const bobby = makeEntity("bob", "Bobby", {});
    const gob = makeEntity("gob", "Goblin", { ac: 12 }, { hp: 7, hpMax: 7 });
    const ctx = newContext({
      caster: bobby,
      entities: [bobby, gob],
      initiativeOrder: [],
      rng: () => 0.999, // d20 → 20, then dice all max
    });
    const records = resolveAttack(
      {
        attackerId: "bob",
        targetIds: ["gob"],
        attackBonus: 5,
        damageNotation: "1d8+3",
        damageType: "slashing",
      },
      ctx,
    );
    // attack record + damage record
    expect(records.length).toBe(2);
    expect(records[0].outcome).toBe("success");
    expect(ctx.states.get("gob")!.hp).toBeLessThan(7);
  });

  it("misses when below AC", () => {
    const bobby = makeEntity("bob", "Bobby", {});
    const gob = makeEntity("gob", "Goblin", { ac: 18 }, { hp: 7, hpMax: 7 });
    const ctx = newContext({
      caster: bobby,
      entities: [bobby, gob],
      initiativeOrder: [],
      rng: () => 0, // d20 → 1
    });
    const records = resolveAttack(
      {
        attackerId: "bob",
        targetIds: ["gob"],
        attackBonus: 0,
        damageNotation: "1d8",
        damageType: "slashing",
      },
      ctx,
    );
    expect(records[0].outcome).toBe("fail");
    expect(ctx.states.get("gob")!.hp).toBe(7);
  });

  it("paralyzed target gives critical on melee within 5ft", () => {
    const bobby = makeEntity("bob", "Bobby", {});
    const gob = makeEntity(
      "gob",
      "Goblin",
      { ac: 30 },
      { hp: 50, hpMax: 50, conditions: [{ conditionId: "paralyzed" }] },
    );
    const ctx = newContext({
      caster: bobby,
      entities: [bobby, gob],
      initiativeOrder: [],
      rng: () => 0.5, // d20 ~= 11, would not hit AC 30 normally
    });
    const records = resolveAttack(
      {
        attackerId: "bob",
        targetIds: ["gob"],
        attackBonus: 0,
        damageNotation: "1d8+3",
        damageType: "slashing",
        meleeWithin5ft: true,
      },
      ctx,
    );
    // auto_critical → hit even though roll < AC
    expect(records[0].outcome).toBe("success");
    expect(records[0].description).toContain("CRITICAL");
  });
});
