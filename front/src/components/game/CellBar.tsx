import type { CSSProperties } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { usePrefersReducedMotion } from '@/lib/usePrefersReducedMotion'
import { shade } from './pixels'
import { statColor, type Stat } from './stats'

export type CellState = 'lit' | 'spent' | 'empty'

/**
 * One state per cell, one cell per point: `value` lit, then the `spent`
 * cells the last action just used (they whiten and fall), then empty
 * ones. Spending more than is left only spends what is left.
 */
export function cellStates(value: number, max: number, spent = 0): CellState[] {
  const total = Math.max(1, Math.round(max))
  const lit = Math.max(0, Math.min(total, Math.round(value)))
  const falling = Math.max(0, Math.min(total - lit, Math.round(spent)))
  return Array.from({ length: total }, (_, i) => (i < lit ? 'lit' : i < lit + falling ? 'spent' : 'empty'))
}

/**
 * A countable resource as a bar of square cells (board « Composant —
 * barre en cases »): spell slots, movement, ammunition, experience. A
 * light runs over the lit cells; at 30 % or less they blink. Cells just
 * spent flash white and fall once (remount to replay); under reduced
 * motion they are simply empty. Two rows give the cells a lit top and a
 * shaded bottom, for a bigger bar.
 */
export function CellBar({
  stat,
  value,
  max,
  spent = 0,
  label,
  rows = 1,
  height = 14,
  width,
  animated = true,
  className,
}: {
  /** The colour of the cells. */
  stat: Stat
  value: number
  max: number
  /** Cells the last action used. */
  spent?: number
  /** What is counted (« Emplacements de sort »); the stat's name by default. */
  label?: string
  rows?: 1 | 2
  /** Height of one cell row. */
  height?: number
  /** Width in pixels; the bar fills its container when omitted. */
  width?: number
  animated?: boolean
  className?: string
}) {
  const { t } = useTranslation()
  const reduced = usePrefersReducedMotion()
  const motion = animated && !reduced
  const states = cellStates(value, max, spent)
  const lit = states.filter((s) => s === 'lit').length
  const low = lit / states.length <= 0.3
  const base = statColor(stat)
  const firstSpent = lit + states.filter((s) => s === 'spent').length - 1

  const cells = Array.from({ length: rows }, (_, r) =>
    states.map((s, i) => {
      const on = rows === 1 ? shade(base, 'light', 0.08) : r === 0 ? shade(base, 'light', 0.2) : shade(base, 'dark', 0.18)
      const off = r === 0 ? '#1e1e1e' : '#171717'
      // Under reduced motion a spent cell is just empty.
      const state: CellState = s === 'spent' && !motion ? 'empty' : s
      const effect = state === 'lit' ? (low ? 'gk-blink' : r === 0 ? 'gk-glint' : '') : state === 'spent' ? 'gk-fall' : ''
      const style: CSSProperties = {
        height,
        background: state === 'empty' ? off : on,
        animationDelay: `${(state === 'spent' ? (firstSpent - i) * 0.1 : i * 0.05).toFixed(2)}s`,
      }
      return { key: `${r}-${i}`, state, effect, style }
    }),
  ).flat()

  return (
    <span
      role="img"
      aria-label={t('game.cells', { name: label ?? t(`game.stats.${stat}`), value: lit, max: states.length })}
      className={cn('gk-cells', motion && 'gk-on', className)}
      style={{ width: width ?? '100%', gridTemplateColumns: `repeat(${states.length}, minmax(0, 1fr))` }}
    >
      {cells.map((c) => (
        <i key={c.key} data-state={c.state} className={c.effect} style={c.style} />
      ))}
    </span>
  )
}
