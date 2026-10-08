import type { Cell, FightEvent, TokenView } from '@/lib/board'
import type { Facing } from '@/features/sprites/look'

/**
 * How the characters move on a map (characters/walk-in-four-directions):
 * the server says where each token is, which way it faces and its last
 * move; this plays what the screen has not shown yet — the walk along
 * that trail, a lunge toward a target, a blink when hit — and breathes
 * at rest. The frames themselves come from the server's sheet (rest,
 * breath, two steps); nothing here draws a pixel of a character.
 *
 * What a screen first sees is never replayed: a refetch after a fog edit
 * finds the same `moves` and the same events, and nothing moves.
 */

/** Milliseconds per cell walked. */
export const STEP_MS = 180
/** Milliseconds per frame of the walk, and of the breath at rest. */
const WALK_FRAME_MS = 200
const BREATH_FRAME_MS = 500
const ACT_MS = 360
const HIT_MS = 420

/** One token at one instant: where to draw it, which sheet, which frame. */
export interface Pose {
  /** In cells, fractional during a walk or a lunge. */
  x: number
  y: number
  facing: Facing
  /** Index into the sheet: 0 rest, 1 breath, 2 and 3 the steps. */
  frame: 0 | 1 | 2 | 3
  /** Hit: drawn faded on this instant. */
  blink: boolean
}

/** The way from `a` to `b`, as the server turns a token (`Direction::toward`). */
export function toward(a: Cell, b: Cell): Facing | null {
  const dx = b[0] - a[0]
  const dy = b[1] - a[1]
  if (dx === 0 && dy === 0) return null
  if (Math.abs(dx) >= Math.abs(dy)) return dx > 0 ? 'east' : 'west'
  return dy > 0 ? 'south' : 'north'
}

const STEP: Record<Facing, Cell> = { east: [1, 0], west: [-1, 0], north: [0, -1], south: [0, 1] }

export class Motion {
  private shown = new Map<string, number>()
  private walks = new Map<string, { trail: Cell[]; start: number }>()
  private lunges = new Map<string, { facing: Facing; start: number }>()
  private hits = new Map<string, number>()
  private events: number | null = null

  /**
   * What the server sent now: start the walks of moves not shown yet,
   * and the lunges and hits of fight events not shown yet.
   */
  update(tokens: readonly TokenView[], events: readonly FightEvent[] | undefined, now: number): void {
    for (const tk of tokens) {
      const before = this.shown.get(tk.id)
      this.shown.set(tk.id, tk.moves)
      if (before === undefined || tk.moves <= before) continue
      if (tk.trail.length >= 2) this.walks.set(tk.id, { trail: tk.trail, start: now })
      else this.walks.delete(tk.id)
    }
    const list = events ?? []
    // A new fight starts its log over.
    const from = this.events === null ? list.length : this.events > list.length ? 0 : this.events
    this.events = list.length
    for (const e of list.slice(from)) {
      if (e.kind === 'acted') {
        const me = tokens.find((tk) => tk.id === e.who)
        if (me && e.targets.length > 0) this.lunges.set(me.id, { facing: me.facing, start: now })
      } else if (e.kind === 'rules' && e.event.event === 'damaged') {
        this.hits.set(e.event.target, now)
      }
    }
  }

  /** Whether something is moving now: the screen redraws every frame until it stops. */
  busy(now: number): boolean {
    for (const [id, w] of this.walks) {
      if (now - w.start < (w.trail.length - 1) * STEP_MS) return true
      this.walks.delete(id)
    }
    for (const [id, l] of this.lunges) {
      if (now - l.start < ACT_MS) return true
      this.lunges.delete(id)
    }
    for (const [id, h] of this.hits) {
      if (now - h < HIT_MS) return true
      this.hits.delete(id)
    }
    return false
  }

  /** Where and how to draw `tk` at `now`; still and at rest when motion is reduced. */
  pose(tk: TokenView, now: number, reduced: boolean): Pose {
    const rest: Pose = { x: tk.at[0], y: tk.at[1], facing: tk.facing, frame: 0, blink: false }
    if (reduced) return rest
    const walk = this.walks.get(tk.id)
    if (walk) {
      const t = now - walk.start
      const i = Math.floor(t / STEP_MS)
      if (i < walk.trail.length - 1) {
        const [a, b] = [walk.trail[i], walk.trail[i + 1]]
        const f = (t - i * STEP_MS) / STEP_MS
        return {
          x: a[0] + (b[0] - a[0]) * f,
          y: a[1] + (b[1] - a[1]) * f,
          facing: toward(a, b) ?? tk.facing,
          frame: Math.floor(t / WALK_FRAME_MS) % 2 === 0 ? 2 : 3,
          blink: false,
        }
      }
    }
    const pose: Pose = { ...rest, frame: Math.floor(now / BREATH_FRAME_MS) % 2 === 0 ? 0 : 1 }
    const lunge = this.lunges.get(tk.id)
    if (lunge && now - lunge.start < ACT_MS) {
      // Out toward the target and back: a third of a cell at the peak.
      const reach = Math.sin(((now - lunge.start) / ACT_MS) * Math.PI) / 3
      const [dx, dy] = STEP[lunge.facing]
      pose.x += dx * reach
      pose.y += dy * reach
    }
    const hit = this.hits.get(tk.id)
    if (hit !== undefined && now - hit < HIT_MS) {
      const t = now - hit
      pose.blink = Math.floor(t / 70) % 2 === 1
      // Knocked back a little, away from where it faces.
      const [dx, dy] = STEP[tk.facing]
      const back = (1 - t / HIT_MS) / 8
      pose.x -= dx * back
      pose.y -= dy * back
    }
    return pose
  }
}
