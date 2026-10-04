import { fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { RollingDie } from './RollingDie'

function stubReducedMotion(reduce: boolean) {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: reduce && query === '(prefers-reduced-motion: reduce)',
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }))
}

describe('RollingDie', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('shows its result at once under reduced motion, never a random face', () => {
    stubReducedMotion(true)
    // A face that is not the result: if the die rolled, it would show it.
    vi.spyOn(Math, 'random').mockReturnValue(0)

    render(<RollingDie value={17} faces={20} />)

    expect(screen.getByText('17')).toBeInTheDocument()
    expect(screen.getByText('17').parentElement).toHaveAttribute('data-rolling', 'false')
  })

  it('rolls until its tumble ends, then settles on the result', () => {
    stubReducedMotion(false)
    vi.spyOn(Math, 'random').mockReturnValue(0)

    const { container } = render(<RollingDie value={17} faces={20} />)

    expect(screen.queryByText('17')).not.toBeInTheDocument()
    expect(screen.getByText('1')).toBeInTheDocument()

    fireEvent.animationEnd(container.firstElementChild as Element)

    expect(screen.getByText('17')).toBeInTheDocument()
  })
})
