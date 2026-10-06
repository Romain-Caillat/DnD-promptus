import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { GenerationPage } from './GenerationPage'

const PITCH = 'Des gobelins pillent un village minier.'
const INPUT = { pitch: PITCH, tone: 'Âpre', themes: '', constraints: '', length: 'one_shot' }

function steps(cast: string, scenes: string, check: string) {
  return [
    { id: 'cast', status: cast, counts: cast === 'done' ? { npcs: 3, adversaries: 2, locations: 5 } : {} },
    { id: 'scenes', status: scenes, counts: scenes === 'done' ? { nodes: 7, clues: 8 } : {} },
    { id: 'check', status: check, counts: check === 'done' ? { errors: 0, warnings: 0, repairs: 1 } : {} },
  ]
}

const JOB = {
  id: 'j1',
  status: 'succeeded',
  input: INPUT,
  steps: steps('done', 'done', 'done'),
  draft: {
    id: 'mine',
    title: 'La mine',
    rules: { id: 'corsaires', version: 1 },
    bible: { pitch: `${PITCH} Tout commence à l’auberge.` },
    nodes: [
      { id: 'sc_ouverture', act: 'acte_1', title: 'L’offre à l’auberge' },
      { id: 'sc_marche', act: 'acte_1', title: 'Le marché de nuit', optional: true },
    ],
    revelations: [{ id: 'rev_traitre', statement: 'Mirelle trahit.', importance: 'critical' }],
    clues: [],
    npcs: [{ id: 'pnj_aldric' }],
  },
  issues: [],
  removed: [{ code: 'REF_DANGLING', path: 'npcs[2].faction', detail: 'x' }],
  repairs: 1,
  dropped: 1,
  costMicros: 2_000,
  error: null,
  detail: null,
  createdAt: '2026-10-06T10:00:00Z',
}

function desk(jobs: unknown[], validated = false) {
  return {
    status: 200,
    body: {
      data: { jobs, estimateMicros: 420_000, spending: { budgetMicros: 5_000_000, spentMicros: 1_000_000 }, validated },
    },
  }
}

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1/generer']}>
      <Routes>
        <Route path="/campagnes/:campaignId/generer" element={<GenerationPage />} />
        <Route path="/campagnes/:campaignId/preparer" element={<p>La relecture</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('GenerationPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('starts a generation from a pitch, shows the draft and applies it', async () => {
    let jobs: unknown[] = []
    const api = mockApi({
      'GET /api/campaigns/c1/generation': () => desk(jobs),
      'POST /api/campaigns/c1/generation': () => {
        jobs = [JOB]
        return { status: 202, body: { data: { ...JOB, status: 'running', steps: steps('running', 'pending', 'pending') } } }
      },
      'POST /api/campaigns/c1/generation/j1/apply': () => ({ status: 200, body: { data: { job: { ...JOB, status: 'applied' } } } }),
    })
    renderPage()

    // The cost before anything.
    expect(await screen.findByText(/au plus 0,42 \$ · reste 4,00 \$/)).toBeInTheDocument()
    const start = screen.getByRole('button', { name: 'Lancer la génération' })
    expect(start).toBeDisabled()
    await userEvent.type(screen.getByLabelText(/L'idée/), PITCH)
    await userEvent.type(screen.getByLabelText('Ton'), 'Âpre')
    await userEvent.click(start)
    expect(sentTo(api, 'POST /api/campaigns/c1/generation')).toEqual([INPUT])

    // The draft: its steps, what it holds, what the validator says.
    const card = await screen.findByRole('region', { name: 'Brouillon prêt' })
    expect(within(card).getByText(/Scènes, révélations et indices — 7 scènes, 8 indices/)).toBeInTheDocument()
    expect(within(card).getByText('2 scènes')).toBeInTheDocument()
    expect(within(card).getByText('1 révélation')).toBeInTheDocument()
    expect(within(card).getByText(/Le marché de nuit/)).toBeInTheDocument()
    expect(within(card).getByText('Le validateur ne trouve rien à redire.')).toBeInTheDocument()
    expect(within(card).getByText(/2 éléments inventés par le modèle ont été écartés/)).toBeInTheDocument()

    await userEvent.click(within(card).getByRole('button', { name: 'Appliquer à la campagne' }))
    expect(await screen.findByText('La relecture')).toBeInTheDocument()
  })

  it('says why a generation failed, and starts none on a validated campaign', async () => {
    mockApi({
      'GET /api/campaigns/c1/generation': () =>
        desk(
          [
            {
              ...JOB,
              status: 'failed',
              draft: null,
              steps: steps('done', 'failed', 'pending'),
              error: 'AI_BUDGET_EXCEEDED',
              detail: null,
            },
          ],
          true,
        ),
    })
    renderPage()

    expect(await screen.findByText(/Le budget IA de la campagne ne suffit pas/)).toBeInTheDocument()
    expect(screen.getByText(/déjà validée/)).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText(/L'idée/), PITCH)
    expect(screen.getByRole('button', { name: 'Lancer la génération' })).toBeDisabled()
    expect(screen.queryByRole('button', { name: 'Appliquer à la campagne' })).not.toBeInTheDocument()
  })
})
