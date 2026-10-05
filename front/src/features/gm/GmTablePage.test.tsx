import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { GmTablePage } from './GmTablePage'

function renderTable() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1']}>
      <Routes>
        <Route path="/campagnes/:campaignId" element={<GmTablePage />} />
      </Routes>
    </MemoryRouter>,
  )
}

const PREVIEW = {
  title: 'Les Cendres de Valombre',
  world: 'Valombre',
  playerHook: 'Une ville minière qui enterre ses morts deux fois.',
}
const MINTED = { code: 'secret', createdAt: '2026-10-04T10:00:00Z', expiresAt: '2026-10-11T10:00:00Z' }
const MARC = {
  id: 'p1',
  nickname: 'Marc',
  role: 'player',
  joinedAt: '2026-10-04T10:00:00Z',
  lastSeenAt: '2026-10-04T10:00:00Z',
  character: { id: 'k1', status: 'draft', name: '', updatedAt: '' },
}

describe('GmTablePage', () => {
  beforeEach(() => {
    localStorage.clear()
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('mints the link, drafts the Discord message from the players’ view and copies it', async () => {
    const writeText = vi.fn(async () => {})
    vi.stubGlobal('navigator', { ...navigator, clipboard: { writeText } })
    const api = mockApi({
      'GET /api/campaigns/c1/player-view': () => ({ status: 200, body: { data: PREVIEW } }),
      'GET /api/campaigns/c1/invite': () => ({ status: 200, body: { data: null } }),
      'GET /api/campaigns/c1/players': () => ({ status: 200, body: { data: [MARC] } }),
      'POST /api/campaigns/c1/invite': () => ({ status: 201, body: { data: MINTED } }),
    })
    renderTable()

    expect(await screen.findByText('Marc')).toBeInTheDocument()
    expect(screen.getByText(/crée son personnage/)).toBeInTheDocument()
    expect(screen.getByText(/personne ne peut rejoindre/)).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Créer le lien' }))

    const link = `${window.location.origin}/rejoindre/secret`
    expect(await screen.findByLabelText("Lien d'invitation")).toHaveValue(link)
    const message = (screen.getByRole('textbox', { name: /Discord/ }) as HTMLTextAreaElement).value
    expect(message).toContain('Les Cendres de Valombre')
    expect(message).toContain(PREVIEW.playerHook)
    expect(message).toContain(link)

    await userEvent.click(screen.getByRole('button', { name: /Copier le lien et le message/ }))
    expect(writeText).toHaveBeenCalledWith(message)
    expect(sentTo(api, 'POST /api/campaigns/c1/invite')).toHaveLength(1)
  })

  it('cannot show a link minted elsewhere: the server only keeps its hash', async () => {
    mockApi({
      'GET /api/campaigns/c1/player-view': () => ({ status: 200, body: { data: PREVIEW } }),
      'GET /api/campaigns/c1/invite': () => ({
        status: 200,
        body: { data: { createdAt: MINTED.createdAt, expiresAt: MINTED.expiresAt } },
      }),
      'GET /api/campaigns/c1/players': () => ({ status: 200, body: { data: [] } }),
    })
    renderTable()

    expect(await screen.findByText(/créé sur un autre appareil/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Nouveau lien' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Copier le lien/ })).toBeDisabled()
  })
})
