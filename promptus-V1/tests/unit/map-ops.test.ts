import { describe, it, expect } from "vitest";
import { applyMapOp } from "@/lib/engine/map-ops";
import { defaultMapId, mapTokens, normalizeWorld, revealedCells } from "@/lib/engine/world";
import { buildDemoStory } from "../../scripts/demo-story";

const story = buildDemoStory((n) => `ent:${n}`);
const crypt = story.maps.find((m) => m.id === "map_crypte")!;
const ids = new Set(["ent:Bobby"]);

describe("applyMapOp", () => {
  it("révèle et cache des cases sans dupliquer", () => {
    let w = applyMapOp(normalizeWorld(null), story, { op: "reveal", mapId: "map_crypte", cells: [[1, 1], [1, 1], [2, 1], [99, 99]] }, ids);
    expect([...revealedCells(crypt, w)].sort()).toEqual(["1,1", "2,1"]);
    w = applyMapOp(w, story, { op: "hide", mapId: "map_crypte", cells: [[1, 1]] }, ids);
    expect([...revealedCells(crypt, w)]).toEqual(["2,1"]);
    w = applyMapOp(w, story, { op: "reveal_all", mapId: "map_crypte" }, ids);
    expect(revealedCells(crypt, w).size).toBe(20 * 14);
  });

  it("déplace un pion en partant des positions de départ de la carte", () => {
    const w = applyMapOp(normalizeWorld(null), story, { op: "move_token", mapId: "map_crypte", entityId: "ent:Bobby", x: 5, y: 11 }, ids);
    const tokens = mapTokens(crypt, w);
    expect(tokens).toHaveLength(crypt.tokens!.length + 1);
    expect(tokens).toContainEqual({ entityId: "ent:Bobby", x: 5, y: 11 });
  });

  it("refuse les cases bloquées, occupées ou hors carte", () => {
    const w = normalizeWorld(null);
    expect(() => applyMapOp(w, story, { op: "move_token", mapId: "map_crypte", entityId: "ent:Bobby", x: 0, y: 0 }, ids)).toThrow(/infranchissable/);
    expect(() => applyMapOp(w, story, { op: "move_token", mapId: "map_crypte", entityId: "ent:Bobby", x: 4, y: 5 }, ids)).toThrow(/occupée/);
    expect(() => applyMapOp(w, story, { op: "move_token", mapId: "map_crypte", entityId: "ent:Bobby", x: 40, y: 5 }, ids)).toThrow(/hors/);
    expect(() => applyMapOp(w, story, { op: "move_token", mapId: "map_crypte", entityId: "inconnu", x: 3, y: 3 }, ids)).toThrow(/inconnue/);
  });

  it("accepte le pion du groupe sur les cartes de voyage", () => {
    const w = applyMapOp(normalizeWorld(null), story, { op: "move_token", mapId: "map_vallee", entityId: "party", x: 2, y: 4 }, ids);
    expect(mapTokens(story.maps.find((m) => m.id === "map_vallee")!, w)).toEqual([{ entityId: "party", x: 2, y: 4 }]);
  });
});

describe("defaultMapId", () => {
  it("montre la carte de combat d’une scène de combat, sinon la carte de région", () => {
    expect(defaultMapId(story, { ...normalizeWorld(null), currentSceneId: "sc_crypte" })).toBe("map_crypte");
    expect(defaultMapId(story, { ...normalizeWorld(null), currentSceneId: "sc_village" })).toBe("map_vallee");
    expect(defaultMapId(story, { ...normalizeWorld(null), currentSceneId: "sc_village", activeMapId: "map_monde" })).toBe("map_monde");
  });
});

import { planPlayerMove } from "@/lib/engine/map-ops";

describe("planPlayerMove", () => {
  const world = {
    ...normalizeWorld(null),
    mapState: {
      map_crypte: {
        revealed: ["5,12", "5,11", "5,10", "5,9", "6,9", "4,5"],
        tokens: [{ entityId: "ent:Bobby", x: 5, y: 12 }, { entityId: "ent:Squelette", x: 4, y: 5 }],
      },
    },
  };
  const base = { map: crypt, world, entityId: "ent:Bobby", budgetCells: 6, usedCells: 0, diagonal: "chebyshev" as const, inCombat: true, isMyTurn: true };

  it("autorise un déplacement révélé dans la portée et en donne le coût", () => {
    // 3 cases tout droit puis 1 : la diagonale raserait une case non révélée.
    expect(planPlayerMove({ ...base, target: { x: 6, y: 9 } })).toBe(4);
  });
  it("refuse hors tour, case cachée, case occupée, portée épuisée, carte de voyage", () => {
    expect(() => planPlayerMove({ ...base, isMyTurn: false, target: { x: 5, y: 11 } })).toThrow(/pas votre tour/);
    expect(() => planPlayerMove({ ...base, target: { x: 5, y: 8 } })).toThrow(/pas révélée/);
    expect(() => planPlayerMove({ ...base, target: { x: 4, y: 5 } })).toThrow(/occupée/);
    expect(() => planPlayerMove({ ...base, usedCells: 5, target: { x: 6, y: 9 } })).toThrow(/reste 1 case/) // 4 > 1;
    const vallee = story.maps.find((m) => m.id === "map_vallee")!;
    expect(() => planPlayerMove({ ...base, map: vallee, target: { x: 1, y: 1 } })).toThrow(/MJ déplace le groupe/);
  });
});
