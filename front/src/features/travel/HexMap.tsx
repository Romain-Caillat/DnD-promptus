import { useId } from 'react'
import { readGrid } from '@/features/map/render'
import type { Cell, MapData } from '@/lib/board'
import type { PinMark } from '@/features/map/pins'
import type { Tileset } from '@/lib/media'
import { cn } from '@/lib/utils'

/** Radius of a hex, in SVG units. */
const R = 26
const W = Math.sqrt(3) * R
/** The material under the fog (`maps::FOG_TERRAIN`). */
const FOG = 'brouillard'
/** Muted fallbacks when a material is not in the tileset. */
const FALLBACK = '#2a2e28'
const WALL = '#3a3a40'
/** A pin's game-piece colours: ivory on black, inverted where the table is. */
const PIN_INK = '#0A0A0A'
const PIN_PAPER = '#EDEDED'

/** Centre of a hex: pointy top, odd rows shifted half a hex right. */
function hexCenter([x, y]: Cell): [number, number] {
  return [W * (x + 0.5 * (y & 1)) + W / 2 + 4, R * 1.5 * y + R + 4]
}

function corners([cx, cy]: [number, number]): string {
  return Array.from({ length: 6 }, (_, i) => {
    const a = (Math.PI / 180) * (60 * i - 30)
    return `${(cx + R * Math.cos(a)).toFixed(1)},${(cy + R * Math.sin(a)).toFixed(1)}`
  }).join(' ')
}

export interface HexRoute {
  /** From the party's hex, the hexes entered in order. */
  hexes: Cell[]
  /** chosen: solid ivory; mine: the caller's vote, in yellow. */
  look: 'chosen' | 'mine' | 'other'
}

/**
 * A world map in hexes (maps/travel-hex-world, board « Voyager »): each
 * hex in its material's muted colour, the fog as a grid of small squares,
 * the places written on the map, the routes proposed as dotted lines (the
 * one chosen solid), the party as one ivory pin, and the scenes' pins
 * (campaign/pin-scenes-on-maps) on the upper half of their hex. The grid
 * carries the rules: this only draws what the server sent.
 */
export function HexMap({
  map,
  tileset,
  party,
  routes = [],
  destination,
  marks = [],
  pins = [],
  onPin,
  onHex,
  className,
}: {
  map: MapData
  tileset: Tileset | null
  party: Cell | null
  routes?: HexRoute[]
  destination?: Cell | null
  /** Hexes ringed in dots: where the atlas enters another map (maps/browse-atlas). */
  marks?: Cell[]
  /** Pins (campaign/pin-scenes-on-maps), on the upper half of their hex so the hex stays tappable. */
  pins?: readonly PinMark[]
  /** A tap on a pin; without it the pins are only drawn. */
  onPin?: (key: string) => void
  /** A tap on a hex (the GM picks a place). */
  onHex?: (cell: Cell) => void
  className?: string
}) {
  const fogId = useId()
  const grid = readGrid(map)
  const width = Math.round(W * (grid.width + 0.5) + 8)
  const height = Math.round(R * 1.5 * (grid.height - 1) + 2 * R + 8)
  const hexes: { cell: Cell; fill: string; fog: boolean }[] = []
  for (let y = 0; y < grid.height; y++) {
    for (let x = 0; x < grid.width; x++) {
      const kind = grid.kind(x, y)
      if (!kind) continue
      const fog = kind.terrain === FOG
      const fill = fog ? (tileset?.void ?? '#07090c') : (tileset?.materials[kind.terrain]?.base ?? (kind.wall ? WALL : FALLBACK))
      hexes.push({ cell: [x, y], fill, fog })
    }
  }
  const line = (cells: Cell[]) => cells.map((c) => hexCenter(c).map((v) => v.toFixed(1)).join(',')).join(' ')
  const pin = party ? hexCenter(party) : null
  const dest = destination ? hexCenter(destination) : null
  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className={cn('w-full rounded-xl border border-line bg-(--color-map-gap)', className)}
      role="img"
      aria-label={map.name}
    >
      <defs>
        <pattern id={fogId} width="8" height="8" patternUnits="userSpaceOnUse">
          <rect width="8" height="8" style={{ fill: 'var(--color-map-void)' }} />
          <rect x="3" y="3" width="2" height="2" style={{ fill: 'var(--color-map-fog-dot)' }} />
        </pattern>
      </defs>
      {hexes.map(({ cell, fill, fog }) => (
        <polygon
          key={`${cell[0]},${cell[1]}`}
          points={corners(hexCenter(cell))}
          fill={fog ? `url(#${fogId})` : fill}
          style={{ stroke: 'var(--color-map-gap)' }}
          strokeWidth={2}
          data-hex={`${cell[0]},${cell[1]}`}
          className={cn(onHex && 'cursor-pointer hover:opacity-80')}
          onClick={onHex ? () => onHex(cell) : undefined}
        />
      ))}
      {routes.map((r, i) =>
        r.hexes.length > 0 && party ? (
          <polyline
            key={i}
            points={line([party, ...r.hexes])}
            fill="none"
            stroke={r.look === 'mine' ? '#FFD60A' : '#F2F2F2'}
            strokeWidth={r.look === 'other' ? 3 : 4}
            strokeDasharray={r.look === 'chosen' ? undefined : '2 7'}
            strokeLinecap="round"
            opacity={r.look === 'other' ? 0.45 : 1}
            pointerEvents="none"
          />
        ) : null,
      )}
      {(map.labels ?? []).map((l) => {
        const [cx, cy] = hexCenter(l.at)
        return (
          <text
            key={`${l.at[0]},${l.at[1]},${l.text}`}
            x={cx}
            y={cy + R + 4}
            textAnchor="middle"
            className="text-[11px] font-bold"
            // Written over the hexes, which keep their colours in both
            // themes: light letters with a dark halo read on any of them.
            style={{ fill: '#f2f2f2', stroke: '#0a0a0a', strokeWidth: 3, paintOrder: 'stroke' }}
            pointerEvents="none"
          >
            {l.text}
          </text>
        )
      })}
      {marks.map((c) => (
        <polygon
          key={`mark${c[0]},${c[1]}`}
          points={corners(hexCenter(c))}
          fill="none"
          stroke="#EDEDED"
          strokeWidth={3}
          strokeDasharray="3 4"
          pointerEvents="none"
          data-mark={`${c[0]},${c[1]}`}
          transform={`scale(0.82)`}
          style={{ transformBox: 'fill-box', transformOrigin: 'center' }}
        />
      ))}
      {dest && (
        <rect
          x={dest[0] - 7}
          y={dest[1] - 7}
          width={14}
          height={14}
          fill="none"
          stroke="#EDEDED"
          strokeWidth={3}
          pointerEvents="none"
        />
      )}
      {pins.map((p) => {
        const [cx, cy] = hexCenter(p.at)
        const y = cy - R * 0.45
        const w = Math.max(16, 7 * p.label.length + 8)
        const ink = p.current ? PIN_PAPER : PIN_INK
        const paper = p.current ? PIN_INK : PIN_PAPER
        return (
          <g
            key={`pin${p.key}`}
            data-pin={p.key}
            role={onPin ? 'button' : 'img'}
            aria-label={p.title}
            aria-pressed={onPin && p.picked ? true : undefined}
            tabIndex={onPin ? 0 : undefined}
            className={cn(onPin ? 'cursor-pointer' : 'pointer-events-none', p.faint && 'opacity-50')}
            onClick={onPin ? () => onPin(p.key) : undefined}
            onKeyDown={
              onPin
                ? (e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault()
                      onPin(p.key)
                    }
                  }
                : undefined
            }
          >
            {p.shape === 'clue' ? (
              <rect x={cx - 6} y={y - 6} width={12} height={12} fill={paper} stroke={PIN_INK} strokeWidth={2} transform={`rotate(45 ${cx} ${y})`} />
            ) : p.shape === 'fight' ? (
              <>
                <rect x={cx - 9} y={y - 9} width={18} height={18} fill={PIN_INK} stroke={PIN_PAPER} strokeWidth={2} />
                <path d={`M${cx - 5} ${y - 5}L${cx + 5} ${y + 5}M${cx + 5} ${y - 5}L${cx - 5} ${y + 5}`} stroke={PIN_PAPER} strokeWidth={2} />
              </>
            ) : p.shape === 'npc' ? (
              <circle cx={cx} cy={y} r={9} fill={paper} stroke={PIN_INK} strokeWidth={2} />
            ) : (
              <rect x={cx - w / 2} y={y - 9} width={w} height={18} fill={paper} stroke={PIN_INK} strokeWidth={2} />
            )}
            {p.picked && (
              <rect x={cx - w / 2 - 4} y={y - 13} width={w + 8} height={26} fill="none" stroke="#EDEDED" strokeWidth={2} strokeDasharray="3 3" />
            )}
            {p.label && p.shape !== 'clue' && p.shape !== 'fight' && (
              <text x={cx} y={y + 4} textAnchor="middle" className="text-[11px] font-bold" style={{ fill: ink }}>
                {p.label}
              </text>
            )}
          </g>
        )
      })}
      {pin && (
        <g pointerEvents="none" data-party="">
          <rect x={pin[0] - 9} y={pin[1] - 9} width={18} height={18} fill="#0A0A0A" transform={`rotate(45 ${pin[0]} ${pin[1]})`} />
          <rect x={pin[0] - 6} y={pin[1] - 6} width={12} height={12} fill="#EDEDED" transform={`rotate(45 ${pin[0]} ${pin[1]})`} />
        </g>
      )}
    </svg>
  )
}
