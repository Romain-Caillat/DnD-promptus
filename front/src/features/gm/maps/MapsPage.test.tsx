import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { MapsPage } from './MapsPage'

const STORY = {
  id: 'mine',
  title: 'La mine',
  rules: { id: 'corsaires', version: 1 },
  bible: {},
  acts: [{ id: 'acte_1', title: 'La descente' }],
  nodes: [
    { id: 'sc_puits', act: 'acte_1', title: 'Le puits' },
    {
      id: 'sc_galerie',
      act: 'acte_1',
      title: 'La galerie',
      encounter: { opponents: [] },
    },
  ],
  npcs: [],
}

const MADE = (id: string, source: string) => ({
  map: {
    id,
    name: 'Carte',
    theme: 'port-1718',
    ambience: {},
    grid: { legend: {}, rows: [] },
  },
  source,
  node: null,
  backdrop: false,
  validatedAt: null,
  updatedAt: '',
})

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1/cartes']}>
      <Routes>
        <Route path="/campagnes/:campaignId/cartes" element={<MapsPage />} />
        <Route path="/campagnes/:campaignId/cartes/:mapId" element={<p>éditeur ouvert</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

function api(extra: Parameters<typeof mockApi>[0] = {}) {
  return mockApi({
    'GET /api/campaigns/c1': () => ({
      status: 200,
      body: {
        data: {
          id: 'c1',
          story: STORY,
          issues: [],
          validatedAt: null,
          updatedAt: '',
        },
      },
    }),
    'GET /api/campaigns/c1/maps': () => ({
      status: 200,
      body: {
        data: {
          maps: [
            {
              ...MADE('la-cale', 'imported'),
              map: { ...MADE('la-cale', 'imported').map, name: 'La cale' },
            },
          ],
          world: [{ id: 'quai', name: 'Le quai' }],
        },
      },
    }),
    ...extra,
  })
}

describe('MapsPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('lists the campaign’s maps and has the co-GM draft one for a scene, opened in the editor', async () => {
    const mock = api({
      'POST /api/campaigns/c1/maps/generate': () => ({
        status: 200,
        body: { data: { ...MADE('la-galerie', 'generated'), dropped: 1 } },
      }),
    })
    renderPage()

    expect(await screen.findByText('La cale')).toBeInTheDocument()
    expect(screen.getByText(/Importée · Brouillon/)).toBeInTheDocument()
    // Fights come first in the list of scenes.
    const scenes = screen.getAllByRole('option').map((o) => o.textContent)
    expect(scenes.slice(1, 3)).toEqual(['La galerie — combat', 'Le puits'])
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Scène' }), 'sc_galerie')
    await userEvent.click(screen.getByRole('button', { name: 'Générer' }))
    expect(sentTo(mock, 'POST /api/campaigns/c1/maps/generate')).toEqual([{ node: 'sc_galerie' }])
    expect(await screen.findByText('éditeur ouvert')).toBeInTheDocument()
  })

  it('says why the model’s map was refused', async () => {
    api({
      'POST /api/campaigns/c1/maps/generate': () => ({
        status: 502,
        body: { error: { code: 'AI_OUTPUT_INVALID', message: 'bad' } },
      }),
    })
    renderPage()
    await userEvent.selectOptions(await screen.findByRole('combobox', { name: 'Scène' }), 'sc_puits')
    await userEvent.click(screen.getByRole('button', { name: 'Générer' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(/n’a pas produit une carte valide/)
  })

  it('sends a Dungeondraft export as it is', async () => {
    const mock = api({
      'POST /api/campaigns/c1/maps/import': () => ({
        status: 201,
        body: { data: MADE('crypte', 'imported') },
      }),
    })
    renderPage()
    const file = new File(['{"format":0.3}'], 'crypte.dd2vtt', {
      type: 'application/json',
    })
    await userEvent.upload(await screen.findByLabelText('Fichier'), file)
    expect(await screen.findByText('éditeur ouvert')).toBeInTheDocument()
    expect(sentTo(mock, 'POST /api/campaigns/c1/maps/import')).toEqual([
      { kind: 'uvtt', name: 'crypte', file: '{"format":0.3}' },
    ])
  })

  it('lays a grid on an image before importing it', async () => {
    // jsdom decodes no image: this one says it is 1000 × 800.
    vi.stubGlobal(
      'Image',
      class {
        naturalWidth = 1000
        naturalHeight = 800
        onload: (() => void) | null = null
        set src(_: string) {
          setTimeout(() => this.onload?.())
        }
      },
    )
    const mock = api({
      'POST /api/campaigns/c1/maps/import': () => ({
        status: 201,
        body: { data: MADE('plan', 'imported') },
      }),
    })
    renderPage()
    await userEvent.type(await screen.findByRole('textbox', { name: 'Nom' }), 'Le phare')
    await userEvent.upload(
      screen.getByLabelText('Fichier'),
      new File([new Uint8Array([137, 80, 78, 71])], 'plan.png', {
        type: 'image/png',
      }),
    )

    expect(await screen.findByText('Caler « Le phare » sur la grille')).toBeInTheDocument()
    expect(screen.getByText('20 × 16 cases')).toBeInTheDocument()
    const cell = screen.getByRole('spinbutton', {
      name: 'Taille d’une case (pixels)',
    })
    await userEvent.clear(cell)
    await userEvent.type(cell, '400')
    expect(screen.getByText(/entre 3 et 64 cases/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Importer' })).toBeDisabled()
    await userEvent.clear(cell)
    await userEvent.type(cell, '64')
    const offset = screen.getByRole('spinbutton', {
      name: 'Décalage horizontal (pixels)',
    })
    await userEvent.clear(offset)
    await userEvent.type(offset, '40')
    await userEvent.click(screen.getByRole('button', { name: 'Importer' }))

    expect(await screen.findByText('éditeur ouvert')).toBeInTheDocument()
    const [sent] = sentTo(mock, 'POST /api/campaigns/c1/maps/import') as Record<string, unknown>[]
    expect(sent).toMatchObject({
      kind: 'image',
      name: 'Le phare',
      cellPx: 64,
      offsetX: 40,
      offsetY: 0,
      columns: 15,
      rows: 12,
    })
    expect(String(sent.image)).toMatch(/^data:image\/png;base64,/)
  })
})
