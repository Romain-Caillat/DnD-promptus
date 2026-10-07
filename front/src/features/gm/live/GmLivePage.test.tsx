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

function renderPage(entry = '/campagnes/c1/soiree') {
  render(
    <MemoryRouter initialEntries={[entry]}>
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

  it('runs on a tablet: a rail of big targets, a check by thumb, a « no » that tells the player why', async () => {
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/session': () => ({ status: 200, body: { data: SCREEN } }),
      'GET /api/campaigns/c1/board': () => ({ status: 200, body: { data: BOARD } }),
      'GET /api/campaigns/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }),
      'POST /api/campaigns/c1/session/requests/r1': () => ({ status: 200, body: { data: {} } }),
    })
    renderPage('/campagnes/c1/soiree?ecran=tablette')

    const rail = await screen.findByRole('navigation', { name: 'Sections' })
    expect(within(rail).getByRole('button', { name: /Scène/ })).toHaveAttribute('aria-current', 'page')
    // Marc has been idle 25 minutes: his seat says so, and the table target carries a dot.
    expect(screen.getByRole('button', { name: 'Donner la main à Borin' })).toHaveTextContent('25 min')

    await userEvent.click(screen.getByRole('button', { name: 'Test Moyen (12)' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/session/requests/r1')).toEqual([
      { kind: 'check', ability: 'DEX', difficulty: 'moyen' },
    ])
    await userEvent.click(screen.getByRole('button', { name: 'Non' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/session/requests/r1')[1]).toEqual({ kind: 'refuse', reason: 'Non, rien ici.' })

    await userEvent.click(within(rail).getByRole('button', { name: /Journal/ }))
    expect(screen.getByText(/Le phare clignote/)).toBeInTheDocument()
    expect(screen.queryByText('La pluie bat les carreaux.')).not.toBeInTheDocument()

    await userEvent.click(within(rail).getByRole('button', { name: /Co-MJ/ }))
    expect(screen.getByRole('complementary', { name: 'Co-MJ' })).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Écran ordinateur' }))
    expect(screen.queryByRole('navigation', { name: 'Sections' })).not.toBeInTheDocument()
    expect(screen.getByText('La pluie bat les carreaux.')).toBeInTheDocument()
  })

  it('answers the co-GM’s adversary turn on a tablet with three big keys', async () => {
    const combatant = (id: string, name: string, side: string) => ({ id, name, side, hit_points: 6, conditions: [] })
    const fight = {
      ...BOARD,
      board: {
        mapId: 'quai',
        map: { id: 'quai', name: 'Le quai', theme: 'port-1718', ambience: {}, grid: { legend: { '.': { terrain: 'pavés' } }, rows: ['...'] } },
        fog: false,
        revealed: [],
        tokens: [],
      },
      encounter: {
        id: 'e1',
        node: 'sc_quai',
        live: true,
        version: 3,
        fight: {
          scene: { combatants: { m1: combatant('m1', 'Marin', 'opposition'), pc: combatant('pc', 'Borin', 'party') } },
          positions: {},
          order: ['m1', 'pc'],
          standing: {},
          round: 1,
          turn: 0,
          end: null,
        },
        maxHitPoints: {},
        proposal: { who: 'm1', version: 3, steps: [], events: [] },
        loot: [],
        events: [],
        reachable: [],
      },
    }
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/session': () => ({ status: 200, body: { data: SCREEN } }),
      'GET /api/campaigns/c1/board': () => ({ status: 200, body: { data: fight } }),
      'GET /api/campaigns/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }),
      'POST /api/campaigns/c1/fight/command': () => ({ status: 200, body: { data: fight } }),
    })
    renderPage('/campagnes/c1/soiree?ecran=tablette')

    await userEvent.click(await screen.findByRole('button', { name: /Carte/ }))
    expect(await screen.findByText('Le co-MJ propose le tour de Marin')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Valider' }))
    await userEvent.click(screen.getByRole('button', { name: 'Il fuit' }))
    await userEvent.click(screen.getByRole('button', { name: 'Il passe son tour' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/fight/command')).toEqual([
      { kind: 'accept' },
      { kind: 'adversary', command: { kind: 'flee' } },
      { kind: 'adversary', command: { kind: 'endTurn' } },
    ])
  })
})
