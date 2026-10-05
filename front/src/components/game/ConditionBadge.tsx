import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { pixelShadow, type Cell } from './pixels'

/** 9 × 9 pixel icons of the board « Composant — badge d’état ». */
const ICONS = {
  poison: ['.XXXXXXX.', 'XXXXXXXXX', 'XX..X..XX', 'XX..X..XX', 'XXXXXXXXX', '.XXX.XXX.', '..XXXXX..', '..X.X.X..', '.........'],
  stun: ['....X....', '...XXX...', 'XXXXXXXXX', '.XXXXXXX.', '..XXXXX..', '..XX.XX..', '.XX...XX.', '.X.....X.', '.........'],
  sleep: ['XXXXX....', '...X.....', '..X......', '.X.......', 'XXXXX....', '.....XXX.', '......X..', '.....XXX.', '.........'],
  invisible: ['..XXXXX..', '.X.....X.', 'X..X.X..X', 'X.......X', 'X.......X', 'X.......X', 'X.X.X.X.X', '.X.X.X.X.', '.........'],
  fire: ['....X....', '...XX....', '...XXX...', '..XXXX.X.', '.XXX.XXX.', '.XX...XX.', '.XX...XX.', '..XX.XX..', '...XXX...'],
  restrained: ['..XXXXX..', '.XX...XX.', '.X.....X.', '.X.....X.', 'XXXXXXXXX', 'XXXX.XXXX', 'XXXX.XXXX', 'XXXXXXXXX', 'XXXXXXXXX'],
  fear: ['...XXX...', '...XXX...', '...XXX...', '...XXX...', '....X....', '.........', '...XXX...', '...XXX...', '.........'],
  blessed: ['....X....', '....X....', '..X.X.X..', '...XXX...', 'XXXXXXXXX', '...XXX...', '..X.X.X..', '....X....', '....X....'],
} as const

export type ConditionIcon = keyof typeof ICONS
export const CONDITION_ICONS = Object.keys(ICONS) as ConditionIcon[]

/**
 * A condition as a square badge with the turns it has left (board
 * « États des personnages »): black for a bane, ivory for a boon, the
 * same light/dark split as buttons. The name and the kind come from the
 * campaign's rule system (`ConditionDef`); the icon is the pixel picture.
 * A new condition lands like a stamp (`fresh`).
 */
export function ConditionBadge({
  icon,
  name,
  kind,
  turns,
  size = 28,
  fresh = false,
  className,
}: {
  icon: ConditionIcon
  name: string
  kind: 'boon' | 'bane'
  /** Turns left; none for a condition that lasts until removed. */
  turns?: number
  size?: number
  fresh?: boolean
  className?: string
}) {
  const { t } = useTranslation()
  const p = Math.max(1, Math.floor((size * 0.72) / 9))
  const shadow = useMemo(() => {
    const color = kind === 'boon' ? '#0a0a0a' : '#f2f2f2'
    const cells: Cell[] = []
    ICONS[icon].forEach((row, y) => [...row].forEach((ch, x) => ch === 'X' && cells.push([x, y, color])))
    return pixelShadow(cells, p)
  }, [icon, kind, p])
  const offset = Math.round((size - 9 * p) / 2)

  return (
    <span
      role="img"
      aria-label={turns !== undefined ? t('game.condition.turns', { name, count: turns }) : name}
      className={cn('gk-badge', kind === 'boon' ? 'gk-boon' : 'gk-bane', fresh && 'gk-fresh', className)}
      data-kind={kind}
      style={{ width: size, height: size }}
    >
      <span className="gk-badge-icon" style={{ left: offset, top: offset, width: 9 * p, height: 9 * p }}>
        <span className="gk-layer" style={{ left: -p, top: -p, width: p, height: p, boxShadow: shadow }} />
      </span>
      {turns !== undefined && size >= 20 && (
        <span aria-hidden className="gk-badge-turns" style={{ fontSize: Math.max(9, Math.round(size * 0.4)) }}>
          {turns}
        </span>
      )}
    </span>
  )
}
