/**
 * Pointers on a map, read as gestures (gm/run-on-tablet): one finger
 * taps, paints or drags; a second finger cancels what the first began
 * and the two pan and pinch until both are lifted. Pure, so the touch
 * logic is tested without a browser.
 */

export interface Pt {
  x: number
  y: number
}

export interface Gesture {
  pts: Record<number, Pt>
  mode: 'none' | 'one' | 'two'
  /** Where the single pointer went down. */
  start: Pt | null
  /** The single pointer travelled beyond a tap. */
  moved: boolean
}

export type Effect =
  | { kind: 'none' }
  /** A first finger went down: a paint may start. */
  | { kind: 'press'; at: Pt }
  /** The single finger moves; `by` since the last move. */
  | { kind: 'drag'; at: Pt; by: Pt }
  /** A tap: down and up without travelling. */
  | { kind: 'tap'; at: Pt }
  /** The single finger lifted after a drag. */
  | { kind: 'release'; at: Pt }
  /** What the single finger began is void (a second finger, a cancel). */
  | { kind: 'cancel' }
  /** Two fingers: zoom by `scale` around `center`, then move by `by`. */
  | { kind: 'pinch'; scale: number; center: Pt; by: Pt }

/** How far a finger may slide and still tap, in CSS pixels. */
export const SLOP = 8

export const idle: Gesture = { pts: {}, mode: 'none', start: null, moved: false }

const dist = (a: Pt, b: Pt) => Math.hypot(a.x - b.x, a.y - b.y)
const mid = (a: Pt, b: Pt): Pt => ({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 })

export function down(g: Gesture, id: number, at: Pt): [Gesture, Effect] {
  const pts = { ...g.pts, [id]: at }
  const n = Object.keys(pts).length
  if (n === 1 && g.mode === 'none') return [{ pts, mode: 'one', start: at, moved: false }, { kind: 'press', at }]
  if (n >= 2 && g.mode === 'one') return [{ ...g, pts, mode: 'two' }, { kind: 'cancel' }]
  return [{ ...g, pts }, { kind: 'none' }]
}

export function move(g: Gesture, id: number, at: Pt): [Gesture, Effect] {
  const before = g.pts[id]
  if (!before) return [g, { kind: 'none' }]
  const pts = { ...g.pts, [id]: at }
  if (g.mode === 'one') {
    const moved = g.moved || (g.start !== null && dist(g.start, at) > SLOP)
    return [{ ...g, pts, moved }, moved ? { kind: 'drag', at, by: { x: at.x - before.x, y: at.y - before.y } } : { kind: 'none' }]
  }
  if (g.mode === 'two') {
    const ids = Object.keys(g.pts).map(Number).slice(0, 2)
    if (ids.length < 2 || !ids.includes(id)) return [{ ...g, pts }, { kind: 'none' }]
    const [a0, b0] = ids.map((i) => g.pts[i])
    const [a1, b1] = ids.map((i) => pts[i])
    const d0 = dist(a0, b0)
    const scale = d0 > 0 ? dist(a1, b1) / d0 : 1
    const m0 = mid(a0, b0)
    const m1 = mid(a1, b1)
    return [{ ...g, pts }, { kind: 'pinch', scale, center: m1, by: { x: m1.x - m0.x, y: m1.y - m0.y } }]
  }
  return [{ ...g, pts }, { kind: 'none' }]
}

export function up(g: Gesture, id: number, at: Pt): [Gesture, Effect] {
  if (!g.pts[id]) return [g, { kind: 'none' }]
  const pts = { ...g.pts }
  delete pts[id]
  const rest = { ...g, pts }
  if (Object.keys(pts).length > 0) return [rest, { kind: 'none' }]
  if (g.mode === 'one') return [idle, g.moved ? { kind: 'release', at } : { kind: 'tap', at }]
  return [idle, { kind: 'none' }]
}

export function cancel(g: Gesture, id: number): [Gesture, Effect] {
  if (!g.pts[id]) return [g, { kind: 'none' }]
  const pts = { ...g.pts }
  delete pts[id]
  const effect: Effect = g.mode === 'one' ? { kind: 'cancel' } : { kind: 'none' }
  return [Object.keys(pts).length === 0 ? idle : { ...g, pts }, effect]
}

/** The zoom a map may take. */
const ZOOM = { min: 0.5, max: 3 }

export function clampZoom(z: number): number {
  return Math.min(ZOOM.max, Math.max(ZOOM.min, z))
}
