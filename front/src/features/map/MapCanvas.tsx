import { useEffect, useRef, useState, type PointerEvent } from 'react'
import { useTranslation } from 'react-i18next'
import type { Cell } from '@/lib/board'
import { usePrefersReducedMotion } from '@/lib/usePrefersReducedMotion'
import { cn } from '@/lib/utils'
import { cancel, clampZoom, down, idle, move, up, type Effect, type Gesture, type Pt } from './gesture'
import { TILE, animated, cellAt, drawScene, readGrid, type Scene } from './render'

/**
 * A map on a canvas, as `render.ts` draws it, scrolling inside its box.
 * A tap answers the cell under it; with `onPaint`, a finger or the mouse
 * dragged answers every cell crossed on release — the GM's fog brush,
 * the cells lit while painting. A drag without a brush slides the map
 * (the mouse on a computer). With `touch` (gm/run-on-tablet) the map is
 * handled like a photo: a second finger cancels the stroke, two fingers
 * slide it, a pinch zooms. Water, weather and flickering lights move
 * unless the device asks for reduced motion.
 */
export function MapCanvas({
  scene,
  onCell,
  onPaint,
  touch = false,
  className,
}: {
  scene: Omit<Scene, 'time'>
  onCell?: (cell: Cell) => void
  onPaint?: (cells: Cell[]) => void
  /** Finger gestures: two fingers slide and pinch, nothing scrolls on its own. */
  touch?: boolean
  className?: string
}) {
  const { t } = useTranslation()
  const box = useRef<HTMLDivElement>(null)
  const canvas = useRef<HTMLCanvasElement>(null)
  const reduced = usePrefersReducedMotion()
  const gesture = useRef<Gesture>(idle)
  const painting = useRef<Cell[] | null>(null)
  const [painted, setPainted] = useState<Cell[] | null>(null)
  const [zoom, setZoom] = useState(1)
  const zoomRef = useRef(1)
  const tile = scene.tile ?? TILE
  const grid = readGrid(scene.map)
  const moving = !reduced && animated(scene.map, scene.tileset)
  const drawn = painted ? { ...scene, highlight: painted } : scene
  const sceneRef = useRef(drawn)

  useEffect(() => {
    sceneRef.current = drawn
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
  }, [moving, scene, painted])

  function cellOf(at: Pt): Cell {
    const el = canvas.current!
    const rect = el.getBoundingClientRect()
    const scale = rect.width > 0 ? el.width / rect.width : 1
    return cellAt((at.x - rect.left) * scale, (at.y - rect.top) * scale, tile)
  }

  function paint(cells: Cell[] | null) {
    painting.current = cells
    setPainted(cells ? [...cells] : null)
  }

  function apply(effect: Effect, mouse: boolean) {
    switch (effect.kind) {
      case 'press':
        if (onPaint) paint([cellOf(effect.at)])
        return
      case 'drag': {
        if (painting.current) {
          const c = cellOf(effect.at)
          const last = painting.current[painting.current.length - 1]
          if (last[0] !== c[0] || last[1] !== c[1]) paint([...painting.current, c])
        } else if (box.current && (mouse || touch)) {
          // A finger without `touch` scrolls the box natively.
          box.current.scrollLeft -= effect.by.x
          box.current.scrollTop -= effect.by.y
        }
        return
      }
      case 'tap':
      case 'release':
        if (painting.current) {
          const cells = painting.current
          paint(null)
          onPaint?.(cells)
        } else if (effect.kind === 'tap') {
          onCell?.(cellOf(effect.at))
        }
        return
      case 'cancel':
        paint(null)
        return
      case 'pinch': {
        if (!touch || !box.current) return
        const el = box.current
        const rect = el.getBoundingClientRect()
        const cx = effect.center.x - rect.left
        const cy = effect.center.y - rect.top
        const before = zoomRef.current
        const next = clampZoom(before * effect.scale)
        const k = next / before
        zoomRef.current = next
        // Widen the canvas now: a scroll set before React renders would be
        // clamped to the old, smaller width (the zoom would grow from the corner).
        if (canvas.current) canvas.current.style.width = `${grid.width * tile * next}px`
        setZoom(next)
        // Keep the point under the fingers where it is, then follow them.
        el.scrollLeft = (el.scrollLeft + cx) * k - cx - effect.by.x
        el.scrollTop = (el.scrollTop + cy) * k - cy - effect.by.y
        return
      }
      case 'none':
        return
    }
  }

  function handle(step: (g: Gesture) => [Gesture, Effect], e: PointerEvent) {
    const [g, effect] = step(gesture.current)
    gesture.current = g
    apply(effect, e.pointerType === 'mouse')
  }

  const id = (e: PointerEvent) => e.pointerId ?? 0
  const at = (e: PointerEvent): Pt => ({ x: e.clientX, y: e.clientY })

  return (
    <div ref={box} className={cn('overflow-auto rounded-xl border border-line bg-black', className)}>
      <canvas
        ref={canvas}
        width={grid.width * tile}
        height={grid.height * tile}
        role="img"
        aria-label={t('map.canvas', { name: scene.map.name })}
        className={cn('block [image-rendering:pixelated]', (touch || onPaint) && 'touch-none')}
        style={{ width: grid.width * tile * zoom, maxWidth: 'none' }}
        onPointerDown={(e) => {
          try {
            e.currentTarget.setPointerCapture(id(e))
          } catch {
            // A synthetic pointer cannot be captured; the gesture still reads.
          }
          handle((g) => down(g, id(e), at(e)), e)
        }}
        onPointerMove={(e) => handle((g) => move(g, id(e), at(e)), e)}
        onPointerUp={(e) =>
          // An up without its down (the press began off the map) still taps.
          handle((g) => (id(e) in g.pts ? up(g, id(e), at(e)) : [g, { kind: 'tap', at: at(e) }]), e)
        }
        onPointerCancel={(e) => handle((g) => cancel(g, id(e)), e)}
      />
    </div>
  )
}
