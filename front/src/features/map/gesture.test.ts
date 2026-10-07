import { describe, expect, it } from 'vitest'
import { SLOP, cancel, down, idle, move, up, type Effect, type Gesture } from './gesture'

/** Run a list of pointer steps and keep every effect. */
function run(steps: ((g: Gesture) => [Gesture, Effect])[]): Effect[] {
  let g = idle
  const out: Effect[] = []
  for (const step of steps) {
    const [next, effect] = step(g)
    g = next
    out.push(effect)
  }
  return out
}

describe('gesture', () => {
  it('reads a finger that barely slides as a tap, not a stroke', () => {
    const effects = run([(g) => down(g, 1, { x: 10, y: 10 }), (g) => move(g, 1, { x: 10 + SLOP - 1, y: 10 }), (g) => up(g, 1, { x: 15, y: 10 })])
    expect(effects.map((e) => e.kind)).toEqual(['press', 'none', 'tap'])
  })

  it('drags past the slop and releases without tapping', () => {
    const effects = run([(g) => down(g, 1, { x: 0, y: 0 }), (g) => move(g, 1, { x: 30, y: 0 }), (g) => move(g, 1, { x: 40, y: 5 }), (g) => up(g, 1, { x: 40, y: 5 })])
    expect(effects[1]).toEqual({ kind: 'drag', at: { x: 30, y: 0 }, by: { x: 30, y: 0 } })
    expect(effects[2]).toEqual({ kind: 'drag', at: { x: 40, y: 5 }, by: { x: 10, y: 5 } })
    expect(effects[3].kind).toBe('release')
  })

  it('cancels the first finger’s stroke when a second lands, then pinches until both lift', () => {
    const effects = run([
      (g) => down(g, 1, { x: 100, y: 100 }),
      (g) => move(g, 1, { x: 130, y: 100 }),
      (g) => down(g, 2, { x: 200, y: 100 }),
      // The fingers spread to twice their distance, around a centre that moves 35 px right.
      (g) => move(g, 2, { x: 270, y: 100 }),
      (g) => up(g, 1, { x: 130, y: 100 }),
      // The finger left behind neither paints nor taps.
      (g) => move(g, 2, { x: 300, y: 100 }),
      (g) => up(g, 2, { x: 300, y: 100 }),
    ])
    expect(effects[2]).toEqual({ kind: 'cancel' })
    expect(effects[3]).toEqual({ kind: 'pinch', scale: 2, center: { x: 200, y: 100 }, by: { x: 35, y: 0 } })
    expect(effects.slice(4).map((e) => e.kind)).toEqual(['none', 'none', 'none'])
  })

  it('voids the stroke when the system cancels the pointer (iOS takes the touch)', () => {
    const effects = run([(g) => down(g, 1, { x: 0, y: 0 }), (g) => move(g, 1, { x: 50, y: 0 }), (g) => cancel(g, 1), (g) => down(g, 1, { x: 5, y: 5 })])
    expect(effects[2]).toEqual({ kind: 'cancel' })
    // The next touch starts afresh.
    expect(effects[3]).toEqual({ kind: 'press', at: { x: 5, y: 5 } })
  })
})
