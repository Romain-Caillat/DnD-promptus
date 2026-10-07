import { act, render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ScreenView } from '@/lib/screens'
import { mockApi, stubReducedMotion } from '@/test-utils'
import { PAIR_POLL_MS, TvPage } from './TvPage'
import { TvStage } from './TvStage'

const VIEW: ScreenView = {
  title: 'Le Brasier',
  shows: { scene: true, map: true, party: true, moments: true },
  session: { number: 4, status: 'lobby', startedAt: null },
  previously: null,
  music: null,
  party: [
    { name: 'Kaël', nickname: 'Marc', look: null, here: true, hitPoints: 10, maxHitPoints: 10 },
    { name: 'Nyx', nickname: 'Camille', look: null, here: false, hitPoints: 8, maxHitPoints: 10 },
  ],
  scene: null,
  board: null,
  moments: [],
  rolls: [],
  tonight: [],
}
const MEDIA = { assets: [], theme: null }

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
}

const last = () => FakeSocket.all[FakeSocket.all.length - 1]

function renderTv() {
  render(
    <MemoryRouter>
      <TvPage />
    </MemoryRouter>,
  )
}

describe('TvPage', () => {
  beforeEach(() => {
    stubReducedMotion(true)
    FakeSocket.all = []
    vi.stubGlobal('WebSocket', FakeSocket)
    vi.useFakeTimers({ shouldAdvanceTime: true })
  })
  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  it('shows its code and QR, then follows the table once the GM typed it', async () => {
    let paired = false
    mockApi({
      'POST /api/tv': () => ({
        status: 200,
        body: { data: paired ? { paired: true, code: null, expiresAt: null } : { paired: false, code: 'K7QF', expiresAt: '' } },
      }),
      'GET /api/tv/show': () => ({ status: 200, body: { data: VIEW } }),
      'GET /api/tv/media': () => ({ status: 200, body: { data: MEDIA } }),
    })
    renderTv()

    expect(await screen.findByRole('img', { name: 'Code de la TV : K7QF' })).toBeInTheDocument()
    expect(screen.getByRole('img', { name: 'QR pour jumeler cette TV' })).toHaveAttribute(
      'src',
      expect.stringContaining('/api/tv/qr.svg?origin='),
    )

    // The GM types K7QF; the TV hears it at its next hello.
    paired = true
    await act(() => vi.advanceTimersByTimeAsync(PAIR_POLL_MS))
    expect(await screen.findByRole('heading', { name: 'Le Brasier' })).toBeInTheDocument()
    expect(screen.getByText('Kaël')).toBeInTheDocument()
    expect(screen.getByText('Nyx')).toBeInTheDocument()
    expect(screen.getByText('La partie commence bientôt')).toBeInTheDocument()
  })

  it('goes back to a code when the GM forgets it', async () => {
    let forgotten = false
    mockApi({
      'POST /api/tv': () => ({
        status: 200,
        body: { data: forgotten ? { paired: false, code: 'M2PX', expiresAt: '' } : { paired: true, code: null, expiresAt: null } },
      }),
      'GET /api/tv/show': () =>
        forgotten ? { status: 401, body: { error: { code: 'NOT_PAIRED' } } } : { status: 200, body: { data: VIEW } },
      'GET /api/tv/media': () => ({ status: 200, body: { data: MEDIA } }),
    })
    renderTv()
    expect(await screen.findByRole('heading', { name: 'Le Brasier' })).toBeInTheDocument()

    // The screen follows its own socket, on the screen route.
    expect(last().url).toMatch(/\/api\/tv\/live$/)
    forgotten = true
    // The live channel says the screens changed; the TV refetches.
    last().push({ type: 'ready', versions: { screens: 2 } })
    expect(await screen.findByRole('img', { name: 'Code de la TV : M2PX' })).toBeInTheDocument()
  })
})

describe('TvStage', () => {
  it('rolls the die everyone saw, against its difficulty, with its outcome', () => {
    stubReducedMotion(true)
    const roll = {
      id: 'r1',
      character: 'Kaël',
      ability: 'Dextérité',
      difficulty: 'Moyen',
      roll: {
        die: '1d20',
        faces: [16],
        natural: 16,
        advantage: 'normal' as const,
        modifiers: [],
        total: 18,
        target: null,
        band: 'success' as const,
      },
      outcome: 'Réussite',
      at: '',
    }
    render(
      <TvStage
        view={{ ...VIEW, session: { number: 4, status: 'live', startedAt: '' }, rolls: [roll] }}
        media={null}
        highlight={{ id: 'r1', at: 0, kind: 'roll', roll }}
      />,
    )
    expect(screen.getByText('Il faut Moyen')).toBeInTheDocument()
    expect(screen.getByText('16 + 2 = 18')).toBeInTheDocument()
    expect(screen.getByText('Réussite')).toBeInTheDocument()
    expect(screen.getByRole('img', { name: /16/ })).toBeInTheDocument()
  })
})
