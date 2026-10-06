import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { LiveScreen } from '@/lib/evening'
import { LaunchPanel } from './LaunchPanel'

const seat = (nickname: string, here: boolean, role = 'player') => ({
  playerId: nickname,
  nickname,
  role,
  online: here,
  here,
  soundOk: here,
  remote: true,
  characterName: null,
})
const LINES = ['Sous l’autel, Borin a trouvé la page.', 'Les Valombre gardaient la porte nord.']
const lobby = (readingLine: number | null, lines = LINES, people = [seat('Marc', true), seat('Hugo', false)]) =>
  ({
    session: { id: 's4', number: 4, status: 'lobby', readingLine },
    readingLines: lines,
    lobby: people,
  }) as unknown as LiveScreen

function renderPanel(screenData: LiveScreen, screens: { id: string; name: string; pairedAt: string; lastSeenAt: string }[] = []) {
  const handlers = {
    onPair: vi.fn(async () => true),
    onWindow: vi.fn(),
    onForget: vi.fn(),
    onReading: vi.fn(),
  }
  render(<LaunchPanel screen={screenData} screens={screens} {...handlers} />)
  return handlers
}

describe('LaunchPanel', () => {
  it('pairs a TV by its code, typed in any case', async () => {
    const h = renderPanel(lobby(null))
    const pair = screen.getByRole('button', { name: 'Jumeler' })
    expect(pair).toBeDisabled()
    await userEvent.type(screen.getByRole('textbox', { name: 'Code de la TV' }), 'k7qf')
    await userEvent.click(pair)
    expect(h.onPair).toHaveBeenCalledWith('K7QF')
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir la fenêtre TV' }))
    expect(h.onWindow).toHaveBeenCalled()
  })

  it('lists the paired screens and forgets one', async () => {
    const h = renderPanel(lobby(null), [{ id: 'tv1', name: 'TV', pairedAt: '2026-10-10T18:16:00Z', lastSeenAt: '' }])
    const list = screen.getByRole('list', { name: 'Écrans jumelés' })
    expect(within(list).getByText('TV')).toBeInTheDocument()
    await userEvent.click(within(list).getByRole('button', { name: 'Oublier cet écran' }))
    expect(h.onForget).toHaveBeenCalledWith('tv1')
  })

  it('checks what is left before starting', () => {
    renderPanel(lobby(null, []), [])
    const checks = within(screen.getByRole('list', { name: 'Avant de lancer' }))
    expect(checks.getByText(/Pas encore là : Hugo/)).toHaveTextContent('·')
    expect(checks.getByText(/pas encore \(facultatif\)/)).toBeInTheDocument()
    expect(checks.getByText(/Précédemment… » est publié/)).toHaveTextContent('·')
    expect(screen.queryByRole('button', { name: /Lire/ })).toBeNull()
  })

  it('says everyone is here, spectators aside', () => {
    renderPanel(lobby(null, LINES, [seat('Marc', true), seat('TV', false, 'spectator')]))
    expect(screen.getByText(/Le joueur est là/)).toHaveTextContent('✓')
  })

  it('reads « Précédemment… » line by line, then stops', async () => {
    const first = renderPanel(lobby(null))
    await userEvent.click(screen.getByRole('button', { name: 'Lire « Précédemment… » à l’écran' }))
    expect(first.onReading).toHaveBeenCalledWith(1)
  })

  it('goes on from the line shown and ends on the last', async () => {
    const h = renderPanel(lobby(1))
    await userEvent.click(screen.getByRole('button', { name: 'Ligne suivante' }))
    expect(h.onReading).toHaveBeenCalledWith(2)
    await userEvent.click(screen.getByRole('button', { name: 'Terminer la lecture' }))
    expect(h.onReading).toHaveBeenLastCalledWith(null)
  })

  it('offers no next line once all are shown', () => {
    renderPanel(lobby(2))
    expect(screen.queryByRole('button', { name: 'Ligne suivante' })).toBeNull()
    expect(screen.getByRole('button', { name: 'Terminer la lecture' })).toBeInTheDocument()
  })
})
