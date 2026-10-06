import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mockApi, stubReducedMotion } from '@/test-utils'
import { TvShowPage } from './TvShowPage'

const CAMPAIGN = { title: 'Les Cendres de Valombre', world: 'W', playerHook: '', clues: [], npcs: [] }
const SCENE = {
  id: 'sc_porte',
  act: 'acte_1',
  title: 'La porte nord',
  readAloud: 'Une porte de bronze, plus haute que la crypte.',
  place: null,
  npcs: [],
  opponents: [],
}
const evening = (over: Record<string, unknown>) => ({
  status: 200,
  body: {
    data: {
      session: { number: 4, status: 'live', startedAt: '2026-10-10T18:30:00Z' },
      previously: null,
      reading: null,
      music: null,
      lobby: [],
      campaign: { ...CAMPAIGN, scene: SCENE },
      journal: [],
      requests: [],
      cards: [],
      feedback: null,
      ...over,
    },
  },
})
const MEDIA = { 'GET /api/play/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }) }
const NO_BOARD = { 'GET /api/play/c1/board': () => ({ status: 200, body: { data: null } }) }

function renderShow() {
  render(
    <MemoryRouter initialEntries={['/tv/c1']}>
      <Routes>
        <Route path="/tv" element={<p>nouveau code</p>} />
        <Route path="/tv/:campaignId" element={<TvShowPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('TvShowPage', () => {
  beforeEach(() => {
    stubReducedMotion(true)
    vi.stubGlobal(
      'WebSocket',
      class {
        close() {}
        addEventListener() {}
        removeEventListener() {}
      },
    )
  })
  afterEach(() => {
    vi.unstubAllGlobals()
    localStorage.clear()
  })

  it('shows the lobby while the table gathers', async () => {
    mockApi({
      ...MEDIA,
      ...NO_BOARD,
      'GET /api/play/c1/evening': () =>
        evening({
          session: { number: 4, status: 'lobby', startedAt: null },
          lobby: [{ playerId: 'p1', nickname: 'Marc', soundOk: true, remote: false }],
        }),
    })
    renderShow()
    const here = await screen.findByRole('list', { name: 'Dans le salon' })
    expect(here).toHaveTextContent('Marc')
    expect(screen.getByText('En attente du MJ…')).toBeInTheDocument()
  })

  it('follows the reading of « Précédemment… », line by line', async () => {
    mockApi({
      ...MEDIA,
      ...NO_BOARD,
      'GET /api/play/c1/evening': () =>
        evening({
          previously: 'Sous l’autel… La porte nord.',
          reading: { lines: ['Sous l’autel, Borin a trouvé la page.', 'La porte nord attend.'], shown: 1 },
          journal: [{ kind: 'clue', text: 'La page arrachée', createdAt: '' }],
        }),
    })
    renderShow()
    await screen.findByRole('list', { name: 'Précédemment…' })
    // Only the lines read so far reach the room (and its screen readers).
    expect(screen.getAllByRole('listitem').map((l) => l.textContent)).toEqual(['Sous l’autel, Borin a trouvé la page.'])
    const lines = screen.getAllByRole('listitem', { hidden: true })
    expect(lines[0]).toHaveClass('opacity-100')
    expect(lines[1]).toHaveClass('opacity-0')
    expect(lines[1]).toHaveAttribute('aria-hidden', 'true')
    // The scene waits behind the reading; the feed too.
    expect(screen.queryByText('La porte nord', { selector: 'h2' })).toBeNull()
    expect(screen.queryByText('La page arrachée')).toBeNull()
  })

  it('shows the scene and the last shared moment', async () => {
    mockApi({
      ...MEDIA,
      ...NO_BOARD,
      'GET /api/play/c1/evening': () =>
        evening({
          journal: [
            { kind: 'scene', text: 'La porte nord', createdAt: '' },
            { kind: 'clue', text: 'Un corbeau sur le linteau', createdAt: '' },
          ],
        }),
    })
    renderShow()
    expect(await screen.findByRole('heading', { name: 'La porte nord' })).toBeInTheDocument()
    expect(screen.getByText(SCENE.readAloud)).toBeInTheDocument()
    expect(screen.getByRole('contentinfo', { name: 'Dernier moment' })).toHaveTextContent('Un corbeau sur le linteau')
  })

  it('goes back to a fresh code once the GM forgot it', async () => {
    localStorage.setItem('promptus.tv.campaign', 'c1')
    mockApi({
      ...MEDIA,
      ...NO_BOARD,
      'GET /api/play/c1/evening': () => ({ status: 401, body: { error: { code: 'NOT_JOINED' } } }),
    })
    renderShow()
    expect(await screen.findByText('nouveau code')).toBeInTheDocument()
    expect(localStorage.getItem('promptus.tv.campaign')).toBeNull()
  })
})
