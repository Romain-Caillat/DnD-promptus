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
  chronicleTitle: '',
  chronicle: '',
  publishedAt: null,
  gmChanges: '',
  music: null,
  previouslyShown: null,
}
const SCREEN = {
  session: SESSION,
  lastEnded: null,
  launch: null,
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

  it('launches: « Précédemment… » sentence by sentence, then the scene where the table stopped', async () => {
    const ENDED = { ...SESSION, id: 's1', number: 1, status: 'ended', recap: 'Le capitaine ment.', publishedAt: '2026-10-05T10:00:00Z' }
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/session': () => ({
        status: 200,
        body: {
          data: {
            ...SCREEN,
            session: { ...SESSION, previouslyShown: 1 },
            lastEnded: ENDED,
            requests: [],
            launch: {
              number: 1,
              lines: ['Vous avez accosté.', 'Une boussole a changé de main.'],
              shown: 1,
              firstScene: { node: 'sc_taverne', title: 'Le Goéland Ivre' },
            },
          },
        },
      }),
      'GET /api/campaigns/c1/board': () => ({ status: 200, body: { data: BOARD } }),
      'GET /api/campaigns/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }),
      'POST /api/campaigns/c1/session/previously/next': () => ({ status: 200, body: { data: SESSION } }),
      'POST /api/campaigns/c1/session/reveal': () => ({ status: 204 }),
    })
    renderPage()

    const reading = await screen.findByRole('region', { name: 'Précédemment… · séance 1' })
    expect(within(reading).getByText('Le capitaine ment.')).toBeInTheDocument()
    await userEvent.click(within(reading).getByRole('button', { name: 'Phrase suivante' }))
    await userEvent.click(within(reading).getByRole('button', { name: 'Envoyer la première scène : Le Goéland Ivre' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/session/previously/next')).toHaveLength(1)
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/session/reveal')).toEqual([{ kind: 'scene', node: 'sc_taverne' }])
  })

  it('rereads the recaps after the evening, sees what to check, and publishes', async () => {
    const ENDED = {
      ...SESSION,
      id: 's1',
      number: 1,
      status: 'ended',
      recap: 'Le capitaine ment.',
      previously: 'La Couronne approche.',
      chronicleTitle: 'Le quai',
      chronicle: 'Une nuit agitée.',
    }
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/session': () => ({
        status: 200,
        body: { data: { ...SCREEN, session: null, lastEnded: ENDED, requests: [] } },
      }),
      'GET /api/campaigns/c1/board': () => ({ status: 200, body: { data: BOARD } }),
      'GET /api/campaigns/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }),
      'GET /api/campaigns/c1/sessions': () => ({
        status: 200,
        body: { data: [{ ...ENDED, warnings: [{ name: 'La Couronne', kind: 'front' }] }] },
      }),
      'GET /api/campaigns/c1/sessions/s1/feedback': () => ({
        status: 200,
        body: { data: { sessionId: 's1', number: 1, players: [], gaps: [], gmChanges: '' } },
      }),
      'PUT /api/campaigns/c1/sessions/s1/recap': () => ({ status: 200, body: { data: { ...ENDED, publishedAt: 'now' } } }),
    })
    renderPage()

    const panel = await screen.findByRole('region', { name: 'Récapitulatifs de la séance 1' })
    expect(within(panel).getByText('Brouillon : les joueurs ne le voient pas encore.')).toBeInTheDocument()
    expect(await within(panel).findByText('« La Couronne » est une menace : les joueurs ne la voient pas.')).toBeInTheDocument()
    const previously = within(panel).getByLabelText('« Précédemment… » (pour les joueurs)')
    await userEvent.clear(previously)
    await userEvent.type(previously, 'Vous avez accosté.')
    await userEvent.click(within(panel).getByRole('button', { name: 'Publier aux joueurs' }))
    expect(sentTo(fetchMock, 'PUT /api/campaigns/c1/sessions/s1/recap')).toEqual([
      {
        recap: 'Le capitaine ment.',
        previously: 'Vous avez accosté.',
        chronicleTitle: 'Le quai',
        chronicle: 'Une nuit agitée.',
        publish: true,
      },
    ])
  })
})
