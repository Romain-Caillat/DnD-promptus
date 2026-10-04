import { vi } from 'vitest'

type Handler = (body: unknown) => { status: number; body?: unknown }

/**
 * Stub `fetch` with one handler per `"METHOD /api/path"`. An unexpected
 * call fails the test with its route, instead of hanging on a promise.
 * Returns the mock, whose calls record every request.
 */
export function mockApi(routes: Record<string, Handler>) {
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const method = (init?.method ?? 'GET').toUpperCase()
    const key = `${method} ${String(input)}`
    const handler = routes[key]
    if (!handler) throw new Error(`unexpected API call: ${key}`)
    const sent = typeof init?.body === 'string' ? JSON.parse(init.body) : undefined
    const { status, body } = handler(sent)
    return new Response(status === 204 ? null : JSON.stringify(body ?? null), {
      status,
      headers: { 'Content-Type': 'application/json' },
    })
  })
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

/** The JSON bodies sent to `"METHOD /api/path"`. */
export function sentTo(fetchMock: ReturnType<typeof mockApi>, key: string): unknown[] {
  return fetchMock.mock.calls
    .filter(([input, init]) => `${(init?.method ?? 'GET').toUpperCase()} ${String(input)}` === key)
    .map(([, init]) => (typeof init?.body === 'string' ? JSON.parse(init.body) : undefined))
}

/**
 * A browser that can use passkeys: a secure context, the WebAuthn API,
 * and `navigator.credentials` answering with the given functions.
 */
export function stubPasskeys(credentials: { create?: unknown; get?: unknown }) {
  vi.stubGlobal('isSecureContext', true)
  vi.stubGlobal('PublicKeyCredential', function PublicKeyCredential() {})
  vi.stubGlobal('navigator', { ...navigator, credentials })
}

export const bytes = (...b: number[]) => new Uint8Array(b).buffer
