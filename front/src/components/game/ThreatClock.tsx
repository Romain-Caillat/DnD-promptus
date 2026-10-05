import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

/**
 * A front's threat clock as a ring of square cells, not a pie (board
 * « Composant — horloge de menace »): `parts` parts of a few squares
 * each, with a gap between parts; filled parts are chalk, and the next
 * one blinks while the GM is about to tick it.
 */
export function ThreatClock({
  parts,
  filled,
  size = 112,
  next = true,
  showCount = true,
  name,
  className,
}: {
  /** 4 to 6 for a front (MEMORY.md §1); 2 to 12 drawn. */
  parts: number
  filled: number
  size?: number
  /** The next part blinks: the GM is about to tick it. */
  next?: boolean
  /** « 3/6 » in the middle, from 56 px up. */
  showCount?: boolean
  /** The front it measures, read by screen readers. */
  name?: string
  className?: string
}) {
  const { t } = useTranslation()
  const done = Math.max(0, Math.min(parts, filled))
  // Squares per part shrink with size so the ring keeps visible gaps.
  const k = size >= 90 ? 5 : size >= 56 ? 3 : 2
  const total = parts * (k + 1)
  const s = Math.max(3, Math.round(size * 0.07))
  const radius = size / 2 - s * 0.8
  const centre = size / 2

  const squares = []
  for (let i = 0; i < total; i++) {
    if (i % (k + 1) === k) continue
    const a = ((-90 + ((i + 0.5) * 360) / total) * Math.PI) / 180
    const part = Math.floor(i / (k + 1))
    const state = part < done ? 'filled' : part === done && next ? 'next' : 'empty'
    squares.push(
      <i
        key={i}
        data-state={state}
        className={cn(state === 'filled' ? 'gk-tick-on' : state === 'next' ? 'gk-tick-next' : 'gk-tick-off')}
        style={{
          left: Math.round(centre + radius * Math.cos(a) - s / 2),
          top: Math.round(centre + radius * Math.sin(a) - s / 2),
          width: s,
          height: s,
          animationDelay: `${((i % (k + 1)) * 0.12).toFixed(2)}s`,
        }}
      />,
    )
  }

  return (
    <span
      role="img"
      aria-label={t('game.clock', { name: name ?? t('game.clockName'), filled: done, parts })}
      className={cn('gk-clock', next && 'gk-on', className)}
      style={{ width: size, height: size }}
    >
      {squares}
      {showCount && size >= 56 && (
        <b aria-hidden style={{ fontSize: Math.round(size * 0.21) }}>
          <span>{t('game.clockCount', { filled: done, parts })}</span>
        </b>
      )}
    </span>
  )
}
