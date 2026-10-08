import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { ReviewPage } from './ReviewPage'

const SHIPS = { map: 'large-de-belle-ile', ship: 'la_machoire', ships: [{ ship: 'hms_greyhound' }] }

function campaign(nodes: Record<string, unknown>[] = []) {
  return {
    id: 'c1',
    story: {
      id: 'corsaires',
      title: 'Corsaires',
      rules: { id: 'corsaires', version: 1 },
      bible: {},
      acts: [{ id: 'acte_1', title: 'Acte I' }],
      nodes: [
        { id: 'sc_quai', act: 'acte_1', title: 'Le quai', map: 'quai_retouche' },
        { id: 'sc_bureau', act: 'acte_1', title: 'Le bureau de Morel' },
        {
          id: 'sc_greyhound',
          act: 'acte_1',
          title: 'Le Greyhound',
          encounter: { opponents: [{ who: 'adv_garde', count: 2 }], vehicles: SHIPS },
        },
        ...nodes,
      ],
      revelations: [],
      clues: [],
      npcs: [
        { id: 'pnj_fanch', name: 'Fanch', disposition: 'hostile', stats: { from_rules: 'marin' } },
        { id: 'pnj_morel', name: 'Morel', disposition: 'neutral' },
      ],
      adversaries: [
        { id: 'adv_marin', name: 'Marin de Gueule-Rouge', stats: { from_rules: 'marin' } },
        { id: 'adv_garde', name: 'Garde royal', stats: { from_rules: 'garde' } },
      ],
      items: [{ id: 'obj_sabre', name: 'Sabre' }],
    },
    issues: [],
    validatedAt: null,
    updatedAt: '2026-10-08T10:00:00Z',
  }
}

const COMMON = {
  'GET /api/campaigns/c1': () => ({ status: 200, body: { data: campaign() } }),
  'GET /api/campaigns/c1/workshop': () => ({ status: 200, body: { data: [] } }),
  'GET /api/campaigns/c1/readiness': () => ({ status: 200, body: { data: [] } }),
}

function renderPage() {
  render(
    <MemoryRouter initialEntries={['/campagnes/c1/preparer']}>
      <Routes>
        <Route path="/campagnes/:campaignId/preparer" element={<ReviewPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

async function openScene(title: RegExp, tab: string) {
  await userEvent.click(await screen.findByRole('button', { name: title }))
  await userEvent.click(screen.getByRole('tab', { name: tab }))
}

describe('SceneSheet', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('plans a fight from the campaign’s adversaries and says when the server refuses it', async () => {
    let refuse = false
    const api = mockApi({
      ...COMMON,
      'POST /api/campaigns/c1/story/edits': () =>
        refuse
          ? {
              status: 400,
              body: {
                error: {
                  code: 'EDIT_SCENE_INVALID',
                  message: 'edit 0: REF_DANGLING nodes[0].encounter.opponents[0].who: `adv_kraken` names nothing',
                },
              },
            }
          : { status: 200, body: { data: { campaign: campaign(), changes: [] } } },
    })
    renderPage()
    await openScene(/Le quai/, 'Combat')

    await userEvent.click(screen.getByRole('button', { name: 'Un combat' }))
    const first = screen.getByRole('group', { name: 'Adversaire 1' })
    // Only adversaries and NPCs with numbers can be fought.
    const who = within(first).getByLabelText('Qui')
    expect(within(who).queryByRole('option', { name: 'Morel' })).toBeNull()
    await userEvent.selectOptions(who, 'Fanch')
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter : Adversaires' }))
    const second = screen.getByRole('group', { name: 'Adversaire 2' })
    const count = within(second).getByLabelText('Nombre')
    await userEvent.clear(count)
    await userEvent.type(count, '4')
    await userEvent.type(screen.getByLabelText('Comment ils se battent (une idée par ligne)'), 'Ils encerclent.')
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter : Butin' }))
    await userEvent.selectOptions(screen.getByLabelText('Objet'), 'Sabre')
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))

    expect(sentTo(api, 'POST /api/campaigns/c1/story/edits')).toEqual([
      {
        edits: [
          {
            op: 'set',
            target: 'sc_quai',
            field: 'encounter',
            value: {
              opponents: [{ who: 'pnj_fanch' }, { who: 'adv_marin', count: 4 }],
              tactics: ['Ils encerclent.'],
            },
          },
          { op: 'set', target: 'sc_quai', field: 'loot', value: [{ item: 'obj_sabre', coins: 10 }] },
        ],
      },
    ])

    // The server refuses: the GM reads why, with the id at fault.
    refuse = true
    await userEvent.click(screen.getByRole('button', { name: 'Un combat' }))
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))
    expect(await screen.findByText(/nommerait quelque chose qui n'existe pas/)).toBeInTheDocument()
    expect(screen.getByText(/adv_kraken/)).toBeInTheDocument()
  })

  it('keeps a ship battle when the boarding changes, and never drops it', async () => {
    const api = mockApi({
      ...COMMON,
      'POST /api/campaigns/c1/story/edits': () => ({
        status: 200,
        body: { data: { campaign: campaign(), changes: [] } },
      }),
    })
    renderPage()
    await openScene(/Le Greyhound/, 'Combat')

    expect(screen.getByRole('button', { name: 'Pas de combat' })).toBeDisabled()
    const count = within(screen.getByRole('group', { name: 'Adversaire 1' })).getByLabelText('Nombre')
    await userEvent.clear(count)
    await userEvent.type(count, '3')
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))
    expect(sentTo(api, 'POST /api/campaigns/c1/story/edits')).toEqual([
      {
        edits: [
          {
            op: 'set',
            target: 'sc_greyhound',
            field: 'encounter',
            value: { opponents: [{ who: 'adv_garde', count: 3 }], vehicles: SHIPS },
          },
        ],
      },
    ])
  })

  it('chooses the scene’s music, listens to it, and saves its ambience', async () => {
    stubReducedMotion(true)
    const api = mockApi({
      ...COMMON,
      'POST /api/campaigns/c1/story/edits': () => ({
        status: 200,
        body: { data: { campaign: campaign(), changes: [] } },
      }),
    })
    renderPage()
    await openScene(/Le bureau/, 'Son')

    await userEvent.type(screen.getByLabelText('Humeur'), 'Feutrée')
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter : Musique' }))
    await userEvent.selectOptions(screen.getByLabelText('Moment'), 'Mystère')
    await userEvent.type(screen.getByLabelText('Titre'), 'Bureau du négrier')
    // A track still to choose: its search is offered, nothing plays yet.
    expect(screen.getByRole('link', { name: 'Chercher sur YouTube' })).toHaveAttribute(
      'href',
      'https://www.youtube.com/results?search_query=Bureau%20du%20n%C3%A9grier',
    )
    expect(screen.queryByRole('button', { name: 'Écouter' })).toBeNull()
    await userEvent.type(screen.getByLabelText('Lien YouTube'), 'https://youtu.be/dQw4w9WgXcQ')
    await userEvent.click(screen.getByRole('button', { name: 'Écouter' }))
    expect(screen.getByRole('button', { name: 'Activer le son' })).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))
    expect(sentTo(api, 'POST /api/campaigns/c1/story/edits')).toEqual([
      {
        edits: [
          {
            op: 'set',
            target: 'sc_bureau',
            field: 'ambience',
            value: {
              mood: 'Feutrée',
              music: [{ mood: 'mystery', title: 'Bureau du négrier', url: 'https://youtu.be/dQw4w9WgXcQ' }],
            },
          },
        ],
      },
    ])
  })

  it('shows the scene’s image, the act’s video on its first scene, and generates the fight’s map', async () => {
    stubReducedMotion(true)
    const media = {
      theme: null,
      assets: [{ id: 'a1', kind: 'scene', subject: 'sc_quai', status: 'pending', direction: 'à l’aube' }],
      plan: {
        images: [],
        videos: [],
        imageMicros: 40000,
        videoMicros: 4000000,
        spending: { budgetMicros: 5000000, spentMicros: 0 },
        running: false,
        configured: true,
      },
    }
    const draft = {
      map: {
        id: 'quai_retouche',
        name: 'Quai retouché',
        theme: 'port-1718',
        ambience: {},
        grid: { legend: { '.': { terrain: 'pierre' } }, rows: ['...', '...', '...'] },
      },
      source: 'generated',
      node: 'sc_quai',
      backdrop: false,
      validatedAt: null,
      updatedAt: '',
    }
    const api = mockApi({
      ...COMMON,
      'GET /api/campaigns/c1/media': () => ({ status: 200, body: { data: media } }),
      'GET /api/campaigns/c1/maps': () => ({
        status: 200,
        body: { data: { maps: [draft], world: [{ id: 'quai-port-louis', name: 'Quai de Port-Louis' }] } },
      }),
      'POST /api/campaigns/c1/media/a1/decision': () => ({ status: 204 }),
      'POST /api/campaigns/c1/maps/quai_retouche/validate': () => ({
        status: 200,
        body: { data: { ...draft, validatedAt: '2026-10-08T10:00:00Z' } },
      }),
      'POST /api/campaigns/c1/maps/generate': () => ({
        status: 200,
        body: { data: { ...draft, map: { ...draft.map, id: 'quai_bis', name: 'Quai bis' }, dropped: 0 } },
      }),
      'POST /api/campaigns/c1/story/edits': () => ({
        status: 200,
        body: { data: { campaign: campaign(), changes: [] } },
      }),
    })
    renderPage()
    await openScene(/Le quai/, 'Visuels')

    // The image waiting for the GM, kept from the sheet.
    const image = await screen.findByRole('article', { name: 'Le quai' })
    await userEvent.click(within(image).getByRole('button', { name: 'Garder' }))
    expect(sentTo(api, 'POST /api/campaigns/c1/media/a1/decision')).toEqual([{ approve: true }])
    // The first scene of the act carries the act's video.
    expect(screen.getByRole('article', { name: 'Acte I' })).toBeInTheDocument()

    // The draft map: seen, validated for the table.
    expect(screen.getByText(/brouillon : la table ne la verra pas/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Valider la carte' }))
    expect(sentTo(api, 'POST /api/campaigns/c1/maps/quai_retouche/validate')).toHaveLength(1)

    // A new map drawn for the scene is linked to it.
    await userEvent.click(screen.getByRole('button', { name: 'Générer une carte pour cette scène' }))
    expect(sentTo(api, 'POST /api/campaigns/c1/maps/generate')).toEqual([{ node: 'sc_quai' }])
    expect(sentTo(api, 'POST /api/campaigns/c1/story/edits')).toEqual([
      { edits: [{ op: 'set', target: 'sc_quai', field: 'map', value: 'quai_bis' }] },
    ])

    // Another scene of the act has no video of its own.
    await userEvent.click(screen.getByRole('button', { name: /Le bureau/ }))
    await userEvent.click(screen.getByRole('tab', { name: 'Visuels' }))
    await screen.findByRole('article', { name: 'Le bureau de Morel' })
    expect(screen.queryByRole('article', { name: 'Acte I' })).toBeNull()
  })
})
