import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mockApi } from '@/test-utils'
import { TvPairPage } from './TvPairPage'

function renderAt(path: string) {
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/tv" element={<TvPairPage />} />
        <Route path="/tv/:campaignId" element={<p>table rejointe</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('TvPairPage', () => {
  beforeEach(() => localStorage.clear())
  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
    localStorage.clear()
  })

  it('shows its code and QR, then joins the table the GM typed it for', async () => {
    let paired = false
    mockApi({
      'POST /api/tv/pairings': () => ({ status: 201, body: { data: { code: 'K7QF', secret: 'sec' } } }),
      'GET /api/tv/pairings/sec': () => ({
        status: 200,
        body: { data: paired ? { state: 'paired', campaign: 'c1' } : { state: 'waiting', code: 'K7QF' } },
      }),
    })
    renderAt('/tv')
    expect(await screen.findByLabelText('Code K7QF')).toHaveTextContent('K7QF')
    expect(screen.getByRole('img', { name: 'QR à scanner avec le téléphone du MJ' })).toHaveAttribute(
      'src',
      '/api/tv/pairings/sec/qr.svg',
    )
    paired = true
    expect(await screen.findByText('table rejointe', {}, { timeout: 4000 })).toBeInTheDocument()
    expect(localStorage.getItem('promptus.tv.campaign')).toBe('c1')
  })

  it('goes back to the table it joined', async () => {
    localStorage.setItem('promptus.tv.campaign', 'c1')
    mockApi({})
    renderAt('/tv')
    expect(await screen.findByText('table rejointe')).toBeInTheDocument()
  })

  it('opens the window the GM shares, already paired, with no code', async () => {
    const fetchMock = mockApi({
      'GET /api/tv/pairings/win': () => ({ status: 200, body: { data: { state: 'paired', campaign: 'c1' } } }),
    })
    renderAt('/tv?cle=win')
    expect(await screen.findByText('table rejointe')).toBeInTheDocument()
    expect(fetchMock.mock.calls.some(([u]) => String(u) === '/api/tv/pairings')).toBe(false)
  })

  it('draws a fresh code when the old one expired', async () => {
    let n = 0
    mockApi({
      'POST /api/tv/pairings': () => {
        n += 1
        return { status: 201, body: { data: { code: n === 1 ? 'AAAA' : 'BBBB', secret: `s${n}` } } }
      },
      'GET /api/tv/pairings/s1': () => ({ status: 404, body: { error: { code: 'TV_PAIRING_UNKNOWN' } } }),
      'GET /api/tv/pairings/s2': () => ({ status: 200, body: { data: { state: 'waiting', code: 'BBBB' } } }),
    })
    renderAt('/tv')
    expect(await screen.findByLabelText('Code BBBB')).toBeInTheDocument()
  })
})
