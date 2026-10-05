import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi } from '@/test-utils'
import { PlayerHomePage } from './PlayerHomePage'

function renderHome() {
  render(
    <MemoryRouter initialEntries={['/partie/c1']}>
      <Routes>
        <Route path="/partie/:campaignId" element={<PlayerHomePage />} />
      </Routes>
    </MemoryRouter>,
  )
}

const CAMPAIGN = {
  campaignId: 'c1',
  title: 'Les Cendres de Valombre',
  world: 'Valombre',
  playerHook: 'Une ville minière.',
  gmName: 'Romain',
}

describe('PlayerHomePage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('shows the campaign and where the character stands, with the GM note', async () => {
    mockApi({
      'GET /api/play/c1/me': () => ({
        status: 200,
        body: {
          data: {
            me: { id: 'p1', nickname: 'Marc', role: 'player' },
            campaign: CAMPAIGN,
            character: {
              id: 'k1',
              status: 'returned',
              sheet: { name: 'Borin' },
              gmNote: 'La Force s’arrête à 17.',
              updatedAt: '',
            },
          },
        },
      }),
    })
    renderHome()

    expect(await screen.findByRole('heading', { name: 'Les Cendres de Valombre' })).toBeInTheDocument()
    expect(screen.getByRole('status')).toHaveTextContent('Le MJ te demande un changement')
    expect(screen.getByText('Borin')).toBeInTheDocument()
    expect(screen.getByText('La Force s’arrête à 17.')).toBeInTheDocument()
  })

  it('tells a browser without a seat to open the invitation link', async () => {
    mockApi({
      'GET /api/play/c1/me': () => ({ status: 401, body: { error: { code: 'NOT_JOINED', message: 'x' } } }),
    })
    renderHome()

    expect(await screen.findByRole('alert')).toHaveTextContent("lien d'invitation")
  })
})
