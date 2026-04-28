// Dice notation parser and roller.
// Supported syntax:
//   "1d20"        -> roll one d20
//   "1d20+5"      -> roll one d20, add 5
//   "8d6"         -> roll 8 d6
//   "2d6+3"       -> 2d6 + 3
//   "8d6/2"       -> roll 8d6 then halve (round down) — used for half-damage on save
//   "1d8-2"       -> roll 1d8 minus 2
//   "5"           -> flat number, returns 5
// Notation is case-insensitive. Whitespace is allowed.

import type { RollDetail } from "./types";

export type Rng = () => number; // returns [0, 1)

export const defaultRng: Rng = () => Math.random();

// ----------------------------------------------------------------------------
// Parse
// ----------------------------------------------------------------------------

export interface ParsedDice {
  count: number; // number of dice (0 if flat)
  faces: number; // dice faces (0 if flat)
  modifier: number; // +/- flat modifier
  half: boolean; // /2 suffix
  flat: boolean; // true when notation is just a number
}

export function parseDiceNotation(input: string): ParsedDice {
  const cleaned = input.replace(/\s+/g, "").toLowerCase();
  if (!cleaned) throw new Error("Empty dice notation");

  // Detect /2 (and other halve forms) suffix
  let half = false;
  let body = cleaned;
  const halfMatch = body.match(/\/(\d+)$/);
  if (halfMatch) {
    if (halfMatch[1] !== "2") {
      throw new Error(`Only /2 suffix is supported, got /${halfMatch[1]}`);
    }
    half = true;
    body = body.slice(0, -halfMatch[0].length);
  }

  // Plain number? (allow negative)
  if (/^-?\d+$/.test(body)) {
    return {
      count: 0,
      faces: 0,
      modifier: parseInt(body, 10),
      half,
      flat: true,
    };
  }

  // NdM(+|-K)?
  const re = /^(\d+)d(\d+)(?:([+\-])(\d+))?$/;
  const m = body.match(re);
  if (!m) throw new Error(`Invalid dice notation: "${input}"`);

  const count = parseInt(m[1], 10);
  const faces = parseInt(m[2], 10);
  const sign = m[3];
  const num = m[4] ? parseInt(m[4], 10) : 0;
  const modifier = sign === "-" ? -num : num;

  if (count <= 0 || count > 100) throw new Error(`Dice count out of range: ${count}`);
  if (faces <= 0 || faces > 1000) throw new Error(`Dice faces out of range: ${faces}`);

  return { count, faces, modifier, half, flat: false };
}

// ----------------------------------------------------------------------------
// Roll
// ----------------------------------------------------------------------------

export interface RollOptions {
  advantage?: boolean;
  disadvantage?: boolean;
  rng?: Rng;
}

export function rollDice(notation: string, opts: RollOptions = {}): RollDetail {
  const parsed = parseDiceNotation(notation);
  const rng = opts.rng ?? defaultRng;

  if (parsed.flat) {
    return {
      notation,
      result: parsed.modifier,
      rolls: [],
      modifier: parsed.modifier,
    };
  }

  const rolls: number[] = [];
  for (let i = 0; i < parsed.count; i++) {
    rolls.push(rollFace(parsed.faces, rng));
  }
  let sum = rolls.reduce((a, b) => a + b, 0);
  if (parsed.half) sum = Math.floor(sum / 2);
  const result = sum + parsed.modifier;
  return {
    notation,
    result,
    rolls,
    modifier: parsed.modifier,
  };
}

function rollFace(faces: number, rng: Rng): number {
  return 1 + Math.floor(rng() * faces);
}

// d20 with advantage/disadvantage handling.
// Always rolls 1d20 (with advantage=2 keep highest, disadvantage=2 keep lowest,
// both true cancel out).
export function rollD20(modifier = 0, opts: RollOptions = {}): RollDetail {
  const rng = opts.rng ?? defaultRng;
  const advantage = !!opts.advantage && !opts.disadvantage;
  const disadvantage = !!opts.disadvantage && !opts.advantage;

  if (advantage || disadvantage) {
    const a = rollFace(20, rng);
    const b = rollFace(20, rng);
    const kept = advantage ? Math.max(a, b) : Math.min(a, b);
    return {
      notation: "1d20" + formatModifier(modifier),
      result: kept + modifier,
      rolls: [a, b],
      modifier,
      advantage,
      disadvantage,
    };
  }

  const r = rollFace(20, rng);
  return {
    notation: "1d20" + formatModifier(modifier),
    result: r + modifier,
    rolls: [r],
    modifier,
  };
}

function formatModifier(mod: number): string {
  if (mod === 0) return "";
  return mod > 0 ? `+${mod}` : `${mod}`;
}
