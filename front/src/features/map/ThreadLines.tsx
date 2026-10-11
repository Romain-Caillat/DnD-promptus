import type { Cell } from '@/lib/board'
import type { ThreadMark } from './pins'

/** The thread's game-piece colours: an ivory line on a black edge, like the pins. */
const INK = '#0A0A0A'
const PAPER = '#EDEDED'

type Pt = [number, number]

/**
 * A segment from `a` to `b`, bent to its left so the way back draws
 * beside it rather than over it, cut short by `inset` at both ends to
 * leave the pins clear: the curve and its arrowhead.
 */
function arrow(a: Pt, b: Pt, inset: number): { d: string; head: string } | null {
  const dx = b[0] - a[0]
  const dy = b[1] - a[1]
  const len = Math.hypot(dx, dy)
  if (len <= 2 * inset + 4) return null
  const u: Pt = [dx / len, dy / len]
  const n: Pt = [u[1], -u[0]]
  const bend = Math.min(len * 0.18, 60)
  const p0: Pt = [a[0] + u[0] * inset, a[1] + u[1] * inset]
  const p2: Pt = [b[0] - u[0] * inset, b[1] - u[1] * inset]
  const c: Pt = [(a[0] + b[0]) / 2 + n[0] * bend, (a[1] + b[1]) / 2 + n[1] * bend]
  // The head follows the curve's last tangent, from the control point.
  const tx = p2[0] - c[0]
  const ty = p2[1] - c[1]
  const tl = Math.hypot(tx, ty) || 1
  const t: Pt = [tx / tl, ty / tl]
  const h = 9
  const f = (v: number) => v.toFixed(1)
  const left: Pt = [p2[0] - t[0] * h - t[1] * h * 0.6, p2[1] - t[1] * h + t[0] * h * 0.6]
  const right: Pt = [p2[0] - t[0] * h + t[1] * h * 0.6, p2[1] - t[1] * h - t[0] * h * 0.6]
  return {
    d: `M${f(p0[0])} ${f(p0[1])}Q${f(c[0])} ${f(c[1])} ${f(p2[0])} ${f(p2[1])}`,
    head: [p2, left, right].map((p) => `${f(p[0])},${f(p[1])}`).join(' '),
  }
}

/**
 * The thread's segments (campaign/draw-story-thread), as SVG for the hex
 * map and the grid map alike: each one way, an arrow, so a scene leading
 * to two places draws two branches and a way back draws beside the way
 * out. Solid, never dotted: dots are routes and Claude's proposals — an
 * arrow Claude proposes (`proposed`) is the one drawn dotted. Only drawn —
 * a tap goes through to the map and the pins.
 */
export function ThreadLines({ marks, point, inset }: { marks: readonly ThreadMark[]; point: (cell: Cell) => Pt; inset: number }) {
  return (
    <g pointerEvents="none">
      {marks.map((m) => {
        const path = arrow(point(m.from), point(m.to), inset)
        if (!path) return null
        const width = m.lit ? 4 : 2.5
        return (
          <g
            key={m.key}
            data-thread={m.key}
            data-lit={m.lit || undefined}
            data-proposed={m.proposed || undefined}
            opacity={m.faint ? 0.3 : 1}
          >
            <title>{m.title}</title>
            {!m.proposed && <path d={path.d} fill="none" stroke={INK} strokeWidth={width + 3} strokeLinecap="round" />}
            <polygon points={path.head} fill={INK} stroke={INK} strokeWidth={3} strokeLinejoin="round" />
            <path
              d={path.d}
              fill="none"
              stroke={PAPER}
              strokeWidth={width}
              strokeLinecap="round"
              strokeDasharray={m.proposed ? '2 6' : undefined}
            />
            <polygon points={path.head} fill={PAPER} />
          </g>
        )
      })}
    </g>
  )
}
