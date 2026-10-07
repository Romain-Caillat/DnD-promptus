import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { ShopPanel } from './ShopPanel'

const line = (over: Record<string, unknown> = {}) => ({
  id: 'compas',
  name: 'Compas enchanté',
  description: 'Pointe toujours vers le nord.',
  price: 8,
  listPrice: 8,
  stock: 1,
  haggled: null,
  ...over,
})
const shop = (over: Record<string, unknown> = {}) => ({
  name: 'Le marché noir de Kerjean',
  lines: [line()],
  haggle: { ability: 'CHA', abilityName: 'Charisme', difficulty: 12, discount: 25 },
  purse: { name: "Pièces d'or", abbr: 'PO', amount: 10 },
  ...over,
})
const ok = (data: unknown) => () => ({ status: 200, body: { data } })

describe('ShopPanel', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('shows nothing while no shop is open', async () => {
    const fetchMock = mockApi({ 'GET /api/play/c1/shop': ok(null) })
    const { container } = render(<ShopPanel campaignId="c1" refreshKey={0} />)
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalled())
    expect(container).toBeEmptyDOMElement()
  })

  it('haggles, then buys at the price won', async () => {
    const fetchMock = mockApi({
      'GET /api/play/c1/shop': ok(shop()),
      'POST /api/play/c1/shop/haggle': ok({
        roll: { total: 15, band: 'success' },
        shop: shop({ lines: [line({ price: 6, haggled: true })] }),
      }),
      'POST /api/play/c1/shop/buy': ok(shop({ lines: [line({ price: 6, haggled: true, stock: 0 })], purse: { name: 'Or', abbr: 'PO', amount: 4 } })),
    })
    render(<ShopPanel campaignId="c1" refreshKey={0} />)
    const counter = await screen.findByRole('region', { name: 'Le marché noir de Kerjean' })
    expect(within(counter).getByText('Ta bourse : 10 PO')).toBeInTheDocument()
    expect(within(counter).getByText(/un jet de Charisme contre 12, −25 %/)).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Marchander · Compas enchanté' }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/shop/haggle')).toEqual([{ line: 'compas' }])
    expect(await screen.findByRole('status')).toHaveTextContent('15 contre 12, le marchand cède')
    // The list price struck through, mine beside it; no second haggle.
    expect(screen.getByText('8 PO').tagName).toBe('S')
    expect(screen.getByText('6 PO')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Marchander · Compas enchanté' })).toBeNull()

    await userEvent.click(screen.getByRole('button', { name: 'Acheter · Compas enchanté' }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/shop/buy')).toEqual([{ line: 'compas', qty: 1 }])
    expect(await screen.findByText('épuisé')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Acheter · Compas enchanté' })).toBeDisabled()
    expect(screen.getByText('Ta bourse : 4 PO')).toBeInTheDocument()
  })

  it('cannot buy what the purse cannot pay, and says why the server refused', async () => {
    mockApi({
      'GET /api/play/c1/shop': ok(shop({ lines: [line({ id: 'sloop', name: 'Un sloop', price: 500, stock: null }), line()] })),
      'POST /api/play/c1/shop/buy': () => ({ status: 409, body: { error: { code: 'OUT_OF_STOCK' } } }),
    })
    render(<ShopPanel campaignId="c1" refreshKey={0} />)
    expect(await screen.findByRole('button', { name: 'Acheter · Un sloop' })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: 'Acheter · Compas enchanté' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Il n’en reste plus assez.')
  })

  it('shows a spectator the counter, without a purse or a button', async () => {
    mockApi({ 'GET /api/play/c1/shop': ok(shop({ purse: null })) })
    render(<ShopPanel campaignId="c1" refreshKey={0} />)
    expect(await screen.findByText('Compas enchanté')).toBeInTheDocument()
    expect(screen.queryByRole('button')).toBeNull()
  })
})
