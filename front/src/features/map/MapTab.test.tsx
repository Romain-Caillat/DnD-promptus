import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { MapTab } from './MapTab'

const MAP = {
  id: 'quai',
  name: 'Le quai',
  theme: 'port-1718',
  ambience: { weather: 'fog' },
  grid: { legend: { '.': { terrain: 'pavés' } }, rows: ['.....', '.....'] },
}
const fighter = (id: string, name: string, party: boolean, mine = false) => ({
  id,
  name,
  party,
  standing: 'in_fight',
  at: party ? [0, 0] : [3, 0],
  hitPoints: party ? 9 : null,
  maxHitPoints: party ? 12 : null,
  down: false,
  conditions: [],
  mine,
})
const board = (fight: unknown = null) => ({
  status: 200,
  body: {
    data: {
      map: MAP,
      fog: true,
      tokens: [
        { id: 'pc-1', name: 'Borin', at: [0, 0], party: true, mine: true, ghost: false },
        { id: 'marin-1', name: 'Marin', at: [3, 0], party: false, mine: false, ghost: false },
      ],
      reachable: [
        { at: [1, 0], cost: 1 },
        { at: [2, 1], cost: 2 },
      ],
      fight,
    },
  },
})
const MEDIA = { 'GET /api/play/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }) }

/** A tap on cell (x, y) of the canvas (32 px a cell, drawn at scale 1). */
function tap(x: number, y: number) {
  const canvas = screen.getByRole('img', { name: 'Carte : Le quai' })
  fireEvent.pointerUp(canvas, { clientX: x * 32 + 5, clientY: y * 32 + 5 })
}

describe('MapTab', () => {
  beforeEach(() => stubReducedMotion(true))
  afterEach(() => vi.unstubAllGlobals())

  it('walks the character to a highlighted cell, along a path the server checks', async () => {
    const fetchMock = mockApi({
      ...MEDIA,
      'GET /api/play/c1/board': () => board(),
      'POST /api/play/c1/board/walk': () => board(),
    })
    render(<MapTab campaignId="c1" refreshKey={0} />)
    expect(await screen.findByRole('heading', { name: 'Le quai' })).toBeInTheDocument()
    expect(screen.getByText('Brouillard')).toBeInTheDocument()
    tap(2, 1)
    await vi.waitFor(() =>
      expect(sentTo(fetchMock, 'POST /api/play/c1/board/walk')).toEqual([
        {
          path: [
            [1, 0],
            [2, 1],
          ],
        },
      ]),
    )
    // Not reachable: nothing is sent.
    tap(4, 1)
    expect(sentTo(fetchMock, 'POST /api/play/c1/board/walk')).toHaveLength(1)
  })

  it('plays a card on a target picked on the map, on my turn', async () => {
    const fight = {
      live: true,
      round: 1,
      active: 'pc-1',
      myTurn: true,
      order: [fighter('pc-1', 'Borin', true, true), fighter('marin-1', 'Marin', false)],
      actionsLeft: 1,
      cards: [
        {
          id: 'estocade',
          name: 'Estocade',
          description: '',
          kind: 'Attaque',
          target: 'one',
          area: null,
          range: 3,
          attackBonus: 3,
          locked: false,
        },
      ],
      events: [{ kind: 'turn_started', who: 'pc-1', round: 1 }],
      loot: [],
      won: null,
    }
    const fetchMock = mockApi({
      ...MEDIA,
      'GET /api/play/c1/board': () => board(fight),
      'POST /api/play/c1/fight': () => board(fight),
    })
    render(<MapTab campaignId="c1" refreshKey={0} />)
    expect(await screen.findByText('À toi, Borin !')).toBeInTheDocument()
    expect(screen.getByText('Au tour de Borin')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Estocade/ }))
    tap(3, 0)
    await userEvent.click(screen.getByRole('button', { name: /Jouer/ }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/fight')).toEqual([
      { kind: 'act', action: 'estocade', targets: ['marin-1'] },
    ])
  })

  it('down at 0, my turn is the death save and nothing else', async () => {
    const down = {
      ...fighter('pc-1', 'Borin', true, true),
      hitPoints: 0,
      down: true,
      conditions: ['Inconscient'],
      deathSaves: { successes: 0, failures: 1, stable: false, ofSuccesses: 3, ofFailures: 3 },
    }
    const fight = {
      live: true,
      round: 2,
      active: 'pc-1',
      myTurn: true,
      order: [down, fighter('marin-1', 'Marin', false)],
      actionsLeft: 0,
      cards: [],
      events: [
        { kind: 'death_save', who: 'pc-1', die: '1d20', natural: 7, difficulty: 10, successes: 0, failures: 1 },
      ],
      loot: [],
      won: null,
      deathSave: { die: '1d20', difficulty: 10 },
    }
    const fetchMock = mockApi({
      ...MEDIA,
      'GET /api/play/c1/board': () => board(fight),
      'POST /api/play/c1/fight': () => board({ ...fight, myTurn: false, active: 'marin-1', deathSave: null }),
    })
    render(<MapTab campaignId="c1" refreshKey={0} />)
    expect(await screen.findByText('Il te faut 10 ou plus')).toBeInTheDocument()
    expect(screen.getAllByText('À toi : jet contre la mort').length).toBeGreaterThan(0)
    expect(screen.getAllByLabelText('0 réussite(s), 1 échec(s)').length).toBeGreaterThan(0)
    expect(screen.getByText('Borin : jet contre la mort, 7 (0 réussite(s), 1 échec(s))')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Fin du tour/ })).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Lancer le jet/ }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/fight')).toEqual([{ kind: 'deathSave' }])
    expect(await screen.findByText('Pas mort. À ton tour, tu lanceras le jet contre la mort.')).toBeInTheDocument()
  })
})
