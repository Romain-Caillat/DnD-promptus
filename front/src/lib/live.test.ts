import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  LiveConnection,
  PING_EVERY_MS,
  PROBE_MS,
  SILENCE_MS,
  liveUrl,
  type LiveStatus,
  type SocketLike,
} from './live'

/** A socket the test plays the server of. */
class FakeSocket implements SocketLike {
  onopen: ((ev: Event) => void) | null = null
  onmessage: ((ev: MessageEvent) => void) | null = null
  onclose: ((ev: CloseEvent) => void) | null = null
  onerror: ((ev: Event) => void) | null = null
  sent: unknown[] = []
  closed = false

  send(data: string) {
    this.sent.push(JSON.parse(data))
  }

  close() {
    this.closed = true
  }

  /** The server sends `msg`. */
  push(msg: unknown) {
    this.onmessage?.(new MessageEvent('message', { data: JSON.stringify(msg) }))
  }

  /** The connection drops. */
  drop() {
    this.onclose?.(new CloseEvent('close'))
  }
}

function setup() {
  const sockets: FakeSocket[] = []
  const changes: string[][] = []
  const statuses: LiveStatus[] = []
  const connection = new LiveConnection({
    url: 'ws://test/api/campaigns/c1/live',
    onChange: (topics) => changes.push(topics),
    onPresence: () => {},
    onStatus: (s) => statuses.push(s),
    createSocket: () => {
      const s = new FakeSocket()
      sockets.push(s)
      return s
    },
  })
  connection.start()
  const last = () => sockets[sockets.length - 1]
  return { connection, sockets, changes, statuses, last }
}

describe('LiveConnection', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('reconnects with a doubling, capped backoff, reset once live again', () => {
    const { sockets, statuses, last } = setup()
    last().push({ type: 'ready', versions: {} })
    expect(statuses.at(-1)).toBe('live')

    // Each failed attempt waits twice as long, up to the cap.
    const waits: number[] = []
    for (let i = 0; i < 6; i++) {
      const before = sockets.length
      last().drop()
      let waited = 0
      while (sockets.length === before) {
        vi.advanceTimersByTime(100)
        waited += 100
      }
      waits.push(waited)
    }
    expect(waits).toEqual([500, 1000, 2000, 4000, 8000, 8000])
    expect(statuses.at(-1)).toBe('reconnecting')

    // Back: the next loss starts from the shortest wait again.
    last().push({ type: 'ready', versions: {} })
    expect(statuses.at(-1)).toBe('live')
    const before = sockets.length
    last().drop()
    vi.advanceTimersByTime(499)
    expect(sockets.length).toBe(before)
    vi.advanceTimersByTime(1)
    expect(sockets.length).toBe(before + 1)
  })

  it('refetches exactly what moved while it was cut off', () => {
    const { changes, last } = setup()
    last().push({ type: 'ready', versions: { world: 3 } })
    expect(changes).toEqual([['world']])

    last().push({ type: 'changed', topic: 'world', version: 4 })
    // A late or repeated signal changes nothing.
    last().push({ type: 'changed', topic: 'world', version: 4 })
    last().push({ type: 'changed', topic: 'world', version: 2 })
    expect(changes).toEqual([['world'], ['world']])

    // Cut off; meanwhile the world moved and a character changed.
    last().drop()
    vi.advanceTimersByTime(500)
    last().push({
      type: 'ready',
      versions: { world: 7, 'character:marc': 1, story: 1 },
    })
    expect(changes.at(-1)).toEqual(['world', 'character:marc', 'story'])

    // Nothing moved: no refetch.
    last().drop()
    vi.advanceTimersByTime(500)
    last().push({ type: 'ready', versions: { world: 7, 'character:marc': 1, story: 1 } })
    expect(changes).toHaveLength(3)

    // A resync is compared the same way.
    last().push({ type: 'resync', versions: { world: 8, 'character:marc': 1, story: 1 } })
    expect(changes.at(-1)).toEqual(['world'])
  })

  it('pings, and gives up on a socket that stays silent', () => {
    const { sockets, last } = setup()
    last().push({ type: 'ready', versions: {} })
    vi.advanceTimersByTime(PING_EVERY_MS)
    expect(last().sent).toEqual([{ type: 'ping' }])
    last().push({ type: 'pong' })

    // The network vanished: the socket stays "open" and says nothing.
    const silent = last()
    vi.advanceTimersByTime(SILENCE_MS + PING_EVERY_MS)
    expect(silent.closed).toBe(true)
    vi.advanceTimersByTime(500)
    expect(sockets.length).toBe(2)
  })

  it('does not wait for the backoff once the device is back online', () => {
    const { sockets, last } = setup()
    last().push({ type: 'ready', versions: {} })
    for (let i = 0; i < 5; i++) {
      last().drop()
      vi.advanceTimersByTime(10_000)
    }
    // Now in the 8 s wait.
    last().drop()
    const before = sockets.length
    window.dispatchEvent(new Event('online'))
    expect(sockets.length).toBe(before + 1)
  })

  it('probes a socket that looks open when the device comes back', () => {
    const { sockets, last } = setup()
    last().push({ type: 'ready', versions: {} })
    const stale = last()
    window.dispatchEvent(new Event('online'))
    expect(stale.sent).toEqual([{ type: 'ping' }])
    vi.advanceTimersByTime(PROBE_MS)
    expect(stale.closed).toBe(true)
    expect(sockets.length).toBe(2)

    // A socket that answers is kept.
    last().push({ type: 'ready', versions: {} })
    const alive = last()
    window.dispatchEvent(new Event('online'))
    alive.push({ type: 'pong' })
    vi.advanceTimersByTime(PROBE_MS)
    expect(alive.closed).toBe(false)
    expect(sockets.length).toBe(2)
  })

  it('stops for good', () => {
    const { connection, sockets, last } = setup()
    const socket = last()
    connection.stop()
    expect(socket.closed).toBe(true)
    window.dispatchEvent(new Event('online'))
    vi.advanceTimersByTime(60_000)
    expect(sockets.length).toBe(1)
  })
})

describe('liveUrl', () => {
  it('follows the page origin, secure when the page is', () => {
    expect(liveUrl('c1', { protocol: 'https:', host: 'promptus.example' } as Location)).toBe(
      'wss://promptus.example/api/campaigns/c1/live',
    )
    expect(liveUrl('c1', { protocol: 'http:', host: 'localhost:4334' } as Location)).toBe(
      'ws://localhost:4334/api/campaigns/c1/live',
    )
  })
})
