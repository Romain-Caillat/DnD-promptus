import { describe, it, expect } from "vitest";
import {
  cellCenter,
  cellKey,
  distance,
  neighbors,
  pixelToCell,
  reachableCells,
  speedInCells,
} from "@/lib/engine/grid";

describe("grilles hexagonales (odd-r)", () => {
  it("a 6 voisins au centre, décalés selon la parité de la rangée", () => {
    expect(neighbors("hex", 3, 2, 10, 10)).toHaveLength(6);
    expect(neighbors("hex", 3, 2, 10, 10)).toContainEqual({ x: 2, y: 1 });
    expect(neighbors("hex", 3, 3, 10, 10)).toContainEqual({ x: 4, y: 2 });
  });
  it("calcule la distance en cases", () => {
    expect(distance("hex", { x: 0, y: 0 }, { x: 3, y: 0 })).toBe(3);
    expect(distance("hex", { x: 0, y: 0 }, { x: 1, y: 2 })).toBe(2);
    expect(distance("hex", { x: 2, y: 4 }, { x: 7, y: 1 })).toBe(7);
  });
  it("retrouve la case sous son propre centre", () => {
    for (const [x, y] of [[0, 0], [5, 3], [9, 7], [4, 1]]) {
      const { cx, cy } = cellCenter("hex", x, y, 20);
      expect(pixelToCell("hex", cx, cy, 20, 10, 8)).toEqual({ x, y });
    }
    expect(pixelToCell("hex", -50, -50, 20, 10, 8)).toBeNull();
  });
  it("compte une case par pas pour le déplacement", () => {
    const map = { grid: { type: "hex" as const, cols: 10, rows: 8 }, cells: [] };
    const r = reachableCells(map, { x: 4, y: 4 }, 2);
    expect(r.get(cellKey(4, 4))).toBe(0);
    expect(r.get(cellKey(6, 4))).toBe(2);
    expect(r.has(cellKey(7, 4))).toBe(false);
  });
});

describe("grilles carrées", () => {
  it("applique la règle de diagonale", () => {
    const a = { x: 0, y: 0 };
    const b = { x: 4, y: 4 };
    expect(distance("square", a, b, "chebyshev")).toBe(4);
    expect(distance("square", a, b, "alternate")).toBe(6);
    expect(distance("square", a, b, "euclidean")).toBe(6);
  });

  it("contourne les murs et ne coupe pas les coins", () => {
    // Mur vertical en x=2 de y=0 à y=3 ; passage en (2,4).
    const cells = [0, 1, 2, 3].map((y) => ({ x: 2, y, blocked: true }));
    const map = { grid: { type: "square" as const, cols: 6, rows: 6 }, cells };
    const r = reachableCells(map, { x: 1, y: 0 }, 6);
    expect(r.has(cellKey(2, 1))).toBe(false);
    // (1,0) → (1,3) = 3 ; (1,3)→(2,4) rase le mur en (2,3) : interdit, donc (1,4)=4, (2,4)=5, (3,4)=6.
    expect(r.get(cellKey(2, 4))).toBe(5);
    expect(r.get(cellKey(3, 4))).toBe(6);
    // (2,4)→(3,3) raserait aussi le mur : (3,3) coûterait 7, hors budget.
    expect(r.has(cellKey(3, 3))).toBe(false);
    expect(r.has(cellKey(3, 0))).toBe(false);
  });

  it("règle 1-2-1 : la deuxième diagonale coûte double", () => {
    const map = { grid: { type: "square" as const, cols: 10, rows: 10 }, cells: [] };
    const r = reachableCells(map, { x: 0, y: 0 }, 3, "alternate");
    expect(r.get(cellKey(1, 1))).toBe(1);
    expect(r.get(cellKey(2, 2))).toBe(3);
    expect(r.has(cellKey(3, 3))).toBe(false);
  });

  it("cases occupées infranchissables en plus", () => {
    const map = { grid: { type: "square" as const, cols: 3, rows: 1 }, cells: [] };
    expect(reachableCells(map, { x: 0, y: 0 }, 5, "chebyshev", new Set([cellKey(1, 0)])).has(cellKey(2, 0))).toBe(false);
  });
});

describe("speedInCells", () => {
  it("convertit une vitesse en pieds selon la taille de case", () => {
    expect(speedInCells({ speed: 30 }, 1.5, 6)).toBe(6);
    expect(speedInCells({ speed: 25 }, 1.5, 6)).toBe(5);
    expect(speedInCells({ speedCells: 8 }, 1.5, 6)).toBe(8);
    expect(speedInCells({}, 1.5, 6)).toBe(6);
  });
});
