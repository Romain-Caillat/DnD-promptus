import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { BetweenView } from '@/lib/between'
import { mockApi, sentTo } from '@/test-utils'
import { BetweenPanel } from './BetweenPanel'

const A = '2099-10-10T18:30:00Z'
const B = '2099-10-17T18:30:00Z'

function between(over: Partial<BetweenView> = {}) {
  const data: BetweenView = {
    lastSession: {
      number: 2,
      endedAt: '2026-10-03T22:00:00Z',
      durationMinutes: 180,
      title: 'Le quai brûle',
      published: true,
      xpGained: 300,
      gains: [{ kind: 'item', label: 'Corde', delta: 1 }],
    },
    previously: {
      number: 2,
      text: 'Le quai a brûlé.',
      clues: ['Un registre maculé'],
      revelations: [],
      openThreads: ['Promesse : ramener le fils du passeur'],
    },
    chronicle: [
      { number: 1, title: 'Le Goéland Ivre', text: 'Tout commence à la taverne.', date: '2026-09-26T18:30:00Z' },
      { number: 2, title: 'Le quai brûle', text: 'Le feu prend.', date: '2026-10-03T18:30:00Z' },
    ],
    next: { number: 3, options: [A, B], chosenAt: null, lobbyAt: null, mine: null, answered: 1 },
    ...over,
  }
  return { status: 200, body: { data } }
}

afterEach(() => vi.unstubAllGlobals())

describe('BetweenPanel', () => {
  it('shows my gains, the published « Précédemment… » and the chronicle', async () => {
    mockApi({ 'GET /api/play/c1/between': () => between() })
    render(<BetweenPanel campaignId="c1" refreshKey={0} seated />)
    expect(await screen.findByText('+300 XP')).toBeInTheDocument()
    expect(screen.getByText('Corde +1')).toBeInTheDocument()
    expect(screen.getByText('Le quai a brûlé.')).toBeInTheDocument()
    expect(screen.getByText('Un registre maculé')).toBeInTheDocument()
    expect(screen.getByText('Promesse : ramener le fils du passeur')).toBeInTheDocument()
    const entries = within(screen.getByRole('list', { name: 'La chronique' })).getAllByRole('listitem')
    expect(entries.map((e) => e.querySelector('b')?.textContent)).toEqual(['Le Goéland Ivre', 'Le quai brûle'])
    expect(screen.getByText('Tout commence à la taverne.')).toBeInTheDocument()
    expect(screen.queryByText('Le MJ relit le résumé de la dernière session.')).not.toBeInTheDocument()
  })

  it('says the recap is still with the GM while it is unpublished', async () => {
    mockApi({
      'GET /api/play/c1/between': () =>
        between({
          lastSession: { ...between().body.data.lastSession!, published: false, title: null },
          previously: null,
          chronicle: [],
        }),
    })
    render(<BetweenPanel campaignId="c1" refreshKey={0} seated />)
    expect(await screen.findByText('Le MJ relit le résumé de la dernière session.')).toBeInTheDocument()
    expect(screen.queryByText('Précédemment…')).not.toBeInTheDocument()
  })

  it('ticks the dates that suit me, all at once', async () => {
    const fetchMock = mockApi({
      'GET /api/play/c1/between': () => between(),
      'PUT /api/play/c1/availability': (sent) =>
        between({ next: { number: 3, options: [A, B], chosenAt: null, lobbyAt: null, mine: (sent as { available: string[] }).available, answered: 2 } }),
    })
    render(<BetweenPanel campaignId="c1" refreshKey={0} seated />)
    expect(await screen.findByText("Tu n'as pas encore répondu.")).toBeInTheDocument()
    const [first, second] = screen.getAllByRole('button')
    await userEvent.click(second!)
    expect(await screen.findByText('2 joueurs ont répondu.')).toBeInTheDocument()
    expect(second).toHaveAttribute('aria-pressed', 'true')
    await userEvent.click(first!)
    expect(sentTo(fetchMock, 'PUT /api/play/c1/availability')).toEqual([{ available: [B] }, { available: [B, A] }])
  })

  it('a spectator sees the dates but cannot answer, nor sees gains', async () => {
    mockApi({ 'GET /api/play/c1/between': () => between() })
    render(<BetweenPanel campaignId="c1" refreshKey={0} seated={false} />)
    expect(await screen.findByText('Dates proposées par le MJ')).toBeInTheDocument()
    for (const b of screen.getAllByRole('button')) expect(b).toBeDisabled()
    expect(screen.queryByText('+300 XP')).not.toBeInTheDocument()
  })

  it('once the date is fixed, gives the lobby time and the calendar reminder', async () => {
    mockApi({
      'GET /api/play/c1/between': () =>
        between({ next: { number: 3, options: [A], chosenAt: A, lobbyAt: '2099-10-10T18:15:00Z', mine: [A], answered: 3 } }),
    })
    render(<BetweenPanel campaignId="c1" refreshKey={0} seated />)
    const link = await screen.findByRole('link', { name: 'Ajouter à mon calendrier' })
    expect(link).toHaveAttribute('href', '/api/play/c1/next-session.ics')
    expect(screen.getByText(/^Salle d'attente ouverte dès/)).toBeInTheDocument()
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })
})
