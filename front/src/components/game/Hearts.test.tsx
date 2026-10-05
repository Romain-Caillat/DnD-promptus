import { render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { stubReducedMotion } from '@/test-utils'
import { heartFills, Hearts } from './Hearts'

const fills = (hp: number, max: number, count?: number) =>
  heartFills(hp, max, count).fills.map((f) => Math.round(f * 100) / 100)

describe('heartFills', () => {
  it('gives 2 HP per heart up to 20 HP, half hearts included', () => {
    expect(fills(7, 12)).toEqual([1, 1, 1, 0.5, 0, 0])
    expect(heartFills(20, 20).fills).toHaveLength(10)
  })

  it('stays at ten hearts beyond 20 HP, each holding max / 10', () => {
    const { perHeart, fills: f } = heartFills(9, 31)
    expect(f).toHaveLength(10)
    expect(perHeart).toBeCloseTo(3.1)
    expect(fills(9, 31).slice(0, 4)).toEqual([1, 1, 0.9, 0])
    expect(fills(60, 120)).toEqual([1, 1, 1, 1, 1, 0, 0, 0, 0, 0])
  })

  it('lets a tight layout ask for fewer hearts', () => {
    expect(fills(9, 31, 8)).toHaveLength(8)
    expect(heartFills(31, 31, 8).fills.every((f) => f === 1)).toBe(true)
  })
})

describe('Hearts', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('says the HP in French and draws ten hearts beyond 20 HP', () => {
    stubReducedMotion(false)
    const { container } = render(<Hearts hp={18} max={22} />)
    expect(screen.getByRole('img', { name: 'Points de vie : 18 sur 22' })).toBeInTheDocument()
    expect(container.querySelectorAll('.gk-heart')).toHaveLength(10)
  })

  it('bursts only the hearts the blow emptied', () => {
    stubReducedMotion(false)
    // 22 HP max: 2.2 HP per heart. 18 → 12 empties hearts 6, 7 and 8 (partly).
    const { container } = render(<Hearts hp={12} max={22} damage={6} />)
    const hearts = [...container.querySelectorAll('.gk-heart')]
    const bursting = hearts.map((h, i) => (h.querySelector('.gk-heart-break') ? i : -1)).filter((i) => i >= 0)
    expect(bursting).toEqual([5, 6, 7, 8])
  })

  it('shows the hearts left without a burst under reduced motion', () => {
    stubReducedMotion(true)
    const { container } = render(<Hearts hp={12} max={22} damage={6} />)
    expect(container.querySelector('.gk-heart-break')).toBeNull()
    expect(container.querySelector('.gk-shard')).toBeNull()
  })
})
