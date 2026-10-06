import { fireEvent, render, screen, within } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import type { RulesView } from '@/lib/rules'
import { RulesPage } from './RulesPage'

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/partie/c1/regles']}>
      <Routes>
        <Route path="/partie/:campaignId/regles" element={<RulesPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

const FOR = { from: 'ability' as const, id: 'FOR' }

/** The Corsaires as the server projects them for a bretteur with FOR 13. */
function corsaires(over: Partial<RulesView> = {}): RulesView {
  return {
    name: 'Corsaires de la Couronne',
    version: 2,
    check: { die: '1d20', faces: 20, advantage: false, modifier: '(score - 10) / 2' },
    abilities: [
      {
        id: 'FOR',
        name: 'Force',
        description: 'Effort physique brut.',
        score: 13,
        modifier: 1,
        needs: [
          { difficulty: 'moyen', fromFace: 9 },
          { difficulty: 'tres_difficile', fromFace: 19 },
        ],
      },
    ],
    difficulties: [
      { id: 'moyen', name: 'Moyen', value: 10, description: 'Escalader un mur.' },
      { id: 'tres_difficile', name: 'Très difficile', value: 20, description: 'Bluffer le gouverneur.' },
    ],
    outcomes: [
      { band: 'critical_failure', name: 'Échec critique', description: 'Ça tourne mal.', natural: [1], xp: 0, damageMultiplier: null },
      { band: 'failure', name: 'Échec', description: 'Ça ne marche pas.', natural: [], xp: 0, damageMultiplier: null },
      { band: 'success', name: 'Réussite', description: 'Ça fonctionne.', natural: [], xp: 1, damageMultiplier: null },
      { band: 'critical_success', name: 'Réussite critique', description: 'Dégâts doublés.', natural: [20], xp: 1, damageMultiplier: 2 },
    ],
    attack: {
      ability: 'first_primary',
      precisionApplies: false,
      armorClass: { name: "Classe d'armure", abbr: 'CA', formula: '10 + mod(DEX)' },
    },
    hitPoints: { name: 'Points de vie', abbr: 'PV', formula: '10' },
    turns: [{ name: 'Combat', actionsPerTurn: 2, limits: [] }],
    actionKinds: [{ name: 'Attaquer', description: 'Une attaque de classe.', cost: 1 }],
    cooldown: 'skip_next_turns',
    applicationTurnCounts: false,
    conditions: [{ name: 'Empoisonné', description: '-1 à tous ses jets.', kind: 'bane' }],
    zeroHp: { rule: 'knockedOut', condition: 'Inconscient', outAfterTurns: 3, outCondition: 'Hors combat' },
    progression: { upgradeEveryXp: 5, upgradePoints: 1, levels: [{ level: 1, xp: 0 }] },
    combat: {
      moveKind: { name: 'Se déplacer', description: '', cost: 1 },
      flee: { kind: { name: 'Fuir', description: '', cost: 2 }, ability: 'Dextérité' },
      coverHalf: -2,
      coverThreeQuarters: -5,
      longRangeDisadvantage: false,
      longRangeModifier: -2,
    },
    groupCheck: 'at_least_half',
    mine: {
      className: 'Bretteur',
      cards: [
        {
          id: 'riposte',
          name: 'Riposte en quarte',
          description: 'Pare puis contre-attaque.',
          kind: 'Attaquer',
          level: 1,
          attackBonus: 1,
          damage: '4',
          heal: null,
          cooldown: 1,
          range: 1,
          cost: 1,
          attackModifiers: [{ source: FOR, value: 1 }],
        },
      ],
      exampleAbility: 'Force',
      exampleDifficulty: 'Moyen',
      examples: [
        {
          die: '1d20',
          faces: [8],
          natural: 8,
          advantage: 'normal',
          modifiers: [{ source: FOR, value: 1 }],
          total: 9,
          target: { against: 'difficulty', id: 'moyen', value: 10 },
          band: 'failure',
        },
        {
          die: '1d20',
          faces: [20],
          natural: 20,
          advantage: 'normal',
          modifiers: [{ source: FOR, value: 1 }],
          total: 21,
          target: { against: 'difficulty', id: 'moyen', value: 10 },
          band: 'critical_success',
        },
      ],
    },
    changes: null,
    ...over,
  }
}

describe('RulesPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('explains a missed roll from its own numbers, and what each card costs', async () => {
    mockApi({ 'GET /api/play/c1/rules': () => ({ status: 200, body: { data: corsaires() } }) })
    renderPage()

    expect(await screen.findByRole('heading', { name: 'Corsaires de la Couronne' })).toBeInTheDocument()
    // The worked roll says why it failed: one short of the difficulty.
    expect(screen.getByText('9 contre Moyen (10) : il manquait 1.')).toBeInTheDocument()
    expect(screen.getByText('Un 20 sur le dé : réussi, quel que soit le total.')).toBeInTheDocument()
    // What the player needs on the die with their Force.
    const force = screen.getByRole('rowheader', { name: /Force/ }).closest('tr')!
    expect(within(force).getByText('9+')).toBeInTheDocument()
    // Each card: cost in actions, cooldown, what its attack adds.
    const card = screen.getByText('Riposte en quarte', { selector: 'b' }).parentElement!
    expect(within(card).getByText('Coûte 1 action')).toBeInTheDocument()
    expect(within(card).getByText('Recharge : 1 tour')).toBeInTheDocument()
    expect(within(card).getByText("Jet d'attaque : 1d20 +1 Force contre la CA de la cible.")).toBeInTheDocument()
    // The natural 1 misses whatever the total.
    expect(screen.getByText('Sur le dé 1 : raté, même si le total dépasse la classe d\'armure.')).toBeInTheDocument()
  })

  it('shows what changed until the player says they read it', async () => {
    const changed = corsaires({
      changes: {
        fromVersion: 1,
        toVersion: 2,
        replaced: false,
        items: [
          {
            section: 'cards',
            subject: 'Bretteur · Riposte en quarte',
            field: 'damage',
            from: { kind: 'text', value: '3' },
            to: { kind: 'text', value: '4' },
          },
        ],
      },
    })
    const fetchMock = mockApi({
      'GET /api/play/c1/rules': () => ({ status: 200, body: { data: changed } }),
      'POST /api/play/c1/rules/seen': () => ({ status: 200, body: { data: corsaires() } }),
    })
    renderPage()

    expect(await screen.findByRole('heading', { name: 'Ce qui change · version 1 → 2' })).toBeInTheDocument()
    const line = screen.getByRole('listitem')
    expect(line).toHaveTextContent('Carte · Bretteur · Riposte en quarte dégâts : 3 → 4')

    fireEvent.click(screen.getByRole('button', { name: /J'ai lu ce qui change/ }))
    expect(await screen.findByText('Tout ce que les règles disent, en une page')).toBeInTheDocument()
    expect(screen.queryByText(/Ce qui change/)).not.toBeInTheDocument()
    expect(sentTo(fetchMock, 'POST /api/play/c1/rules/seen')).toHaveLength(1)
  })

  it('reads the rules to a spectator without numbers of their own', async () => {
    mockApi({
      'GET /api/play/c1/rules': () => ({ status: 200, body: { data: corsaires({ mine: null }) } }),
    })
    renderPage()

    expect(await screen.findByText('Un jet de dé')).toBeInTheDocument()
    expect(screen.queryByText('Tes jets')).not.toBeInTheDocument()
    expect(screen.queryByText('Recharge : 1 tour')).not.toBeInTheDocument()
  })
})
