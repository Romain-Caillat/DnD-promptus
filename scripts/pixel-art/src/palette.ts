import type { Rgb } from './image'

/**
 * The house palette, in ramps from dark to light. Fixed rather than
 * learned, because a learned palette drops whatever is rare in the
 * batch, and skin is rare: small faces came out grey. Every ramp a
 * character needs has its own entries here.
 */
export const HOUSE_32 = [
  // outline
  '1a1423',
  // metal, cloth, stone
  '3b3743', '5e5a68', '8b8896', 'b8b6c2', 'e2e1e8', 'ffffff',
  // skin, light to dark through the ramp, plus a deep one
  '4e2c22', '7a4632', 'b8714f', 'e2a47c', 'f5d0b0',
  // hair, fire
  '6b2418', 'a8381e', 'e2682c',
  // gold, light
  '8f5d1c', 'd49d2c', 'f7df6b',
  // leather, wood
  '3e2618', '6c4428', 'a06e40',
  // goblins, plants, healing
  '1f3a26', '3b6b35', '6ba346', 'b5d86a',
  // cloth, water
  '1f2c5c', '3b62ab', '82b4e3',
  // magic
  '5b3079', 'a66fd8',
  // blood
  '7c1b27', 'd33c3c',
].map(parseHex)

export function parseHex(hex: string): Rgb {
  const h = hex.replace(/^#/, '')
  if (!/^[0-9a-f]{6}$/i.test(h)) throw new Error(`not a colour: ${hex}`)
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)]
}

/** Reads a Lospec `.hex` file: one colour per line. */
export function parsePaletteFile(text: string): Rgb[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line !== '')
    .map(parseHex)
}

export type Lab = readonly [number, number, number]

function linear(c: number): number {
  const s = c / 255
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
}

/** OKLab: distances in it match what the eye sees far better than RGB. */
export function toOklab([r, g, b]: Rgb): Lab {
  const lr = linear(r)
  const lg = linear(g)
  const lb = linear(b)
  const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb)
  const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb)
  const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb)
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ]
}

export function labDistance(a: Lab, b: Lab): number {
  return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2])
}

export function nearestIndex(lab: Lab, palette: readonly Lab[]): number {
  let best = 0
  let bestDistance = Infinity
  palette.forEach((entry, i) => {
    const d = labDistance(lab, entry)
    if (d < bestDistance) {
      bestDistance = d
      best = i
    }
  })
  return best
}
