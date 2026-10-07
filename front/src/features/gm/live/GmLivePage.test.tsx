import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { GmLivePage } from './GmLivePage'

const SESSION = {
  id: 's2',
  number: 2,
  status: 'live',
  openedAt: '',
  startedAt: '2026-10-06T20:00:00Z',
  endedAt: null,
  recap: '',
  previously: '',
  gmChanges: '',
  music: null,
  readingLine: null,
}
const SCREEN = {
  session: SESSION,
  readingLines: [],
  lastEnded: null,
  lobby: [
    {
      playerId: 'p1',
      nickname: 'Marc',
      role: 'player',
      online: true,
      here: true,
      soundOk: true,
      remote: true,
      characterName: 'Borin',
    },
  ],
  scene: {
    node: {
      id: 'sc_taverne',
      title: 'Le Goéland Ivre',
      act: 'acte_1',
      read_aloud: 'La pluie bat les carreaux.',
      ambience: { music: [{ mood: 'calm', title: 'Taverne', url: 'https://youtu.be/abcdefghijk' }] },
    },
    clues: [{ id: 'cl_gwen', text: 'Gwen a vu une lanterne.', discovery: 'En parlant à Gwen', found: false, revelation: 'rev' }],
    npcs: [],
    exits: [{ to: 'sc_port', label: 'Vers le port', title: 'Le port', visited: false }],
  },
  nodes: [],
  startNode: 'sc_taverne',
  gaps: [],
  fronts: [],
  factions: [],
  goals: [],
  requests: [
    {
      id: 'r1',
      playerId: 'p1',
      card: { kind: 'ability', ability: 'DEX' },
      text: 'Je subtilise la clé.',
      status: 'pending',
      gmReason: null,
      check: null,
      roll: null,
      contested: false,
      createdAt: '',
      nickname: 'Marc',
      characterName: 'Borin',
      cardName: 'Dextérité',
      rulings: [],
    },
  ],
  journal: [{ id: 'j1', kind: 'note', text: 'Le phare clignote.', shared: false, ref: null, createdAt: '' }],
  spotlight: [
    {
      playerId: 'p1',
      nickname: 'Marc',
      characterId: 'k1',
      characterName: 'Borin',
      idleMinutes: 25,
      pendingRequests: 1,
      hooks: [],
      alert: true,
    },
  ],
  drafts: [],
  alertAfterMinutes: 20,
  rules: {
    abilities: [
      ['FOR', 'Force'],
      ['DEX', 'Dextérité'],
    ],
    difficulties: [
      ['facile', 'Facile', 8],
      ['moyen', 'Moyen', 12],
      ['dur', 'Difficile', 15],
    ],
  },
  ai: { configured: true, spending: { budgetMicros: 1_000_000, spentMicros: 0 } },
}
const BOARD = { board: null, maps: [{ id: 'quai', name: 'Le quai', nodes: [] }], encounters: [], encounter: null, conditions: null }

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1/soiree']}>
      <Routes>
        <Route path="/campagnes/:campaignId/soiree" element={<GmLivePage />} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('GmLivePage', () => {
  beforeEach(() => {
    stubReducedMotion(true)
    vi.stubGlobal(
      'WebSocket',
      class {
        close() {}
        addEventListener() {}
        removeEventListener() {}
      },
    )
  })
  afterEach(() => vi.unstubAllGlobals())

  it('runs the scene: a check asked, a clue revealed, a track played, a map shown', async () => {
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/session': () => ({ status: 200, body: { data: SCREEN } }),
      'GET /api/campaigns/c1/board': () => ({ status: 200, body: { data: BOARD } }),
      'GET /api/campaigns/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }),
      'POST /api/campaigns/c1/session/requests/r1': () => ({ status: 200, body: { data: {} } }),
      'POST /api/campaigns/c1/session/reveal': () => ({ status: 204 }),
      'PUT /api/campaigns/c1/session/music': () => ({ status: 200, body: { data: null } }),
      'POST /api/campaigns/c1/board': () => ({ status: 200, body: { data: BOARD } }),
      'POST /api/campaigns/c1/session/spotlight/p1': () => ({ status: 204 }),
    })
    renderPage()

    expect(await screen.findByRole('heading', { name: 'Séance 2 · en jeu' })).toBeInTheDocument()
    expect(screen.getByText('La pluie bat les carreaux.')).toBeInTheDocument()
    expect(screen.getByText(/caché/)).toBeInTheDocument()

    const requests = screen.getByRole('region', { name: 'Demandes (1)' })
    await userEvent.click(within(requests).getByRole('button', { name: 'Demander un test' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/session/requests/r1')).toEqual([
      { kind: 'check', ability: 'DEX', difficulty: 'moyen' },
    ])

    await userEvent.click(screen.getByRole('button', { name: 'Révéler' }))
    await userEvent.click(screen.getByRole('button', { name: /Calme · Taverne/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Montrer : Le quai' }))
    await userEvent.click(screen.getByRole('button', { name: 'Donner la main' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/session/reveal')).toEqual([{ kind: 'clue', clue: 'cl_gwen' }])
    expect(sentTo(fetchMock, 'PUT /api/campaigns/c1/session/music')).toEqual([{ track: 0 }])
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/board')).toEqual([{ map: 'quai' }])
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/session/spotlight/p1')).toHaveLength(1)
  })
})
