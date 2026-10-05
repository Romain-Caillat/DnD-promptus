/**
 * The live channel of a campaign (`session/stream-live-changes`).
 *
 * The socket carries invalidations, never data: "the world changed, it
 * is at version 8". Whoever holds the world refetches it through the
 * API. Every (re)connection starts with the current version of each
 * topic, so a client that was cut off refetches exactly what moved while
 * it was away — the same on a `resync`, sent when the server lost
 * signals for it.
 *
 * Kept free of React so the reconnection logic is tested on its own
 * with a fake socket and fake timers.
 */

/** Who is connected to the campaign. */
export interface Presence {
  gmOnline: boolean
  /** Player ids, each once. */
  players: string[]
}

export type LiveStatus = 'connecting' | 'live' | 'reconnecting'

type ServerMessage =
  | { type: 'ready' | 'resync'; versions: Record<string, number> }
  | { type: 'changed'; topic: string; version: number }
  | ({ type: 'presence' } & Presence)
  | { type: 'pong' }

/** The part of `WebSocket` this file uses, so tests can pass a fake. */
export interface SocketLike {
  onopen: ((ev: Event) => void) | null
  onmessage: ((ev: MessageEvent) => void) | null
  onclose: ((ev: CloseEvent) => void) | null
  onerror: ((ev: Event) => void) | null
  send(data: string): void
  close(): void
}

export interface LiveOptions {
  url: string
  /** Topics that moved since this client last knew them: refetch them. */
  onChange: (topics: string[]) => void
  onPresence: (presence: Presence) => void
  onStatus: (status: LiveStatus) => void
  createSocket?: (url: string) => SocketLike
}

/** First retry delay; it doubles on each failure up to `RETRY_MAX_MS`. */
const RETRY_BASE_MS = 500
const RETRY_MAX_MS = 8_000
/** A client ping, since browsers hide WebSocket ping frames. */
export const PING_EVERY_MS = 10_000
/**
 * Silence after which the socket is presumed dead: a phone that lost its
 * network keeps a socket that looks open and never closes on its own.
 */
export const SILENCE_MS = 25_000
/** How long a socket has to answer a ping after the device came back. */
export const PROBE_MS = 3_000

/** The delay before retry number `attempt` (0 for the first). */
function retryDelay(attempt: number): number {
  return Math.min(RETRY_BASE_MS * 2 ** attempt, RETRY_MAX_MS)
}

/** `ws(s)://<this origin>/api/campaigns/<id>/live`. */
export function liveUrl(campaignId: string, location: Location = window.location): string {
  const scheme = location.protocol === 'https:' ? 'wss' : 'ws'
  return `${scheme}://${location.host}/api/campaigns/${encodeURIComponent(campaignId)}/live`
}

/**
 * One campaign's live connection: connects, pings, reconnects with an
 * exponential backoff (at once when the device says it is back online or
 * the page becomes visible again), and reports what changed.
 */
export class LiveConnection {
  private readonly options: LiveOptions
  private readonly known = new Map<string, number>()
  private socket: SocketLike | null = null
  private attempt = 0
  private stopped = false
  private everLive = false
  private retryTimer: ReturnType<typeof setTimeout> | undefined
  private pingTimer: ReturnType<typeof setInterval> | undefined
  private lastHeard = 0
  /** Messages received, so a probe knows whether an answer came. */
  private heard = 0
  private probeTimer: ReturnType<typeof setTimeout> | undefined

  constructor(options: LiveOptions) {
    this.options = options
  }

  start(): void {
    this.stopped = false
    window.addEventListener('online', this.reconnectNow)
    document.addEventListener('visibilitychange', this.onVisibility)
    this.options.onStatus('connecting')
    this.open()
  }

  stop(): void {
    this.stopped = true
    window.removeEventListener('online', this.reconnectNow)
    document.removeEventListener('visibilitychange', this.onVisibility)
    clearTimeout(this.retryTimer)
    clearTimeout(this.probeTimer)
    this.drop()
  }

  private open(): void {
    const create = this.options.createSocket ?? ((url: string) => new WebSocket(url))
    const socket = create(this.options.url)
    this.socket = socket
    this.lastHeard = Date.now()
    socket.onmessage = (ev) => {
      if (this.socket !== socket) return
      this.lastHeard = Date.now()
      this.heard += 1
      this.receive(String(ev.data))
    }
    socket.onclose = () => {
      if (this.socket === socket) this.lost()
    }
    socket.onerror = () => {
      if (this.socket === socket) this.lost()
    }
    this.pingTimer = setInterval(() => {
      if (Date.now() - this.lastHeard > SILENCE_MS) {
        this.lost()
        return
      }
      try {
        socket.send(JSON.stringify({ type: 'ping' }))
      } catch {
        this.lost()
      }
    }, PING_EVERY_MS)
  }

  private receive(raw: string): void {
    let msg: ServerMessage
    try {
      msg = JSON.parse(raw) as ServerMessage
    } catch {
      return
    }
    switch (msg.type) {
      case 'ready':
      case 'resync': {
        if (msg.type === 'ready') {
          this.attempt = 0
          this.everLive = true
          this.options.onStatus('live')
        }
        const moved = Object.entries(msg.versions)
          .filter(([topic, version]) => this.known.get(topic) !== version)
          .map(([topic, version]) => {
            this.known.set(topic, version)
            return topic
          })
        if (moved.length > 0) this.options.onChange(moved)
        break
      }
      case 'changed':
        if (msg.version > (this.known.get(msg.topic) ?? 0)) {
          this.known.set(msg.topic, msg.version)
          this.options.onChange([msg.topic])
        }
        break
      case 'presence':
        this.options.onPresence({ gmOnline: msg.gmOnline, players: msg.players })
        break
      default:
        break
    }
  }

  /** Close the current socket, if any, without scheduling anything. */
  private drop(): void {
    clearInterval(this.pingTimer)
    const socket = this.socket
    this.socket = null
    if (socket) {
      socket.onopen = socket.onmessage = socket.onclose = socket.onerror = null
      try {
        socket.close()
      } catch {
        // Already closed.
      }
    }
  }

  /** The socket died: retry after the backoff. */
  private lost(): void {
    this.drop()
    if (this.stopped) return
    this.options.onStatus(this.everLive ? 'reconnecting' : 'connecting')
    clearTimeout(this.retryTimer)
    this.retryTimer = setTimeout(() => this.open(), retryDelay(this.attempt))
    this.attempt += 1
  }

  /**
   * The device is back online, or the page visible again: without a
   * socket, reconnect at once instead of waiting for the backoff; with
   * one, check it still answers — after a network change it may be dead
   * while looking open.
   */
  private readonly reconnectNow = (): void => {
    if (this.stopped) return
    if (this.socket === null) {
      clearTimeout(this.retryTimer)
      this.open()
      return
    }
    const socket = this.socket
    const asked = this.heard
    try {
      socket.send(JSON.stringify({ type: 'ping' }))
    } catch {
      this.lost()
      return
    }
    clearTimeout(this.probeTimer)
    this.probeTimer = setTimeout(() => {
      if (this.socket === socket && this.heard === asked) {
        this.drop()
        this.options.onStatus('reconnecting')
        this.open()
      }
    }, PROBE_MS)
  }

  private readonly onVisibility = (): void => {
    if (document.visibilityState === 'visible') this.reconnectNow()
  }
}
