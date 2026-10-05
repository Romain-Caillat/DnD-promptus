/**
 * Pixel art drawn with one element and a `box-shadow` per cell, the way
 * the design canvas draws it (gems, hearts, items, condition icons): the
 * cells stay crisp at any zoom, with no raster asset.
 */

/** One drawn cell: column, row (both in cells) and its colour. */
export type Cell = readonly [x: number, y: number, color: string]

/**
 * `box-shadow` for a list of cells of `p` pixels. The shadow-casting
 * element is a `p × p` box placed one cell up and left of the drawing
 * (`left: -p; top: -p`), so cell (x, y) lands at
 * (origin + (x + offset) · p, …).
 */
export function pixelShadow(cells: readonly Cell[], p: number, offset = 1, origin = 0): string {
  if (cells.length === 0) return 'none'
  const at = (v: number) => `${origin + (v + offset) * p}px`
  return cells.map(([x, y, color]) => `${at(x)} ${at(y)} 0 0 ${color}`).join(', ')
}

/**
 * A pattern (rows of `X` and `.`) with a one-cell outline around it:
 * `X` stays `X`, every empty cell touching one becomes `o`. The result is
 * one cell larger on every side.
 */
export function outlined(pattern: readonly string[]): string[] {
  const at = (x: number, y: number) => (pattern[y] ?? '')[x] === 'X'
  const width = Math.max(...pattern.map((r) => r.length))
  const rows: string[] = []
  for (let y = -1; y <= pattern.length; y++) {
    let row = ''
    for (let x = -1; x <= width; x++) {
      if (at(x, y)) row += 'X'
      else row += at(x + 1, y) || at(x - 1, y) || at(x, y + 1) || at(x, y - 1) ? 'o' : '.'
    }
    rows.push(row)
  }
  return rows
}

/**
 * A colour pushed towards white or the table black by `t` (0 to 1), as
 * CSS: the base may be a token (`var(--color-stat-hp)`), so the mix is
 * left to the browser instead of being computed from a copied hex.
 */
export function shade(color: string, to: 'light' | 'dark', t: number): string {
  if (t <= 0) return color
  const pct = Math.round(Math.min(1, t) * 1000) / 10
  return `color-mix(in srgb, ${color}, ${to === 'light' ? '#ffffff' : '#0a0a0a'} ${pct}%)`
}
