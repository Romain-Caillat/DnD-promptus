// Géométrie des grilles — pur TypeScript.
// Hexagones « pointe en haut », rangées impaires décalées (odd-r).
// Carrés : 8 voisins, coût des diagonales selon le ruleset.

import type { GameMap } from "./story";

export type GridType = "hex" | "square";
export type DiagonalRule = "chebyshev" | "alternate" | "euclidean";

export interface Cell {
  x: number;
  y: number;
}

export const cellKey = (x: number, y: number) => `${x},${y}`;

export function parseKey(key: string): Cell {
  const [x, y] = key.split(",").map(Number);
  return { x, y };
}

// ----------------------------------------------------------------------------
// Coordonnées écran
// ----------------------------------------------------------------------------

const SQRT3 = Math.sqrt(3);

/** Taille totale en pixels d'une grille, pour une case de `size` (rayon d'hexagone ou côté de carré). */
export function gridPixelSize(type: GridType, cols: number, rows: number, size: number): { width: number; height: number } {
  if (type === "square") return { width: cols * size, height: rows * size };
  const w = SQRT3 * size;
  return { width: w * cols + (rows > 1 ? w / 2 : 0), height: 1.5 * size * (rows - 1) + 2 * size };
}

export function cellCenter(type: GridType, x: number, y: number, size: number): { cx: number; cy: number } {
  if (type === "square") return { cx: (x + 0.5) * size, cy: (y + 0.5) * size };
  const w = SQRT3 * size;
  return { cx: w * (x + 0.5 * (y & 1)) + w / 2, cy: y * 1.5 * size + size };
}

/** Sommets d'un hexagone pointe en haut, sous forme de points SVG. */
export function hexPoints(cx: number, cy: number, size: number): string {
  const pts: string[] = [];
  for (let i = 0; i < 6; i++) {
    const a = (Math.PI / 180) * (60 * i - 30);
    pts.push(`${(cx + size * Math.cos(a)).toFixed(2)},${(cy + size * Math.sin(a)).toFixed(2)}`);
  }
  return pts.join(" ");
}

/** Case sous un point (pixels), ou null hors grille. */
export function pixelToCell(type: GridType, px: number, py: number, size: number, cols: number, rows: number): Cell | null {
  let best: Cell | null = null;
  if (type === "square") {
    best = { x: Math.floor(px / size), y: Math.floor(py / size) };
  } else {
    // Rangée approchée puis la case la plus proche parmi les candidates voisines.
    const approxY = Math.round((py - size) / (1.5 * size));
    let bestD = Infinity;
    for (let y = approxY - 1; y <= approxY + 1; y++) {
      const approxX = Math.round((px - SQRT3 * size * (0.5 + 0.5 * (y & 1))) / (SQRT3 * size));
      for (let x = approxX - 1; x <= approxX + 1; x++) {
        const { cx, cy } = cellCenter("hex", x, y, size);
        const d = (cx - px) ** 2 + (cy - py) ** 2;
        if (d < bestD) {
          bestD = d;
          best = { x, y };
        }
      }
    }
  }
  if (!best || best.x < 0 || best.y < 0 || best.x >= cols || best.y >= rows) return null;
  return best;
}

// ----------------------------------------------------------------------------
// Voisins et distances
// ----------------------------------------------------------------------------

const HEX_DIRS_EVEN: [number, number][] = [[1, 0], [0, -1], [-1, -1], [-1, 0], [-1, 1], [0, 1]];
const HEX_DIRS_ODD: [number, number][] = [[1, 0], [1, -1], [0, -1], [-1, 0], [0, 1], [1, 1]];
const SQUARE_DIRS: [number, number][] = [[1, 0], [-1, 0], [0, 1], [0, -1], [1, 1], [1, -1], [-1, 1], [-1, -1]];

export function neighbors(type: GridType, x: number, y: number, cols: number, rows: number): Cell[] {
  const dirs = type === "square" ? SQUARE_DIRS : y & 1 ? HEX_DIRS_ODD : HEX_DIRS_EVEN;
  return dirs
    .map(([dx, dy]) => ({ x: x + dx, y: y + dy }))
    .filter((c) => c.x >= 0 && c.y >= 0 && c.x < cols && c.y < rows);
}

function toCube(x: number, y: number): { q: number; r: number } {
  return { q: x - (y - (y & 1)) / 2, r: y };
}

/** Distance en cases entre deux cases. */
export function distance(type: GridType, a: Cell, b: Cell, diagonal: DiagonalRule = "chebyshev"): number {
  if (type === "hex") {
    const A = toCube(a.x, a.y);
    const B = toCube(b.x, b.y);
    const dq = A.q - B.q;
    const dr = A.r - B.r;
    return (Math.abs(dq) + Math.abs(dr) + Math.abs(dq + dr)) / 2;
  }
  const dx = Math.abs(a.x - b.x);
  const dy = Math.abs(a.y - b.y);
  const diag = Math.min(dx, dy);
  const straight = Math.max(dx, dy) - diag;
  switch (diagonal) {
    case "chebyshev":
      return diag + straight;
    case "alternate":
      return straight + diag + Math.floor(diag / 2);
    case "euclidean":
      return Math.round(Math.sqrt(dx * dx + dy * dy));
  }
}

// ----------------------------------------------------------------------------
// Déplacement
// ----------------------------------------------------------------------------

export function blockedSet(map: Pick<GameMap, "cells">): Set<string> {
  return new Set(map.cells.filter((c) => c.blocked).map((c) => cellKey(c.x, c.y)));
}

/**
 * Cases atteignables depuis `start` avec un budget de `maxCost` cases.
 * Les cases bloquées ne se traversent pas ; en carrés, une diagonale ne
 * passe pas entre deux obstacles (ni en rasant un mur).
 * Renvoie le coût minimal pour chaque case atteignable (départ inclus, coût 0).
 */
export function reachableCells(
  map: Pick<GameMap, "grid" | "cells">,
  start: Cell,
  maxCost: number,
  diagonal: DiagonalRule = "chebyshev",
  extraBlocked: Set<string> = new Set(),
): Map<string, number> {
  const { type, cols, rows } = map.grid;
  const blocked = blockedSet(map);
  for (const k of extraBlocked) blocked.add(k);
  const isFree = (x: number, y: number) => x >= 0 && y >= 0 && x < cols && y < rows && !blocked.has(cellKey(x, y));

  // État = case + parité des diagonales (règle 1-2-1 uniquement).
  const best = new Map<string, number>();
  const stateCost = new Map<string, number>();
  const queue: { x: number; y: number; cost: number; parity: number }[] = [{ ...start, cost: 0, parity: 0 }];
  stateCost.set(`${cellKey(start.x, start.y)}|0`, 0);

  while (queue.length) {
    // File de priorité simple : les grilles restent petites (≤ 200×200).
    let i = 0;
    for (let j = 1; j < queue.length; j++) if (queue[j].cost < queue[i].cost) i = j;
    const cur = queue.splice(i, 1)[0];
    const key = cellKey(cur.x, cur.y);
    if (cur.cost > (stateCost.get(`${key}|${cur.parity}`) ?? Infinity)) continue;
    if (cur.cost < (best.get(key) ?? Infinity)) best.set(key, cur.cost);

    for (const n of neighbors(type, cur.x, cur.y, cols, rows)) {
      if (!isFree(n.x, n.y)) continue;
      const isDiag = type === "square" && n.x !== cur.x && n.y !== cur.y;
      if (isDiag && (!isFree(n.x, cur.y) || !isFree(cur.x, n.y))) continue;
      let step = 1;
      let parity = cur.parity;
      if (isDiag) {
        if (diagonal === "alternate") {
          step = parity === 0 ? 1 : 2;
          parity = 1 - parity;
        } else if (diagonal === "euclidean") {
          step = 1.5;
        }
      }
      const cost = cur.cost + step;
      if (cost > maxCost + 1e-9) continue;
      const sk = `${cellKey(n.x, n.y)}|${parity}`;
      if (cost < (stateCost.get(sk) ?? Infinity)) {
        stateCost.set(sk, cost);
        queue.push({ x: n.x, y: n.y, cost, parity });
      }
    }
  }
  return best;
}

/** Vitesse d'une fiche en cases : `speedCells`, sinon `speed` en pieds (5e) converti, sinon défaut du ruleset. */
export function speedInCells(attributes: Record<string, unknown>, cellMeters: number, fallback: number): number {
  if (typeof attributes.speedCells === "number") return attributes.speedCells;
  if (typeof attributes.speed === "number" && attributes.speed > 0) {
    return Math.max(1, Math.round((attributes.speed * 0.3048) / cellMeters));
  }
  return fallback;
}
