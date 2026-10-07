import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { FallenPanel } from './FallenPanel'

const FALLEN = { name: 'Borin', lastWords: '', diedAt: '2026-10-06T21:00:00Z' }
const WORDS = 'Dis à Dorn que je suis descendu le chercher.'

function home(over: Record<string, unknown>) {
  return {
    status: 200,
    body: {
      data: {
        me: { id: 'p1', nickname: 'Marc', role: 'player' },
        campaign: { title: 'C', hook: '', gmName: 'Romain' },
        character: null,
        fallen: FALLEN,
        ...over,
      },
    },
  }
}

afterEach(() => vi.unstubAllGlobals())

describe('FallenPanel', () => {
  it('says the last words once, then they stay', async () => {
    const fetchMock = mockApi({
      'PUT /api/play/c1/last-words': () => home({ fallen: { ...FALLEN, lastWords: WORDS } }),
    })
    const onHome = vi.fn()
    render(<FallenPanel campaignId="c1" fallen={FALLEN} onHome={onHome} />)
    expect(screen.getByText('Borin est tombé.')).toBeInTheDocument()
    const say = screen.getByRole('button', { name: /Dire ses derniers mots/ })
    expect(say).toBeDisabled()
    await userEvent.type(screen.getByLabelText('Ce que Borin dit en tombant'), `  ${WORDS} `)
    await userEvent.click(say)
    expect(sentTo(fetchMock, 'PUT /api/play/c1/last-words')).toEqual([{ text: WORDS }])
    expect(onHome.mock.calls[0]![0].fallen.lastWords).toBe(WORDS)
  })

  it('shows the words said, and creates another character', async () => {
    const fetchMock = mockApi({
      'POST /api/play/c1/new-character': () => home({ character: { id: 'c2', status: 'draft' } }),
    })
    const onHome = vi.fn()
    render(<FallenPanel campaignId="c1" fallen={{ ...FALLEN, lastWords: WORDS }} onHome={onHome} />)
    expect(screen.getByText(`« ${WORDS} »`)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Dire ses derniers mots/ })).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Créer un nouveau personnage/ }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/new-character')).toHaveLength(1)
    expect(onHome.mock.calls[0]![0].character.id).toBe('c2')
  })

  it('says why it did not work', async () => {
    mockApi({
      'POST /api/play/c1/new-character': () => ({
        status: 409,
        body: { error: { code: 'CHARACTER_EXISTS', message: 'x' } },
      }),
    })
    render(<FallenPanel campaignId="c1" fallen={FALLEN} onHome={vi.fn()} />)
    await userEvent.click(screen.getByRole('button', { name: /Créer un nouveau personnage/ }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Tu as déjà un personnage à la table.')
  })
})
