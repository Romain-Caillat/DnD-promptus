const API_BASE = '/api'

/**
 * What the player or GM needs to know about the server, not the raw HTTP
 * status: the UI maps each value to its own translated sentence.
 */
export type HealthStatus = 'ok' | 'database-down' | 'unreachable'

/**
 * `GET /api/health`. Never throws: a network failure, a proxy error
 * while the server restarts, or an unexpected body all read as
 * `unreachable`; only the server's own `DATABASE_UNAVAILABLE` 503 reads
 * as `database-down`.
 */
export async function fetchHealth(signal?: AbortSignal): Promise<HealthStatus> {
  let response: Response
  try {
    response = await fetch(`${API_BASE}/health`, { signal })
  } catch (e) {
    if (signal?.aborted) throw e
    return 'unreachable'
  }
  const body: unknown = await response.json().catch(() => null)
  if (response.ok && readPath(body, 'data', 'status') === 'ok') return 'ok'
  if (response.status === 503 && readPath(body, 'error', 'code') === 'DATABASE_UNAVAILABLE') {
    return 'database-down'
  }
  return 'unreachable'
}

/**
 * A refused API call. `code` is the server's machine-readable code
 * (`{ error: { code } }`), `UNREACHABLE` when no answer came back, and
 * `UNEXPECTED` when the answer was not the API's envelope.
 */
export class ApiError extends Error {
  readonly status: number
  readonly code: string
  /** The server's own words, for the codes whose screen shows them (a map's `MAP_INVALID` says, in French, what is wrong). */
  readonly detail: string | null

  constructor(status: number, code: string, detail: string | null = null) {
    super(`API error ${status} ${code}`)
    this.name = 'ApiError'
    this.status = status
    this.code = code
    this.detail = detail
  }
}

/**
 * Call the API and return the `data` of its envelope (`undefined` for a
 * 204). Throws `ApiError` on anything else. The GM session travels in
 * its HttpOnly cookie, which `fetch` sends on same-origin calls.
 */
export async function apiRequest<T>(method: string, path: string, body?: unknown): Promise<T> {
  let response: Response
  try {
    response = await fetch(`${API_BASE}${path}`, {
      method,
      credentials: 'same-origin',
      headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    })
  } catch {
    throw new ApiError(0, 'UNREACHABLE')
  }
  if (response.status === 204) return undefined as T
  const json: unknown = await response.json().catch(() => null)
  if (!response.ok) {
    const code = readPath(json, 'error', 'code')
    const detail = readPath(json, 'error', 'message')
    throw new ApiError(
      response.status,
      typeof code === 'string' ? code : 'UNEXPECTED',
      typeof detail === 'string' ? detail : null,
    )
  }
  if (typeof json !== 'object' || json === null || !('data' in json)) {
    throw new ApiError(response.status, 'UNEXPECTED')
  }
  return (json as { data: T }).data
}

function readPath(value: unknown, ...keys: string[]): unknown {
  let current = value
  for (const key of keys) {
    if (typeof current !== 'object' || current === null) return undefined
    current = (current as Record<string, unknown>)[key]
  }
  return current
}
