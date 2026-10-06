import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { MapEditorPage } from './MapEditorPage'

const ROOM = {
  id: 'salle',
  name: 'Salle',
  theme: 'port-1718',
  ambience: {},
  grid: {
    legend: {
      '#': { terrain: 'pierre', wall: true },
      '.': { terrain: 'pierre' },
    },
    rows: ['#####', '#...#', '#####'],
  },
}

const saved = (map: unknown, validatedAt: string | null = null) => ({
  status: 200,
  body: {
    data: {
      map,
      source: 'editor',
      node: null,
      backdrop: false,
      validatedAt,
      updatedAt: '',
    },
  },
})

const cell = (x: number, y: number) => ({
  clientX: x * 32 + 5,
  clientY: y * 32 + 5,
})

describe('MapEditorPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('paints a wall, saves the map, then validates it for the table', async () => {
    stubReducedMotion(true)
    const mock = mockApi({
      'GET /api/campaigns/c1/maps/salle': () => saved(ROOM),
      'GET /api/campaigns/c1/media': () => ({
        status: 200,
        body: { data: { theme: null, assets: [] } },
      }),
      'PUT /api/campaigns/c1/maps/salle': (body) => saved(body),
      'POST /api/campaigns/c1/maps/salle/validate': () => saved(ROOM, '2026-10-06T10:00:00Z'),
    })
    render(
      <MemoryRouter initialEntries={['/campagnes/c1/cartes/salle']}>
        <Routes>
          <Route path="/campagnes/:campaignId/cartes/:mapId" element={<MapEditorPage />} />
        </Routes>
      </MemoryRouter>,
    )

    const canvas = await screen.findByRole('img', { name: 'Carte : Salle' })
    expect(screen.getByRole('button', { name: 'Enregistrer' })).toBeDisabled()
    // The wall brush is chosen first.
    fireEvent.pointerDown(canvas, cell(2, 1))
    fireEvent.pointerUp(canvas, cell(2, 1))
    await userEvent.click(screen.getByRole('radio', { name: 'Porte' }))
    fireEvent.pointerUp(canvas, cell(3, 1))
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))

    const [put] = sentTo(mock, 'PUT /api/campaigns/c1/maps/salle') as (typeof ROOM & { doors: unknown[] })[]
    expect(put.grid.rows).toEqual(['#####', '#.###', '#####'])
    expect(put.doors).toEqual([{ id: 'porte-1', at: [3, 1], state: 'closed', layer: 'base' }])

    await userEvent.click(await screen.findByRole('button', { name: 'Valider' }))
    expect(sentTo(mock, 'POST /api/campaigns/c1/maps/salle/validate')).toHaveLength(1)
    expect(await screen.findByText('Validée')).toBeInTheDocument()
  })
})
