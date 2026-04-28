import { describe, it, expect } from "vitest";
import { parseDiceNotation, rollDice, rollD20 } from "@/lib/engine/dice";

describe("parseDiceNotation", () => {
  it("parses a simple NdM", () => {
    expect(parseDiceNotation("1d20")).toEqual({
      count: 1,
      faces: 20,
      modifier: 0,
      half: false,
      flat: false,
    });
  });

  it("parses NdM+K", () => {
    expect(parseDiceNotation("2d6+3")).toMatchObject({
      count: 2,
      faces: 6,
      modifier: 3,
    });
  });

  it("parses NdM-K", () => {
    expect(parseDiceNotation("1d8-1")).toMatchObject({
      count: 1,
      faces: 8,
      modifier: -1,
    });
  });

  it("parses /2 suffix for half damage", () => {
    expect(parseDiceNotation("8d6/2")).toMatchObject({
      count: 8,
      faces: 6,
      half: true,
    });
  });

  it("parses a flat number", () => {
    expect(parseDiceNotation("5")).toMatchObject({
      flat: true,
      modifier: 5,
    });
  });

  it("rejects garbage notation", () => {
    expect(() => parseDiceNotation("XYZ")).toThrow();
    expect(() => parseDiceNotation("1d20+3+5")).toThrow();
  });

  it("rejects out-of-range dice", () => {
    expect(() => parseDiceNotation("0d20")).toThrow();
    expect(() => parseDiceNotation("200d6")).toThrow();
  });
});

describe("rollDice", () => {
  it("returns a deterministic result with a fixed RNG", () => {
    // RNG that always returns 0.0 -> rolls minimum face
    const rng = () => 0;
    const r = rollDice("3d6+2", { rng });
    expect(r.rolls).toEqual([1, 1, 1]);
    expect(r.result).toBe(5); // 3 + 2
  });

  it("halves with /2 suffix", () => {
    const rng = () => 0.999; // max face
    const r = rollDice("8d6/2", { rng });
    expect(r.rolls).toEqual([6, 6, 6, 6, 6, 6, 6, 6]);
    // Sum = 48, halved -> 24
    expect(r.result).toBe(24);
  });

  it("returns flat number directly", () => {
    const r = rollDice("7", { rng: () => 0 });
    expect(r.result).toBe(7);
    expect(r.rolls).toEqual([]);
  });
});

describe("rollD20", () => {
  it("rolls a single d20 normally", () => {
    const r = rollD20(5, { rng: () => 0.95 }); // ~face 20
    expect(r.rolls).toHaveLength(1);
    expect(r.result).toBe(r.rolls[0] + 5);
  });

  it("with advantage rolls 2 and keeps highest", () => {
    let i = 0;
    const seq = [0.05, 0.95]; // 2 then 20
    const rng = () => seq[i++];
    const r = rollD20(0, { advantage: true, rng });
    expect(r.rolls).toHaveLength(2);
    expect(r.advantage).toBe(true);
    expect(r.result).toBe(20);
  });

  it("with disadvantage rolls 2 and keeps lowest", () => {
    let i = 0;
    const seq = [0.95, 0.05];
    const rng = () => seq[i++];
    const r = rollD20(2, { disadvantage: true, rng });
    expect(r.disadvantage).toBe(true);
    expect(r.result).toBe(2 + 2); // kept the 2
  });

  it("advantage + disadvantage cancel out", () => {
    const r = rollD20(0, { advantage: true, disadvantage: true, rng: () => 0.5 });
    expect(r.rolls).toHaveLength(1);
    expect(r.advantage).toBeFalsy();
    expect(r.disadvantage).toBeFalsy();
  });
});
