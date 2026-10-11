import { act, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { AtlasPage } from '@/features/gm/atlas/AtlasPage'

const ALERT = {
  severity: 'warning',
  code: 'THREE_CLUE_RULE',
  path: 'revelations[0]',
  detail: 'critical revelation `rev_route` has 1 clue(s) in 1 node(s)',
}

function campaign(
  over: {
    summary?: string
    clues?: unknown[]
    issues?: unknown[]
    validatedAt?: string | null
    morel?: string
    revision?: number
  } = {},
) {
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
      npcs: [{ id: 'pnj_morel', name: over.morel ?? 'Morel', disposition: 'neutral', motivation: 'Survivre.' }],
    },
    issues: over.issues ?? [ALERT],
    settings: { playerCount: 4, aiBudgetCents: 0 },
    archivedAt: null,
    validatedAt: over.validatedAt ?? null,
    updatedAt: '2026-10-06T10:00:00Z',
    revision: over.revision ?? 0,
  }
}

/** The browser's WebSocket, played by the test. */
class FakeSocket {
  static all: FakeSocket[] = []
  onopen: (() => void) | null = null
  onmessage: ((ev: { data: string }) => void) | null = null
  onclose: (() => void) | null = null
  onerror: (() => void) | null = null

  constructor() {
    FakeSocket.all.push(this)
  }

  send() {}
  close() {}

  push(msg: unknown) {
    act(() => this.onmessage?.({ data: JSON.stringify(msg) }))
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
  alerts: [{ id: 'sc_reve', name: 'Le rêve de Borin', code: 'NO_PLACE', detail: 'x' }],
}
const READY = { ...NOT_READY, ready: true, done: 5, checks: [], alerts: [] }

/** The Atlas around the drawer: no map, the two scenes for the banner. */
const ATLAS = {
  revision: 4,
  roots: [],
  maps: [],
  places: [],
  unplaced: [],
  issues: [],
  castable: {
    npcs: [],
    clues: [],
    fights: [],
    scenes: [
      { id: 'sc_quai', title: 'Le quai', number: 1 },
      { id: 'sc_bureau', title: 'Le bureau de Morel', number: 2 },
    ],
  },
  acts: [],  frieze: { acts: [], outside: [] },
}
const AROUND = {
  'GET /api/campaigns/c1/atlas': () => ({ status: 200, body: { data: ATLAS } }),
  'GET /api/campaigns/c1/media': () => ({ status: 200, body: { data: { theme: null, assets: [] } } }),
  'GET /api/campaigns/c1/proposals': () => ({ status: 200, body: { data: [] } }),
  'GET /api/rule-systems': () => ({ status: 200, body: { data: [] } }),
}

/** The preparation lives in the Atlas's drawer (gm/prep-on-atlas). */
function renderPage(at = '/campagnes/c1/atlas?onglet=graph') {
  render(
    <MemoryRouter initialEntries={[at]}>
      <Routes>
        <Route path="/campagnes/:campaignId/atlas" element={<AtlasPage />} />
        <Route path="/campagnes/:campaignId/table" element={<p>La table</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('PrepDesk, in the Atlas’s drawer', () => {
  beforeEach(() => {
    stubReducedMotion(true)
  })
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
      ...AROUND,
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
    await userEvent.click(screen.getByRole('button', { name: 'Cohérence' }))
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

  it('lights up what Claude wrote while the screen is open, and takes it back', async () => {
    vi.stubGlobal('WebSocket', FakeSocket)
    FakeSocket.all = []
    let current = campaign({ revision: 4 })
    let journal: unknown[] = []
    const api = mockApi({
      ...AROUND,
      'GET /api/campaigns/c1': () => ({ status: 200, body: { data: current } }),
      'GET /api/campaigns/c1/workshop': () => ({ status: 200, body: { data: [] } }),
      'GET /api/campaigns/c1/readiness': () => ({ status: 200, body: { data: [READY] } }),
      'GET /api/campaigns/c1/revisions?since=4': () => ({
        status: 200,
        body: { data: { revision: 4 + journal.length, entries: journal } },
      }),
      'POST /api/campaigns/c1/revisions/5/undo': () => {
        current = campaign({ revision: 6 })
        return { status: 200, body: { data: current } }
      },
    })
    renderPage()
    await screen.findByRole('button', { name: /Le quai/ })
    const socket = FakeSocket.all[0]
    socket.push({ type: 'ready', versions: { story: 1, prep: 4 } })

    // Claude renames Morel from Claude Code: no reload.
    current = campaign({ revision: 5, morel: 'Morel le borgne' })
    journal = [
      {
        revision: 5,
        author: 'token',
        target: 'story',
        changes: [
          { op: 'add', kind: 'clue', id: 'cl_registre', name: 'Un registre raturé', place: 'sc_quai' },
          { op: 'set', kind: 'npc', id: 'pnj_morel', name: 'Morel le borgne' },
        ],
        undone: false,
        undoable: true,
        createdAt: '2026-10-06T10:02:00Z',
      },
    ]
    socket.push({ type: 'changed', topic: 'story', version: 2 })
    socket.push({ type: 'changed', topic: 'prep', version: 5 })
    socket.push({ type: 'activity', author: 'token', revision: 5 })
    // Named by the scene its clue lands in, which is lit.
    expect(await screen.findByText('Claude écrit · sur Le quai')).toBeInTheDocument()
    expect(within(screen.getByRole('button', { name: /Le quai/ })).getByText('Modifié')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Fiches/ }))
    const morel = await screen.findByRole('button', { name: /Morel le borgne/ })
    expect(within(morel).getByText('Modifié')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }))
    // Morel is back; the banner is gone.
    expect(await screen.findByRole('button', { name: /^Morel\s*(Modifié)?$/ })).toBeInTheDocument()
    expect(screen.queryByText(/Claude/)).not.toBeInTheDocument()
    expect(api.mock.calls.some(([u, i]) => String(u).endsWith('/revisions/5/undo') && i?.method === 'POST')).toBe(true)
  })

  it('does not validate a campaign with errors', async () => {
    mockApi({
      ...AROUND,
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
    // A scene standing on no map is said, apart from what the act lacks.
    expect(screen.getByText('« Le rêve de Borin » ne se trouve sur aucune carte (permis : rêve, souvenir).')).toBeInTheDocument()
  })

  it('opens on the scene of an Atlas pin, and sends a scene with no spot to the map, not to the co-GM', async () => {
    mockApi({
      ...AROUND,
      'GET /api/campaigns/c1': () => ({
        status: 200,
        body: {
          data: campaign({
            issues: [{ severity: 'warning', code: 'SCENE_WITHOUT_PLACE', path: 'nodes[1].location', detail: 'x' }],
          }),
        },
      }),
      'GET /api/campaigns/c1/workshop': () => ({ status: 200, body: { data: [] } }),
      'GET /api/campaigns/c1/readiness': () => ({ status: 200, body: { data: [READY] } }),
    })
    renderPage('/campagnes/c1/atlas?onglet=graph&scene=sc_bureau')

    expect(await screen.findByRole('region', { name: 'Fiche de la scène « Le bureau de Morel »' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Cohérence' }))
    expect(screen.queryByRole('button', { name: 'Proposer une correction' })).not.toBeInTheDocument()
    // Pinning it is the map's: the drawer closes on it — here, a campaign with no map yet.
    await userEvent.click(screen.getByRole('button', { name: 'L’épingler sur la carte' }))
    expect(await screen.findByRole('region', { name: 'Pas encore de carte' })).toBeInTheDocument()
  })
})
