import { useEffect, useRef, type PointerEvent } from 'react'
import { useTranslation } from 'react-i18next'
import type { Cell } from '@/lib/board'
import { usePrefersReducedMotion } from '@/lib/usePrefersReducedMotion'
import { cn } from '@/lib/utils'
import { TILE, animated, cellAt, drawScene, readGrid, type Scene } from './render'

/**
 * A map on a canvas, as `render.ts` draws it, scrolling inside its box
 * on a phone. A tap answers the cell under it; dragging (`onPaint`)
 * answers every cell crossed — the GM's fog brush. Water, weather and
 * flickering lights move unless the device asks for reduced motion.
 */
export function MapCanvas({
  scene,
  onCell,
  onPaint,
  className,
}: {
  scene: Omit<Scene, 'time'>
  onCell?: (cell: Cell) => void
  onPaint?: (cells: Cell[]) => void
  className?: string
}) {
  const { t } = useTranslation()
  const canvas = useRef<HTMLCanvasElement>(null)
  const reduced = usePrefersReducedMotion()
  const sceneRef = useRef(scene)
  const painting = useRef<Cell[] | null>(null)
  const tile = scene.tile ?? TILE
  const grid = readGrid(scene.map)
  const moving = !reduced && animated(scene.map, scene.tileset)

  useEffect(() => {
    sceneRef.current = scene
  })

  useEffect(() => {
    const ctx = canvas.current?.getContext('2d')
    if (!ctx) return
    if (!moving) {
      drawScene(ctx, { ...sceneRef.current, time: 0 })
      return
    }
    let frame = 0
    const loop = (now: number) => {
      drawScene(ctx, { ...sceneRef.current, time: now })
      frame = requestAnimationFrame(loop)
    }
    frame = requestAnimationFrame(loop)
    return () => cancelAnimationFrame(frame)
  }, [moving, scene])

  function cellOf(e: PointerEvent<HTMLCanvasElement>): Cell {
    const rect = e.currentTarget.getBoundingClientRect()
    const scale = rect.width > 0 ? e.currentTarget.width / rect.width : 1
    return cellAt((e.clientX - rect.left) * scale, (e.clientY - rect.top) * scale, tile)
  }

  return (
    <div className={cn('overflow-auto rounded-xl border border-line bg-black', className)}>
      <canvas
        ref={canvas}
        width={grid.width * tile}
        height={grid.height * tile}
        role="img"
        aria-label={t('map.canvas', { name: scene.map.name })}
        className="block [image-rendering:pixelated]"
        style={{ width: grid.width * tile, maxWidth: 'none' }}
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
