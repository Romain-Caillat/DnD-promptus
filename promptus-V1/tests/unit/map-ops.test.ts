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
