import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { PlayView } from '@/lib/play'
import { mockApi, sentTo } from '@/test-utils'
import { SharePanel } from './SharePanel'

const PLAY = {
  resources: [{ id: 'or', name: "Pièces d'or", abbr: 'PO', amount: 10 }],
  inventory: [
    { key: 'k1', itemId: 'dague', name: 'Dague de ceinture', description: '', qty: 2, consumable: false, equipped: false },
  ],
} as unknown as PlayView
const CHARACTER = { id: 'ch1' }

describe('SharePanel', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('gives coins, then a bag line, to a friend', async () => {
    const onChanged = vi.fn()
    const fetchMock = mockApi({
      'GET /api/play/c1/party': () => ({ status: 200, body: { data: [{ id: 'lyra', name: 'Lyra', nickname: 'Camille' }] } }),
      'POST /api/play/c1/character/give': () => ({ status: 200, body: { data: CHARACTER } }),
    })
    render(<SharePanel campaignId="c1" play={PLAY} onChanged={onChanged} />)
    expect(await screen.findByRole('option', { name: 'Lyra (Camille)' })).toBeInTheDocument()
    const amount = screen.getByRole('spinbutton', { name: 'Combien' })
    fireEvent.change(amount, { target: { value: '3' } })
    await userEvent.click(screen.getByRole('button', { name: 'Donner' }))
    await vi.waitFor(() => expect(onChanged).toHaveBeenCalledWith(CHARACTER))

    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Quoi' }), 'k1')
    await userEvent.click(screen.getByRole('button', { name: 'Donner' }))
    await vi.waitFor(() => expect(onChanged).toHaveBeenCalledTimes(2))
    expect(sentTo(fetchMock, 'POST /api/play/c1/character/give')).toEqual([
      { to: 'lyra', coins: 3 },
      { to: 'lyra', entry: 'k1', qty: 1 },
    ])
  })

  it('will not give more than the purse holds', async () => {
    mockApi({
      'GET /api/play/c1/party': () => ({ status: 200, body: { data: [{ id: 'lyra', name: 'Lyra', nickname: 'Camille' }] } }),
    })
    render(<SharePanel campaignId="c1" play={PLAY} onChanged={vi.fn()} />)
    const amount = await screen.findByRole('spinbutton', { name: 'Combien' })
    fireEvent.change(amount, { target: { value: '11' } })
    expect(screen.getByRole('button', { name: 'Donner' })).toBeDisabled()
  })

  it('hides itself when nobody else is in play', async () => {
    const fetchMock = mockApi({ 'GET /api/play/c1/party': () => ({ status: 200, body: { data: [] } }) })
    const { container } = render(<SharePanel campaignId="c1" play={PLAY} onChanged={vi.fn()} />)
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalled())
    expect(container).toBeEmptyDOMElement()
  })
})
