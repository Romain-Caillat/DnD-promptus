import { describe, it, expect } from "vitest";
import {
  CONDITIONS,
  computeRollFlags,
  isIncapacitated,
} from "@/lib/engine/conditions";
import type { ActiveCondition } from "@/lib/engine/types";

describe("CONDITIONS catalog", () => {
  it("contains the 14 standard 5e conditions", () => {
    const expected = [
      "blinded",
      "charmed",
      "deafened",
      "frightened",
      "grappled",
      "incapacitated",
      "invisible",
      "paralyzed",
      "petrified",
      "poisoned",
      "prone",
      "restrained",
      "stunned",
      "unconscious",
    ];
    for (const id of expected) {
      expect(CONDITIONS[id]).toBeDefined();
      expect(CONDITIONS[id].id).toBe(id);
    }
  });
});

describe("computeRollFlags", () => {
  it("poisoned attacker has disadvantage on attacks", () => {
    const cond: ActiveCondition = { conditionId: "poisoned" };
    const flags = computeRollFlags({
      actorConditions: [cond],
      kind: "attack",
    });
    expect(flags.disadvantage).toBe(true);
    expect(flags.advantage).toBe(false);
  });

  it("paralyzed target gives advantage to attacker", () => {
    const cond: ActiveCondition = { conditionId: "paralyzed" };
    const flags = computeRollFlags({
      actorConditions: [],
      targetConditions: [cond],
      kind: "attack",
    });
    expect(flags.advantage).toBe(true);
  });

  it("paralyzed target produces auto_critical for melee within 5ft", () => {
    const cond: ActiveCondition = { conditionId: "paralyzed" };
    const flags = computeRollFlags({
      actorConditions: [],
      targetConditions: [cond],
      kind: "attack",
      meleeWithin5ft: true,
    });
    expect(flags.autoCritical).toBe(true);
  });

  it("paralyzed target auto-fails STR saves", () => {
    const cond: ActiveCondition = { conditionId: "paralyzed" };
    const flags = computeRollFlags({
      actorConditions: [cond],
      kind: "save",
      saveStat: "STR",
    });
    expect(flags.autoFail).toBe(true);
  });

  it("paralyzed target does NOT auto-fail INT saves (out of scope)", () => {
    const cond: ActiveCondition = { conditionId: "paralyzed" };
    const flags = computeRollFlags({
      actorConditions: [cond],
      kind: "save",
      saveStat: "INT",
    });
    expect(flags.autoFail).toBe(false);
  });

  it("blinded vs invisible cancels advantage and disadvantage", () => {
    const flags = computeRollFlags({
      actorConditions: [{ conditionId: "blinded" }],
      targetConditions: [{ conditionId: "invisible" }],
      kind: "attack",
    });
    // Blinded gives self disadvantage; invisible target gives attacker disadvantage too.
    // Both = stronger disadvantage. We don't double; flags are simply true for one of them.
    expect(flags.disadvantage).toBe(true);
    expect(flags.advantage).toBe(false);
  });

  it("invisible attacker + blinded target → advantage stays", () => {
    const flags = computeRollFlags({
      actorConditions: [{ conditionId: "invisible" }],
      targetConditions: [{ conditionId: "blinded" }],
      kind: "attack",
    });
    expect(flags.advantage).toBe(true);
  });

  it("returns neutral when no relevant conditions", () => {
    const flags = computeRollFlags({
      actorConditions: [],
      kind: "attack",
    });
    expect(flags.advantage).toBe(false);
    expect(flags.disadvantage).toBe(false);
  });
});

describe("isIncapacitated", () => {
  it("true for paralyzed/stunned/unconscious", () => {
    expect(isIncapacitated([{ conditionId: "paralyzed" }])).toBe(true);
    expect(isIncapacitated([{ conditionId: "stunned" }])).toBe(true);
    expect(isIncapacitated([{ conditionId: "unconscious" }])).toBe(true);
    expect(isIncapacitated([{ conditionId: "incapacitated" }])).toBe(true);
  });
  it("false for poisoned alone", () => {
    expect(isIncapacitated([{ conditionId: "poisoned" }])).toBe(false);
  });
});
