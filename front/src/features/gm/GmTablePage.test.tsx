import { render, screen, within } from '@testing-library/react'
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

const SEEN = '2026-10-04T11:00:00Z'
const borinSeat = (status: string, resubmitted = false) => ({
  ...MARC,
  character: {
    id: 'k1',
    status,
    name: 'Borin',
    classId: 'guerrier',
    className: 'Guerrier',
    look: null,
    resubmitted,
    updatedAt: SEEN,
  },
})
const ABILITIES = [
  { id: 'FOR', name: 'Force' },
  { id: 'SAG', name: 'Sagesse' },
]
const borinReview = (over: Record<string, unknown>) => ({
  id: 'k1',
  playerId: 'p1',
  nickname: 'Marc',
  status: 'submitted',
  sheet: { name: 'Borin', classId: 'guerrier', abilities: { FOR: 18, SAG: 12 }, backstory: 'Son frère Dorn a disparu dans la mine.' },
  gmNote: null,
  updatedAt: SEEN,
  reviewedSheet: null,
  reviewedAt: null,
  changes: [],
  checks: [],
  rulesName: 'Corsaires',
  className: 'Guerrier',
  abilities: ABILITIES,
  ...over,
})
const tableRoutes = (seat: unknown) => ({
  'GET /api/campaigns/c1/player-view': () => ({ status: 200, body: { data: PREVIEW } }),
  'GET /api/campaigns/c1/invite': () => ({ status: 200, body: { data: null } }),
  'GET /api/campaigns/c1/players': () => ({ status: 200, body: { data: [seat] } }),
})

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
  it('returns a sheet the rules flag with the GM’s word, rewritten from the proposed one', async () => {
    const flag = { severity: 'warning', code: 'ABILITY_ABOVE_MAX', path: 'abilities.FOR', message: 'La Force dépasse 17.' }
    const api = mockApi({
      ...tableRoutes(borinSeat('submitted')),
      'GET /api/campaigns/c1/characters/k1': () => ({ status: 200, body: { data: borinReview({ checks: [flag] }) } }),
      'POST /api/campaigns/c1/characters/k1/return': () => ({ status: 204 }),
    })
    renderTable()

    await userEvent.click(await screen.findByRole('button', { name: /Marc/ }))
    const checks = await screen.findByRole('complementary', { name: 'Vérification' })
    expect(within(checks).getByText('La Force dépasse 17.', { selector: 'span' })).toBeInTheDocument()
    const note = within(checks).getByRole('textbox', { name: /Mot pour Marc/ })
    expect(note).toHaveValue('La Force dépasse 17.')

    await userEvent.clear(note)
    await userEvent.type(note, 'Baisse la Force à 17, garde le point ailleurs.')
    await userEvent.click(screen.getByRole('button', { name: /Renvoyer à Marc avec ce mot/ }))

    expect(sentTo(api, 'POST /api/campaigns/c1/characters/k1/return')).toEqual([
      { seen: SEEN, note: 'Baisse la Force à 17, garde le point ailleurs.' },
    ])
  })

  it('lets the GM validate a flagged sheet anyway: the rules never block', async () => {
    const flag = { severity: 'warning', code: 'ABILITY_ABOVE_MAX', path: 'abilities.FOR', message: 'La Force dépasse 17.' }
    const api = mockApi({
      ...tableRoutes(borinSeat('submitted')),
      'GET /api/campaigns/c1/characters/k1': () => ({ status: 200, body: { data: borinReview({ checks: [flag] }) } }),
      'POST /api/campaigns/c1/characters/k1/validate': () => ({ status: 204 }),
    })
    renderTable()

    await userEvent.click(await screen.findByRole('button', { name: /Marc/ }))
    await userEvent.click(await screen.findByRole('button', { name: /Valider quand même/ }))

    expect(sentTo(api, 'POST /api/campaigns/c1/characters/k1/validate')).toEqual([{ seen: SEEN }])
  })

  it('shows only what changed since the return of a corrected sheet, then validates it', async () => {
    const api = mockApi({
      ...tableRoutes(borinSeat('submitted', true)),
      'GET /api/campaigns/c1/characters/k1': () => ({
        status: 200,
        body: {
          data: borinReview({
            sheet: { name: 'Borin', classId: 'guerrier', abilities: { FOR: 17, SAG: 13 } },
            reviewedSheet: { name: 'Borin', classId: 'guerrier', abilities: { FOR: 18, SAG: 12 } },
            reviewedAt: '2026-10-04T10:30:00Z',
            changes: [
              { path: 'abilities.FOR', before: 18, after: 17 },
              { path: 'abilities.SAG', before: 12, after: 13 },
            ],
          }),
        },
      }),
      'POST /api/campaigns/c1/characters/k1/validate': () => ({ status: 204 }),
    })
    renderTable()

    expect(await screen.findByText(/corrigé · à relire/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Marc/ }))
    expect(await screen.findByText('Force : 18 → 17')).toBeInTheDocument()
    expect(screen.getByText('Sagesse : 12 → 13')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: /Valider Borin/ }))
    expect(sentTo(api, 'POST /api/campaigns/c1/characters/k1/validate')).toEqual([{ seen: SEEN }])
  })

  it('says so when the sheet changed during the reading, and reloads it', async () => {
    let reads = 0
    mockApi({
      ...tableRoutes(borinSeat('submitted')),
      'GET /api/campaigns/c1/characters/k1': () => {
        reads += 1
        return { status: 200, body: { data: borinReview({}) } }
      },
      'POST /api/campaigns/c1/characters/k1/validate': () => ({
        status: 409,
        body: { error: { code: 'CHARACTER_CHANGED' } },
      }),
    })
    renderTable()

    await userEvent.click(await screen.findByRole('button', { name: /Marc/ }))
    await userEvent.click(await screen.findByRole('button', { name: /Valider Borin/ }))

    expect(await screen.findByText(/La fiche a changé pendant ta lecture/)).toBeInTheDocument()
    expect(reads).toBe(2)
  })

  it('adds a secret hook drawn from a backstory, tied to a front of the story', async () => {
    const api = mockApi({
      ...tableRoutes(borinSeat('validated')),
      'GET /api/campaigns/c1/characters/k1': () => ({
        status: 200,
        body: { data: borinReview({ status: 'validated' }) },
      }),
      'GET /api/campaigns/c1/hooks': () => ({
        status: 200,
        body: { data: { hooks: [], targets: [{ id: 'f-mine', kind: 'front', title: 'Le mort qui marche' }] } },
      }),
      'POST /api/campaigns/c1/hooks': (body) => ({
        status: 201,
        body: { data: { id: 'h1', ...(body as object) } },
      }),
    })
    renderTable()

    await screen.findByText('Marc')
    await userEvent.click(screen.getByRole('button', { name: /Accroches secrètes/ }))
    expect(await screen.findByText('Son frère Dorn a disparu dans la mine.')).toBeInTheDocument()
    await userEvent.type(screen.getByRole('textbox', { name: "L'accroche" }), 'Dorn est le mort qui marche')
    await userEvent.click(screen.getByRole('checkbox', { name: /Le mort qui marche/ }))
    await userEvent.click(screen.getByRole('button', { name: /Ajouter l'accroche/ }))

    expect(sentTo(api, 'POST /api/campaigns/c1/hooks')).toEqual([
      { characterId: 'k1', title: 'Dorn est le mort qui marche', body: '', links: ['f-mine'] },
    ])
    expect(await screen.findByText(/tirée de Borin/)).toBeInTheDocument()
  })
})
