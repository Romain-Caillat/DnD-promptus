import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { LiveScreen } from '@/lib/evening'
import { FactionsPanel } from './FactionsPanel'

const SCREEN: Pick<LiveScreen, 'factions' | 'goals'> = {
  factions: [
    { id: 'fac_sereth', name: 'Les Sereth', affinity: 0, min: -5, max: 5, met: true, rivals: ['Les Vorr'] },
    { id: 'fac_vorr', name: 'Les Vorr', affinity: -5, min: -5, max: 5, met: false, rivals: [] },
  ],
  goals: [
    { id: 'but_lentille_echo', title: 'Obtenir la Lentille-écho', heldBy: 'Les Sereth', done: false },
    { id: 'but_moelle_vive', title: 'Obtenir la Moelle vive', heldBy: null, done: true },
  ],
}

function renderPanel(live = true) {
  const onReveal = vi.fn()
  render(<FactionsPanel screen={SCREEN} live={live} onReveal={onReveal} />)
  return onReveal
}

describe('FactionsPanel', () => {
  it('moves a gauge, meets a faction and ticks a goal', async () => {
    const onReveal = renderPanel()
    expect(screen.getByRole('meter', { name: 'Affinité de Les Sereth' })).toHaveAttribute('aria-valuenow', '0')
    expect(screen.getByText('Rivaux : Les Vorr (un gain leur coûte autant)')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Les Sereth : +1' }))
    expect(onReveal).toHaveBeenLastCalledWith({ kind: 'faction', faction: 'fac_sereth', delta: 1 })
    await userEvent.click(screen.getByRole('button', { name: 'Les Sereth : −1' }))
    expect(onReveal).toHaveBeenLastCalledWith({ kind: 'faction', faction: 'fac_sereth', delta: -1 })

    // The Vorr, at their floor, cannot lose more; not met yet.
    expect(screen.getByRole('button', { name: 'Les Vorr : −1' })).toBeDisabled()
    const vorr = screen.getByText('Les Vorr', { selector: 'span.font-bold' }).closest('li')!
    await userEvent.click(within(vorr).getByRole('button', { name: 'Rencontrer' }))
    expect(onReveal).toHaveBeenLastCalledWith({ kind: 'faction', faction: 'fac_vorr', delta: 0 })

    await userEvent.click(screen.getByRole('checkbox', { name: /Lentille-écho/ }))
    expect(onReveal).toHaveBeenLastCalledWith({ kind: 'goal', goal: 'but_lentille_echo', done: true })
    await userEvent.click(screen.getByRole('checkbox', { name: /Moelle vive/ }))
    expect(onReveal).toHaveBeenLastCalledWith({ kind: 'goal', goal: 'but_moelle_vive', done: false })
  })

  it('waits for the live session', () => {
    renderPanel(false)
    expect(screen.getByText('Lancez la partie pour bouger une jauge ou cocher un objectif.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Les Sereth : +1' })).toBeDisabled()
    expect(screen.getByRole('checkbox', { name: /Lentille-écho/ })).toBeDisabled()
  })
})
