import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { ReviewPage } from './ReviewPage'

const ALERT = {
  severity: 'warning',
  code: 'THREE_CLUE_RULE',
  path: 'revelations[0]',
  detail: 'critical revelation `rev_route` has 1 clue(s) in 1 node(s)',
}

function campaign(over: { summary?: string; clues?: unknown[]; issues?: unknown[]; validatedAt?: string | null } = {}) {
  return {
    id: 'c1',
    story: {
      id: 'corsaires',
      title: 'Corsaires',
      rules: { id: 'corsaires', version: 1 },
      bible: { pitch: 'Un brick à reprendre.' },
      acts: [{ id: 'acte_1', title: 'Acte I' }],
      nodes: [
        { id: 'sc_quai', act: 'acte_1', title: 'Le quai', summary: over.summary ?? 'Une bagarre.' },
        { id: 'sc_bureau', act: 'acte_1', title: 'Le bureau de Morel', optional: true },
      ],
      revelations: [{ id: 'rev_route', statement: 'La route du Greyhound', importance: 'critical' }],
      clues: over.clues ?? [{ id: 'cl_carte', revelation: 'rev_route', node: 'sc_bureau', text: 'Une carte.' }],
      npcs: [{ id: 'pnj_morel', name: 'Morel', disposition: 'neutral', motivation: 'Survivre.' }],
    },
    issues: over.issues ?? [ALERT],
    validatedAt: over.validatedAt ?? null,
    updatedAt: '2026-10-06T10:00:00Z',
  }
}

const PROPOSAL = {
  id: 'p1',
  prompt: 'THREE_CLUE_RULE revelations[0]',
  reply: 'J’ajoute un indice sur le quai.',
  changes: [
    { op: 'add', kind: 'clue', id: 'cl_route_quai', name: 'Un marin bavard', place: 'sc_quai' },
  ],
  stale: false,
  dropped: 1,
  status: 'pending',
  createdAt: '2026-10-06T10:01:00Z',
}

const NOT_READY = {
  act: 'acte_1',
  title: 'Acte I',
  ready: false,
  done: 3,
  total: 5,
  checks: [
    {
      kind: 'knowledge_paths',
      done: 0,
      total: 1,
      gaps: [
        { id: 'rev_route', name: 'La route du Greyhound', code: 'ONLY_OPTIONAL', detail: 'x' },
        { id: 'rev_route', name: 'La route du Greyhound', code: 'FEW_PATHS', paths: 1, detail: 'x' },
      ],
    },
    { kind: 'player_hooks', done: 1, total: 1, gaps: [] },
  ],
}
const READY = { ...NOT_READY, ready: true, done: 5, checks: [] }

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1/preparer']}>
      <Routes>
        <Route path="/campagnes/:campaignId/preparer" element={<ReviewPage />} />
        <Route path="/campagnes/:campaignId/table" element={<p>La table</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('ReviewPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('corrects a scene, accepts the co-GM’s fix of an alert, and validates', async () => {
    const fixed = campaign({
      summary: 'La bagarre éclate.',
      clues: [
        { id: 'cl_carte', revelation: 'rev_route', node: 'sc_bureau', text: 'Une carte.' },
        { id: 'cl_route_quai', revelation: 'rev_route', node: 'sc_quai', text: 'Un marin bavard' },
      ],
      issues: [],
    })
    const api = mockApi({
      'GET /api/campaigns/c1': () => ({ status: 200, body: { data: campaign() } }),
      'GET /api/campaigns/c1/workshop': () => ({ status: 200, body: { data: [] } }),
      'GET /api/campaigns/c1/readiness': () => ({ status: 200, body: { data: [READY] } }),
      'POST /api/campaigns/c1/story/edits': () => ({
        status: 200,
        body: { data: { campaign: campaign({ summary: 'La bagarre éclate.' }), changes: [] } },
      }),
      'POST /api/campaigns/c1/workshop': () => ({ status: 201, body: { data: PROPOSAL } }),
      'POST /api/campaigns/c1/workshop/p1/accept': () => ({
        status: 200,
        body: { data: { campaign: fixed, proposal: { ...PROPOSAL, status: 'accepted', changes: [] } } },
      }),
      'POST /api/campaigns/c1/story/validate': () => ({
        status: 200,
        body: { data: { ...fixed, validatedAt: '2026-10-06T10:05:00Z' } },
      }),
    })
    renderPage()

    // The graph: select the quay and correct its summary.
    await userEvent.click(await screen.findByRole('button', { name: /Le quai/ }))
    const summary = screen.getByLabelText('En bref (MJ)')
    await userEvent.clear(summary)
    await userEvent.type(summary, 'La bagarre éclate.')
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))
    expect(await screen.findByDisplayValue('La bagarre éclate.')).toBeInTheDocument()
    expect(sentTo(api, 'POST /api/campaigns/c1/story/edits')).toEqual([
      { edits: [{ op: 'set', target: 'sc_quai', field: 'summary', value: 'La bagarre éclate.' }] },
    ])

    // The alert: the co-GM proposes a clue, the GM accepts it.
    await userEvent.click(screen.getByRole('button', { name: /Cohérence · 1/ }))
    expect(screen.getByText(/moins de trois indices/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Proposer une correction' }))
    expect(sentTo(api, 'POST /api/campaigns/c1/workshop')).toEqual([
      { issue: { code: 'THREE_CLUE_RULE', path: 'revelations[0]' } },
    ])
    expect(await screen.findByText('Indice « Un marin bavard » → Le quai')).toBeInTheDocument()
    expect(screen.getByText(/1 changement écarté/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Accepter' }))
    expect(await screen.findByText('Acceptée')).toBeInTheDocument()
    expect(screen.getByText(/la campagne tient debout/)).toBeInTheDocument()

    // Validated, it opens on the invitation.
    await userEvent.click(screen.getByRole('button', { name: 'Valider' }))
    const panel = screen.getByRole('region', { name: 'Valider la campagne' })
    await userEvent.click(within(panel).getByRole('button', { name: /Valider la campagne/ }))
    expect(await screen.findByText(/elle est jouable/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Inviter les joueurs/ }))
    expect(await screen.findByText('La table')).toBeInTheDocument()
  })

  it('does not validate a campaign with errors', async () => {
    mockApi({
      'GET /api/campaigns/c1': () => ({
        status: 200,
        body: {
          data: campaign({
            issues: [{ severity: 'error', code: 'REF_DANGLING', path: 'nodes[0].location', detail: 'x' }],
          }),
        },
      }),
      'GET /api/campaigns/c1/workshop': () => ({ status: 200, body: { data: [] } }),
      'GET /api/campaigns/c1/readiness': () => ({ status: 200, body: { data: [NOT_READY] } }),
    })
    renderPage()

    await userEvent.click(await screen.findByRole('button', { name: 'Valider' }))
    expect(screen.getByRole('button', { name: /1 erreur à corriger d'abord/ })).toBeDisabled()
    // The gauge says what the act lacks.
    expect(await screen.findByText('« La route du Greyhound » ne se trouve que dans des scènes facultatives.')).toBeInTheDocument()
    expect(screen.getByText("« La route du Greyhound » n'a qu'un chemin sur trois.")).toBeInTheDocument()
    expect(screen.getAllByRole('meter', { name: '3 sur 5' }).length).toBeGreaterThan(0)
  })
})
