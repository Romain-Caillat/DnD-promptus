import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ShopsPanel } from '@/features/gm/live/ShopsPanel'
import type { PlayView } from '@/lib/play'
import type { ShopsScreen, TradeView } from '@/lib/trade'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { GiveForm } from './GiveForm'
import { Market } from './Market'

const ROLL = {
  die: '1d20',
  faces: [14],
  natural: 14,
  advantage: 'normal',
  modifiers: [{ source: { from: 'ability', id: 'CHA' }, value: -1 }],
  total: 13,
  target: { against: 'difficulty', id: 'moyen', value: 10 },
  band: 'success',
}

const TRADE: TradeView = {
  purse: { name: "Pièces d'or", abbr: 'PO', amount: 10 },
  shops: [
    {
      id: 's1',
      name: 'Le marché noir de Kerjean',
      keeper: 'Dents-de-Fer',
      abbr: 'PO',
      lines: [
        { key: 'obj_corde', name: 'Corde de 30 m', description: 'Solide.', price: 2, discountedPrice: null, stock: null },
        { key: 'obj_epee', name: 'Épée de bonne facture', description: '', price: 15, discountedPrice: null, stock: 1 },
      ],
      haggle: { ability: 'CHA', abilityName: 'Charisme', difficulty: 10, label: 'Moyen', discountPercent: 50, mine: null },
      surcharge: 0,
    },
  ],
  companions: [{ id: 'c2', name: 'Borin' }],
}

const HAGGLED: TradeView = {
  ...TRADE,
  shops: [
    {
      ...TRADE.shops[0],
      lines: TRADE.shops[0].lines.map((l) => ({ ...l, discountedPrice: Math.ceil(l.price / 2) })),
      haggle: {
        ...TRADE.shops[0].haggle!,
        mine: { band: 'success', outcome: 'Réussite', roll: ROLL as never, discountLeft: true },
      },
    },
  ],
}

beforeEach(() => stubReducedMotion(true))
afterEach(() => vi.unstubAllGlobals())

describe('Market', () => {
  it('buys one unit, haggles once, then offers the won discount', async () => {
    const fetchMock = mockApi({
      'GET /api/play/c1/trade': () => ({ status: 200, body: { data: TRADE } }),
      'POST /api/play/c1/shops/s1/buy': () => ({ status: 200, body: { data: TRADE } }),
      'POST /api/play/c1/shops/s1/haggle': () => ({ status: 200, body: { data: HAGGLED } }),
    })
    render(<Market campaignId="c1" refreshKey={0} seated />)
    expect(await screen.findByText('Le marché noir de Kerjean')).toBeInTheDocument()
    expect(screen.getByText('Ta bourse : 10 PO')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Acheter · Corde de 30 m' }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/shops/s1/buy')).toEqual([{ line: 'obj_corde', discounted: false }])
    await userEvent.click(screen.getByRole('button', { name: /Marchander/ }))
    expect(await screen.findByText(/un achat de ton choix à −50 %/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Marchander/ })).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Acheter à 8 PO' }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/shops/s1/buy')[1]).toEqual({ line: 'obj_epee', discounted: true })
  })

  it('says why the server refused a purchase', async () => {
    mockApi({
      'GET /api/play/c1/trade': () => ({ status: 200, body: { data: TRADE } }),
      'POST /api/play/c1/shops/s1/buy': () => ({ status: 409, body: { error: { code: 'NOT_ENOUGH', message: '' } } }),
    })
    render(<Market campaignId="c1" refreshKey={0} seated />)
    await userEvent.click(await screen.findByRole('button', { name: 'Acheter · Épée de bonne facture' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Pas assez dans ta bourse.')
  })

  it('lets a spectator look without buying, and shows nothing when no shop is open', async () => {
    mockApi({ 'GET /api/play/c1/trade': () => ({ status: 200, body: { data: TRADE } }) })
    const { unmount } = render(<Market campaignId="c1" refreshKey={0} seated={false} />)
    expect(await screen.findByText('Corde de 30 m')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Acheter/ })).not.toBeInTheDocument()
    unmount()
    mockApi({ 'GET /api/play/c1/trade': () => ({ status: 200, body: { data: { ...TRADE, shops: [] } } }) })
    const { container } = render(<Market campaignId="c1" refreshKey={0} seated />)
    await waitFor(() => expect(container).toBeEmptyDOMElement())
  })
})

describe('GiveForm', () => {
  it('hands money to a companion', async () => {
    const fetchMock = mockApi({
      'GET /api/play/c1/trade': () => ({ status: 200, body: { data: TRADE } }),
      'POST /api/play/c1/character/give': () => ({ status: 200, body: { data: TRADE } }),
    })
    const play = { inventory: [], resources: [] } as unknown as PlayView
    render(<GiveForm campaignId="c1" play={play} />)
    await userEvent.selectOptions(await screen.findByLabelText('Quoi'), "Pièces d'or")
    await userEvent.selectOptions(screen.getByLabelText('À qui'), 'Borin')
    await userEvent.clear(screen.getByLabelText('Combien'))
    await userEvent.type(screen.getByLabelText('Combien'), '3')
    await userEvent.click(screen.getByRole('button', { name: 'Donner' }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/character/give')).toEqual([{ to: 'c2', amount: 3 }])
    expect(await screen.findByText("C'est donné.")).toBeInTheDocument()
  })
})

const SHOPS: ShopsScreen = {
  shops: [
    {
      id: 's1',
      name: 'Le marché noir de Kerjean',
      keeper: 'Dents-de-Fer',
      npc: 'pnj_dents_de_fer',
      open: false,
      currency: 'or',
      lines: [
        { key: 'obj_boussole_amiral', storyItem: 'obj_boussole_amiral', price: 25, stock: 1, hidden: true, displayName: "Boussole d'Amiral" },
      ],
      haggle: null,
      surcharge: 0,
      haggles: [],
    },
  ],
  sellable: [],
  keepers: [{ id: 'pnj_dents_de_fer', name: 'Yann Kerjean', title: 'Dents-de-Fer' }],
  currency: { id: 'or', name: "Pièces d'or", abbr: 'PO' },
  abilities: [],
  difficulties: [],
}

describe('ShopsPanel', () => {
  it('opens a shop to the players and brings out the hidden compass', async () => {
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/shops': () => ({ status: 200, body: { data: SHOPS } }),
      'POST /api/campaigns/c1/shops/s1/open': () => ({ status: 200, body: { data: SHOPS } }),
      'POST /api/campaigns/c1/shops/s1/reveal': () => ({ status: 200, body: { data: SHOPS } }),
    })
    render(<ShopsPanel campaignId="c1" refreshKey={0} />)
    await userEvent.click(await screen.findByRole('button', { name: 'Ouvrir aux joueurs' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/shops/s1/open')).toEqual([{ open: true }])
    await userEvent.click(screen.getByRole('button', { name: "Sortir de sous le comptoir · Boussole d'Amiral" }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/shops/s1/reveal')).toEqual([{ line: 'obj_boussole_amiral' }])
    // The NPC already has a shop: not offered twice.
    expect(screen.queryByRole('button', { name: 'Boutique de Dents-de-Fer' })).not.toBeInTheDocument()
  })

  it('tells the GM to add a currency when the rules have none', async () => {
    mockApi({
      'GET /api/campaigns/c1/shops': () => ({ status: 200, body: { data: { ...SHOPS, shops: [], currency: null } } }),
    })
    render(<ShopsPanel campaignId="c1" refreshKey={0} />)
    expect(await screen.findByText(/ajoute une ressource dans l'éditeur de règles/)).toBeInTheDocument()
  })
})
