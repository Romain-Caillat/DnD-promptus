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

function readPath(value: unknown, ...keys: string[]): unknown {
  let current = value
  for (const key of keys) {
    if (typeof current !== 'object' || current === null) return undefined
    current = (current as Record<string, unknown>)[key]
  }
  return current
}
