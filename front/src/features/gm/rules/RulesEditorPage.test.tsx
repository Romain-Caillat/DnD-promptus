import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { RulesEditorPage } from './RulesEditorPage'

const EMPTY_REPORT: Record<string, unknown[]> = {
  lint: [],
  lintAdded: [],
  lintRemoved: [],
  changes: [],
  story: [],
  fights: [],
}

const PRESET_V1 = {
  version: 1,
  note: '',
  lockedAt: null,
  createdAt: '2026-10-01T10:00:00Z',
  updatedAt: '2026-10-01T10:00:00Z',
  preset: true,
}

function editor(draft: unknown = null, next: number | null = null) {
  return {
    rulesId: 'corsaires',
    name: 'Corsaires de la Couronne',
    current: 1,
    next,
    versions: draft ? [{ ...PRESET_V1, version: 2, preset: false }, PRESET_V1] : [PRESET_V1],
    draft,
  }
}

function draft(houseRules: unknown[] = [], report = EMPTY_REPORT) {
  return {
    version: 2,
    note: '',
    yaml: 'id: corsaires\nversion: 2\n',
    document: { id: 'corsaires', version: 2, house_rules: houseRules },
    report,
  }
}

const FIRE = { id: 'le_feu_effraie', name: 'Le feu effraie', text: 'Un marin touché fuit un tour.' }

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1/regles']}>
      <Routes>
        <Route path="/campagnes/:campaignId/regles" element={<RulesEditorPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('RulesEditorPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('drafts the rules, adds a house rule, shows what it touches, and locks it', async () => {
    const touched = {
      ...EMPTY_REPORT,
      changes: [
        {
          section: 'house_rules',
          subject: 'Le feu effraie',
          field: 'added',
          from: null,
          to: null,
        },
      ],
    }
    const api = mockApi({
      'GET /api/campaigns/c1/rules': () => ({ status: 200, body: { data: editor() } }),
      'POST /api/campaigns/c1/rules/draft': () => ({ status: 200, body: { data: editor(draft()) } }),
      'PUT /api/campaigns/c1/rules/draft': () => ({
        status: 200,
        body: { data: editor(draft([FIRE], touched)) },
      }),
      'POST /api/campaigns/c1/rules/draft/lock': () => ({ status: 200, body: { data: editor(null, 2) } }),
    })
    renderPage()

    await userEvent.click(await screen.findByRole('button', { name: /Commencer un brouillon/ }))
    expect(await screen.findByText('Brouillon v2')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Règles maison' }))
    await userEvent.type(screen.getByLabelText('Nom'), 'Le feu effraie')
    await userEvent.type(screen.getByLabelText('Texte lu par les joueurs'), 'Un marin touché fuit un tour.')
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter' }))
    expect(screen.getByText('Modifications non enregistrées')).toBeInTheDocument()
    // Unsaved edits cannot be locked.
    expect(screen.getByRole('button', { name: /Verrouiller la v2/ })).toBeDisabled()

    await userEvent.click(screen.getByRole('button', { name: /Enregistrer le brouillon/ }))
    expect(await screen.findByText('1 changement lu par les joueurs')).toBeInTheDocument()
    expect(sentTo(api, 'PUT /api/campaigns/c1/rules/draft')).toEqual([
      { document: { id: 'corsaires', version: 2, house_rules: [FIRE] }, note: '' },
    ])

    await userEvent.click(screen.getByRole('button', { name: 'Tester' }))
    expect(screen.getByText('Règle maison · Le feu effraie')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: /Verrouiller la v2/ }))
    expect(await screen.findByText(/La version 2 est verrouillée/)).toBeInTheDocument()
  })

  it('says why a text that does not load is refused', async () => {
    mockApi({
      'GET /api/campaigns/c1/rules': () => ({ status: 200, body: { data: editor(draft()) } }),
      'PUT /api/campaigns/c1/rules/draft': () => ({
        status: 400,
        body: { error: { code: 'RULES_INVALID', message: 'invalid' } },
      }),
    })
    renderPage()

    await userEvent.click(await screen.findByRole('button', { name: 'Texte' }))
    await userEvent.type(screen.getByRole('textbox', { name: 'Texte' }), 'oops: {{')
    await userEvent.click(screen.getByRole('button', { name: /Enregistrer le brouillon/ }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Ce texte ne se charge pas')
  })
})
