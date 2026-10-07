import { useEffect, useRef, useState, type PointerEvent } from 'react'
import { useTranslation } from 'react-i18next'
import type { Cell, FightEvent, TokenView } from '@/lib/board'
import { usePrefersReducedMotion } from '@/lib/usePrefersReducedMotion'
import { cn } from '@/lib/utils'
import { FACINGS, sheetUrl } from '@/features/sprites/look'
import { Motion, type Pose } from './motion'
import { TILE, animated, cellAt, drawScene, readGrid, type Scene } from './render'

/**
 * Canvas pixels per screen pixel: the map is drawn at twice its size and
 * shown at its size, so a character's pixels land on whole canvas pixels
 * (a sprite pixel is three of them) and stay crisp on a phone.
 */
const RESOLUTION = 2

/**
 * A map on a canvas, as `render.ts` draws it, scrolling inside its box
 * on a phone. A tap answers the cell under it; dragging (`onPaint`)
 * answers every cell crossed — the GM's fog brush. Water, weather and
 * flickering lights move unless the device asks for reduced motion.
 *
 * Characters are their sprites (characters/walk-in-four-directions):
 * they breathe at rest, walk the last move the server sent cell by cell
 * facing each step, lunge at whom they strike and blink when hit
 * (`fightEvents`) — all still under reduced motion.
 */
export function MapCanvas({
  scene,
  fightEvents,
  onCell,
  onPaint,
  className,
}: {
  scene: Omit<Scene, 'time' | 'sprites' | 'poses'>
  /** The fight's log, for the lunges and hits it has not shown yet. */
  fightEvents?: readonly FightEvent[]
  onCell?: (cell: Cell) => void
  onPaint?: (cells: Cell[]) => void
  className?: string
}) {
  const { t } = useTranslation()
  const canvas = useRef<HTMLCanvasElement>(null)
  const reduced = usePrefersReducedMotion()
  const sceneRef = useRef(scene)
  const painting = useRef<Cell[] | null>(null)
  const motion = useRef(new Motion())
  const tile = scene.tile ?? TILE
  const grid = readGrid(scene.map)
  const sprites = useSheets(scene.tokens)
  const hasSprites = Object.keys(sprites).length > 0
  const moving = !reduced && (animated(scene.map, scene.tileset) || hasSprites)

  useEffect(() => {
    sceneRef.current = scene
  })

  // What the server sent: the walks and strikes not shown yet.
  useEffect(() => {
    motion.current.update(scene.tokens, fightEvents, performance.now())
  }, [scene.tokens, fightEvents])

  useEffect(() => {
    const ctx = canvas.current?.getContext('2d')
    if (!ctx) return
    const draw = (time: number) => {
      const s = sceneRef.current
      const poses: Record<string, Pose> = {}
      for (const tk of s.tokens) poses[tk.id] = motion.current.pose(tk, time, reduced)
      drawScene(ctx, { ...s, tile: (s.tile ?? TILE) * RESOLUTION, time, sprites, poses })
    }
    if (!moving) {
      draw(0)
      return
    }
    // Every frame while something walks or the map moves on its own;
    // at rest, only when the breath changes frame.
    const always = animated(sceneRef.current.map, sceneRef.current.tileset)
    let frame = 0
    let last = -1
    const loop = (now: number) => {
      const beat = Math.floor(now / 250)
      if (always || motion.current.busy(now) || beat !== last) {
        last = beat
        draw(now)
      }
      frame = requestAnimationFrame(loop)
    }
    frame = requestAnimationFrame(loop)
    return () => cancelAnimationFrame(frame)
  }, [moving, reduced, scene, sprites])

  function cellOf(e: PointerEvent<HTMLCanvasElement>): Cell {
    const rect = e.currentTarget.getBoundingClientRect()
    const scale = rect.width > 0 ? e.currentTarget.width / rect.width : RESOLUTION
    return cellAt((e.clientX - rect.left) * scale, (e.clientY - rect.top) * scale, tile * RESOLUTION)
  }

  return (
    <div className={cn('overflow-auto rounded-xl border border-line bg-black', className)}>
      <canvas
        ref={canvas}
        width={grid.width * tile * RESOLUTION}
        height={grid.height * tile * RESOLUTION}
        role="img"
        aria-label={t('map.canvas', { name: scene.map.name })}
        className="block [image-rendering:pixelated]"
        style={{ width: grid.width * tile, height: grid.height * tile, maxWidth: 'none' }}
        onPointerDown={(e) => {
          if (onPaint) painting.current = [cellOf(e)]
        }}
        onPointerMove={(e) => {
          if (!painting.current) return
          const c = cellOf(e)
          const last = painting.current[painting.current.length - 1]
          if (last[0] !== c[0] || last[1] !== c[1]) painting.current.push(c)
        }}
        onPointerUp={(e) => {
          if (painting.current && onPaint) {
            onPaint(painting.current)
            painting.current = null
            return
          }
          onCell?.(cellOf(e))
        }}
      />
    </div>
  )
}

/**
 * The sheets of the tokens' looks, loaded, by URL: the four directions
 * of each look, so a walk can turn without waiting for an image.
 */
function useSheets(tokens: readonly TokenView[]): Record<string, HTMLImageElement> {
  const urls = [...new Set(tokens.flatMap((tk) => (tk.look ? FACINGS.map((f) => sheetUrl(tk.look!, f)) : [])))]
  const signature = urls.join('\n')
  const [sheets, setSheets] = useState<Record<string, HTMLImageElement>>({})

  useEffect(() => {
    let live = true
    for (const url of signature ? signature.split('\n') : []) {
      const img = new Image()
      img.onload = () => {
        if (live) setSheets((s) => (s[url] ? s : { ...s, [url]: img }))
      }
      img.src = url
    }
    return () => {
      live = false
    }
  }, [signature])

  return sheets
}
