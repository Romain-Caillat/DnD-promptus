import { describe, it, expect } from "vitest";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { attackGeometry, creatureAttacks, hasLineOfSight, lineSteps, metersToCells } from "@/lib/engine/combat";
import type { GameMap } from "@/lib/engine/story";

const room = (blocked: [number, number][] = []): Pick<GameMap, "grid" | "cells"> => ({
  grid: { type: "square", cols: 12, rows: 12 },
  cells: blocked.map(([x, y]) => ({ x, y, blocked: true })),
});
const geo = (map: Pick<GameMap, "grid" | "cells">, from: [number, number], to: [number, number], rangeMeters: number, longRangeMeters?: number) =>
  attackGeometry({
    map,
    from: { x: from[0], y: from[1] },
    to: { x: to[0], y: to[1] },
    attack: { rangeMeters, longRangeMeters },
    cellMeters: 1.5,
    diagonal: "chebyshev",
  });

describe("creatureAttacks", () => {
  it("lit les attaques de la fiche et ignore les entrées invalides", () => {
    const attacks = creatureAttacks(
      {
        attacks: [
          { name: "Épée longue", bonus: 5, damage: "1d8+3", damageType: "slashing" },
          { name: "Arc court", bonus: 4, damage: "1d6+2", damageType: "piercing", rangeMeters: 24, longRangeMeters: 96 },
          { name: "", damage: "1d4" },
          "n'importe quoi",
        ],
      },
      DND5E_RULESET,
    );
    expect(attacks.map((a) => a.id)).toEqual(["epee_longue", "arc_court"]);
    expect(attacks[0].rangeMeters).toBe(1.5);
    expect(attacks[1].longRangeMeters).toBe(96);
  });

  it("déduit une attaque à mains nues sans attaque définie", () => {
    const [a] = creatureAttacks({ abilityScores: { STR: 16 }, proficiencyBonus: 2 }, DND5E_RULESET);
    expect(a).toMatchObject({ id: "unarmed", bonus: 5, damage: "1+3", rangeMeters: 1.5 });
  });
});

describe("portée", () => {
  it("convertit les mètres en cases (au moins le contact)", () => {
    expect(metersToCells(1.5, 1.5)).toBe(1);
    expect(metersToCells(9, 1.5)).toBe(6);
    expect(metersToCells(1, 1.5)).toBe(1);
  });

  it("corps à corps : seulement au contact, diagonale comprise", () => {
    expect(geo(room(), [2, 2], [3, 3], 1.5)).toMatchObject({ melee: true, inRange: true, reason: null });
    expect(geo(room(), [2, 2], [4, 2], 1.5)).toMatchObject({ melee: false, inRange: false });
    expect(geo(room(), [2, 2], [4, 2], 1.5).reason).toContain("Hors de portée");
  });

  it("portée longue : possible avec désavantage, puis hors de portée", () => {
    // 24 m = 16 cases ; 30 m = 20 cases.
    expect(geo(room(), [0, 0], [10, 0], 9, 30)).toMatchObject({ inRange: true, longRange: true, reason: null });
    expect(geo(room(), [0, 0], [5, 0], 9, 30)).toMatchObject({ longRange: false });
    expect(geo(room(), [0, 0], [11, 11], 9, 15).inRange).toBe(false);
  });
});

describe("ligne de vue", () => {
  it("un mur entre les deux bloque le tir", () => {
    const map = room([[4, 2]]);
    expect(hasLineOfSight(map, { x: 2, y: 2 }, { x: 6, y: 2 })).toBe(false);
    expect(geo(map, [2, 2], [6, 2], 24).reason).toBe("Pas de ligne de vue (obstacle)");
    expect(hasLineOfSight(map, { x: 2, y: 3 }, { x: 6, y: 3 })).toBe(true);
  });

  it("un coin exact ne bloque que si les deux cases voisines bloquent", () => {
    const steps = lineSteps("square", { x: 0, y: 0 }, { x: 2, y: 2 });
    expect(steps[0]).toHaveLength(2); // coin entre (1,0) et (0,1)
    expect(hasLineOfSight(room([[1, 0]]), { x: 0, y: 0 }, { x: 2, y: 2 })).toBe(true);
    expect(hasLineOfSight(room([[1, 0], [0, 1]]), { x: 0, y: 0 }, { x: 2, y: 2 })).toBe(false);
    expect(hasLineOfSight(room([[1, 1]]), { x: 0, y: 0 }, { x: 2, y: 2 })).toBe(false);
  });

  it("au contact, la ligne de vue est toujours acquise", () => {
    expect(geo(room([[3, 2], [2, 3]]), [2, 2], [3, 3], 1.5).reason).toBeNull();
  });

  it("fonctionne sur les hexagones", () => {
    const hex: Pick<GameMap, "grid" | "cells"> = { grid: { type: "hex", cols: 10, rows: 10 }, cells: [{ x: 3, y: 2, blocked: true }] };
    expect(hasLineOfSight(hex, { x: 1, y: 2 }, { x: 5, y: 2 })).toBe(false);
    expect(hasLineOfSight(hex, { x: 1, y: 5 }, { x: 5, y: 5 })).toBe(true);
  });
});
