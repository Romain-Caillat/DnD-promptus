import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { stubReducedMotion } from '@/test-utils'
import { GameComponentsSection } from './GameComponentsSection'

describe('GameComponentsSection', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('shows every component, and the bag opens from the bottom', async () => {
    stubReducedMotion(true)
    render(<GameComponentsSection />)
    expect(screen.getAllByRole('img', { name: /^Points de vie : / }).length).toBeGreaterThan(0)
    expect(screen.getAllByRole('img', { name: 'Rareté : divine' }).length).toBeGreaterThan(0)
    expect(screen.getAllByRole('img', { name: 'Hache des Valombre, légendaire' }).length).toBeGreaterThan(0)
    expect(screen.getAllByRole('status')).toHaveLength(7)

    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir le sac' }))
    expect(await screen.findByRole('dialog', { name: 'Ton sac' })).toBeInTheDocument()
  })
})
