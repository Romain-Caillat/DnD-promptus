import { render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { stubReducedMotion } from '@/test-utils'
import { CellBar, cellStates } from './CellBar'

describe('cellStates', () => {
  it('lights the points left, then the spent cells, then empty ones', () => {
    expect(cellStates(2, 6, 2)).toEqual(['lit', 'lit', 'spent', 'spent', 'empty', 'empty'])
  })

  it('never spends more cells than the bar has left', () => {
    expect(cellStates(4, 6, 5)).toEqual(['lit', 'lit', 'lit', 'lit', 'spent', 'spent'])
    expect(cellStates(9, 4)).toEqual(['lit', 'lit', 'lit', 'lit'])
  })
})

describe('CellBar', () => {
  afterEach(() => vi.unstubAllGlobals())

  const states = (container: HTMLElement) =>
    [...container.querySelectorAll('[data-state]')].map((c) => c.getAttribute('data-state'))

  it('counts what is left, not the cells still falling', () => {
    stubReducedMotion(false)
    const { container } = render(<CellBar stat="move" value={2} max={6} spent={2} label="Cases restantes" />)
    expect(screen.getByRole('img', { name: 'Cases restantes : 2 sur 6' })).toBeInTheDocument()
    expect(states(container)).toEqual(['lit', 'lit', 'spent', 'spent', 'empty', 'empty'])
  })

  it('shows spent cells as simply empty under reduced motion', () => {
    stubReducedMotion(true)
    const { container } = render(<CellBar stat="move" value={2} max={6} spent={2} />)
    expect(states(container)).toEqual(['lit', 'lit', 'empty', 'empty', 'empty', 'empty'])
  })
})
