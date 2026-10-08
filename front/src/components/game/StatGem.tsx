import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { pixelShadow, shade, type Cell } from './pixels'
import { shapeMask, statFill, type Stat } from './stats'

/** Light bands in a wave: each lights up in turn, then fades. */
const BANDS = 8
const TAU = Math.PI * 2

/**
 * Where a cell sits in its stat's wave of light (0 lights first, 1 last):
 * HP beats from the centre out, MOVE runs left to right, ATK crosses on
 * the diagonal, AC closes from the edges in, MAG spirals, INIT turns
 * clockwise (MEMORY.md §2).
 */
const WAVE: Record<Stat, (u: number, v: number) => number> = {
  hp: (u, v) => Math.min(1, Math.hypot(u - 0.5, v - 0.45) / 0.62),
  move: (u) => u,
  atk: (u, v) => (u + v) / 2,
  ac: (u, v) => 1 - Math.min(1, Math.hypot(u - 0.5, v - 0.45) / 0.62),
  mag: (u, v) => ((Math.atan2(v - 0.5, u - 0.5) + Math.PI) / TAU + Math.hypot(u - 0.5, v - 0.5)) % 1,
  init: (u, v) => (Math.atan2(u - 0.5, -(v - 0.5)) + Math.PI) / TAU,
}

/** The gem's grid: 7, 9 or 11 cells wide by size. */
function gridSize(size: number) {
  return size < 30 ? 7 : size < 56 ? 9 : 11
}

function drawGem(stat: Stat, size: number) {
  const n = gridSize(size)
  const c = Math.max(2, Math.floor(size / n))
  const gap = c >= 5 ? 1 : 0
  const off = Math.floor((size - c * n) / 2)
  const mask = shapeMask(stat, n)
  const on = (x: number, y: number) => x >= 0 && y >= 0 && x < n && y < n && mask[y * n + x]
  const base = statFill(stat)

  const cells: [number, number][] = []
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) if (on(x, y)) cells.push([x, y])

  // Lit from the top left, darker towards the bottom right, a darker rim
  // and one white highlight pixel.
  const body: Cell[] = cells.map(([x, y]) => {
    const edge = !on(x - 1, y) || !on(x + 1, y) || !on(x, y - 1) || !on(x, y + 1)
    const t = (x + y) / (2 * (n - 1))
    let color = t < 0.5 ? shade(base, 'light', (0.5 - t) * 0.7) : shade(base, 'dark', (t - 0.5) * 0.55)
    if (edge) color = shade(color, 'dark', 0.28)
    if (x === Math.floor(n * 0.3) && y === Math.floor(n * 0.25)) color = '#ffffff'
    return [x, y, color]
  })

  const wave = WAVE[stat]
  const glint = shade(base, 'light', 0.7)
  const bands = Array.from({ length: BANDS }, (_, band) =>
    cells
      .filter(([x, y]) => Math.min(BANDS - 1, Math.floor(wave((x + 0.5) / n, (y + 0.5) / n) * BANDS)) === band)
      .map(([x, y]): Cell => [x, y, glint]),
  )

  // Cells are drawn as squares of `c - gap`, offset to centre the grid.
  const shift = (list: Cell[]) => pixelShadow(list, c, 1, off)
  return { cell: c - gap, c, body: shift(body), bands: bands.map(shift) }
}

/**
 * A stat gem (board « Composant — gemme »): the stat's shape as a small
 * pixel grid, its value on top, and a wave of light whose path belongs
 * to the stat. HP beats at heartbeat pace, the others at the gem pace.
 */
export function StatGem({
  stat,
  value = '',
  size = 64,
  animated = true,
  className,
}: {
  stat: Stat
  /** Shown on the gem, as data gives it (`+5`, `18`, `d10`). */
  value?: string
  size?: number
  /** The wave of light; off for gems in a list, where many would flicker. */
  animated?: boolean
  className?: string
}) {
  const { t } = useTranslation()
  const gem = useMemo(() => drawGem(stat, size), [stat, size])
  const name = t(`game.stats.${stat}`)
  const duration = stat === 'hp' ? 1.3 : 2.6
  const step = (duration * 0.45) / BANDS

  return (
    <span
      role="img"
      aria-label={value ? t('game.gem', { name, value }) : name}
      className={cn('gk-gem', animated && 'gk-on', className)}
      style={{ width: size, height: size }}
    >
      <span
        className="gk-layer"
        style={{ left: -gem.c, top: -gem.c, width: gem.cell, height: gem.cell, boxShadow: gem.body }}
      />
      {gem.bands.map((shadow, i) => (
        <span
          key={i}
          className="gk-layer gk-glint"
          style={{
            left: -gem.c,
            top: -gem.c,
            width: gem.cell,
            height: gem.cell,
            boxShadow: shadow,
            animationDelay: `${(i * step).toFixed(3)}s`,
            animationDuration: `${duration}s`,
          }}
        />
      ))}
      {value && (
        <span
          aria-hidden
          className="gk-gem-value"
          style={{
            fontSize: Math.round(size * (value.length > 2 ? 0.27 : 0.33)),
            marginTop: stat === 'hp' ? -Math.round(size * 0.02) : stat === 'ac' ? -Math.round(size * 0.05) : 0,
          }}
        >
          {value}
        </span>
      )}
    </span>
  )
}
