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
}
const SCREEN = {
  session: SESSION,
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
  factions: [
    { id: 'fac_sereth', name: 'Les Sereth', diplomacy: 'Le respect.', affinity: 0, start: 0, min: -5, max: 5, rivals: ['fac_vorr'], known: false },
    { id: 'fac_vorr', name: 'Les Vorr', diplomacy: '', affinity: -3, start: -3, min: -5, max: 5, rivals: [], known: true },
  ],
  goals: [{ id: 'but_lentille_echo', title: 'Obtenir la Lentille-écho', heldBy: 'fac_sereth', status: 'known' }],
  companions: [{ id: 'pnj_lumen', name: 'LUMEN', title: 'L’IA de bord', roleplay: '' }],
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
const SCREENS = { screens: [], shows: { scene: true, map: true, party: true, moments: true } }
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
      'GET /api/campaigns/c1/screens': () => ({ status: 200, body: { data: SCREENS } }),
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

  it('moves a faction, ticks a goal and makes the companion speak', async () => {
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/session': () => ({ status: 200, body: { data: SCREEN } }),
      'GET /api/campaigns/c1/board': () => ({ status: 200, body: { data: BOARD } }),
      'GET /api/campaigns/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }),
      'GET /api/campaigns/c1/screens': () => ({ status: 200, body: { data: SCREENS } }),
      'POST /api/campaigns/c1/session/reveal': () => ({ status: 204 }),
      'POST /api/campaigns/c1/session/copilot': () => ({ status: 201, body: { data: {} } }),
    })
    renderPage()

    const factions = await screen.findByRole('region', { name: 'Factions et objectifs' })
    expect(within(factions).getByRole('img', { name: 'Les Vorr : affinité -3' })).toBeInTheDocument()
    expect(within(factions).getByText('Rivaux : Les Vorr')).toBeInTheDocument()
    await userEvent.click(within(factions).getByRole('button', { name: /Monter l'affinité de Les Sereth/ }))
    await userEvent.click(within(factions).getByRole('button', { name: 'Faire connaître' }))
    await userEvent.click(within(factions).getByRole('button', { name: 'Atteint' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/session/reveal')).toEqual([
      { kind: 'affinity', faction: 'fac_sereth', delta: 1 },
      { kind: 'faction', faction: 'fac_sereth' },
      { kind: 'goal', goal: 'but_lentille_echo', status: 'done' },
    ])

    await userEvent.click(screen.getByRole('button', { name: 'Faire parler LUMEN' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/session/copilot')).toEqual([{ kind: 'npc', prompt: '', npc: 'pnj_lumen' }])
  })

  it('pairs the TV of the living room with its code and narrows what it shows', async () => {
    const paired = {
      screens: [{ id: 'tv1', kind: 'tv', pairedAt: '', online: false }],
      shows: { scene: true, map: true, party: true, moments: true },
    }
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/session': () => ({ status: 200, body: { data: SCREEN } }),
      'GET /api/campaigns/c1/board': () => ({ status: 200, body: { data: BOARD } }),
      'GET /api/campaigns/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }),
      'GET /api/campaigns/c1/screens': () => ({ status: 200, body: { data: SCREENS } }),
      'POST /api/campaigns/c1/screens': (b) =>
        (b as { code: string }).code === 'K7QF'
          ? { status: 201, body: { data: paired } }
          : { status: 404, body: { error: { code: 'NO_SUCH_CODE' } } },
      'PUT /api/campaigns/c1/screens/shows': (b) => ({ status: 200, body: { data: { ...paired, shows: b } } }),
    })
    renderPage()

    const panel = await screen.findByRole('region', { name: 'Écran partagé' })
    const code = within(panel).getByRole('textbox', { name: 'Code affiché par la TV' })
    await userEvent.type(code, 'zzzz')
    await userEvent.click(within(panel).getByRole('button', { name: 'Jumeler' }))
    expect(await within(panel).findByRole('alert')).toHaveTextContent('Aucune TV n')
    await userEvent.clear(code)
    await userEvent.type(code, 'k7qf')
    await userEvent.click(within(panel).getByRole('button', { name: 'Jumeler' }))
    expect(await within(panel).findByText('TV')).toBeInTheDocument()
    expect(within(panel).getByText('hors ligne')).toBeInTheDocument()

    await userEvent.click(within(panel).getByRole('checkbox', { name: /Le groupe et ses cœurs/ }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/screens')).toEqual([{ code: 'ZZZZ' }, { code: 'K7QF' }])
    expect(sentTo(fetchMock, 'PUT /api/campaigns/c1/screens/shows')).toEqual([
      { scene: true, map: true, party: false, moments: true },
    ])
  })
})
