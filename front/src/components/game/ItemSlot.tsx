import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { RarityPips } from './GameCard'
import type { PixelSprite } from './itemSprites'
import { pixelShadow, type Cell } from './pixels'
import type { Rarity } from './rarity'

/** The 12 × 12 sprite plus its one-cell black outline. */
function drawSprite(sprite: PixelSprite, p: number) {
  const rows = sprite.rows
  const at = (x: number, y: number) => y >= 0 && y < rows.length && x >= 0 && x < 12 && rows[y][x] !== '.'
  const cells: Cell[] = []
  for (let y = -1; y <= rows.length; y++) {
    for (let x = -1; x <= 12; x++) {
      if (at(x, y)) cells.push([x, y, sprite.palette[rows[y][x]] ?? '#ff00ff'])
      else if (at(x + 1, y) || at(x - 1, y) || at(x, y + 1) || at(x, y - 1)) cells.push([x, y, '#050505'])
    }
  }
  return { shadow: pixelShadow(cells, p, 2), height: rows.length + 2 }
}

const SPARKS = [
  { x: -6, y: 10, d: 0 },
  { x: 86, y: 4, d: 0.7 },
  { x: 90, y: 78, d: 1.4 },
]

/**
 * An item in a square inventory slot, like Terraria (board « Composant —
 * objet en case »). The item keeps its colours; its rarity is the slot
 * frame's material (the six of the skill cards) plus 1 to 6 diamonds on
 * the frame. The quantity sits bottom right; the chosen slot blinks.
 */
export function ItemSlot({
  sprite,
  name,
  rarity = 'common',
  size = 72,
  quantity,
  selected = false,
  framed = true,
  pips = true,
  className,
}: {
  /** No sprite: an empty slot. */
  sprite?: PixelSprite
  /** The item's name, read by screen readers. */
  name?: string
  rarity?: Rarity
  size?: number
  quantity?: number
  selected?: boolean
  /** Without its frame: the bare item (in a chest, in the hand). */
  framed?: boolean
  /** The rarity diamonds, from 56 px up. */
  pips?: boolean
  className?: string
}) {
  const { t } = useTranslation()
  const frame = framed ? Math.max(3, Math.round(size * 0.07)) : 0
  // Item pixels: 12 cells plus outline, about two thirds of the slot.
  const p = Math.max(1, Math.floor((framed ? size * 0.72 : size) / 14))
  const drawn = useMemo(() => (sprite ? drawSprite(sprite, p) : null), [sprite, p])
  const pipSize = Math.max(3, Math.round(size * 0.06))
  const showPips = framed && drawn && pips && size >= 56
  const rarityName = t(`game.rarity.${rarity}`)
  const label = !drawn
    ? t('game.item.empty')
    : quantity !== undefined
      ? t('game.item.withQuantity', { name: name ?? '', rarity: rarityName, count: quantity })
      : t('game.item.label', { name: name ?? '', rarity: rarityName })

  return (
    <span
      role="img"
      aria-label={label}
      className={cn('gk-slot', selected && 'gk-selected', !drawn && 'gk-empty', framed && drawn && rarity === 'divine' && 'gk-float', className)}
      data-rarity={rarity}
      style={{ width: size, height: size }}
    >
      {framed && rarity === 'divine' && <span className="gk-rays" aria-hidden />}
      {framed && (
        <>
          <span className="gk-slot-frame" />
          <span className="gk-slot-well" style={{ inset: frame }} />
        </>
      )}
      {drawn && (
        <span
          className="gk-slot-item"
          style={{
            left: Math.round((size - 14 * p) / 2),
            top: Math.round((size - drawn.height * p) / 2) - (framed ? Math.round(size * 0.03) : 0),
            width: 14 * p,
            height: drawn.height * p,
          }}
        >
          <span className="gk-layer" style={{ left: -p, top: -p, width: p, height: p, boxShadow: drawn.shadow }} />
        </span>
      )}
      {quantity !== undefined && drawn && (
        <span aria-hidden className="gk-slot-qty" style={{ fontSize: Math.max(10, Math.round(size * 0.22)) }}>
          {quantity}
        </span>
      )}
      {showPips && (
        <span aria-hidden className="gk-slot-pips" style={{ bottom: Math.round(frame / 2 - pipSize / 2) }}>
          <RarityPips rarity={rarity} size={pipSize} />
        </span>
      )}
      {framed &&
        drawn &&
        (rarity === 'legendary' || rarity === 'divine') &&
        size >= 48 &&
        SPARKS.map((s) => (
          <span
            key={s.x}
            aria-hidden
            className="gk-spark"
            style={{
              left: `${s.x}%`,
              top: `${s.y}%`,
              width: Math.max(6, Math.round(size * 0.14)),
              height: Math.max(6, Math.round(size * 0.14)),
              animationDelay: `${s.d}s`,
            }}
          />
        ))}
    </span>
  )
}
