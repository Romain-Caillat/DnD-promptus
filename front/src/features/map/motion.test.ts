import { describe, expect, it } from 'vitest'
import type { FightEvent, TokenView } from '@/lib/board'
import { Motion, STEP_MS, toward } from './motion'

function token(over: Partial<TokenView> = {}): TokenView {
  return {
    id: 'pc-1',
    name: 'Marc',
    at: [3, 5],
    party: true,
    mine: true,
    ghost: false,
    look: null,
    facing: 'east',
    trail: [],
    moves: 0,
    ...over,
  }
}

describe('a character on the map', () => {
  it('turns the way the server does, the profile on a perfect diagonal', () => {
    expect(toward([3, 3], [5, 3])).toBe('east')
    expect(toward([3, 3], [3, 1])).toBe('north')
    expect(toward([3, 3], [4, 6])).toBe('south')
    expect(toward([3, 3], [2, 2])).toBe('west')
    expect(toward([3, 3], [3, 3])).toBeNull()
  })

  it('does not replay the move it found on the map when the screen opened', () => {
    const m = new Motion()
    const tk = token({ at: [3, 4], facing: 'north', trail: [[3, 5], [3, 4]], moves: 4 })
    m.update([tk], undefined, 0)
    expect(m.busy(10)).toBe(false)
    expect(m.pose(tk, 10, false)).toMatchObject({ x: 3, y: 4, facing: 'north' })
  })

  it('walks a new move cell by cell, facing each step, then rests', () => {
    const m = new Motion()
    m.update([token()], undefined, 0)
    const moved = token({ at: [4, 4], facing: 'north', trail: [[3, 5], [4, 5], [4, 4]], moves: 1 })
    m.update([moved], undefined, 1000)
    expect(m.busy(1000)).toBe(true)
    // Halfway through the first step: between the two cells, facing east, a step frame.
    const first = m.pose(moved, 1000 + STEP_MS / 2, false)
    expect(first.x).toBeCloseTo(3.5)
    expect(first.y).toBe(5)
    expect(first.facing).toBe('east')
    expect([2, 3]).toContain(first.frame)
    // On the second step it turns north.
    const second = m.pose(moved, 1000 + STEP_MS * 1.5, false)
    expect(second.facing).toBe('north')
    expect(second.y).toBeCloseTo(4.5)
    // Arrived: on its cell, the server's facing, breathing.
    const done = 1000 + STEP_MS * 2 + 1
    expect(m.busy(done)).toBe(false)
    expect(m.pose(moved, done, false)).toMatchObject({ x: 4, y: 4, facing: 'north' })
    expect([0, 1]).toContain(m.pose(moved, done, false).frame)
    // The same board fetched again (a fog edit) walks nothing.
    m.update([moved], undefined, 5000)
    expect(m.busy(5000)).toBe(false)
  })

  it('shows only the end of a move whose start was in the fog', () => {
    const m = new Motion()
    m.update([], undefined, 0)
    const npc = token({ id: 'gueule-rouge-1', party: false, at: [4, 5], trail: [[4, 5]], moves: 1 })
    m.update([npc], undefined, 10)
    expect(m.busy(10)).toBe(false)
    const later = { ...npc, trail: [[4, 5]] as TokenView['trail'], moves: 2 }
    m.update([later], undefined, 20)
    expect(m.busy(20)).toBe(false)
  })

  it('stands still under reduced motion', () => {
    const m = new Motion()
    m.update([token()], undefined, 0)
    const moved = token({ at: [4, 5], trail: [[3, 5], [4, 5]], moves: 1 })
    m.update([moved], undefined, 100)
    expect(m.pose(moved, 100 + STEP_MS / 2, true)).toEqual({ x: 4, y: 5, facing: 'east', frame: 0, blink: false })
  })

  it('lunges at the target it strikes, and the target blinks when hit', () => {
    const m = new Motion()
    const me = token({ facing: 'south' })
    const foe = token({ id: 'marin-1', party: false, at: [3, 6], facing: 'north' })
    const before: FightEvent[] = [{ kind: 'round_started', round: 1 }]
    m.update([me, foe], before, 0)
    const after: FightEvent[] = [
      ...before,
      { kind: 'acted', who: 'pc-1', action: 'sabre', targets: ['marin-1'] },
      { kind: 'rules', event: { event: 'damaged', target: 'marin-1', breakdown: { total: 0 }, hp_before: 0, hp_after: 0 } },
    ]
    m.update([me, foe], after, 1000)
    const lunge = m.pose(me, 1180, false)
    expect(lunge.y).toBeGreaterThan(5.2)
    expect(lunge.x).toBe(3)
    const blinks = [1000, 1070, 1140, 1210].map((t) => m.pose(foe, t, false).blink)
    expect(blinks).toContain(true)
    expect(blinks).toContain(false)
    expect(m.busy(2000)).toBe(false)
    // The log fetched again strikes nobody twice.
    m.update([me, foe], after, 3000)
    expect(m.busy(3000)).toBe(false)
  })
})
