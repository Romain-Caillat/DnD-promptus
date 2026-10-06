import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { JoinPage } from './JoinPage'

function renderJoin(code = 'abc') {
  render(
    <MemoryRouter initialEntries={[`/rejoindre/${code}`]}>
      <Routes>
        <Route path="/rejoindre/:code" element={<JoinPage />} />
        <Route path="/partie/:campaignId" element={<p>accueil joueur</p>} />
        <Route path="/partie/:campaignId/personnage" element={<p>créateur de personnage</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

const INVITATION = {
  campaignId: 'c1',
  title: 'Les Cendres de Valombre',
  world: 'Valombre',
  playerHook: 'Une ville minière qui enterre ses morts deux fois.',
  gmName: 'Romain',
}
const NOT_JOINED = { status: 401, body: { error: { code: 'NOT_JOINED', message: 'x' } } }

function home(nickname: string, role: 'player' | 'spectator') {
  return {
    me: { id: 'p1', nickname, role },
    campaign: INVITATION,
    character: role === 'player' ? { id: 'k1', status: 'draft', sheet: {}, gmNote: null, updatedAt: '' } : null,
  }
}

describe('JoinPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('lets Marc pick a nickname and create his character, without an account', async () => {
    const api = mockApi({
      'GET /api/join/abc': () => ({ status: 200, body: { data: INVITATION } }),
      'GET /api/play/c1/me': () => NOT_JOINED,
      'POST /api/join/abc': () => ({ status: 201, body: { data: home('Marc', 'player') } }),
    })
    renderJoin()

    expect(await screen.findByRole('heading', { name: 'Les Cendres de Valombre' })).toBeInTheDocument()
    expect(screen.getByText('Romain t\'invite')).toBeInTheDocument()
    expect(screen.getByText(INVITATION.playerHook)).toBeInTheDocument()

    await userEvent.type(screen.getByLabelText('Ton pseudo'), 'Marc')
    await userEvent.click(screen.getByRole('button', { name: /Créer mon personnage/ }))

    expect(await screen.findByText('créateur de personnage')).toBeInTheDocument()
    expect(sentTo(api, 'POST /api/join/abc')).toEqual([{ nickname: 'Marc', role: 'player' }])
  })

  it('joins as a spectator to watch only', async () => {
    const api = mockApi({
      'GET /api/join/abc': () => ({ status: 200, body: { data: INVITATION } }),
      'GET /api/play/c1/me': () => NOT_JOINED,
      'POST /api/join/abc': () => ({ status: 201, body: { data: home('Léa', 'spectator') } }),
    })
    renderJoin()

    await userEvent.type(await screen.findByLabelText('Ton pseudo'), 'Léa')
    await userEvent.click(screen.getByRole('button', { name: /Regarder seulement/ }))

    expect(await screen.findByText('accueil joueur')).toBeInTheDocument()
    expect(sentTo(api, 'POST /api/join/abc')).toEqual([{ nickname: 'Léa', role: 'spectator' }])
  })

  it('asks for a nickname before sending anything, and says when it is taken', async () => {
    const api = mockApi({
      'GET /api/join/abc': () => ({ status: 200, body: { data: INVITATION } }),
      'GET /api/play/c1/me': () => NOT_JOINED,
      'POST /api/join/abc': () => ({
        status: 400,
        body: { error: { code: 'NICKNAME_TAKEN', message: 'x' } },
      }),
    })
    renderJoin()

    await userEvent.click(await screen.findByRole('button', { name: /Créer mon personnage/ }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Choisis un pseudo')
    expect(sentTo(api, 'POST /api/join/abc')).toHaveLength(0)

    await userEvent.type(screen.getByLabelText('Ton pseudo'), 'Marc')
    await userEvent.click(screen.getByRole('button', { name: /Créer mon personnage/ }))
    expect(await screen.findByRole('alert')).toHaveTextContent('déjà pris')
  })

  it('offers to resume when this browser already has its seat', async () => {
    const api = mockApi({
      'GET /api/join/abc': () => ({ status: 200, body: { data: INVITATION } }),
      'GET /api/play/c1/me': () => ({ status: 200, body: { data: home('Marc', 'player') } }),
    })
    renderJoin()

    expect(await screen.findByText(/déjà sa place à cette table : Marc/)).toBeInTheDocument()
    expect(screen.queryByLabelText('Ton pseudo')).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Reprendre/ }))

    expect(await screen.findByText('accueil joueur')).toBeInTheDocument()
    expect(api.mock.calls.some(([, init]) => init?.method === 'POST')).toBe(false)
  })

  it('explains a dead link', async () => {
    mockApi({
      'GET /api/join/old': () => ({
        status: 404,
        body: { error: { code: 'INVITE_NOT_FOUND', message: 'x' } },
      }),
    })
    renderJoin('old')

    expect(await screen.findByRole('alert')).toHaveTextContent('ne marche plus')
  })
})
