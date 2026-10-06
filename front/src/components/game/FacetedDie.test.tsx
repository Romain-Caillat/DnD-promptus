import { act, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { stubReducedMotion } from '@/test-utils'
import { DICE, FacetedDie, ROLL_MS, facesOf } from './FacetedDie'

describe('FacetedDie', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  it('lands on the face the server rolled, for every die', () => {
    stubReducedMotion(false)
    for (const faces of DICE) {
      const server = faces === 100 ? 7 : faces
      const { unmount } = render(<FacetedDie faces={faces} value={server} />)
      expect(screen.getByRole('img')).toHaveAttribute('data-rolling', 'true')
      act(() => vi.advanceTimersByTime(ROLL_MS))
      expect(screen.getByRole('img')).toHaveAttribute('data-rolling', 'false')
      expect(screen.getByTestId('die-face')).toHaveTextContent(faces === 100 ? '07' : String(server))
      unmount()
    }
  })

  it('shows the value at once under reduced motion', () => {
    stubReducedMotion(true)
    render(<FacetedDie faces={20} value={13} />)
    expect(screen.getByTestId('die-face')).toHaveTextContent('13')
    expect(screen.getByRole('img')).toHaveAccessibleName(/13/)
  })

  it('reads the die from the rules', () => {
    expect(facesOf('1d20')).toBe(20)
    expect(facesOf('2d6')).toBe(6)
    expect(facesOf('1d100')).toBe(100)
    expect(facesOf('1d7')).toBe(20)
  })
})
