import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { PlaySheets } from './PlaySheets'

const play = (over: Record<string, unknown> = {}) => ({
  level: 1,
  totalXp: 0,
  xpBar: 0,
  xpBarMax: 5,
  upgradePoints: 0,
  nextLevelXp: 5,
  hitPoints: 10,
  maxHitPoints: 10,
  armorClass: 12,
  initiative: 2,
  abilities: [],
  cards: [],
  resources: [{ id: 'or', name: "Pièces d'or", abbr: 'PO', amount: 10 }],
  inventory: [
    { key: 'sabre', itemId: 'sabre', name: "Sabre d'abordage", description: '', qty: 1, consumable: false, equipped: true },
  ],
  ...over,
})

const board = (lyra: Record<string, unknown>, history: unknown[] = [], fallen: unknown[] = []) => ({
  rulesKnown: true,
  sheets: [
    {
      characterId: 'k1',
      playerId: 'p1',
      nickname: 'Camille',
      name: 'Lyra',
      className: 'Bretteur',
      look: null,
      play: play(lyra),
    },
  ],
  items: [{ id: 'rhum', name: 'Fiole de rhum fortifiant', description: 'Restaure 3 PV.', consumable: true }],
  resources: [{ id: 'or', name: "Pièces d'or", abbr: 'PO' }],
  history,
  fallen,
})

describe('PlaySheets', () => {
  beforeEach(() => {
    stubReducedMotion(true)
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('gives XP in one gesture, then shows the new sheet and the history', async () => {
    let current = board({})
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/sheets': () => ({ status: 200, body: { data: current } }),
      'POST /api/campaigns/c1/characters/k1/adjust': () => {
        current = board({ totalXp: 1, xpBar: 1 }, [
          { id: 'h1', characterId: 'k1', actor: 'gm', kind: 'xp', label: null, before: 0, after: 1, createdAt: '2026-10-06T20:04:00Z' },
        ])
        return { status: 200, body: { data: current.sheets[0] } }
      },
    })
    render(<PlaySheets campaignId="c1" refreshKey={0} />)

    const card = await screen.findByRole('listitem', { name: 'Lyra' })
    expect(within(card).getByText('10 / 10 PV')).toBeInTheDocument()
    expect(within(card).getByText(/10 PO/)).toBeInTheDocument()
    // Whole: nothing to heal.
    expect(within(card).getByRole('button', { name: '+1 PV' })).toBeDisabled()
    await userEvent.click(within(card).getByRole('button', { name: '+1 XP' }))

    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/characters/k1/adjust')).toEqual([{ kind: 'xp', delta: 1 }])
    expect(await within(card).findByText(/^1 XP/)).toBeInTheDocument()
    const history = screen.getByRole('heading', { name: 'Historique' }).parentElement!
    expect(within(history).getByRole('listitem')).toHaveTextContent('Lyra · +1 XP (1 au total)')
  })

  it('names an item the rules do not list, and says when there is not enough to take', async () => {
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/sheets': () => ({ status: 200, body: { data: board({}) } }),
      'POST /api/campaigns/c1/characters/k1/adjust': (body) =>
        (body as { kind: string }).kind === 'resource'
          ? { status: 409, body: { error: { code: 'NOT_ENOUGH', message: 'x' } } }
          : { status: 200, body: { data: board({}).sheets[0] } },
    })
    render(<PlaySheets campaignId="c1" refreshKey={0} />)
    const card = await screen.findByRole('listitem', { name: 'Lyra' })
    await userEvent.click(within(card).getByRole('button', { name: 'Plus…' }))

    await userEvent.selectOptions(within(card).getByLabelText('Donner un objet'), 'other')
    await userEvent.type(within(card).getByLabelText("Nom de l'objet"), 'Lanterne du phare')
    await userEvent.click(within(card).getByRole('button', { name: 'Donner à Lyra' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/characters/k1/adjust')).toEqual([
      { kind: 'giveItem', name: 'Lanterne du phare', description: '' },
    ])

    await userEvent.click(within(card).getByRole('button', { name: '−1 PO' }))
    expect(await within(card).findByRole('alert')).toHaveTextContent("Il n'en a pas assez.")
  })
  it('decides a death at 0, after a confirmation, and lists the fallen', async () => {
    let current = board({ hitPoints: 0 })
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/sheets': () => ({ status: 200, body: { data: current } }),
      'POST /api/campaigns/c1/characters/k1/death': () => {
        current = {
          ...board({}, [
            { id: 'h1', characterId: 'k1', actor: 'gm', kind: 'death', label: 'Lyra', before: 3, after: 0, createdAt: '2026-10-06T21:00:00Z' },
          ], [
            {
              characterId: 'k1',
              nickname: 'Camille',
              name: 'Lyra',
              className: 'Bretteur',
              look: null,
              level: 3,
              lastWords: null,
              next: null,
              diedAt: '2026-10-06T21:00:00Z',
            },
          ]),
          sheets: [],
        }
        return { status: 204 }
      },
    })
    render(<PlaySheets campaignId="c1" refreshKey={0} />)

    const card = await screen.findByRole('listitem', { name: 'Lyra' })
    await userEvent.click(within(card).getByRole('button', { name: 'Il meurt…' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/characters/k1/death')).toHaveLength(0)
    await userEvent.click(within(card).getByRole('button', { name: 'Confirmer la mort de Lyra' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/characters/k1/death')).toHaveLength(1)
    const fallen = (await screen.findByRole('heading', { name: 'Tombés' })).parentElement!
    expect(within(fallen).getByText('Lyra · niveau 3 · joué par Camille')).toBeInTheDocument()
    expect(within(fallen).getByText('Pas encore de derniers mots.')).toBeInTheDocument()
    const history = screen.getByRole('heading', { name: 'Historique' }).parentElement!
    expect(within(history).getByRole('listitem')).toHaveTextContent('Lyra · meurt, au niveau 3')
  })
})
