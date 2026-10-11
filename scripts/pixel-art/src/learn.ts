import { alphaAt, rgbAt, type Bitmap, type Rgb } from './image'
import { labDistance, nearestIndex, toOklab, type Lab } from './palette'

/**
 * Learns one shared palette from a batch of sprites: k-means in OKLab
 * over every opaque pixel. A stock palette bends the model's colours
 * (grey armour turns blue); a learned one keeps them, and sharing it
 * across the batch is what keeps the sprites consistent.
 *
 * Colours are weighted by the square root of their count, so that a
 * cloak's hundred pixels do not drown out the two pixels of an eye.
 */
export function learnPalette(sprites: readonly Bitmap[], size: number, iterations = 25): Rgb[] {
  const counts = new Map<string, { rgb: Rgb; count: number }>()
  for (const sprite of sprites) {
    for (let y = 0; y < sprite.height; y++) {
      for (let x = 0; x < sprite.width; x++) {
        if (alphaAt(sprite, x, y) < 128) continue
        const rgb = rgbAt(sprite, x, y)
        const key = rgb.join()
        counts.set(key, { rgb, count: (counts.get(key)?.count ?? 0) + 1 })
      }
    }
  }
  const points = [...counts.values()].map(({ rgb, count }) => ({ rgb, lab: toOklab(rgb), weight: Math.sqrt(count) }))
  if (points.length <= size) return points.map((p) => p.rgb)

  // Deterministic k-means++: the heaviest point, then each time the
  // point that the current centres explain worst.
  const centres: Lab[] = [points.reduce((a, b) => (b.weight > a.weight ? b : a)).lab]
  while (centres.length < size) {
    let worst = points[0]!
    let worstScore = -1
    for (const p of points) {
      const d = Math.min(...centres.map((c) => labDistance(p.lab, c)))
      const score = p.weight * d * d
      if (score > worstScore) {
        worstScore = score
        worst = p
      }
    }
    centres.push(worst.lab)
  }

  for (let round = 0; round < iterations; round++) {
    const sums = centres.map(() => ({ l: 0, a: 0, b: 0, w: 0 }))
    for (const p of points) {
      const s = sums[nearestIndex(p.lab, centres)]!
      s.l += p.lab[0] * p.weight
      s.a += p.lab[1] * p.weight
      s.b += p.lab[2] * p.weight
      s.w += p.weight
    }
    sums.forEach((s, i) => {
      if (s.w > 0) centres[i] = [s.l / s.w, s.a / s.w, s.b / s.w]
    })
  }

  // Each centre becomes the real colour closest to it: no invented hues.
  return centres.map((c) => points[nearestIndex(c, points.map((p) => p.lab))]!.rgb)
}

export function toHexFile(palette: readonly Rgb[]): string {
  const sorted = [...palette].sort((a, b) => toOklab(a)[0] - toOklab(b)[0])
  return sorted.map((rgb) => rgb.map((c) => c.toString(16).padStart(2, '0')).join('')).join('\n') + '\n'
}
