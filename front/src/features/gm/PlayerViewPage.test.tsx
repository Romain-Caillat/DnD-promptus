import { act, render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi } from '@/test-utils'
import { PlayerViewPage } from './PlayerViewPage'

/** The browser's WebSocket, played by the test. */
class FakeSocket {
  static all: FakeSocket[] = []
  readonly url: string
  onopen: (() => void) | null = null
  onmessage: ((ev: { data: string }) => void) | null = null
  onclose: (() => void) | null = null
  onerror: (() => void) | null = null

  constructor(url: string) {
    this.url = url
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

const last = () => FakeSocket.all[FakeSocket.all.length - 1]

function view(clues: string[]) {
  return {
    status: 200,
    body: {
      data: {
        title: 'Le Phare de Kerbrume',
        world: 'Côte bretonne, 1718',
        playerHook: '',
        scene: null,
        clues,
        npcs: [],
      },
    },
  }
}

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1/vue-joueurs']}>
      <Routes>
        <Route path="/campagnes/:campaignId/vue-joueurs" element={<PlayerViewPage />} />
        <Route path="/connexion" element={<p>page de connexion</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('PlayerViewPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    FakeSocket.all = []
  })

  it('follows the table live and catches up after a cut', async () => {
    vi.stubGlobal('WebSocket', FakeSocket)
    let clues: string[] = []
    mockApi({ 'GET /api/campaigns/c1/player-view': () => view(clues) })
    renderPage()

    expect(await screen.findByRole('heading', { name: 'Le Phare de Kerbrume' })).toBeInTheDocument()
    expect(screen.getByText("Aucun indice trouvé pour l'instant.")).toBeInTheDocument()
    expect(last().url).toMatch(/\/api\/campaigns\/c1\/live$/)
    expect(screen.getByText('Connexion…')).toBeInTheDocument()

    last().push({ type: 'ready', versions: {} })
    expect(screen.getByText('En direct')).toBeInTheDocument()
    last().push({ type: 'presence', gmOnline: true, players: ['p1', 'p2'] })
    expect(screen.getByText('2 joueurs connectés')).toBeInTheDocument()

    // The GM reveals a clue: the page refetches the projection.
    clues = ['Une lanterne brisée']
    last().push({ type: 'changed', topic: 'world', version: 1 })
    expect(await screen.findByText('Une lanterne brisée')).toBeInTheDocument()

    // The phone loses its network while two more clues are found.
    last().drop()
    expect(screen.getByText('Reconnexion…')).toBeInTheDocument()
    clues = ['Une lanterne brisée', 'Un registre raturé', 'Des pas dans le sable']
    await act(() => new Promise((r) => setTimeout(r, 600)))
    expect(FakeSocket.all).toHaveLength(2)
    last().push({ type: 'ready', versions: { world: 3 } })
    expect(screen.getByText('En direct')).toBeInTheDocument()
    expect(await screen.findByText('Des pas dans le sable')).toBeInTheDocument()
    expect(screen.getByText('Un registre raturé')).toBeInTheDocument()
  })

  it('sends a signed-out visitor to the sign-in page', async () => {
    vi.stubGlobal('WebSocket', FakeSocket)
    mockApi({
      'GET /api/campaigns/c1/player-view': () => ({
        status: 401,
        body: { error: { code: 'UNAUTHENTICATED', message: 'x' } },
      }),
    })
    renderPage()
    expect(await screen.findByText('page de connexion')).toBeInTheDocument()
  })
})
