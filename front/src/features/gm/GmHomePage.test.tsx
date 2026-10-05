import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { GmHomePage } from './GmHomePage'

function renderHome() {
  render(
    <MemoryRouter initialEntries={['/']}>
      <Routes>
        <Route path="/" element={<GmHomePage />} />
        <Route path="/connexion" element={<p>page de connexion</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

const ME = { status: 200, body: { data: { id: 'g1', displayName: 'Romain' } } }
const INVITE = {
  id: 'i1',
  createdAt: '2026-10-04T10:00:00Z',
  expiresAt: '2026-10-11T10:00:00Z',
}

describe('GmHomePage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('sends a visitor without a session to the sign-in page', async () => {
    mockApi({
      'GET /api/me': () => ({
        status: 401,
        body: { error: { code: 'UNAUTHENTICATED', message: 'x' } },
      }),
    })
    renderHome()

    expect(await screen.findByText('page de connexion')).toBeInTheDocument()
  })

  it('mints an invitation link, then revokes the invitation', async () => {
    let pending: (typeof INVITE)[] = []
    const api = mockApi({
      'GET /api/me': () => ME,
      'GET /api/campaigns': () => ({ status: 200, body: { data: [] } }),
      'GET /api/gm-invites': () => ({ status: 200, body: { data: pending } }),
      'POST /api/gm-invites': () => {
        pending = [INVITE]
        return { status: 201, body: { data: { ...INVITE, code: 'secret code' } } }
      },
      'DELETE /api/gm-invites/i1': () => {
        pending = []
        return { status: 204 }
      },
    })
    renderHome()

    expect(await screen.findByRole('heading', { name: 'Bonjour, Romain' })).toBeInTheDocument()
    expect(await screen.findByText('Aucune invitation en attente.')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Créer une invitation' }))

    expect(await screen.findByLabelText("Lien d'invitation")).toHaveValue(
      `${window.location.origin}/inscription?code=secret%20code`,
    )
    const item = (await screen.findByText(/^Expire le/)).closest('li')!
    await userEvent.click(within(item).getByRole('button', { name: 'Révoquer' }))

    expect(await screen.findByText('Aucune invitation en attente.')).toBeInTheDocument()
    expect(sentTo(api, 'DELETE /api/gm-invites/i1')).toHaveLength(1)
  })

  it('signs out and returns to the sign-in page', async () => {
    const api = mockApi({
      'GET /api/me': () => ME,
      'GET /api/campaigns': () => ({ status: 200, body: { data: [] } }),
      'GET /api/gm-invites': () => ({ status: 200, body: { data: [] } }),
      'POST /api/auth/sign-out': () => ({ status: 204 }),
    })
    renderHome()

    await userEvent.click(await screen.findByRole('button', { name: 'Se déconnecter' }))

    expect(await screen.findByText('page de connexion')).toBeInTheDocument()
    expect(sentTo(api, 'POST /api/auth/sign-out')).toHaveLength(1)
  })
})
