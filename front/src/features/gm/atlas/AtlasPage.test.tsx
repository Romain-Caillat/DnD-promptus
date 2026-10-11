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
  thread: { segments: [], stops: [] },
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
  acts: [],
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

  drop() {
    act(() => this.onclose?.())
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
      '/campagnes/c1/atlas?carte=region&onglet=graph&scene=sc_reve',
    )

    // On the place map, the tavern's pin lists its two scenes.
    fireEvent.click(container.querySelector('[data-hex="1,0"]')!)
    await screen.findByRole('img', { name: 'Carte : Le quai' })
    await userEvent.click(screen.getByRole('button', { name: /La taverne — scènes/ }))
    const chosen = screen.getByRole('region', { name: 'Épingle : La taverne' })
    expect(within(chosen).getByRole('link', { name: '1. L’arrivée' })).toHaveAttribute(
      'href',
      '/campagnes/c1/atlas?carte=quai&onglet=graph&scene=sc_a',
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

  it('draws the thread between the pins, branches included, synced with the scene list and redrawn on a revision', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    FakeSocket.all = []
    const scenes = [
      { id: 'sc_a', title: 'L’arrivée', number: 1 },
      { id: 'sc_b', title: 'Le port', number: 2 },
      { id: 'sc_c', title: 'Le phare', number: 3 },
      { id: 'sc_d', title: 'Le départ', number: 4 },
    ]
    const end = (place: string, at: [number, number]) => ({ place, at })
    const town = end('l_ville', [1, 0])
    const light = end('l_phare', [3, 2])
    const isle = end('l_ile', [0, 2])
    const stop = (scene: string, place: string, act: string, start = false, last = false) => ({ scene, place, act, start, end: last })
    const thread = {
      segments: [
        // sc_a branches: to the port inside the town (no line), and out to the lighthouse.
        { from: town, to: light, steps: [{ from: 'sc_a', to: 'sc_c', act: 'a1' }] },
        { from: light, to: town, steps: [{ from: 'sc_c', to: 'sc_b', act: 'a1' }] },
        { from: town, to: isle, steps: [{ from: 'sc_b', to: 'sc_d', act: 'a1' }] },
      ],
      stops: [stop('sc_a', 'l_ville', 'a1', true), stop('sc_b', 'l_ville', 'a1'), stop('sc_c', 'l_phare', 'a1'), stop('sc_d', 'l_ile', 'a2', true, true)],
    }
    const pins = [
      { location: 'l_ville', name: 'Port-Louis', at: [1, 0], scenes: [scenes[0], scenes[1]] },
      { location: 'l_phare', name: 'Le phare', at: [3, 2], scenes: [scenes[2]] },
      { location: 'l_ile', name: 'L’île', at: [0, 2], scenes: [scenes[3]] },
    ]
    // Inside the town, on the place map: the tavern and the port, act 1 only.
    const quayPins = [
      { location: 'l_taverne', name: 'La taverne', at: [0, 0], scenes: [scenes[0]] },
      { location: 'l_port', name: 'Le port', at: [3, 2], scenes: [scenes[1]] },
    ]
    const quayThread = {
      segments: [{ from: end('l_taverne', [0, 0]), to: end('l_port', [3, 2]), steps: [{ from: 'sc_a', to: 'sc_b', act: 'a1' }] }],
      stops: [stop('sc_a', 'l_taverne', 'a1', true), stop('sc_b', 'l_port', 'a1')],
    }
    let atlas = {
      ...ATLAS,
      maps: ATLAS.maps.map((m) =>
        m.map.id === 'region'
          ? { ...m, pins, thread }
          : m.map.id === 'quai'
            ? { ...m, pins: quayPins, thread: quayThread }
            : m,
      ),
      places: pins.map((p) => ({ id: p.location, name: p.name, parent: null, anchor: { map: 'region', at: p.at } })),
      castable: { ...ATLAS.castable, scenes },
      acts: [
        { id: 'a1', title: 'Le port' },
        { id: 'a2', title: 'Le large' },
      ],
    }
    mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(atlas),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
    })
    const { container } = renderPage()
    const lines = () => [...container.querySelectorAll('[data-thread]')].map((g) => g.getAttribute('data-thread'))
    const lit = () => [...container.querySelectorAll('[data-thread][data-lit]')].map((g) => g.getAttribute('data-thread'))

    // Every way between two pins is its own arrow: the town branches.
    await screen.findByRole('button', { name: /Port-Louis — scènes/ })
    expect(lines()).toEqual(['l_ville>l_phare', 'l_phare>l_ville', 'l_ville>l_ile'])
    const list = screen.getByRole('region', { name: 'Le fil de la campagne' })
    expect(within(list).getByRole('button', { name: /1\. L’arrivée/ })).toHaveTextContent('départ')
    expect(within(list).getByRole('button', { name: /4\. Le départ/ })).toHaveTextContent('fin possible')

    // A scene from the list lights its arrows and chooses its pin…
    await userEvent.click(within(list).getByRole('button', { name: /3\. Le phare/ }))
    expect(lit()).toEqual(['l_ville>l_phare', 'l_phare>l_ville'])
    expect(screen.getByRole('button', { name: /Le phare — scènes/ })).toHaveAttribute('aria-pressed', 'true')
    // …and a pin from the map marks its scenes in the list.
    await userEvent.click(screen.getByRole('button', { name: /Port-Louis — scènes/ }))
    const marked = within(list)
      .getAllByRole('button')
      .filter((b) => b.getAttribute('aria-current') === 'true')
      .map((b) => b.getAttribute('data-scene'))
    expect(marked).toEqual(['sc_a', 'sc_b'])
    expect(lit()).toEqual(['l_ville>l_phare', 'l_phare>l_ville', 'l_ville>l_ile'])

    // One act at a time.
    await userEvent.click(within(list).getByRole('button', { name: 'Le large' }))
    expect(lines()).toEqual([])
    expect(within(list).getAllByRole('listitem')).toHaveLength(1)
    // Into the town with act 2 chosen: the town has act 1 only, so its
    // whole thread shows, drawn over the grid map too.
    fireEvent.click(container.querySelector('[data-hex="1,0"]')!)
    await screen.findByRole('img', { name: 'Carte : Le quai' })
    expect(lines()).toEqual(['l_taverne>l_port'])
    const inTown = screen.getByRole('region', { name: 'Le fil de la campagne' })
    expect(within(inTown).getAllByRole('listitem')).toHaveLength(2)
    await userEvent.click(screen.getByRole('button', { name: 'Ressortir' }))
    await screen.findByRole('img', { name: 'Les côtes' })
    await userEvent.click(within(screen.getByRole('region', { name: 'Le fil de la campagne' })).getByRole('button', { name: 'Tous les actes' }))

    // An exit added in the scene editor: the revision redraws the thread.
    atlas = {
      ...atlas,
      revision: 4,
      maps: atlas.maps.map((m) =>
        m.map.id === 'region'
          ? { ...m, thread: { ...thread, segments: [...thread.segments, { from: isle, to: light, steps: [{ from: 'sc_d', to: 'sc_c', act: 'a2' }] }] } }
          : m,
      ),
    }
    FakeSocket.all[0].push({ type: 'ready', versions: { prep: 3 } })
    FakeSocket.all[0].push({ type: 'changed', topic: 'prep', version: 4 })
    await waitFor(() => expect(lines()).toContain('l_ile>l_phare'))
    expect(lines()).toHaveLength(4)
  })

  it('warns of two linked scenes no path joins, naming the scene', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    mockApi({
      'GET /api/campaigns/c1/atlas': () =>
        ok({
          ...ATLAS,
          castable: { ...ATLAS.castable, scenes: [{ id: 'sc_a', title: 'L’arrivée', number: 1 }] },
          issues: [{ severity: 'warning', code: 'EXIT_WITHOUT_PATH', path: 'nodes[0].exits[1]', detail: '' }],
        }),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
    })
    renderPage()
    const issues = await screen.findByRole('list', { name: 'Épingles et poses à revoir' })
    expect(issues).toHaveTextContent('1. L’arrivée mène à une scène qu’aucun chemin ne relie')
  })

  it('draws Claude’s batch dotted, keeps or discards its items with a word, and hears a new one arrive', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    FakeSocket.all = []
    const crypt = { location: 'l_crypte', name: 'La crypte', at: [2, 2], scenes: [{ id: 'sc_c', title: 'La crypte', number: 3 }] }
    const batch = {
      id: 'p1',
      author: 'token',
      prompt: 'Peupler le cimetière',
      reply: '',
      changes: [],
      stale: false,
      dropped: 0,
      status: 'pending',
      focus: { map: 'region', at: [2, 2] },
      items: [
        { key: 'e0', status: 'pending', stale: false, change: { op: 'set', kind: 'location', id: 'l_crypte', name: 'La crypte', field: 'anchor' } },
        { key: 'e1', status: 'pending', stale: false, change: { op: 'add', kind: 'clue', id: 'cl_os', name: 'Un os' } },
      ],
      createdAt: '2026-10-11T10:00:00Z',
    }
    let atlas: Record<string, unknown> = {
      ...ATLAS,
      claude: [batch],
      claudeHand: false,
      maps: ATLAS.maps.map((m) =>
        m.map.id === 'region'
          ? { ...m, proposed: [{ proposal: 'p1', pins: [crypt], cast: { npcs: [], clues: [], fights: [] }, thread: [], cells: [] }] }
          : m,
      ),
    }
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(atlas),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'POST /api/campaigns/c1/proposals/p1/decide': () => ok({ campaign: {}, proposal: batch }),
      'PUT /api/campaigns/c1/claude': () => ({ status: 409, body: { error: { code: 'EVENING_OPEN', message: '' } } }),
    })
    const { container } = renderPage()

    // The proposed pin is Claude's, dotted; a tap opens its batch.
    const pin = await screen.findByRole('button', { name: /▶ Claude · La crypte — scènes/ })
    expect(pin.getAttribute('data-proposed')).toBe('true')
    expect(container.querySelectorAll('[data-proposed]')).toHaveLength(1)
    fireEvent.click(pin)
    const panel = screen.getByRole('region', { name: 'Propositions de Claude' })
    expect(within(panel).getByRole('button', { name: '▶ Claude · Peupler le cimetière' })).toHaveAttribute('aria-pressed', 'true')

    // One item discarded with a word, then everything left kept.
    await userEvent.type(within(panel).getByRole('textbox', { name: 'Un mot pour Claude' }), 'trop évident')
    await userEvent.click(within(panel).getAllByRole('button', { name: 'Jeter' })[1])
    await userEvent.click(within(panel).getByRole('button', { name: 'Tout garder' }))
    await waitFor(() => expect(sentTo(fetchMock, 'POST /api/campaigns/c1/proposals/p1/decide')).toHaveLength(2))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/proposals/p1/decide')).toEqual([
      { discard: ['e1'], word: 'trop évident' },
      { all: 'keep' },
    ])

    // The hand cannot be given during an evening: said, not hidden.
    await userEvent.click(within(panel).getByRole('button', { name: 'Claude a la main' }))
    expect(await within(panel).findByRole('alert')).toHaveTextContent('Une soirée est ouverte')

    // A new batch arrives on the GM-only topic.
    atlas = { ...atlas, claude: [batch, { ...batch, id: 'p2', prompt: 'Deux indices sur la colline' }] }
    FakeSocket.all[0].push({ type: 'ready', versions: { proposals: 1 } })
    FakeSocket.all[0].push({ type: 'changed', topic: 'proposals', version: 2 })
    expect(await within(panel).findByRole('button', { name: '▶ Claude · Deux indices sur la colline' })).toBeInTheDocument()
  })

  it('shows the players’ view from the server alone: no scene numbers, no proposals, refreshed live', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    FakeSocket.all = []
    const town = { location: 'l_ville', name: 'Port-Louis', at: [1, 0], scenes: [{ id: 'sc_a', title: 'L’arrivée', number: 1 }] }
    const crypt = { location: 'l_crypte', name: 'La crypte', at: [2, 2], scenes: [{ id: 'sc_c', title: 'La crypte', number: 3 }] }
    const atlas = {
      ...ATLAS,
      claude: [],
      maps: ATLAS.maps.map((m) =>
        m.map.id === 'region'
          ? {
              ...m,
              pins: [town],
              proposed: [{ proposal: 'p1', pins: [crypt], cast: { npcs: [], clues: [], fights: [] }, thread: [], cells: [] }],
            }
          : m,
      ),
    }
    // What the server's projection sends: the visited pin by its name, a found clue by its text.
    let seen = {
      shown: false,
      board: {
        map: { id: 'region', name: 'Les côtes', scale: 'world', theme: 't', ambience: {}, grid: grid(4, 3) },
        fog: true,
        tokens: [{ id: 'groupe', kind: 'character', name: 'Le groupe', at: [0, 0] }],
        reachable: [],
        fight: null,
        pins: [{ name: 'Port-Louis', at: [1, 0], current: true }],
        clues: [],
      },
    }
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(atlas),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'GET /api/campaigns/c1/maps/region/players': () => ok(seen),
      'GET /api/campaigns/c1/player-view': () =>
        ok({ title: 'Les Corsaires', world: 'Bretagne', playerHook: 'Une lettre de marque.', scene: null, clues: ['Un sceau brisé'], npcs: [] }),
    })
    const { container } = renderPage()

    // The GM's view: the pin with its scene numbers, Claude's dotted one.
    await screen.findByRole('button', { name: /Port-Louis — scènes : 1\. L’arrivée/ })
    expect(container.querySelectorAll('[data-proposed]')).toHaveLength(1)

    await userEvent.click(screen.getByRole('button', { name: 'Vue joueurs' }))
    await screen.findByText(/recevraient la télé et les téléphones/)
    expect(sentTo(fetchMock, 'GET /api/campaigns/c1/maps/region/players')).toHaveLength(1)
    // Only what the route returned: the pin by its name, no number, nothing of Claude's.
    expect(container.querySelectorAll('[data-proposed]')).toHaveLength(0)
    expect(screen.queryByText(/L’arrivée/)).toBeNull()
    expect(screen.queryByRole('button', { name: /scènes/ })).toBeNull()
    expect(screen.queryByRole('group', { name: 'Calques de l’histoire' })).toBeNull()
    // Beside it, what the players' screens say.
    expect(await screen.findByText('Une lettre de marque.')).toBeInTheDocument()
    expect(screen.getByText('Un sceau brisé')).toBeInTheDocument()

    // The table moves: the view follows.
    seen = { ...seen, shown: true }
    FakeSocket.all[0].push({ type: 'ready', versions: { map: 1 } })
    FakeSocket.all[0].push({ type: 'changed', topic: 'map', version: 2 })
    expect(await screen.findByText(/Sur la table en ce moment/)).toBeInTheDocument()

    // Back to the GM's view.
    await userEvent.click(screen.getByRole('button', { name: 'Vue MJ' }))
    expect(await screen.findByRole('button', { name: /Port-Louis — scènes : 1\. L’arrivée/ })).toBeInTheDocument()
  })

  it('keeps the players’ view live next to the table, and catches up after a cut', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    FakeSocket.all = []
    const last = () => FakeSocket.all[FakeSocket.all.length - 1]
    let clues: string[] = []
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(ATLAS),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'GET /api/campaigns/c1/maps/region/players': () =>
        ok({
          shown: true,
          board: { map: ATLAS.maps[0].map, fog: true, tokens: [], reachable: [], fight: null, pins: [], clues: [] },
        }),
      'GET /api/campaigns/c1/player-view': () =>
        ok({ title: 'Le Phare de Kerbrume', world: 'Côte bretonne, 1718', playerHook: '', scene: null, clues, npcs: [] }),
    })
    render(
      <MemoryRouter initialEntries={['/campagnes/c1/atlas?vue=joueurs']}>
        <Routes>
          <Route path="/campagnes/:campaignId/atlas" element={<AtlasPage />} />
        </Routes>
      </MemoryRouter>,
    )

    expect(await screen.findByRole('heading', { name: 'Le Phare de Kerbrume' })).toBeInTheDocument()
    expect(screen.getByText("Aucun indice trouvé pour l'instant.")).toBeInTheDocument()
    expect(screen.getByText('Connexion…')).toBeInTheDocument()
    last().push({ type: 'ready', versions: {} })
    expect(screen.getByText('En direct')).toBeInTheDocument()
    last().push({ type: 'presence', gmOnline: true, players: ['p1', 'p2'] })
    expect(screen.getByText('2 joueurs connectés')).toBeInTheDocument()

    // The GM reveals a clue: both the map and the text are read again.
    clues = ['Une lanterne brisée']
    last().push({ type: 'changed', topic: 'world', version: 1 })
    expect(await screen.findByText('Une lanterne brisée')).toBeInTheDocument()
    expect(sentTo(fetchMock, 'GET /api/campaigns/c1/maps/region/players').length).toBeGreaterThan(1)

    // The network drops while two more clues are found.
    last().drop()
    expect(screen.getByText('Reconnexion…')).toBeInTheDocument()
    clues = ['Une lanterne brisée', 'Un registre raturé', 'Des pas dans le sable']
    await act(() => new Promise((r) => setTimeout(r, 600)))
    last().push({ type: 'ready', versions: { world: 3 } })
    expect(screen.getByText('En direct')).toBeInTheDocument()
    expect(await screen.findByText('Des pas dans le sable')).toBeInTheDocument()
    expect(screen.getByText('Un registre raturé')).toBeInTheDocument()
  })

  it('shows the scene in HD-2D, what it lacks, and keeps a PixelLab image with one tap (campaign/watch-prep-in-hd2d)', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    FakeSocket.all = []
    let needs = {
      tileset: 't',
      palette: [],
      generations: 1,
      needs: [
        { kind: 'element', subject: 't/material/herbe', name: 'Sol · Herbe', form: 'texture', status: 'missing', asset: null, generations: 1 },
        { kind: 'element', subject: 't/wall', name: 'Murs · Le port', form: 'texture', status: 'pending', asset: 'a-mur', generations: 0 },
      ],
    }
    const api = mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(ATLAS),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'GET /api/campaigns/c1/elements?map=quai': () => ok(needs),
      'POST /api/campaigns/c1/media/a-mur/decision': () => {
        needs = { ...needs, needs: [needs.needs[0]!, { ...needs.needs[1]!, status: 'approved' }] }
        return { status: 204 }
      },
    })
    render(
      <MemoryRouter initialEntries={['/campagnes/c1/atlas?carte=quai&rendu=hd2d']}>
        <Routes>
          <Route path="/campagnes/:campaignId/atlas" element={<AtlasPage />} />
        </Routes>
      </MemoryRouter>,
    )
    // No WebGL here: the scene says so, the side panel still works.
    await screen.findByText(/ne sait pas dessiner la scène en 3D/)
    expect(screen.getByRole('button', { name: 'HD-2D' })).toHaveAttribute('aria-pressed', 'true')
    const missing = await screen.findByRole('list', { name: 'Ce qui manque encore' })
    expect(within(missing).getByText('Sol · Herbe')).toBeInTheDocument()
    expect(screen.getByText(/1 génération/)).toBeInTheDocument()

    expect(screen.getByText('Murs · Le port')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Garder' }))
    expect(sentTo(api, 'POST /api/campaigns/c1/media/a-mur/decision')).toEqual([{ approve: true }])
    await waitFor(() => expect(screen.queryByText('Murs · Le port')).toBeNull())

    // Back to the plan: the flat map again.
    await userEvent.click(screen.getByRole('button', { name: 'Plan' }))
    await screen.findByRole('img', { name: 'Carte : Le quai' })
  })
})
