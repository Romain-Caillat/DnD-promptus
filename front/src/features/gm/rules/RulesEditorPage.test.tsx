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
  houseRules: [],
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
    document: {
      id: 'corsaires',
      version: 2,
      house_rules: houseRules,
      conditions: [{ id: 'effraye', name: 'Effrayé' }],
      traits: [{ id: 'mort_vivant', name: 'Mort-vivant' }],
      damage_types: [{ id: 'feu', name: 'Feu' }],
    },
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
      { document: { ...draft().document, house_rules: [FIRE] }, note: '' },
    ])

    await userEvent.click(screen.getByRole('button', { name: 'Tester' }))
    expect(screen.getByText('Règle maison · Le feu effraie')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: /Verrouiller la v2/ }))
    expect(await screen.findByText(/La version 2 est verrouillée/)).toBeInTheDocument()
  })

  it('has the co-GM formalise a house rule, shows its cases, and adds it to the draft', async () => {
    const formal = {
      when: 'hit',
      damage_type: 'feu',
      target: { traits: ['mort_vivant'] },
      effects: [{ apply: { condition: 'effraye', turns: 1, to: 'targets' } }],
      players: 'effect',
      cases: [],
    }
    const proposal = {
      id: FIRE.id,
      formal,
      problems: [],
      dropped: [{ kind: 'trait', id: 'chef_invente' }],
      remark: 'Les chefs aussi ? Dis-le si tu veux les exclure.',
      cases: [
        {
          name: 'Torche contre zombie',
          expect: 'applies',
          natural: 19,
          fired: true,
          passed: true,
          effects: [{ event: 'condition_applied', target: 'target', name: 'Effrayé', turns: 1 }],
          error: null,
        },
        {
          name: 'Épée contre zombie',
          expect: 'nothing',
          natural: 19,
          fired: false,
          passed: true,
          effects: [],
          error: null,
        },
      ],
    }
    const api = mockApi({
      'GET /api/campaigns/c1/rules': () => ({ status: 200, body: { data: editor(draft([FIRE])) } }),
      'POST /api/campaigns/c1/rules/house-rules/formalise': () => ({ status: 200, body: { data: proposal } }),
      'PUT /api/campaigns/c1/rules/draft': () => ({ status: 200, body: { data: editor(draft([FIRE])) } }),
    })
    renderPage()

    await userEvent.click(await screen.findByRole('button', { name: 'Règles maison' }))
    expect(screen.getByText(/Pas encore formalisée/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Formaliser avec le co-MJ/ }))

    const card = await screen.findByRole('region', { name: 'La règle formalisée' })
    expect(sentTo(api, 'POST /api/campaigns/c1/rules/house-rules/formalise')).toEqual([
      { id: FIRE.id, name: FIRE.name, text: FIRE.text },
    ])
    expect(card).toHaveTextContent("une attaque touche · avec des dégâts de type Feu · la cible : avec l'étiquette Mort-vivant")
    expect(card).toHaveTextContent('Effrayé pendant 1 tour(s), à la cible')
    expect(card).toHaveTextContent("voient l'effet, pas la règle")
    expect(card).toHaveTextContent("Retiré, inconnu de tes règles : l'étiquette « chef_invente »")
    expect(card).toHaveTextContent('Torche contre zombie')
    expect(card).toHaveTextContent('la cible : Effrayé')
    expect(card).toHaveTextContent('Les chefs aussi ?')

    // The GM lets players read the rule, then adds it.
    await userEvent.click(screen.getByRole('checkbox', { name: 'Montrer la règle aux joueurs' }))
    await userEvent.click(screen.getByRole('button', { name: /Ajouter la règle/ }))
    expect(screen.getByText(/Formalisée : le serveur l'applique/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Enregistrer le brouillon/ }))
    await screen.findByText('Brouillon v2')
    const [saved] = sentTo(api, 'PUT /api/campaigns/c1/rules/draft') as { document: { house_rules: unknown[] } }[]
    expect(saved.document.house_rules).toEqual([{ ...FIRE, formal: { ...formal, players: 'rule' } }])
  })

  it('says why the co-GM could not formalise, with the editor\'s own words', async () => {
    mockApi({
      'GET /api/campaigns/c1/rules': () => ({ status: 200, body: { data: editor(draft([FIRE])) } }),
      'POST /api/campaigns/c1/rules/house-rules/formalise': () => ({
        status: 400,
        body: { error: { code: 'RULES_INVALID', message: 'invalid' } },
      }),
    })
    renderPage()

    await userEvent.click(await screen.findByRole('button', { name: 'Règles maison' }))
    await userEvent.click(screen.getByRole('button', { name: /Formaliser avec le co-MJ/ }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Ce texte ne se charge pas')
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
