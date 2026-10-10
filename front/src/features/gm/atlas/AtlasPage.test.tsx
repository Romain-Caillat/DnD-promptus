import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { TABLET_QUERY } from '@/lib/useMediaQuery'
import { mockApi, stubReducedMotion } from '@/test-utils'
import { AtlasPage } from './AtlasPage'

const grid = (w: number, h: number) => ({
  legend: { '.': { terrain: 'herbe' } },
  rows: Array.from({ length: h }, () => '.'.repeat(w)),
})

const sheet = (
  id: string,
  name: string,
  scale: 'world' | 'place' | 'encounter',
  extra: Record<string, unknown> = {},
  tree: Record<string, unknown> = {},
) => ({
  map: { id, name, scale, theme: 't', ambience: {}, grid: grid(4, 3), ...extra },
  origin: 'world',
  parent: null,
  children: [],
  links: [],
  backdrop: false,
  validated: true,
  ...tree,
})

const ATLAS = {
  revision: 3,
  roots: ['region', 'rues'],
  maps: [
    sheet(
      'region',
      'Les côtes',
      'world',
      {
        layers: [
          { id: 'base', name: 'Côtes', visibility: 'all' },
          { id: 'secrets', name: 'Secrets', visibility: 'gm' },
        ],
        exits: [{ id: 'vers-quai', cells: [[1, 0]], to: 'quai' }],
        labels: [
          { text: 'Port-Louis', at: [1, 0] },
          { text: 'Mouillage anglais', at: [3, 2], layer: 'secrets' },
        ],
      },
      { children: ['quai'] },
    ),
    sheet(
      'quai',
      'Le quai',
      'place',
      {
        exits: [
          { id: 'vers-cale', cells: [[2, 1]], to: 'cale' },
          { id: 'vers-rues', cells: [[0, 1]], to: 'rues' },
        ],
      },
      { parent: { map: 'region', exit: 'vers-quai' }, children: ['cale'], links: [{ to: 'rues', exit: 'vers-rues' }] },
    ),
    sheet('cale', 'La cale', 'encounter', {}, { parent: { map: 'quai', exit: 'vers-cale' }, origin: 'campaign' }),
    sheet('rues', 'Les rues', 'place'),
  ],
}

const ok = (data: unknown) => ({ status: 200, body: { data } })

/** The browser's WebSocket, played by the test. */
class FakeSocket {
  static all: FakeSocket[] = []
  onopen: (() => void) | null = null
  onmessage: ((ev: { data: string }) => void) | null = null
  onclose: (() => void) | null = null
  onerror: (() => void) | null = null

  constructor() {
    FakeSocket.all.push(this)
  }

  send() {}
  close() {}

  push(msg: unknown) {
    act(() => this.onmessage?.({ data: JSON.stringify(msg) }))
  }
}

function renderPage() {
  return render(
    <MemoryRouter initialEntries={['/campagnes/c1/atlas']}>
      <Routes>
        <Route path="/campagnes/:campaignId/atlas" element={<AtlasPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

const crumbs = () =>
  within(screen.getByRole('navigation', { name: 'Où vous êtes dans l’atlas' }))
    .getAllByRole('button')
    .map((b) => b.textContent)

describe('AtlasPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('enters a place from its hex, an encounter from the place, and zooms out where it came from', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(ATLAS),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
    })
    const { container } = renderPage()

    // The region opens first, its place ringed where it opens.
    await screen.findByRole('img', { name: 'Les côtes' })
    expect(crumbs()).toEqual(['Région · Les côtes'])
    expect(container.querySelector('[data-mark="1,0"]')).not.toBeNull()
    expect(screen.getByRole('button', { name: 'Ressortir' })).toBeDisabled()

    fireEvent.click(container.querySelector('[data-hex="1,0"]')!)
    await screen.findByRole('img', { name: 'Carte : Le quai' })
    expect(crumbs()).toEqual(['Région · Les côtes', 'Lieu · Le quai'])

    await userEvent.click(screen.getByRole('button', { name: 'Entrer : La cale' }))
    await screen.findByRole('img', { name: 'Carte : La cale' })
    expect(crumbs()).toEqual(['Région · Les côtes', 'Lieu · Le quai', 'Rencontre · La cale'])

    // Straight back up to the region from the breadcrumb: nowhere further out.
    const breadcrumb = screen.getByRole('navigation', { name: 'Où vous êtes dans l’atlas' })
    await userEvent.click(within(breadcrumb).getByRole('button', { name: /Les côtes/ }))
    await screen.findByRole('img', { name: 'Les côtes' })
    expect(screen.getByRole('button', { name: 'Ressortir' })).toBeDisabled()
    fireEvent.click(container.querySelector('[data-hex="1,0"]')!)
    await screen.findByRole('img', { name: 'Carte : Le quai' })

    // Next door to the quai, then back out: to the quai, not to a parent.
    await userEvent.click(screen.getByRole('button', { name: 'Aller à : Les rues' }))
    await screen.findByRole('img', { name: 'Carte : Les rues' })
    expect(crumbs()).toEqual(['Lieu · Les rues'])
    await userEvent.click(screen.getByRole('button', { name: 'Ressortir' }))
    await screen.findByRole('img', { name: 'Carte : Le quai' })
    await userEvent.click(screen.getByRole('button', { name: 'Ressortir' }))
    await screen.findByRole('img', { name: 'Les côtes' })
  })

  it('hides a layer on demand and hears a new map arrive', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    FakeSocket.all = []
    let atlas: typeof ATLAS = ATLAS
    mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(atlas),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
    })
    renderPage()
    await screen.findByText('Mouillage anglais')

    await userEvent.click(screen.getByRole('button', { name: 'Secrets' }))
    expect(screen.queryByText('Mouillage anglais')).not.toBeInTheDocument()
    expect(screen.getByText('Port-Louis')).toBeInTheDocument()

    // Claude (or the GM elsewhere) adds a map: the strip shows it.
    atlas = { ...ATLAS, roots: [...ATLAS.roots, 'crypte'], maps: [...ATLAS.maps, sheet('crypte', 'La crypte', 'encounter')] }
    FakeSocket.all[0].push({ type: 'ready', versions: { prep: 3 } })
    FakeSocket.all[0].push({ type: 'changed', topic: 'prep', version: 4 })
    const strip = screen.getByRole('navigation', { name: 'Toutes les cartes' })
    expect(await within(strip).findByRole('button', { name: /La crypte/ })).toBeInTheDocument()
  })

  it('draws a place light while the fingers move it, then sharp again', async () => {
    stubReducedMotion(true, [TABLET_QUERY])
    vi.stubGlobal('WebSocket', FakeSocket)
    mockApi({
      'GET /api/campaigns/c1/atlas': () => ok({ ...ATLAS, roots: ['quai'] }),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
    })
    renderPage()
    const canvas = (await screen.findByRole('img', { name: 'Carte : Le quai' })) as HTMLCanvasElement
    const sharp = canvas.width

    fireEvent.pointerDown(canvas, { pointerId: 1, clientX: 10, clientY: 10 })
    fireEvent.pointerDown(canvas, { pointerId: 2, clientX: 60, clientY: 10 })
    fireEvent.pointerMove(canvas, { pointerId: 2, clientX: 90, clientY: 10 })
    await waitFor(() => expect(canvas.width).toBe(sharp / 2))

    fireEvent.pointerUp(canvas, { pointerId: 2, clientX: 90, clientY: 10 })
    fireEvent.pointerUp(canvas, { pointerId: 1, clientX: 10, clientY: 10 })
    await waitFor(() => expect(canvas.width).toBe(sharp))
  })
})
