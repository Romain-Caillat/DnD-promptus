import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { MapEditorPage } from './MapEditorPage'

const ROOM = {
  id: 'salle',
  name: 'Salle',
  theme: 'port-1718',
  ambience: {},
  grid: {
    legend: {
      '#': { terrain: 'pierre', wall: true },
      '.': { terrain: 'pierre' },
    },
    rows: ['#####', '#...#', '#####'],
  },
}

const WALLED = { ...ROOM, grid: { ...ROOM.grid, rows: ['#####', '#.###', '#####'] } }

const shown = (map: unknown, revision: number, validatedAt: string | null = null) => ({
  map,
  source: 'editor',
  node: null,
  backdrop: false,
  validatedAt,
  updatedAt: '',
  revision,
})

const ok = (data: unknown) => ({ status: 200, body: { data } })

const cell = (x: number, y: number) => ({
  clientX: x * 32 + 5,
  clientY: y * 32 + 5,
})

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
  render(
    <MemoryRouter initialEntries={['/campagnes/c1/cartes/salle']}>
      <Routes>
        <Route path="/campagnes/:campaignId/cartes/:mapId" element={<MapEditorPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

const MEDIA = () => ok({ theme: null, assets: [] })

describe('MapEditorPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('sends each gesture as operations on the revision read, then validates the map', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    const mock = mockApi({
      'GET /api/campaigns/c1/maps/salle': () => ok(shown(ROOM, 3)),
      'GET /api/campaigns/c1/media': MEDIA,
      'POST /api/campaigns/c1/maps/salle/ops': (body) => {
        const { revision } = body as { revision: number }
        return ok(shown(WALLED, revision + 1))
      },
      'POST /api/campaigns/c1/maps/salle/validate': () => ok(shown(WALLED, 5, '2026-10-06T10:00:00Z')),
      'POST /api/campaigns/c1/revisions/5/undo': () => ok({}),
    })
    renderPage()

    const canvas = await screen.findByRole('img', { name: 'Carte : Salle' })
    // The wall brush is chosen first.
    fireEvent.pointerDown(canvas, cell(2, 1))
    fireEvent.pointerUp(canvas, cell(2, 1))
    await waitFor(() => expect(sentTo(mock, 'POST /api/campaigns/c1/maps/salle/ops')).toHaveLength(1))
    await userEvent.click(screen.getByRole('radio', { name: 'Porte' }))
    fireEvent.pointerUp(canvas, cell(1, 1))
    await waitFor(() => expect(sentTo(mock, 'POST /api/campaigns/c1/maps/salle/ops')).toHaveLength(2))

    expect(sentTo(mock, 'POST /api/campaigns/c1/maps/salle/ops')).toEqual([
      {
        revision: 3,
        ops: [{ op: 'paint', cells: [[2, 1]], kind: { terrain: 'pierre', wall: true } }],
      },
      { revision: 4, ops: [{ op: 'door', at: [1, 1], layer: 'base' }] },
    ])

    // The GM takes back their last gesture through the journal.
    await userEvent.click(screen.getByRole('button', { name: 'Annuler mon dernier geste' }))
    await waitFor(() => expect(sentTo(mock, 'POST /api/campaigns/c1/revisions/5/undo')).toHaveLength(1))

    await userEvent.click(await screen.findByRole('button', { name: 'Valider' }))
    expect(sentTo(mock, 'POST /api/campaigns/c1/maps/salle/validate')).toHaveLength(1)
    expect(await screen.findByText('Validée')).toBeInTheDocument()
  })

  it('reloads the fresh map and replays a gesture refused as stale, and hears Claude live', async () => {
    stubReducedMotion(true)
    vi.stubGlobal('WebSocket', FakeSocket)
    FakeSocket.all = []
    const CLAUDE = { ...ROOM, starts: [{ id: 'depart-1', side: 'neutral', at: [3, 1], entity: 'pnj_gwen' }] }
    let latest = shown(ROOM, 3)
    const mock = mockApi({
      'GET /api/campaigns/c1/maps/salle': () => ok(latest),
      'GET /api/campaigns/c1/media': MEDIA,
      'POST /api/campaigns/c1/maps/salle/ops': (body) => {
        const { revision } = body as { revision: number }
        if (revision < latest.revision) {
          return {
            status: 409,
            body: { error: { code: 'MAP_STALE', message: '', fresh: latest } },
          }
        }
        latest = shown({ ...CLAUDE, grid: WALLED.grid }, revision + 1)
        return ok(latest)
      },
    })
    renderPage()
    const canvas = await screen.findByRole('img', { name: 'Carte : Salle' })

    // Claude places an NPC while the editor is open: the GM hears it.
    latest = shown(CLAUDE, 4)
    FakeSocket.all[0].push({ type: 'ready', versions: { prep: 3 } })
    FakeSocket.all[0].push({ type: 'changed', topic: 'prep', version: 4 })
    const reads = () => mock.mock.calls.filter(([u]) => String(u).endsWith('/maps/salle')).length
    await waitFor(() => expect(reads()).toBeGreaterThanOrEqual(2))

    // Claude writes again; the GM's wall goes out on the old revision.
    latest = shown(CLAUDE, 6)
    fireEvent.pointerDown(canvas, cell(2, 1))
    fireEvent.pointerUp(canvas, cell(2, 1))
    expect(await screen.findByText(/votre geste rejoué/)).toBeInTheDocument()
    const sent = sentTo(mock, 'POST /api/campaigns/c1/maps/salle/ops') as { revision: number }[]
    expect(sent.map((b) => b.revision)).toEqual([4, 6])
    expect(latest.revision).toBe(7)
  })
})
