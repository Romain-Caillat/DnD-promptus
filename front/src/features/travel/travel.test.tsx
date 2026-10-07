import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MapTab } from '@/features/map/MapTab'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { TravelMoments } from './PlayerTravel'
import { TravelPanel } from './TravelPanel'

/** A world map: sea, coast, land; the hex at [3, 0] still under the fog. */
const WORLD = {
  id: 'cotes',
  name: 'Les côtes',
  scale: 'world' as const,
  theme: 'cotes-1718',
  ambience: {},
  grid: {
    legend: { '~': { terrain: 'mer' }, '#': { terrain: 'terre' }, '?': { terrain: 'brouillard', void: true } },
    rows: ['~~#?', '~~~~'],
  },
  labels: [{ text: 'Le Palais', at: [3, 1] as [number, number] }],
}
const MEDIA = { 'GET /api/play/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }) }
const BOARD = {
  status: 200,
  body: {
    data: {
      map: WORLD,
      fog: true,
      tokens: [{ id: 'party', name: 'Le groupe', at: [0, 0], party: true, mine: false, ghost: false }],
      reachable: [],
      fight: null,
    },
  },
}

const route = (name: string, voters: string[], mine: boolean) => ({
  name,
  description: '',
  days: 2,
  portions: 5,
  hexes: [
    [1, 0],
    [2, 1],
    [3, 1],
  ],
  terrains: [['la haute mer', 3]],
  voters,
  mine,
})

function travel(journey: Record<string, unknown>) {
  return {
    status: 200,
    body: {
      data: {
        mapName: 'Les côtes',
        day: 2,
        portion: 'Après-midi',
        night: false,
        portions: ['Matin', 'Après-midi', 'Soir'],
        supplies: { name: 'Vivres', value: 8, perDay: 2 },
        journey: {
          destination: 'Le Palais',
          destinationAt: [3, 1],
          routes: [route('Le large', ['Lyra'], false), route('La côte', [], false)],
          chosen: null,
          arrived: false,
          progress: null,
          events: [],
          check: null,
          watch: null,
          ...journey,
        },
      },
    },
  }
}

describe('the journey on a phone', () => {
  beforeEach(() => stubReducedMotion(true))
  afterEach(() => vi.unstubAllGlobals())

  it('shows the world map in hexes and votes for a route with one tap', async () => {
    const fetchMock = mockApi({
      ...MEDIA,
      'GET /api/play/c1/board': () => BOARD,
      'GET /api/play/c1/travel': () => travel({}),
      'POST /api/play/c1/travel': () =>
        travel({ routes: [route('Le large', ['Lyra'], false), route('La côte', ['Borin'], true)] }),
    })
    render(<MapTab campaignId="c1" refreshKey={0} />)
    const map = await screen.findByRole('img', { name: 'Les côtes' })
    // One hex per cell, the fogged one drawn as fog, and the party's pin.
    expect(map.querySelectorAll('polygon')).toHaveLength(8)
    expect(map.querySelector('[data-hex="3,0"]')?.getAttribute('fill')).toMatch(/^url\(/)
    expect(map.querySelector('[data-party]')).not.toBeNull()
    expect(await screen.findByText('Jour 2 · Après-midi')).toBeInTheDocument()
    expect(screen.getByText('Vivres : 8 (−2 par jour)')).toBeInTheDocument()
    expect(screen.getByText('Pour : Lyra')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: /La côte/ }))
    await vi.waitFor(() => expect(sentTo(fetchMock, 'POST /api/play/c1/travel')).toEqual([{ kind: 'vote', route: 1 }]))
    expect(await screen.findByText('Pour : Borin')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /La côte/ })).toHaveAttribute('aria-pressed', 'true')
  })

  it('gives the watcher the GM’s words and the choice to wake the others', async () => {
    const watch = (mine: boolean, woken = false) => ({
      chosen: 0,
      watch: {
        slots: [
          { name: 'Premier quart', who: 'Borin', mine },
          { name: 'Quart de minuit', who: 'Lyra', mine: false },
        ],
        current: 0,
        mine,
        message: mine ? 'Des yeux brillent entre les arbres.' : null,
        woken,
      },
    })
    const fetchMock = mockApi({
      'GET /api/play/c1/travel': () => travel(watch(true)),
      'POST /api/play/c1/travel': () => travel(watch(true, true)),
    })
    render(<TravelMoments campaignId="c1" refreshKey={0} />)
    expect(await screen.findByText('Ton tour de garde')).toBeInTheDocument()
    expect(screen.getByText('Des yeux brillent entre les arbres.')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Réveiller les autres/ }))
    await vi.waitFor(() => expect(sentTo(fetchMock, 'POST /api/play/c1/travel')).toEqual([{ kind: 'wake' }]))
    expect(await screen.findByText('Le groupe est réveillé.')).toBeInTheDocument()
  })

  it('tells the others who watches, without the GM’s words', async () => {
    mockApi({
      'GET /api/play/c1/travel': () =>
        travel({
          chosen: 0,
          watch: {
            slots: [{ name: 'Premier quart', who: 'Borin', mine: false }],
            current: 0,
            mine: false,
            message: null,
            woken: false,
          },
        }),
    })
    render(<TravelMoments campaignId="c1" refreshKey={0} />)
    expect(await screen.findByText('Le groupe dort. Borin veille.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Réveiller/ })).toBeNull()
  })

  it('rolls the group check on the phone and shows the group’s result', async () => {
    const check = (rolled: boolean) => ({
      chosen: 0,
      check: {
        label: 'Ne pas se perdre',
        abilityName: 'Sagesse',
        difficulty: 12,
        needed: 1,
        rolls: [
          { name: 'Borin', total: rolled ? 9 : null, success: rolled ? false : null, mine: true },
          { name: 'Lyra', total: 19, success: true, mine: false },
        ],
        canRoll: !rolled,
        myRoll: rolled
          ? {
              die: '1d20',
              faces: [8],
              natural: 8,
              advantage: 'normal',
              modifiers: [{ source: { from: 'ability', id: 'SAG' }, value: 1 }],
              total: 9,
              target: { against: 'difficulty', id: null, value: 12 },
              band: 'failure',
            }
          : null,
        success: rolled ? true : null,
      },
    })
    const fetchMock = mockApi({
      'GET /api/play/c1/travel': () => travel(check(false)),
      'POST /api/play/c1/travel': () => travel(check(true)),
    })
    render(<TravelMoments campaignId="c1" refreshKey={0} />)
    await userEvent.click(await screen.findByRole('button', { name: /Lancer le dé/ }))
    await vi.waitFor(() => expect(sentTo(fetchMock, 'POST /api/play/c1/travel')).toEqual([{ kind: 'roll' }]))
    expect(await screen.findByText('Le groupe réussit.')).toBeInTheDocument()
    const rolls = screen.getByRole('list', { name: 'Les jets du groupe' })
    expect(within(rolls).getByText('Borin · 9')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Lancer le dé/ })).toBeNull()
  })
})

describe('the journey on the GM screen', () => {
  afterEach(() => vi.unstubAllGlobals())

  const gm = (proposal: unknown, events: unknown[] = []) => ({
    status: 200,
    body: {
      data: {
        mapId: 'cotes',
        mapName: 'Les côtes',
        travel: {
          mapId: 'cotes',
          party: { at: [1, 0], revealed: [], clock: { day: 1, portion: 1, night: false }, supplies: 6, way: { hexes: [[1, 0], [2, 1], [3, 1]], step: 1, bank: 0 } },
          journey: {
            destination: { name: 'Le Palais', at: [3, 1] },
            routes: [{ name: 'Le large', description: '', route: { hexes: [[1, 0], [2, 1], [3, 1]], steps: 3, portions: 2, days: 1, terrains: [] } }],
            votes: {},
            chosen: 0,
            arrived: false,
            events,
            check: null,
            watch: null,
          },
          proposal,
          version: 3,
        },
        places: [{ name: 'Le Palais', at: [3, 1], map: 'le-palais', scene: null, secret: false }],
        members: [{ character: 'k1', player: 'p1', name: 'Borin' }],
        guide: { portions: ['Matin', 'Après-midi', 'Soir'], watches: ['Premier quart'], supplies: 'Vivres', perPersonPerDay: 1, stepsPerPortion: 2 },
        abilities: [{ id: 'SAG', name: 'Sagesse' }],
        difficulties: [],
      },
    },
  })
  const PROPOSAL = {
    day: 1,
    portion: 'Matin',
    terrain: 'la haute mer',
    events: [
      { id: 'voile', title: 'Une voile à l’horizon', text: 'La vigie crie « Voile ! ».', gm_notes: 'L’escorte du Greyhound.' },
      { id: 'grain', title: 'Un grain soudain', text: 'Le ciel noircit.' },
    ],
  }

  it('keeps one drawn event, worded by the GM; its GM notes stay on the GM screen', async () => {
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/travel': () => gm(PROPOSAL),
      'POST /api/campaigns/c1/travel': () => gm(null, [{ day: 1, portion: 'Matin', event: 'voile', title: 'Une voile à l’horizon', text: 'Au loin, une voile.' }]),
    })
    render(<TravelPanel campaignId="c1" map={WORLD} tileset={null} refreshKey={0} />)
    expect(await screen.findByText('Le co-MJ propose · la haute mer')).toBeInTheDocument()
    expect(screen.getByText('→ L’escorte du Greyhound.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Garder' })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: /Une voile à l’horizon/ }))
    const text = screen.getByRole('textbox', { name: 'Le texte lu à la table' })
    await userEvent.clear(text)
    await userEvent.type(text, 'Au loin, une voile.')
    await userEvent.click(screen.getByRole('button', { name: 'Garder' }))
    await vi.waitFor(() =>
      expect(sentTo(fetchMock, 'POST /api/campaigns/c1/travel')).toEqual([
        { kind: 'keep', event: 'voile', text: 'Au loin, une voile.' },
      ]),
    )
    expect(await screen.findByText('Une voile à l’horizon')).toBeInTheDocument()
    expect(screen.queryByText('Le co-MJ propose · la haute mer')).toBeNull()
  })

  it('plays the next portion with the portion’s name on the button', async () => {
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/travel': () => gm(null),
      'POST /api/campaigns/c1/travel': () => gm(null),
    })
    render(<TravelPanel campaignId="c1" map={WORLD} tileset={null} refreshKey={0} />)
    await userEvent.click(await screen.findByRole('button', { name: 'Portion suivante · Après-midi' }))
    await userEvent.click(screen.getByRole('button', { name: 'Perdre une portion' }))
    await vi.waitFor(() =>
      expect(sentTo(fetchMock, 'POST /api/campaigns/c1/travel')).toEqual([{ kind: 'advance' }, { kind: 'advance', hold: true }]),
    )
  })
})
