import { useMemo, type CSSProperties } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { usePrefersReducedMotion } from '@/lib/usePrefersReducedMotion'
import { outlined, pixelShadow, type Cell } from './pixels'

/** At most ten hearts on screen (MEMORY.md §2). */
const MAX_HEARTS = 10

/**
 * How much each heart holds and how full each one is (0 to 1). The rule
 * is Terraria's: 2 HP per heart, at most ten hearts; beyond 20 HP max the
 * row stays at ten hearts and each one holds max / 10 (board « Composant —
 * cœurs de vie »). A layout short on room may ask for fewer hearts with
 * `count`; each then holds max / count.
 */
export function heartFills(hp: number, max: number, count?: number): { perHeart: number; fills: number[] } {
  const top = Math.max(1, max)
  const n = count && count > 0 ? count : Math.min(MAX_HEARTS, Math.ceil(top / 2))
  const perHeart = top / n
  const fills = Array.from({ length: n }, (_, i) => clamp01((hp - i * perHeart) / perHeart))
  return { perHeart, fills }
}

function clamp01(v: number) {
  return Math.max(0, Math.min(1, v))
}

// The heart, 7 × 6, with its black outline: 9 × 8 cells.
const HEART = outlined(['.XX.XX.', 'XXXXXXX', 'XXXXXXX', '.XXXXX.', '..XXX..', '...X...'])

// Pixel palette of the board: lit top left, a white highlight, a darker
// bottom; an empty heart is the same shape in greys.
function red(x: number, y: number) {
  if (x === 2 && y === 2) return '#ffffff'
  if (y <= 2 && x <= 3) return '#ff8792'
  return y >= 5 ? '#c22a3c' : '#ff4d5e'
}
function grey(x: number, y: number) {
  if (x === 2 && y === 2) return '#4a4a4a'
  return y >= 5 ? '#1e1e1e' : '#2a2a2a'
}

/** A heart filled from the left up to `fill`, by whole columns. */
function heartShadow(fill: number, p: number) {
  const cells: Cell[] = []
  HEART.forEach((row, y) =>
    [...row].forEach((ch, x) => {
      if (ch === '.') return
      const color = ch === 'o' ? '#0a0a0a' : fill > 0.97 || (x - 0.5) / 7 < fill ? red(x, y) : grey(x, y)
      cells.push([x, y, color])
    }),
  )
  return pixelShadow(cells, p)
}

export type HeartsState = 'auto' | 'rest' | 'hit' | 'danger'

/**
 * Hit points as a row of pixel hearts (board « Composant — cœurs de
 * vie »). At rest the beat runs from heart to heart; at 30 % or less the
 * hearts race; on a hit, every heart the blow emptied bursts into pixels,
 * the last one first. The burst plays once: remount (change the `key`)
 * to show a new hit. Under reduced motion the hearts simply show the HP
 * left, with no burst.
 */
export function Hearts({
  hp,
  max,
  damage = 0,
  count,
  px = 3,
  state = 'auto',
  animated = true,
  className,
}: {
  hp: number
  max: number
  /** HP the last blow took: those hearts burst. */
  damage?: number
  /** Fewer hearts than the rule gives, for a tight layout. */
  count?: number
  /** Size of one heart pixel. */
  px?: number
  state?: HeartsState
  animated?: boolean
  className?: string
}) {
  const { t } = useTranslation()
  const reduced = usePrefersReducedMotion()
  const motion = animated && !reduced
  const top = Math.max(1, max)
  const resolved: Exclude<HeartsState, 'auto'> =
    state !== 'auto' ? state : hp / top <= 0.3 ? 'danger' : damage > 0 ? 'hit' : 'rest'

  const hearts = useMemo(() => {
    const before = hp + (resolved === 'hit' ? damage : 0)
    const now = heartFills(hp, top, count)
    const then = heartFills(before, top, count)
    // The last heart the blow reached bursts first.
    const firstToBreak = Math.ceil(before / now.perHeart) - 1
    return now.fills.map((fill, i) => {
      const burst = resolved === 'hit' && then.fills[i] > fill
      return {
        fill,
        base: heartShadow(fill, px),
        over: burst ? heartShadow(then.fills[i], px) : '',
        burst,
        beat: fill > 0 ? (resolved === 'danger' ? 'gk-race' : resolved === 'rest' ? 'gk-beat' : '') : '',
        delay: burst ? (firstToBreak - i) * 0.22 : i * 0.07,
      }
    })
  }, [hp, top, count, damage, resolved, px])

  const layer: CSSProperties = { left: -px, top: -px, width: px, height: px }
  const shard: CSSProperties = {
    left: Math.round(3.6 * px),
    top: Math.round(2.6 * px),
    width: px + 1,
    height: px + 1,
  }

  return (
    <span
      role="img"
      aria-label={t('game.hearts', { hp, max: top })}
      className={cn('gk-hearts', motion && 'gk-on', className)}
      style={{ gap: Math.max(1, px - 1) }}
    >
      {hearts.map((h, i) => (
        <span
          key={i}
          className={cn('gk-heart', h.beat)}
          data-fill={Math.round(h.fill * 100) / 100}
          style={{ width: 9 * px, height: 8 * px, animationDelay: `${h.delay.toFixed(2)}s` }}
        >
          <span className="gk-layer" style={{ ...layer, boxShadow: h.base }} />
          {h.burst && motion && (
            <>
              <span
                className="gk-layer gk-heart-break"
                style={{ ...layer, boxShadow: h.over, animationDelay: `${h.delay.toFixed(2)}s` }}
              />
              {[1, 2, 3, 4].map((s) => (
                <i
                  key={s}
                  className={`gk-shard gk-shard-${s}`}
                  style={{ ...shard, animationDelay: `${h.delay.toFixed(2)}s` }}
                />
              ))}
            </>
          )}
        </span>
      ))}
    </span>
  )
}
