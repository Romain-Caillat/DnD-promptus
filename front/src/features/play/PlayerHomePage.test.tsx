import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
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
        music: null,
        lobby: [],
        campaign: { ...CAMPAIGN, scene: null, clues: [], npcs: [] },
        journal: [],
        requests: [],
        cards: [],
        feedback: null,
      },
    },
  }),
  'GET /api/play/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }),
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
          },
        },
      }),
    })
    renderHome()

    await userEvent.click(await screen.findByRole('button', { name: 'Journal' }))
    expect(await screen.findByText('Gwen a vu une lanterne sur la falaise.')).toBeInTheDocument()
    expect(screen.getByText('Loïc')).toBeInTheDocument()
    expect(screen.getByText('Une ville minière.')).toBeInTheDocument()
  })

  it('gives a spectator the game, the map and the journal, no sheet', async () => {
    mockApi({
      ...EVENING,
      'GET /api/play/c1/me': () => home(null, 'spectator'),
      'GET /api/play/c1/view': () => ({
        status: 200,
        body: { data: { ...CAMPAIGN, party: [], scene: null, clues: [], npcs: [] } },
      }),
    })
    renderHome()

    expect(await screen.findByText('Les corsaires ont accosté.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Carte' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Journal' }))
    expect(await screen.findByText("Le groupe n'a encore rien découvert.")).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Personnage' })).not.toBeInTheDocument()
  })
})
