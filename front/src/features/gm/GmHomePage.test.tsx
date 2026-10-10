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

  it('shows each campaign as a card that reopens it, archived ones apart', async () => {
    mockApi({
      'GET /api/me': () => ME,
      'GET /api/gm-tokens': () => ({ status: 200, body: { data: [] } }),
      'GET /api/rule-systems': () => ({
        status: 200,
        body: {
          data: [
            {
              id: 'corsaires',
              version: 1,
              name: 'Corsaires de la Couronne',
              description: '',
              abilities: [],
              hitPoints: { abbr: 'PV', name: 'Points de vie' },
              armorClass: { abbr: 'CA', name: "Classe d'armure" },
            },
          ],
        },
      }),
      'GET /api/campaigns': () => ({
        status: 200,
        body: {
          data: [
            {
              id: 'c1',
              title: 'Les Cendres de Valombre',
              world: 'Fantasy',
              rules: { id: 'corsaires', version: 1 },
              playerCount: 6,
              playersSeated: 2,
              archivedAt: null,
              lastActivityAt: '2026-10-04T10:00:00Z',
            },
            {
              id: 'c2',
              title: 'Le Brasier',
              world: '',
              rules: { id: 'brasier', version: 1 },
              playerCount: 4,
              playersSeated: 0,
              archivedAt: '2026-09-01T10:00:00Z',
              lastActivityAt: '2026-09-01T10:00:00Z',
            },
          ],
        },
      }),
    })
    renderHome()

    const valombre = await screen.findByRole('link', { name: /Les Cendres de Valombre/ })
    expect(valombre).toHaveAttribute('href', '/campagnes/c1')
    expect(valombre).toHaveTextContent('Fantasy · en préparation')
    expect(valombre).toHaveTextContent('Corsaires de la Couronne')
    expect(valombre).toHaveTextContent('2 joueurs installés sur 6')
    expect(valombre).toHaveTextContent('Dernière activité : 4 octobre 2026')

    const shelf = screen.getByRole('heading', { name: 'Archivées' }).closest('section')!
    const brasier = within(shelf).getByRole('link', { name: /Le Brasier/ })
    expect(brasier).toHaveAttribute('href', '/campagnes/c2')
    expect(brasier).toHaveTextContent('Aucun joueur installé · 4 prévus')
    expect(screen.getByRole('link', { name: /Nouvelle campagne/ })).toHaveAttribute('href', '/campagnes/nouvelle')
  })

  it('signs out and returns to the sign-in page', async () => {
    const api = mockApi({
      'GET /api/me': () => ME,
      'GET /api/campaigns': () => ({ status: 200, body: { data: [] } }),
      'GET /api/rule-systems': () => ({ status: 200, body: { data: [] } }),
      'GET /api/gm-tokens': () => ({ status: 200, body: { data: [] } }),
      'POST /api/auth/sign-out': () => ({ status: 204 }),
    })
    renderHome()

    await userEvent.click(await screen.findByRole('button', { name: 'Se déconnecter' }))

    expect(await screen.findByText('page de connexion')).toBeInTheDocument()
    expect(sentTo(api, 'POST /api/auth/sign-out')).toHaveLength(1)
  })
})
