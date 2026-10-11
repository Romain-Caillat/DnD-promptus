import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes, useLocation } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { AtlasPage } from './atlas/AtlasPage'

/**
 * campaign/write-story-by-map-scale — entering a campaign
 * (`/campagnes/:id`) opens its Atlas on the widest map, with « L'idée »
 * in the side panel and the campaign's actions in the header.
 */

const PRESET = {
  id: 'corsaires',
  version: 1,
  name: 'Corsaires de la Couronne',
  description: '',
  abilities: [{ abbr: 'CHA', name: 'Charisme' }],
  hitPoints: { abbr: 'PV', name: 'Points de vie' },
  armorClass: { abbr: 'CA', name: "Classe d'armure" },
}

interface Over {
  title?: string
  archivedAt?: string | null
  aiBudgetCents?: number
  tone?: string
}

function detail(over: Over = {}) {
  return {
    id: 'c1',
    story: {
      id: 'corsaires',
      title: over.title ?? 'Les Corsaires',
      world: 'Caraïbes, 1720',
      rules: { id: 'corsaires', version: 1 },
      bible: {
        pitch: 'La Couronne trahit ses corsaires.',
        player_hook: 'Une lettre de marque vous attend.',
        tone: over.tone ?? 'Cape et épée',
        themes: ['La liberté'],
        truths: ['Le gouverneur est ruiné.'],
      },
      acts: [{ id: 'acte_1', title: 'Le brick', summary: 'Reprendre le brick.' }],
      fronts: [
        {
          id: 'navy',
          name: 'La Navy',
          goal: 'Pendre les corsaires',
          steps: [{ label: 'Patrouilles' }, { label: 'Blocus', description: 'Port-Louis fermé' }],
        },
      ],
    },
    world: {},
    issues: [],
    settings: { playerCount: 4, aiBudgetCents: over.aiBudgetCents ?? 1000 },
    archivedAt: over.archivedAt ?? null,
    updatedAt: '2026-10-11T10:00:00Z',
    revision: 7,
  }
}

const grid = (w: number, h: number) => ({
  legend: { '.': { terrain: 'herbe' } },
  rows: Array.from({ length: h }, () => '.'.repeat(w)),
})

const sheet = (id: string, name: string, scale: 'world' | 'place', tree: Record<string, unknown> = {}) => ({
  map: { id, name, scale, theme: 't', ambience: {}, grid: grid(4, 3) },
  origin: 'campaign',
  parent: null,
  children: [],
  links: [],
  backdrop: false,
  validated: false,
  pins: [],
  cast: { npcs: [], clues: [], fights: [] },
  thread: { segments: [], stops: [] },
  ...tree,
})

function atlas(maps: unknown[], roots: string[], claude: unknown[] = []) {
  return {
    revision: 7,
    roots,
    maps,
    places: [],
    unplaced: [],
    issues: [],
    castable: { npcs: [], clues: [], fights: [], scenes: [] },
    acts: [{ id: 'acte_1', title: 'Le brick' }],
    claude,
  }
}

// The place first among the roots: the Atlas still opens on the region.
const MAPS = atlas(
  [
    sheet('quai', 'Le quai de Port-Louis', 'place'),
    sheet('region', 'Les côtes', 'world'),
  ],
  ['quai', 'region'],
)

const ok = (data: unknown) => ({ status: 200, body: { data } })

/** The address the page is at, for the drawer's tab. */
function Where() {
  const at = useLocation()
  return <output aria-label="adresse">{at.pathname + at.search}</output>
}

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1']}>
      <Routes>
        <Route
          path="/campagnes/:campaignId"
          element={
            <>
              <AtlasPage />
              <Where />
            </>
          }
        />
      </Routes>
    </MemoryRouter>,
  )
}

const idea = () => screen.getByRole('region', { name: 'L’idée' })

describe('entering a campaign (campaign/write-story-by-map-scale)', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('opens the Atlas on the widest map with « L’idée », and saves the idea, its tone and the budget', async () => {
    stubReducedMotion(true)
    let current = detail()
    const api = mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(MAPS),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'GET /api/campaigns/c1': () => ok(current),
      'GET /api/rule-systems': () => ok([PRESET]),
      'PUT /api/campaigns/c1/settings': () => {
        current = detail({ title: 'Les Cendres', aiBudgetCents: 250 })
        return ok(current)
      },
      'POST /api/campaigns/c1/story/edits': () => {
        current = detail({ title: 'Les Cendres', aiBudgetCents: 250, tone: 'Noir' })
        return ok({ campaign: current, changes: [] })
      },
    })
    renderPage()

    // The region, not the place listed first; the campaign's name and state above it.
    await screen.findByRole('img', { name: 'Les côtes' })
    expect(await screen.findByRole('heading', { name: 'Les Corsaires', level: 1 })).toBeInTheDocument()
    expect(screen.getByText(/En préparation · rien n’est encore montré/)).toBeInTheDocument()
    const actions = screen.getByRole('group', { name: 'Actions de la campagne' })
    expect(within(actions).getAllByRole('button').map((b) => b.textContent)).toEqual([
      'Mener la soirée',
      'Inviter la table',
      'Règles',
      'Archiver',
    ])

    expect(within(idea()).getByLabelText(/Ton idée/)).toHaveValue('La Couronne trahit ses corsaires.')
    expect(within(idea()).getByLabelText(/L’accroche/)).toHaveValue('Une lettre de marque vous attend.')
    expect(within(idea()).getByLabelText('Ton')).toHaveValue('Cape et épée')
    expect(within(idea()).getByLabelText('Thèmes · un par ligne')).toHaveValue('La liberté')
    // The acts and fronts below, the technical settings folded.
    expect(within(idea()).getByText('Le brick')).toBeInTheDocument()
    expect(within(idea()).getByText('La Navy')).toBeInTheDocument()
    expect(await within(idea()).findByText('CHA · Charisme')).toBeInTheDocument()

    // « Titre » names an act's title too: the idea's own is in its form.
    const form = within(screen.getByRole('form', { name: 'L’idée' }))
    await userEvent.clear(form.getByLabelText('Titre'))
    await userEvent.type(form.getByLabelText('Titre'), 'Les Cendres')
    await userEvent.clear(within(idea()).getByLabelText('Ton'))
    await userEvent.type(within(idea()).getByLabelText('Ton'), 'Noir')
    await userEvent.click(within(idea()).getByText('Réglages techniques'))
    await userEvent.clear(within(idea()).getByLabelText('Budget IA ($)'))
    await userEvent.type(within(idea()).getByLabelText('Budget IA ($)'), '2,5')
    await userEvent.click(within(idea()).getByRole('button', { name: 'Enregistrer l’idée' }))

    expect(await screen.findByText('L’idée est enregistrée.')).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Les Cendres', level: 1 })).toBeInTheDocument()
    expect(within(idea()).getByLabelText('Ton')).toHaveValue('Noir')
    expect(sentTo(api, 'PUT /api/campaigns/c1/settings')).toEqual([
      {
        title: 'Les Cendres',
        world: 'Caraïbes, 1720',
        pitch: 'La Couronne trahit ses corsaires.',
        playerHook: 'Une lettre de marque vous attend.',
        playerCount: 4,
        aiBudgetCents: 250,
      },
    ])
    expect(sentTo(api, 'POST /api/campaigns/c1/story/edits')).toEqual([
      { edits: [{ op: 'set', target: 'bible', field: 'tone', value: 'Noir' }] },
    ])
  })

  it('edits an act and a front’s clock, keeping a step’s description', async () => {
    stubReducedMotion(true)
    const api = mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(MAPS),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'GET /api/campaigns/c1': () => ok(detail()),
      'GET /api/rule-systems': () => ok([]),
      'POST /api/campaigns/c1/story/edits': () => ok({ campaign: detail(), changes: [] }),
    })
    renderPage()

    await userEvent.click(await within(await screen.findByRole('region', { name: 'L’idée' })).findByText('Le brick'))
    await userEvent.type(within(idea()).getByLabelText('Lu à l’ouverture de l’acte'), 'La mer est grosse.')
    const act = within(idea()).getByLabelText('Lu à l’ouverture de l’acte').closest('details')!
    await userEvent.click(within(act as HTMLElement).getByRole('button', { name: 'Enregistrer' }))
    await waitFor(() => expect(sentTo(api, 'POST /api/campaigns/c1/story/edits')).toHaveLength(1))

    await userEvent.click(within(idea()).getByText('La Navy'))
    const clock = within(idea()).getByLabelText(/Son horloge/)
    expect(clock).toHaveValue('Patrouilles\nBlocus')
    await userEvent.clear(clock)
    await userEvent.type(clock, 'Rumeurs{Enter}Blocus{Enter}Pendaisons')
    const front = clock.closest('details')!
    await userEvent.click(within(front as HTMLElement).getByRole('button', { name: 'Enregistrer' }))
    await waitFor(() => expect(sentTo(api, 'POST /api/campaigns/c1/story/edits')).toHaveLength(2))

    expect(sentTo(api, 'POST /api/campaigns/c1/story/edits')).toEqual([
      { edits: [{ op: 'set', target: 'acte_1', field: 'opening', value: 'La mer est grosse.' }] },
      {
        edits: [
          {
            op: 'set',
            target: 'navy',
            field: 'steps',
            value: [{ label: 'Rumeurs' }, { label: 'Blocus', description: 'Port-Louis fermé' }, { label: 'Pendaisons' }],
          },
        ],
      },
    ])
  })

  it('shows Claude’s proposal dotted under the field it would change, and keeps it there', async () => {
    stubReducedMotion(true)
    const batch = {
      id: 'p1',
      author: 'token',
      prompt: 'Une accroche plus sèche',
      reply: '',
      changes: [],
      stale: false,
      dropped: 0,
      status: 'pending',
      createdAt: '2026-10-11T10:00:00Z',
      items: [
        {
          key: 'e0',
          status: 'pending',
          stale: false,
          change: {
            op: 'set',
            kind: 'bible',
            id: 'bible',
            name: '',
            field: 'player_hook',
            before: 'Une lettre de marque vous attend.',
            after: 'On vous pendra à l’aube.',
          },
        },
        {
          key: 'e1',
          status: 'pending',
          stale: false,
          change: { op: 'add', kind: 'act', id: 'acte_2', name: 'La chasse' },
        },
      ],
    }
    const api = mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(atlas(MAPS.maps, MAPS.roots, [batch])),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'GET /api/campaigns/c1': () => ok(detail()),
      'GET /api/rule-systems': () => ok([]),
      'POST /api/campaigns/c1/proposals/p1/decide': () => ok({ campaign: detail(), proposal: batch }),
    })
    renderPage()

    const proposal = await within(await screen.findByRole('region', { name: 'L’idée' })).findByText(
      '▶ Claude propose : On vous pendra à l’aube.',
    )
    // The field keeps the campaign as it is until the GM keeps it.
    expect(within(idea()).getByLabelText(/L’accroche/)).toHaveValue('Une lettre de marque vous attend.')
    expect(within(idea()).getByText('▶ Claude propose d’ajouter « La chasse »')).toBeInTheDocument()

    await userEvent.click(within(proposal.closest('li')!).getByRole('button', { name: 'Garder' }))
    await waitFor(() => expect(sentTo(api, 'POST /api/campaigns/c1/proposals/p1/decide')).toEqual([{ keep: ['e0'] }]))
  })

  it('archives from the header after a confirmation, and takes it off the shelf', async () => {
    stubReducedMotion(true)
    let archivedAt: string | null = null
    const api = mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(MAPS),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'GET /api/campaigns/c1': () => ok(detail({ archivedAt })),
      'GET /api/rule-systems': () => ok([]),
      'PUT /api/campaigns/c1/archive': (body) => {
        archivedAt = (body as { archived: boolean }).archived ? '2026-10-05T09:00:00Z' : null
        return ok(detail({ archivedAt }))
      },
    })
    renderPage()

    await userEvent.click(await screen.findByRole('button', { name: 'Archiver' }))
    expect(sentTo(api, 'PUT /api/campaigns/c1/archive')).toHaveLength(0)
    await userEvent.click(screen.getByRole('button', { name: 'Archiver cette campagne ?' }))
    expect(await screen.findByText(/Archivée le 5 octobre 2026/)).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Désarchiver' }))
    expect(await screen.findByText(/En préparation · rien/)).toBeInTheDocument()
    expect(sentTo(api, 'PUT /api/campaigns/c1/archive')).toEqual([{ archived: true }, { archived: false }])
  })

  it('shows « L’idée » alone for a campaign with no map, and the way to its maps', async () => {
    stubReducedMotion(true)
    mockApi({
      'GET /api/campaigns/c1/atlas': () => ok(atlas([], [])),
      'GET /api/campaigns/c1/media': () => ok({ theme: null, assets: [] }),
      'GET /api/campaigns/c1': () => ok(detail()),
      'GET /api/rule-systems': () => ok([]),
      'GET /api/campaigns/c1/maps': () => ok({ maps: [], world: [] }),
    })
    renderPage()

    expect(await screen.findByRole('region', { name: 'L’idée' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Pas encore de carte' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Les cartes de la campagne' }))
    expect(screen.getByRole('status', { name: 'adresse' })).toHaveTextContent('/campagnes/c1?onglet=cartes')
  })

  it('says so when the campaign is not the GM’s', async () => {
    stubReducedMotion(true)
    const notFound = () => ({ status: 404, body: { error: { code: 'NOT_FOUND', message: 'x' } } })
    mockApi({
      'GET /api/campaigns/c1/atlas': notFound,
      'GET /api/campaigns/c1/media': notFound,
      'GET /api/campaigns/c1': notFound,
      'GET /api/rule-systems': () => ok([]),
    })
    renderPage()

    expect(await screen.findByRole('alert')).toHaveTextContent('Cette campagne n\'existe pas, ou elle n\'est pas à vous.')
  })
})
