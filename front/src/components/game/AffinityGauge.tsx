import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

type AffinityCell = 'favour' | 'grudge' | 'empty'

/**
 * One cell per point on each side of neutral, from `min` to `max`: the
 * cells between zero and `value` are filled, ivory on the favour side,
 * black with a white edge on the grudge side (the boon / bane split of
 * the condition badges). Zero has no cell: it is the mark in the middle.
 */
function affinityCells(value: number, min: number, max: number): { left: AffinityCell[]; right: AffinityCell[] } {
  const lo = Math.min(0, Math.round(min))
  const hi = Math.max(0, Math.round(max))
  const v = Math.max(lo, Math.min(hi, Math.round(value)))
  // Left side read outward from the middle, then reversed for display.
  const left = Array.from({ length: -lo }, (_, i): AffinityCell => (v <= -(i + 1) ? 'grudge' : 'empty')).reverse()
  const right = Array.from({ length: hi }, (_, i): AffinityCell => (v >= i + 1 ? 'favour' : 'empty'))
  return { left, right }
}

/**
 * A faction's standing with the party (campaign/track-factions-and-goals):
 * a bar of square cells either side of a neutral mark, the number beside
 * it. Countable things are cells in this game (`MEMORY.md` §2), and the
 * sign is read from the side, never from a hue.
 */
export function AffinityGauge({
  name,
  value,
  min,
  max,
  cell = 12,
  className,
}: {
  /** The faction, for the accessible label. */
  name: string
  value: number
  min: number
  max: number
  cell?: number
  className?: string
}) {
  const { t } = useTranslation()
  const { left, right } = affinityCells(value, min, max)
  const square = (c: AffinityCell, key: string) => (
    <i
      key={key}
      data-state={c}
      className={cn(
        'block rounded-[2px] border',
        c === 'favour' && 'border-ivory bg-ivory',
        c === 'grudge' && 'border-chalk bg-ink',
        c === 'empty' && 'border-line bg-well',
      )}
      style={{ width: cell, height: cell }}
    />
  )
  return (
    <span
      role="img"
      aria-label={t('game.affinity', { name, value: value > 0 ? `+${value}` : `${value}` })}
      className={cn('inline-flex items-center gap-[3px]', className)}
    >
      {left.map((c, i) => square(c, `l${i}`))}
      <i className="mx-[2px] block w-[2px] bg-chalk" style={{ height: cell + 6 }} />
      {right.map((c, i) => square(c, `r${i}`))}
      <b className="ml-1.5 min-w-[2ch] text-caption tabular-nums">{value > 0 ? `+${value}` : value}</b>
    </span>
  )
}
