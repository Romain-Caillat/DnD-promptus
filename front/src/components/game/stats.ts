/**
 * The six stats, their colour token and their shape (MEMORY.md §2):
 * colour is never the only cue. Shapes are drawn as small pixel grids,
 * with the same geometry as the canvas `Gemme` board (`StatGem`).
 */
export const STATS = ['hp', 'atk', 'ac', 'mag', 'move', 'init'] as const
export type Stat = (typeof STATS)[number]

/** Literal class names, so Tailwind sees them in the source. */
export const STAT_BG: Record<Stat, string> = {
  hp: 'bg-stat-hp',
  atk: 'bg-stat-atk',
  ac: 'bg-stat-ac',
  mag: 'bg-stat-mag',
  move: 'bg-stat-move',
  init: 'bg-stat-init',
}

export const STAT_TEXT: Record<Stat, string> = {
  hp: 'text-stat-hp',
  atk: 'text-stat-atk',
  ac: 'text-stat-ac',
  mag: 'text-stat-mag',
  move: 'text-stat-move',
  init: 'text-stat-init',
}

// (u, v) in [0, 1]², v downwards: is the cell centre inside the shape?
const INSIDE: Record<Stat, (u: number, v: number) => boolean> = {
  hp: (u, v) => {
    const x = (u - 0.5) * 2.6
    const y = (0.6 - v) * 2.6
    return (x * x + y * y - 1) ** 3 - x * x * y * y * y <= 0
  },
  atk: (u, v) => Math.abs(u - 0.5) + Math.abs(v - 0.5) <= 0.52,
  ac: (u, v) =>
    v >= 0.04 &&
    !(v < 0.1 && Math.abs(u - 0.5) > 0.36) &&
    (v <= 0.58
      ? Math.abs(u - 0.5) <= 0.46
      : Math.abs(u - 0.5) <= 0.46 * (1 - (v - 0.58) / 0.42) + 0.04),
  mag: (u, v) => Math.abs(v - 0.5) <= 0.47 && Math.abs(u - 0.5) <= 0.52 - 0.5 * Math.abs(v - 0.5),
  move: (u, v) => {
    const t = 1 - 2 * Math.abs(v - 0.5)
    return u <= 0.66 + 0.34 * t && u >= 0.32 * t
  },
  init: (u, v) => (u - 0.5) ** 2 + (v - 0.5) ** 2 <= 0.26,
}

/** Row-major mask of an n × n grid: true where the cell is filled. */
export function shapeMask(stat: Stat, n: number): boolean[] {
  const inside = INSIDE[stat]
  return Array.from({ length: n * n }, (_, i) => inside(((i % n) + 0.5) / n, (Math.floor(i / n) + 0.5) / n))
}

/** The stat colour as a CSS value: the token itself, never a copied hex. */
export function statColor(stat: Stat): string {
  return `var(--color-stat-${stat})`
}
