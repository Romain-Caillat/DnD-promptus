import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { TABLET_QUERY } from '@/lib/useMediaQuery'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
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
  pins: [],
  cast: { npcs: [], clues: [], fights: [] },
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
  places: [],
  unplaced: [],
  issues: [],
  castable: { npcs: [], clues: [], fights: [], scenes: [] },
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

  it('lists the scenes of a pin, opens their sheet, and pins a place on the cell tapped', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    const tavern = {
      location: 'l_taverne',
      name: 'La taverne',
      at: [2, 1],
      scenes: [
        { id: 'sc_a', title: 'L’arrivée', number: 1 },
        { id: 'sc_c', title: 'La proposition', number: 3, map: 'cale' },
      ],
    }
    const town = { location: 'l_ville', name: 'Port-Louis', at: [1, 0], scenes: [...tavern.scenes, { id: 'sc_b', title: 'Le port', number: 2 }] }
    let atlas = {
      ...ATLAS,
      maps: ATLAS.maps.map((m) =>
        m.map.id === 'region' ? { ...m, pins: [town] } : m.map.id === 'quai' ? { ...m, pins: [tavern] } : m,
      ),
      places: [
        { id: 'l_ville', name: 'Port-Louis', parent: null, anchor: { map: 'region', at: [1, 0] } },
        { id: 'l_crypte', name: 'La crypte', parent: null, anchor: null },
      ],
      unplaced: [{ id: 'sc_reve', title: 'Le rêve', number: 4 }],
    }
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(atlas),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'POST /api/campaigns/c1/story/edits': () => {
        atlas = { ...atlas, revision: 4 }
        return ok({ campaign: {}, changes: [] })
      },
      'POST /api/campaigns/c1/maps/generate': () => ok({ map: { id: 'carte-arrivee' }, dropped: 0 }),
    })
    const { container } = renderPage()

    // On the region, the town's pin carries its scene numbers.
    const pin = await screen.findByRole('button', { name: /Port-Louis — scènes : 1\. L’arrivée/ })
    expect(pin.textContent).toBe('1·3·2')
    // The scene standing nowhere is said, and opens its sheet.
    expect(screen.getByRole('link', { name: '4. Le rêve' })).toHaveAttribute(
      'href',
      '/campagnes/c1/preparer?onglet=graph&scene=sc_reve',
    )

    // On the place map, the tavern's pin lists its two scenes.
    fireEvent.click(container.querySelector('[data-hex="1,0"]')!)
    await screen.findByRole('img', { name: 'Carte : Le quai' })
    await userEvent.click(screen.getByRole('button', { name: /La taverne — scènes/ }))
    const chosen = screen.getByRole('region', { name: 'Épingle : La taverne' })
    expect(within(chosen).getByRole('link', { name: '1. L’arrivée' })).toHaveAttribute(
      'href',
      '/campagnes/c1/preparer?onglet=graph&scene=sc_a',
    )
    // A scene with its fight map opens it; one without may generate it.
    expect(within(chosen).getByRole('button', { name: 'Voir sa carte' })).toBeInTheDocument()
    expect(within(chosen).getByRole('button', { name: 'Générer sa carte' })).toBeInTheDocument()

    // The pin tool: a place, then the cell — one story edit on its anchor.
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Lieu à épingler' }), 'l_crypte')
    await userEvent.click(screen.getByRole('button', { name: 'Épingler sur cette carte' }))
    expect(screen.getByRole('status')).toHaveTextContent('Touchez la case où se trouve « La crypte ».')
    const canvas = screen.getByRole('img', { name: 'Carte : Le quai' })
    canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: canvas.clientWidth, height: canvas.clientHeight }) as DOMRect
    fireEvent.pointerDown(canvas, { pointerId: 1, clientX: 1, clientY: 1 })
    fireEvent.pointerUp(canvas, { pointerId: 1, clientX: 1, clientY: 1 })
    await waitFor(() => expect(sentTo(fetchMock, 'POST /api/campaigns/c1/story/edits')).toHaveLength(1))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/story/edits')[0]).toEqual({
      edits: [{ op: 'set', target: 'l_crypte', field: 'anchor', value: { map: 'quai', at: [0, 0] } }],
    })

    // From the pin, a scene without a fight map generates one, attached to it.
    await userEvent.click(screen.getByRole('button', { name: /La taverne — scènes/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Générer sa carte' }))
    await waitFor(() => expect(sentTo(fetchMock, 'POST /api/campaigns/c1/story/edits')).toHaveLength(2))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/maps/generate')).toEqual([{ node: 'sc_a' }])
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/story/edits')[1]).toEqual({
      edits: [{ op: 'set', target: 'sc_a', field: 'map', value: 'carte-arrivee' }],
    })
  })

  it('places an NPC for one scene, opens a fight’s battle plan, takes a clue off, and dims a story layer', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    const cast = {
      npcs: [{ npc: 'pnj_oswin', name: 'Oswin', index: 0, at: [1, 1] }],
      clues: [{ clue: 'cl_tombe', text: 'Une tombe ouverte', at: [2, 2], scene: 'sc_a' }],
      fights: [{ scene: 'sc_a', title: 'La crypte', number: 1, at: [3, 1], map: 'cale', opponents: 3, placed: 2 }],
    }
    const atlas = {
      ...ATLAS,
      roots: ['quai'],
      maps: ATLAS.maps.map((m) => (m.map.id === 'quai' ? { ...m, cast } : m)),
      castable: {
        npcs: [{ id: 'pnj_oswin', name: 'Oswin', spots: [{ map: 'quai', at: [1, 1] }] }],
        clues: [{ id: 'cl_tombe', text: 'Une tombe ouverte', scene: 'sc_a', anchor: { map: 'quai', at: [2, 2] } }],
        fights: [{ scene: 'sc_a', title: 'La crypte', number: 1, map: 'cale', anchor: { map: 'quai', at: [3, 1] } }],
        scenes: [{ id: 'sc_a', title: 'La crypte', number: 1 }],
      },
    }
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(atlas),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'POST /api/campaigns/c1/story/edits': () => ok({ campaign: {}, changes: [] }),
    })
    renderPage()
    await screen.findByRole('img', { name: 'Carte : Le quai' })

    // The NPC, the clue and the fight stand on the map; Personnages dims the NPC.
    expect(screen.getByRole('button', { name: 'Oswin' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Indice : Une tombe ouverte' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Personnages' }))
    expect(screen.queryByRole('button', { name: 'Oswin' })).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Personnages' }))

    // Oswin for one scene only: a kind, an NPC, a scene, then the cell.
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Choisir un PNJ…' }), 'pnj_oswin')
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Pour quelle scène' }), 'sc_a')
    await userEvent.click(screen.getByRole('button', { name: 'Poser sur cette carte' }))
    expect(screen.getByRole('status')).toHaveTextContent('Touchez la case où poser « Oswin ».')
    const canvas = screen.getByRole('img', { name: 'Carte : Le quai' })
    canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: canvas.clientWidth, height: canvas.clientHeight }) as DOMRect
    fireEvent.pointerDown(canvas, { pointerId: 1, clientX: 1, clientY: 1 })
    fireEvent.pointerUp(canvas, { pointerId: 1, clientX: 1, clientY: 1 })
    await waitFor(() => expect(sentTo(fetchMock, 'POST /api/campaigns/c1/story/edits')).toHaveLength(1))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/story/edits')[0]).toEqual({
      edits: [
        {
          op: 'set',
          target: 'pnj_oswin',
          field: 'spots',
          value: [
            { map: 'quai', at: [1, 1] },
            { map: 'quai', at: [0, 0], scene: 'sc_a' },
          ],
        },
      ],
    })

    // The clue comes off the map.
    await userEvent.click(screen.getByRole('button', { name: 'Indice : Une tombe ouverte' }))
    await userEvent.click(screen.getByRole('button', { name: 'Retirer de la carte' }))
    await waitFor(() => expect(sentTo(fetchMock, 'POST /api/campaigns/c1/story/edits')).toHaveLength(2))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/story/edits')[1]).toEqual({
      edits: [{ op: 'set', target: 'cl_tombe', field: 'anchor', value: null }],
    })

    // The fight says what its battle plan places, and opens it.
    await userEvent.click(screen.getByRole('button', { name: 'Combat de la scène 1. La crypte' }))
    const fight = screen.getByRole('region', { name: 'Combat : La crypte' })
    expect(within(fight).getByText('Adversaires déjà placés sur le plan : 2 / 3')).toBeInTheDocument()
    await userEvent.click(within(fight).getByRole('button', { name: 'Ouvrir le plan de combat' }))
    expect(await screen.findByRole('img', { name: 'Carte : La cale' })).toBeInTheDocument()
  })
})
