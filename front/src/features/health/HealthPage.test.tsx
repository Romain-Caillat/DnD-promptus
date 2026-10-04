import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { HealthPage } from './HealthPage'

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

describe('HealthPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('tells the database is down, then recovers on retry', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        jsonResponse(503, { error: { code: 'DATABASE_UNAVAILABLE', message: 'x' } }),
      )
      .mockResolvedValueOnce(jsonResponse(200, { data: { status: 'ok' } }))
    vi.stubGlobal('fetch', fetchMock)

    render(<HealthPage />)

    expect(
      await screen.findByText('Le serveur répond, mais sa base de données est injoignable.'),
    ).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Réessayer' }))

    expect(
      await screen.findByText('Le serveur et sa base de données répondent.'),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Réessayer' })).not.toBeInTheDocument()
  })

  it('tells the server is unreachable when the request fails', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')))

    render(<HealthPage />)

    expect(await screen.findByText('Le serveur est injoignable.')).toBeInTheDocument()
  })
})
