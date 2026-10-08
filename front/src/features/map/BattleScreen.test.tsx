import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { MapTab } from './MapTab'

const SEA = {
  id: 'large',
  name: 'Au large',
  theme: 'mer',
  ambience: {},
  grid: { legend: { '~': { terrain: 'mer', water: 'deep' } }, rows: ['~~~~~~', '~~~~~~'] },
}
const ship = (id: string, name: string, party: boolean, at: [number, number], known = party) => ({
  id,
  name,
  party,
  at,
  facing: party ? 'e' : 'w',
  standing: 'afloat',
  known,
  hull: known ? 20 : null,
  maxHull: known ? 24 : null,
  screen: known ? 6 : null,
  maxScreen: known ? 8 : null,
  armor: known ? 12 : null,
  morale: known ? 10 : null,
  maxMorale: known ? 10 : null,
  damages: [],
})
const battle = {
  live: true,
  boarding: false,
  round: 1,
  crewTurn: true,
  active: 'La Mâchoire',
  map: SEA,
  current: null,
  gauges: { hull: 'Coque', screen: 'Voilure', armor: 'Bordé', morale: 'Moral', power: 'Équipage' },
  ships: [ship('la_machoire', 'La Mâchoire', true, [0, 0]), ship('hms_greyhound', 'HMS Greyhound', false, [4, 0])],
  crew: [{ id: 'pc-1', name: 'Borin', station: 'pont', npc: false, done: false, mine: true }],
  stations: [{ id: 'pont', name: 'Pont et mousqueterie', description: '', holder: 'Borin', down: false }],
  power: null,
  me: {
    id: 'pc-1',
    station: 'pont',
    actions: 2,
    attacks: 1,
    done: false,
    myTurn: true,
    stationCost: 1,
    options: [
      {
        id: 'feu_de_mousqueterie',
        name: 'Feu de mousqueterie',
        description: '',
        cost: 1,
        attack: true,
        check: null,
        aim: 'ship',
        usable: true,
        targets: [{ ship: 'hms_greyhound', weapon: 'mousquets' }],
        reach: [],
        turns: 0,
        damages: [],
      },
    ],
  },
  events: [{ kind: 'round_started', round: 1 }],
  won: null,
  reason: null,
}

describe('the ship battle on the Map tab', () => {
  beforeEach(() => stubReducedMotion(true))
  afterEach(() => vi.unstubAllGlobals())

  it('fires my station at a target the server offers, the unscanned enemy kept unknown', async () => {
    const fetchMock = mockApi({
      'GET /api/play/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }),
      'GET /api/play/c1/board': () => ({ status: 200, body: { data: null } }),
      'GET /api/play/c1/battle': () => ({ status: 200, body: { data: battle } }),
      'POST /api/play/c1/battle': () => ({ status: 200, body: { data: battle } }),
    })
    render(<MapTab campaignId="c1" refreshKey={0} />)
    expect(await screen.findByText('À ton poste, Pont et mousqueterie !')).toBeInTheDocument()
    expect(screen.getByText('Coque 20/24')).toBeInTheDocument()
    expect(screen.getByText('Inconnu : à scanner')).toBeInTheDocument()

    const act = screen.getByRole('button', { name: 'Agir' })
    expect(act).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: /Feu de mousqueterie/ }))
    await userEvent.click(screen.getByRole('button', { name: 'HMS Greyhound' }))
    await userEvent.click(screen.getByRole('button', { name: 'Agir' }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/battle')).toEqual([
      { kind: 'act', action: 'feu_de_mousqueterie', aim: { target: 'hms_greyhound', weapon: 'mousquets' } },
    ])
  })
})
