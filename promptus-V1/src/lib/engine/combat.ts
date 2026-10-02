// Combat sur grille : attaques des fiches, portée, ligne de vue.
// Fonctions pures, partagées par le serveur (validation) et les écrans
// (surbrillance des cibles atteignables).

import { abilityModifier, type Ruleset } from "./ruleset";
import { cellKey, distance, type Cell, type DiagonalRule } from "./grid";
import type { GameMap } from "./story";
import { mapTokens, type WorldState } from "./world";

/** Attaque d'une fiche (attributes.attacks). Portées en mètres. */
export interface CreatureAttack {
  id: string;
  name: string;
  bonus: number;
  damage: string;
  damageType: string;
  /** Allonge ou portée normale (1,5 m = corps à corps). */
  rangeMeters: number;
  /** Portée longue : au-delà de la portée normale, avec désavantage. */
  longRangeMeters?: number;
}

const num = (v: unknown): number | undefined => (typeof v === "number" && Number.isFinite(v) ? v : undefined);
const str = (v: unknown): string | undefined => (typeof v === "string" && v.trim() ? v.trim() : undefined);

function slug(s: string): string {
  return (
    s
      .normalize("NFD")
      .replace(/[̀-ͯ]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_|_$/g, "") || "attaque"
  );
}

/**
 * Attaques lisibles d'une fiche. Les entrées mal formées sont ignorées ; sans
 * attaque définie, une attaque à mains nues est déduite de la Force.
 */
export function creatureAttacks(attributes: Record<string, unknown>, ruleset: Ruleset): CreatureAttack[] {
  const raw = Array.isArray(attributes.attacks) ? (attributes.attacks as unknown[]) : [];
  const out: CreatureAttack[] = [];
  const seen = new Set<string>();
  for (const r of raw) {
    if (!r || typeof r !== "object") continue;
    const a = r as Record<string, unknown>;
    const name = str(a.name);
    const damage = str(a.damage);
    if (!name || !damage) continue;
    let id = str(a.id) ?? slug(name);
    while (seen.has(id)) id = `${id}_2`;
    seen.add(id);
    const range = num(a.rangeMeters) ?? ruleset.movement.local.cellMeters;
    const long = num(a.longRangeMeters);
    out.push({
      id,
      name,
      bonus: num(a.bonus) ?? 0,
      damage,
      damageType: str(a.damageType) ?? ruleset.damageTypes[0]?.id ?? "bludgeoning",
      rangeMeters: range,
      longRangeMeters: long && long > range ? long : undefined,
    });
  }
  if (out.length) return out;

  const scores = (attributes.abilityScores as Record<string, number> | undefined) ?? {};
  const str_ = ruleset.abilities[0]?.id;
  const mod = str_ && typeof scores[str_] === "number" ? abilityModifier(ruleset, scores[str_]) : 0;
  const prof = num(attributes.proficiencyBonus) ?? 0;
  const bludgeoning = ruleset.damageTypes.find((d) => d.id === "bludgeoning")?.id ?? ruleset.damageTypes[0]?.id ?? "bludgeoning";
  return [
    {
      id: "unarmed",
      name: "Coup à mains nues",
      bonus: mod + prof,
      damage: mod > 0 ? `1+${mod}` : "1",
      damageType: bludgeoning,
      rangeMeters: ruleset.movement.local.cellMeters,
    },
  ];
}

/** Mètres → cases (au moins une case : le contact). */
export function metersToCells(meters: number, cellMeters: number): number {
  return Math.max(1, Math.floor(meters / cellMeters + 1e-9));
}

// ----------------------------------------------------------------------------
// Ligne de vue
// ----------------------------------------------------------------------------

/**
 * Étapes du segment a→b entre centres de cases, extrémités exclues. Chaque
 * étape est une case, ou deux cases quand la ligne passe exactement par un
 * coin (elle ne bloque alors que si les deux cases bloquent).
 */
export function lineSteps(type: GameMap["grid"]["type"], a: Cell, b: Cell): Cell[][] {
  if (type === "hex") return hexLine(a, b).slice(1, -1).map((c) => [c]);
  const out: Cell[][] = [];
  const nx = Math.abs(b.x - a.x);
  const ny = Math.abs(b.y - a.y);
  const sx = Math.sign(b.x - a.x);
  const sy = Math.sign(b.y - a.y);
  let x = a.x;
  let y = a.y;
  let ix = 0;
  let iy = 0;
  while (ix < nx || iy < ny) {
    const cmp = (1 + 2 * ix) * ny - (1 + 2 * iy) * nx;
    if (cmp === 0) {
      out.push([{ x: x + sx, y }, { x, y: y + sy }]);
      x += sx;
      y += sy;
      ix++;
      iy++;
    } else if (cmp < 0) {
      x += sx;
      ix++;
    } else {
      y += sy;
      iy++;
    }
    if (x !== b.x || y !== b.y) out.push([{ x, y }]);
  }
  return out;
}

function toCube(x: number, y: number) {
  const q = x - (y - (y & 1)) / 2;
  return { q, r: y, s: -q - y };
}

function hexLine(a: Cell, b: Cell): Cell[] {
  const A = toCube(a.x, a.y);
  const B = toCube(b.x, b.y);
  const n = Math.max(Math.abs(A.q - B.q), Math.abs(A.r - B.r), Math.abs(A.s - B.s));
  const out: Cell[] = [];
  for (let i = 0; i <= n; i++) {
    const t = n === 0 ? 0 : i / n;
    // Léger décalage pour éviter les égalités sur les arêtes.
    let q = A.q + (B.q - A.q) * t + 1e-6;
    let r = A.r + (B.r - A.r) * t + 1e-6;
    let s = A.s + (B.s - A.s) * t - 2e-6;
    let rq = Math.round(q);
    let rr = Math.round(r);
    const rs = Math.round(s);
    const dq = Math.abs(rq - q);
    const dr = Math.abs(rr - r);
    const ds = Math.abs(rs - s);
    if (dq > dr && dq > ds) rq = -rr - rs;
    else if (dr > ds) rr = -rq - rs;
    q = rq;
    r = rr;
    s = -q - r;
    out.push({ x: q + (r - (r & 1)) / 2, y: r });
  }
  return out;
}

/** Vrai si aucune case infranchissable ne coupe la ligne a→b. */
export function hasLineOfSight(map: Pick<GameMap, "grid" | "cells">, a: Cell, b: Cell): boolean {
  const blocked = new Set(map.cells.filter((c) => c.blocked).map((c) => cellKey(c.x, c.y)));
  return lineSteps(map.grid.type, a, b).every((step) => step.some((c) => !blocked.has(cellKey(c.x, c.y))));
}

// ----------------------------------------------------------------------------
// Géométrie d'une attaque
// ----------------------------------------------------------------------------

export interface AttackGeometry {
  distanceCells: number;
  distanceMeters: number;
  /** Au contact (une case) : utile pour les critiques automatiques. */
  melee: boolean;
  inRange: boolean;
  /** Portée longue : désavantage. */
  longRange: boolean;
  lineOfSight: boolean;
  /** Raison du refus, ou null si l'attaque est possible. */
  reason: string | null;
}

export function attackGeometry(input: {
  map: Pick<GameMap, "grid" | "cells">;
  from: Cell;
  to: Cell;
  attack: Pick<CreatureAttack, "rangeMeters" | "longRangeMeters">;
  cellMeters: number;
  diagonal: DiagonalRule;
}): AttackGeometry {
  const { map, from, to, attack, cellMeters, diagonal } = input;
  const d = distance(map.grid.type, from, to, diagonal);
  const normal = metersToCells(attack.rangeMeters, cellMeters);
  const long = attack.longRangeMeters ? metersToCells(attack.longRangeMeters, cellMeters) : normal;
  const inRange = d <= long;
  const longRange = inRange && d > normal;
  const los = d <= 1 || hasLineOfSight(map, from, to);
  const fmt = (m: number) => `${m.toLocaleString("fr-FR")} m`;
  const reason = !inRange
    ? `Hors de portée : ${fmt(d * cellMeters)} pour ${fmt(attack.longRangeMeters ?? attack.rangeMeters)} maximum`
    : !los
      ? "Pas de ligne de vue (obstacle)"
      : null;
  return { distanceCells: d, distanceMeters: d * cellMeters, melee: d <= 1, inRange, longRange, lineOfSight: los, reason };
}

/** Position de deux créatures sur la carte affichée (null si l'une manque). */
export function tokenPositions(
  map: GameMap,
  world: WorldState,
  a: string,
  b: string,
): { from: Cell; to: Cell } | null {
  const tokens = mapTokens(map, world);
  const from = tokens.find((t) => t.entityId === a);
  const to = tokens.find((t) => t.entityId === b);
  return from && to ? { from, to } : null;
}
