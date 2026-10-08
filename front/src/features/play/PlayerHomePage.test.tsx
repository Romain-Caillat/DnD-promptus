import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { DESKTOP_QUERY } from '@/lib/useMediaQuery'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { PlayerHomePage } from './PlayerHomePage'

function renderHome() {
  render(
    <MemoryRouter initialEntries={['/partie/c1']}>
      <Routes>
        <Route path="/partie/:campaignId" element={<PlayerHomePage />} />
      </Routes>
    </MemoryRouter>,
  )
}

const CAMPAIGN = {
  campaignId: 'c1',
  title: 'Les Cendres de Valombre',
  world: 'Valombre',
  playerHook: 'Une ville minière.',
  gmName: 'Romain',
}

const CARD = {
  kind: 'Attaque',
  attackBonus: 3,
  damage: '3',
  heal: null,
  cooldown: 0,
  range: 1,
  description: '',
}
const bag = (equipped: boolean) => [
  { key: 'sabre', itemId: 'sabre', name: "Sabre d'abordage", description: 'Lame courbe.', qty: 1, consumable: false, equipped },
  { key: 'rhum', itemId: 'rhum', name: 'Fiole de rhum fortifiant', description: 'Restaure 3 PV.', qty: 2, consumable: true, equipped: false },
]
const inPlay = (equipped: boolean) => ({
  id: 'k1',
  status: 'validated',
  sheet: { name: 'Lyra', classId: 'bretteur' },
  gmNote: null,
  updatedAt: '',
  peopleName: null,
  className: 'Bretteur',
  stats: null,
  play: {
    level: 1,
    totalXp: 3,
    xpBar: 3,
    xpBarMax: 5,
    upgradePoints: 0,
    nextLevelXp: 5,
    hitPoints: 7,
    maxHitPoints: 10,
    armorClass: 12,
    initiative: 2,
    abilities: [{ id: 'FOR', name: 'Force', score: 13, modifier: 1 }],
    cards: [
      { ...CARD, id: 'estocade', name: 'Estocade', level: 1 },
      { ...CARD, id: 'riposte', name: 'Riposte en quarte', level: 3 },
    ],
    resources: [{ id: 'or', name: "Pièces d'or", abbr: 'PO', amount: 15 }],
    inventory: bag(equipped),
  },
})
/** The evening before any session: what the Game tab reads first. */
const EVENING = {
  'GET /api/play/c1/evening': () => ({
    status: 200,
    body: {
      data: {
        session: null,
        previously: 'Les corsaires ont accosté.',
        launch: null,
        music: null,
        lobby: [],
        campaign: { ...CAMPAIGN, scene: null, clues: [], npcs: [], factions: [], goals: [] },
        journal: [],
        requests: [],
        cards: [],
        feedback: null,
      },
    },
  }),
  'GET /api/play/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }),
  'GET /api/play/c1/chronicle': () => ({ status: 200, body: { data: [] } }),
}

const home = (character: unknown, role = 'player') => ({
  status: 200,
  body: { data: { me: { id: 'p1', nickname: 'Camille', role }, campaign: CAMPAIGN, character } },
})

describe('PlayerHomePage', () => {
  beforeEach(() => {
    stubReducedMotion(true)
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('shows the campaign and where the character stands, with the GM note', async () => {
    mockApi({
      'GET /api/play/c1/me': () => ({
        status: 200,
        body: {
          data: {
            me: { id: 'p1', nickname: 'Marc', role: 'player' },
            campaign: CAMPAIGN,
            character: {
              id: 'k1',
              status: 'returned',
              sheet: { name: 'Borin' },
              gmNote: 'La Force s’arrête à 17.',
              updatedAt: '',
            },
          },
        },
      }),
    })
    renderHome()

    expect(await screen.findByRole('heading', { name: 'Les Cendres de Valombre' })).toBeInTheDocument()
    expect(screen.getByRole('status')).toHaveTextContent('Le MJ te demande un changement')
    expect(screen.getByText('Borin')).toBeInTheDocument()
    expect(screen.getByText('La Force s’arrête à 17.')).toBeInTheDocument()
  })

  it('tells a browser without a seat to open the invitation link', async () => {
    mockApi({
      'GET /api/play/c1/me': () => ({ status: 401, body: { error: { code: 'NOT_JOINED', message: 'x' } } }),
    })
    renderHome()

    expect(await screen.findByRole('alert')).toHaveTextContent("lien d'invitation")
  })

  it('shows the character in play, the GM\'s numbers, and lets her carry an item', async () => {
    const fetchMock = mockApi({
      ...EVENING,
      'GET /api/play/c1/me': () => home(inPlay(false)),
      'POST /api/play/c1/character/equip': () => ({ status: 200, body: { data: inPlay(true) } }),
    })
    renderHome()

    // In play, the game comes first: « Précédemment… » until the GM opens the evening.
    expect(await screen.findByText('Les corsaires ont accosté.')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Personnage' }))
    expect(await screen.findByText('7 / 10 PV')).toBeInTheDocument()
    expect(screen.getByRole('img', { name: 'Points de vie : 7 sur 10' })).toBeInTheDocument()
    expect(screen.getByText('Encore 2 XP avant un point d\'amélioration · niveau 2 à 5 XP (tu en as 3)', { exact: false })).toBeInTheDocument()
    expect(screen.getByText('au niveau 3')).toBeInTheDocument()
    expect(screen.getByText('15')).toBeInTheDocument()
    expect(screen.getByText('Tu n\'as rien en main.')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: "L'équiper · Sabre d'abordage" }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/character/equip')).toEqual([{ entry: 'sabre', equipped: true }])
    const worn = screen.getByRole('heading', { name: 'Sur toi' }).parentElement!
    expect(await within(worn).findByText("Sabre d'abordage")).toBeInTheDocument()
  })

  it('reads in the Journal what the group knows', async () => {
    mockApi({
      ...EVENING,
      'GET /api/play/c1/me': () => home(inPlay(false)),
      'GET /api/play/c1/view': () => ({
        status: 200,
        body: {
          data: {
            ...CAMPAIGN,
            party: [],
            scene: null,
            clues: ['Gwen a vu une lanterne sur la falaise.'],
            npcs: [{ id: 'pnj_loic', name: 'Loïc', title: 'Le mousse', appearance: '' }],
            factions: [
              { id: 'fac_sereth', name: 'Les Sereth', description: 'Des tortues anciennes.', affinity: 2, min: -5, max: 5, rivals: ['Les Vorr'] },
            ],
            goals: [{ title: 'Obtenir la Lentille-écho', description: '', done: true, heldBy: 'Les Sereth' }],
          },
        },
      }),
      'GET /api/play/c1/between': () => ({
        status: 200,
        body: {
          data: {
            open: null,
            last: null,
            levelUp: null,
            chronicle: [
              { number: 1, endedAt: '2026-10-03T18:30:00Z', title: 'Le quai de Port-Louis', text: 'La boussole a changé de main.' },
            ],
            openThreads: [],
          },
        },
      }),
    })
    renderHome()

    await userEvent.click(await screen.findByRole('button', { name: 'Journal' }))
    expect(await screen.findByText('Gwen a vu une lanterne sur la falaise.')).toBeInTheDocument()
    expect(screen.getByText('Loïc')).toBeInTheDocument()
    expect(screen.getByText('Une ville minière.')).toBeInTheDocument()
    // The chronicle: one entry per session the GM published.
    const chronicle = screen.getByRole('region', { name: 'La chronique' })
    expect(within(chronicle).getByText('Le quai de Port-Louis')).toBeInTheDocument()
    expect(within(chronicle).getByText('La boussole a changé de main.')).toBeInTheDocument()
    // The factions it knows, with the party's standing; a goal reached.
    expect(screen.getByRole('img', { name: 'Les Sereth : affinité +2' })).toBeInTheDocument()
    expect(screen.getByText('Rivaux : Les Vorr')).toBeInTheDocument()
    expect(screen.getByText('✓ Obtenir la Lentille-écho')).toBeInTheDocument()
  })

  it('gives a spectator the game, the map and the journal, no sheet', async () => {
    mockApi({
      ...EVENING,
      'GET /api/play/c1/me': () => home(null, 'spectator'),
      'GET /api/play/c1/view': () => ({
        status: 200,
        body: { data: { ...CAMPAIGN, party: [], scene: null, clues: [], npcs: [], factions: [], goals: [] } },
      }),
    })
    renderHome()

    expect(await screen.findByText('Les corsaires ont accosté.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Carte' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Journal' }))
    expect(await screen.findByText("Le groupe n'a encore rien découvert.")).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Personnage' })).not.toBeInTheDocument()
  })
  it('goes through a new level: the hit points, die or average, then the new card', async () => {
    const up = (due: number[], gains: unknown[]) => ({
      ...inPlay(false),
      play: {
        ...inPlay(false).play,
        level: 4,
        maxHitPoints: due.length ? 10 : 19,
        levelUp: {
          from: 3,
          to: 4,
          hitPoints: { dice: '1d10', average: 6, ability: 'Constitution', modifier: 3, due },
          gains,
          cards: [{ ...CARD, id: 'botte', name: 'Botte secrète', level: 4 }],
        },
      },
    })
    const fetchMock = mockApi({
      ...EVENING,
      'GET /api/play/c1/me': () => home(up([4], [])),
      'POST /api/play/c1/character/level-up': (body) =>
        body && (body as { kind: string }).kind === 'seen'
          ? { status: 200, body: { data: inPlay(false) } }
          : {
              status: 200,
              body: {
                data: up([], [{ level: 4, method: 'average', faces: [], base: 6, modifier: 3, amount: 9 }]),
              },
            },
    })
    renderHome()

    await userEvent.click(await screen.findByRole('button', { name: 'Personnage' }))
    expect(await screen.findByText('Lyra passe au niveau 4.')).toBeInTheDocument()
    expect(screen.getByText('Botte secrète')).toBeInTheDocument()
    // The level's hit points first: « C'est noté » waits for them.
    expect(screen.getByRole('button', { name: /C'est noté/ })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: /La moyenne/ }))
    expect(await screen.findByText('Niveau 4 : +9 PV')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /C'est noté/ }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/character/level-up')).toEqual([
      { kind: 'hitPoints', level: 4, method: 'average' },
      { kind: 'seen' },
    ])
    expect(screen.queryByText('Botte secrète')).not.toBeInTheDocument()
  })

  it('after a death: the last words once, then what comes next', async () => {
    const fallen = (lastWords: string | null, next: string | null) => ({
      characterId: 'k1',
      name: 'Borin',
      className: 'Bretteur',
      look: null,
      level: 3,
      lastWords,
      next,
      diedAt: '2026-10-07T20:00:00Z',
    })
    const dead = (lastWords: string | null, next: string | null, character: unknown = null) => ({
      status: 200,
      body: {
        data: { me: { id: 'p1', nickname: 'Marc', role: 'player' }, campaign: CAMPAIGN, character, fallen: fallen(lastWords, next) },
      },
    })
    const words = 'Dis à Dorn que je suis descendu le chercher.'
    const fetchMock = mockApi({
      ...EVENING,
      'GET /api/play/c1/me': () => dead(null, null),
      'POST /api/play/c1/fate/words': () => dead(words, null),
      'POST /api/play/c1/fate/next': () =>
        dead(words, 'new', { id: 'k2', status: 'draft', sheet: { name: '' }, gmNote: null, updatedAt: '' }),
    })
    renderHome()

    expect(await screen.findByText('Bretteur · Tombé · niveau 3')).toBeInTheDocument()
    expect(screen.getByRole('status')).toHaveTextContent('Ton personnage est tombé')
    await userEvent.type(screen.getByRole('textbox', { name: 'Ses derniers mots' }), words)
    await userEvent.click(screen.getByRole('button', { name: /Dire ses derniers mots/ }))
    expect(await screen.findByText(`« ${words} »`)).toBeInTheDocument()
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: /Créer un nouveau personnage/ }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/fate/words')).toEqual([{ text: words }])
    expect(sentTo(fetchMock, 'POST /api/play/c1/fate/next')).toEqual([{ next: 'new' }])
    // The new character is a draft: the death screen gives way to the creator.
    expect(
      await screen.findByText('Brouillon. Ta place est gardée : crée-le quand tu veux, en deux minutes.'),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Attendre une accroche/ })).not.toBeInTheDocument()
  })

  it('unfolds the game side by side on a computer: sheet, scene, map and journal at once, no tabs', async () => {
    stubReducedMotion(true, [DESKTOP_QUERY])
    const fetchMock = mockApi({
      ...EVENING,
      'GET /api/play/c1/me': () => home(inPlay(false)),
      'GET /api/play/c1/board': () => ({ status: 200, body: { data: null } }),
      'GET /api/play/c1/battle': () => ({ status: 200, body: { data: null } }),
      'GET /api/play/c1/view': () => ({
        status: 200,
        body: { data: { ...CAMPAIGN, party: [], scene: null, clues: ['Gwen a vu une lanterne.'], npcs: [], factions: [], goals: [] } },
      }),
    })
    renderHome()

    expect(await screen.findByText('Les corsaires ont accosté.')).toBeInTheDocument()
    expect(screen.getByRole('complementary', { name: 'Personnage' })).toHaveTextContent('7 / 10 PV')
    expect(await screen.findByText('Gwen a vu une lanterne.')).toBeInTheDocument()
    expect(await within(screen.getByRole('region', { name: 'Carte' })).findByText(/pas encore montré de carte/)).toBeInTheDocument()
    expect(screen.queryByRole('navigation', { name: 'Onglets' })).not.toBeInTheDocument()
    // Each part is fetched once: nothing is mounted twice behind the layout.
    expect(fetchMock.mock.calls.filter(([url]) => String(url) === '/api/play/c1/evening')).toHaveLength(1)
  })
})
