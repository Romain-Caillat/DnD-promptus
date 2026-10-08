import { useEffect, useRef, useState, type PointerEvent } from 'react'
import { useTranslation } from 'react-i18next'
import type { Cell, FightEvent, TokenView } from '@/lib/board'
import { usePrefersReducedMotion } from '@/lib/usePrefersReducedMotion'
import { cn } from '@/lib/utils'
import { FACINGS, sheetUrl } from '@/features/sprites/look'
import { cancel, clampZoom, down, idle, move, up, type Effect, type Gesture, type Pt } from './gesture'
import { Motion, type Pose } from './motion'
import { TILE, animated, cellAt, drawScene, readGrid, type Scene } from './render'

/**
 * Canvas pixels per screen pixel: the map is drawn at twice its size and
 * shown at its size, so a character's pixels land on whole canvas pixels
 * (a sprite pixel is three of them) and stay crisp on a phone.
 */
const RESOLUTION = 2

/**
 * A map on a canvas, as `render.ts` draws it, scrolling inside its box.
 * A tap answers the cell under it; with `onPaint`, a finger or the mouse
 * dragged answers every cell crossed on release — the GM's fog brush,
 * the cells lit while painting. A drag without a brush slides the map
 * (the mouse on a computer). With `touch` (gm/run-on-tablet) the map is
 * handled like a photo: a second finger cancels the stroke, two fingers
 * slide it, a pinch zooms. Water, weather and flickering lights move
 * unless the device asks for reduced motion.
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
  touch = false,
  className,
}: {
  scene: Omit<Scene, 'time' | 'sprites' | 'poses'>
  /** The fight's log, for the lunges and hits it has not shown yet. */
  fightEvents?: readonly FightEvent[]
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
  const motion = useRef(new Motion())
  const tile = scene.tile ?? TILE
  const grid = readGrid(scene.map)
  const sprites = useSheets(scene.tokens)
  const hasSprites = Object.keys(sprites).length > 0
  const moving = !reduced && (animated(scene.map, scene.tileset) || hasSprites)
  const drawn = painted ? { ...scene, highlight: painted } : scene
  const sceneRef = useRef(drawn)

  useEffect(() => {
    sceneRef.current = drawn
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
  }, [moving, reduced, scene, sprites, painted])

  function cellOf(at: Pt): Cell {
    const el = canvas.current!
    const rect = el.getBoundingClientRect()
    const scale = rect.width > 0 ? el.width / rect.width : RESOLUTION
    return cellAt((at.x - rect.left) * scale, (at.y - rect.top) * scale, tile * RESOLUTION)
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
        if (canvas.current) {
          canvas.current.style.width = `${grid.width * tile * next}px`
          canvas.current.style.height = `${grid.height * tile * next}px`
        }
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
        width={grid.width * tile * RESOLUTION}
        height={grid.height * tile * RESOLUTION}
        role="img"
        aria-label={t('map.canvas', { name: scene.map.name })}
        className={cn('block [image-rendering:pixelated]', (touch || onPaint) && 'touch-none')}
        style={{ width: grid.width * tile * zoom, height: grid.height * tile * zoom, maxWidth: 'none' }}
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
