// Opérations du MJ sur les cartes : brouillard, pions, carte affichée.
// Fonction pure : renvoie un nouvel état du monde.

import { blockedSet, cellKey } from "./grid";
import type { CampaignStory } from "./story";
import { PARTY_TOKEN, mapTokens, type WorldState } from "./world";

export type MapOp =
  | { op: "reveal" | "hide"; mapId: string; cells: [number, number][] }
  | { op: "reveal_all" | "hide_all"; mapId: string }
  | { op: "move_token"; mapId: string; entityId: string; x: number; y: number }
  | { op: "remove_token"; mapId: string; entityId: string }
  | { op: "set_active"; mapId: string | null };

export class MapOpError extends Error {}

export function applyMapOp(
  world: WorldState,
  story: CampaignStory,
  op: MapOp,
  knownEntityIds: Set<string>,
): WorldState {
  const next: WorldState = structuredClone(world);
  if (op.op === "set_active") {
    if (op.mapId !== null && !story.maps.some((m) => m.id === op.mapId)) throw new MapOpError(`Carte inconnue : ${op.mapId}`);
    next.activeMapId = op.mapId ?? undefined;
    return next;
  }
  const map = story.maps.find((m) => m.id === op.mapId);
  if (!map) throw new MapOpError(`Carte inconnue : ${op.mapId}`);
  const rt = (next.mapState[map.id] ??= { revealed: [] });
  const inBounds = (x: number, y: number) =>
    Number.isInteger(x) && Number.isInteger(y) && x >= 0 && y >= 0 && x < map.grid.cols && y < map.grid.rows;

  switch (op.op) {
    case "reveal":
    case "hide": {
      const set = new Set(rt.revealed);
      for (const [x, y] of op.cells) {
        if (!inBounds(x, y)) continue;
        if (op.op === "reveal") set.add(cellKey(x, y));
        else set.delete(cellKey(x, y));
      }
      rt.revealed = [...set];
      return next;
    }
    case "reveal_all": {
      const all: string[] = [];
      for (let y = 0; y < map.grid.rows; y++) for (let x = 0; x < map.grid.cols; x++) all.push(cellKey(x, y));
      rt.revealed = all;
      return next;
    }
    case "hide_all":
      rt.revealed = [];
      return next;
    case "move_token": {
      if (op.entityId !== PARTY_TOKEN && !knownEntityIds.has(op.entityId)) throw new MapOpError(`Fiche inconnue : ${op.entityId}`);
      if (!inBounds(op.x, op.y)) throw new MapOpError(`Case (${op.x}, ${op.y}) hors de la carte`);
      if (blockedSet(map).has(cellKey(op.x, op.y))) throw new MapOpError("Case infranchissable");
      const tokens = mapTokens(map, world).filter((t) => t.entityId !== op.entityId);
      if (tokens.some((t) => t.x === op.x && t.y === op.y)) throw new MapOpError("Case déjà occupée");
      rt.tokens = [...tokens, { entityId: op.entityId, x: op.x, y: op.y }];
      return next;
    }
    case "remove_token":
      rt.tokens = mapTokens(map, world).filter((t) => t.entityId !== op.entityId);
      return next;
  }
}
