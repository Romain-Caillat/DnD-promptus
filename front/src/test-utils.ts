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
 * A device that does, or does not, ask for reduced motion; `matching`
 * lists the other media queries it answers (a computer, a tablet).
 */
export function stubReducedMotion(reduce: boolean, matching: string[] = []) {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: (reduce && query === '(prefers-reduced-motion: reduce)') || matching.includes(query),
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }))
}
