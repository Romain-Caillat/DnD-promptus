import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { CampaignPage } from './CampaignPage'

const PRESET = {
  id: 'corsaires',
  version: 1,
  name: 'Corsaires de la Couronne',
  description: '',
  abilities: [{ abbr: 'CHA', name: 'Charisme' }],
  hitPoints: { abbr: 'PV', name: 'Points de vie' },
  armorClass: { abbr: 'CA', name: "Classe d'armure" },
}

function detail(over: { title?: string; archivedAt?: string | null; aiBudgetCents?: number } = {}) {
  return {
    id: 'c1',
    story: {
      id: 'valombre',
      title: over.title ?? 'Valombre',
      world: 'Fantasy',
      rules: { id: 'corsaires', version: 1 },
      bible: { pitch: 'Le maire paie le culte.', player_hook: 'Une lettre vous attend.' },
    },
    world: {},
    issues: [],
    settings: { playerCount: 6, aiBudgetCents: over.aiBudgetCents ?? 1000 },
    archivedAt: over.archivedAt ?? null,
    updatedAt: '2026-10-04T10:00:00Z',
  }
}

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1']}>
      <Routes>
        <Route path="/campagnes/:campaignId" element={<CampaignPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('CampaignPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('reopens a campaign with its settings and rules, and saves a change', async () => {
    const api = mockApi({
      'GET /api/campaigns/c1': () => ({ status: 200, body: { data: detail() } }),
      'GET /api/rule-systems': () => ({ status: 200, body: { data: [PRESET] } }),
      'PUT /api/campaigns/c1/settings': () => ({
        status: 200,
        body: { data: detail({ title: 'Les Cendres', aiBudgetCents: 250 }) },
      }),
    })
    renderPage()

    expect(await screen.findByRole('heading', { name: 'Valombre' })).toBeInTheDocument()
    expect(screen.getByLabelText(/Ton idée/)).toHaveValue('Le maire paie le culte.')
    expect(screen.getByLabelText('Budget IA ($)')).toHaveValue('10')
    expect(await screen.findByText('CHA · Charisme')).toBeInTheDocument()

    await userEvent.clear(screen.getByLabelText('Titre'))
    await userEvent.type(screen.getByLabelText('Titre'), 'Les Cendres')
    await userEvent.clear(screen.getByLabelText('Budget IA ($)'))
    await userEvent.type(screen.getByLabelText('Budget IA ($)'), '2,5')
    await userEvent.click(screen.getByRole('button', { name: /Enregistrer les réglages/ }))

    expect(await screen.findByText('Réglages enregistrés.')).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Les Cendres' })).toBeInTheDocument()
    expect(sentTo(api, 'PUT /api/campaigns/c1/settings')).toEqual([
      {
        title: 'Les Cendres',
        world: 'Fantasy',
        pitch: 'Le maire paie le culte.',
        playerHook: 'Une lettre vous attend.',
        playerCount: 6,
        aiBudgetCents: 250,
      },
    ])
  })

  it('archives after a confirmation, and brings the campaign back', async () => {
    let archivedAt: string | null = null
    const api = mockApi({
      'GET /api/campaigns/c1': () => ({ status: 200, body: { data: detail() } }),
      'GET /api/rule-systems': () => ({ status: 200, body: { data: [PRESET] } }),
      'PUT /api/campaigns/c1/archive': (body) => {
        archivedAt = (body as { archived: boolean }).archived ? '2026-10-05T09:00:00Z' : null
        return { status: 200, body: { data: detail({ archivedAt }) } }
      },
    })
    renderPage()

    await userEvent.click(await screen.findByRole('button', { name: 'Archiver' }))
    expect(sentTo(api, 'PUT /api/campaigns/c1/archive')).toHaveLength(0)
    await userEvent.click(screen.getByRole('button', { name: 'Archiver cette campagne ?' }))
    expect(await screen.findByText(/Archivée le 5 octobre 2026/)).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Rouvrir la campagne' }))
    expect(await screen.findByText(/En préparation/)).toBeInTheDocument()
    expect(sentTo(api, 'PUT /api/campaigns/c1/archive')).toEqual([{ archived: true }, { archived: false }])
  })

  it('says so when the campaign is not the GM’s', async () => {
    mockApi({
      'GET /api/campaigns/c1': () => ({ status: 404, body: { error: { code: 'NOT_FOUND', message: 'x' } } }),
      'GET /api/rule-systems': () => ({ status: 200, body: { data: [] } }),
    })
    renderPage()

    expect(await screen.findByRole('alert')).toHaveTextContent('Cette campagne n’existe pas ou n’est pas la vôtre.')
  })
})
