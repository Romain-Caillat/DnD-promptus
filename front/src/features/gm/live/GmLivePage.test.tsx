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
    // Marc has been idle 25 minutes: his seat says so.
    const seats = screen.getByRole('region', { name: 'Les places' })
    expect(within(seats).getByRole('button', { name: /Borin/ })).toHaveTextContent('25 min')

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

    // A seat opens the table; only « Donner la main » there records a moment.
    await userEvent.click(within(seats).getByRole('button', { name: /Borin/ }))
    expect(within(rail).getByRole('button', { name: /Table/ })).toHaveAttribute('aria-current', 'page')
    expect(screen.getByRole('button', { name: 'Donner la main' })).toBeInTheDocument()

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
