import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { MediaPage } from './MediaPage'

const STORY = {
  id: 'mine',
  title: 'La mine',
  rules: { id: 'corsaires', version: 1 },
  bible: {},
  acts: [{ id: 'acte_1', title: 'La descente' }],
  nodes: [
    { id: 'sc_puits', act: 'acte_1', title: 'Le puits' },
    { id: 'sc_galerie', act: 'acte_1', title: 'La galerie' },
  ],
  npcs: [{ id: 'pnj_maud', name: 'Maud' }],
}

function plan(over: Record<string, unknown> = {}) {
  return {
    images: [
      { kind: 'scene', subject: 'sc_galerie' },
      { kind: 'npc', subject: 'pnj_maud' },
    ],
    videos: [{ kind: 'intro', subject: 'acte_1' }],
    imageMicros: 40_000,
    videoMicros: 4_000_000,
    spending: { budgetMicros: 10_000_000, spentMicros: 0 },
    running: false,
    configured: true,
    ...over,
  }
}

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1/medias']}>
      <Routes>
        <Route path="/campagnes/:campaignId/medias" element={<MediaPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('MediaPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('says what drawing everything missing costs, with or without the videos, and starts it', async () => {
    const api = mockApi({
      'GET /api/campaigns/c1': () => ({
        status: 200,
        body: { data: { id: 'c1', story: STORY, issues: [], validatedAt: null, updatedAt: '' } },
      }),
      'GET /api/campaigns/c1/media': () => ({
        status: 200,
        body: {
          data: {
            theme: null,
            plan: plan(),
            assets: [{ id: 'a1', kind: 'scene', subject: 'sc_puits', direction: '', status: 'pending', error: null }],
          },
        },
      }),
      'POST /api/campaigns/c1/media/batch': () => ({ status: 202, body: { data: { queued: [] } } }),
      'POST /api/campaigns/c1/media/a1/decision': () => ({ status: 204 }),
    })
    renderPage()

    expect(await screen.findByText(/2 sujets à dessiner, 1 actes à filmer/)).toBeInTheDocument()
    expect(screen.getByText(/au plus 0,08 \$ · reste 10,00 \$/)).toBeInTheDocument()
    await userEvent.click(screen.getByLabelText(/Avec la vidéo d'introduction/))
    expect(screen.getByText(/au plus 4,08 \$/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Dessiner les 3 médias' }))
    expect(sentTo(api, 'POST /api/campaigns/c1/media/batch')).toEqual([{ videos: true }])

    // A drawing waits for the GM, who keeps it.
    const puits = screen.getByRole('article', { name: 'Le puits' })
    expect(within(puits).getByRole('img', { name: 'Le puits' })).toHaveAttribute(
      'src',
      '/api/campaigns/c1/media/a1/image',
    )
    await userEvent.click(within(puits).getByRole('button', { name: 'Garder' }))
    expect(sentTo(api, 'POST /api/campaigns/c1/media/a1/decision')).toEqual([{ approve: true }])
  })

  it('shows a video being made, a kept one, and why a drawing failed', async () => {
    mockApi({
      'GET /api/campaigns/c1': () => ({
        status: 200,
        body: { data: { id: 'c1', story: STORY, issues: [], validatedAt: null, updatedAt: '' } },
      }),
      'GET /api/campaigns/c1/media': () => ({
        status: 200,
        body: {
          data: {
            theme: null,
            plan: plan({ images: [], videos: [], running: true }),
            assets: [
              { id: 'v2', kind: 'intro', subject: 'acte_1', direction: '', status: 'drawing', error: null },
              { id: 'v1', kind: 'intro', subject: 'acte_1', direction: '', status: 'approved', error: null },
              { id: 'a2', kind: 'npc', subject: 'pnj_maud', direction: '', status: 'rejected', error: 'Contenu refusé' },
            ],
          },
        },
      }),
    })
    renderPage()

    const act = await screen.findByRole('article', { name: 'La descente' })
    expect(within(act).getByText(/Vidéo en cours de génération/)).toBeInTheDocument()
    expect(within(act).getByLabelText('La descente')).toHaveAttribute('src', '/api/campaigns/c1/media/v1/image')
    expect(within(act).queryByRole('button', { name: 'Redessiner' })).not.toBeInTheDocument()
    const maud = screen.getByRole('article', { name: 'Maud' })
    expect(within(maud).getByText('Contenu refusé')).toBeInTheDocument()
    expect(within(maud).getByRole('button', { name: 'Redessiner' })).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Dessin en cours…' })).toBeDisabled()
  })
})
