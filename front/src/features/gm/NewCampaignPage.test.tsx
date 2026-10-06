import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes, useParams } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { NewCampaignPage } from './NewCampaignPage'

const PRESETS = [
  {
    id: 'corsaires',
    version: 1,
    name: 'Corsaires de la Couronne',
    description: 'One-shot pirate, 1718.',
    abilities: [
      { abbr: 'FOR', name: 'Force' },
      { abbr: 'DEX', name: 'Dextérité' },
    ],
    hitPoints: { abbr: 'PV', name: 'Points de vie' },
    armorClass: { abbr: 'CA', name: "Classe d'armure" },
  },
  {
    id: 'brasier',
    version: 1,
    name: 'Le Brasier',
    description: 'Campagne spatiale.',
    abilities: [
      { abbr: 'FOR', name: 'Force' },
      { abbr: 'INT', name: 'Intelligence' },
    ],
    hitPoints: { abbr: 'PV', name: 'Points de vie' },
    armorClass: { abbr: 'CA', name: "Classe d'armure" },
  },
]

function Opened() {
  const { campaignId } = useParams()
  return <p>campagne {campaignId}</p>
}

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/nouvelle']}>
      <Routes>
        <Route path="/campagnes/nouvelle" element={<NewCampaignPage />} />
        <Route path="/campagnes/:campaignId" element={<Opened />} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('NewCampaignPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('creates a campaign on the chosen rules, with its pitch and settings, then opens it', async () => {
    const api = mockApi({
      'GET /api/rule-systems': () => ({ status: 200, body: { data: PRESETS } }),
      'POST /api/campaigns': () => ({ status: 201, body: { data: { id: 'c9' } } }),
    })
    renderPage()
    const next = () => userEvent.click(screen.getByRole('button', { name: /Suivant/ }))

    // A title is needed before the rules.
    await next()
    expect(screen.getByRole('alert')).toHaveTextContent('Donne un titre à la campagne.')
    await userEvent.type(screen.getByLabelText('Titre'), 'Les Cendres de Valombre')
    await userEvent.type(screen.getByLabelText('Univers'), 'Fantasy minière')
    await next()

    // The rules step shows the stat names each system brings.
    const brasier = await screen.findByRole('button', { name: /Le Brasier/ })
    expect(brasier).toHaveTextContent('INT · Intelligence')
    expect(screen.getByRole('button', { name: /Corsaires de la Couronne/ })).toHaveAttribute('aria-pressed', 'true')
    await userEvent.click(brasier)
    expect(brasier).toHaveAttribute('aria-pressed', 'true')
    await next()

    await userEvent.type(screen.getByLabelText(/Ton idée/), 'Une ville qui enterre ses morts deux fois.')
    await userEvent.type(screen.getByLabelText(/L’accroche/), 'Le maire vous a fait venir.')
    await userEvent.clear(screen.getByLabelText('Joueurs'))
    await userEvent.type(screen.getByLabelText('Joueurs'), '5')
    await userEvent.clear(screen.getByLabelText('Budget IA ($)'))
    await userEvent.type(screen.getByLabelText('Budget IA ($)'), '12,50')
    await userEvent.click(screen.getByRole('button', { name: /Créer la campagne/ }))

    expect(await screen.findByText('campagne c9')).toBeInTheDocument()
    expect(sentTo(api, 'POST /api/campaigns')).toEqual([
      {
        title: 'Les Cendres de Valombre',
        world: 'Fantasy minière',
        rules: { id: 'brasier', version: 1 },
        pitch: 'Une ville qui enterre ses morts deux fois.',
        playerHook: 'Le maire vous a fait venir.',
        playerCount: 5,
        aiBudgetCents: 1250,
      },
    ])
  })

  it('refuses a table size out of range without sending anything', async () => {
    const api = mockApi({
      'GET /api/rule-systems': () => ({ status: 200, body: { data: PRESETS } }),
    })
    renderPage()
    await userEvent.type(screen.getByLabelText('Titre'), 'Valombre')
    await userEvent.click(screen.getByRole('button', { name: /Suivant/ }))
    await screen.findByRole('button', { name: /Le Brasier/ })
    await userEvent.click(screen.getByRole('button', { name: /Suivant/ }))
    await userEvent.clear(screen.getByLabelText('Joueurs'))
    await userEvent.type(screen.getByLabelText('Joueurs'), '13')
    await userEvent.click(screen.getByRole('button', { name: /Créer la campagne/ }))

    expect(screen.getByRole('alert')).toHaveTextContent('Le nombre de joueurs va de 1 à 12.')
    expect(sentTo(api, 'POST /api/campaigns')).toHaveLength(0)
  })
})
