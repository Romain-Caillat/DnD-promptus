import { apiRequest } from './api'
import type { Cell } from './board'
import type { RollBreakdown } from './rules'

const play = (campaignId: string) => `/play/${encodeURIComponent(campaignId)}`
const gm = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`

// ——— What a player sees (`projection::travel::TravelView`) ———

interface RouteView {
  name: string
  description: string
  days: number
  portions: number
  hexes: Cell[]
  terrains: [string, number][]
  voters: string[]
  mine: boolean
}

interface CheckView {
  label: string
  abilityName: string
  difficulty: number
  needed: number
  rolls: { name: string; total: number | null; success: boolean | null; mine: boolean }[]
  canRoll: boolean
  myRoll: RollBreakdown | null
  success: boolean | null
}

interface WatchView {
  slots: { name: string; who: string | null; mine: boolean }[]
  current: number | null
  mine: boolean
  message: string | null
  woken: boolean
}

interface JourneyView {
  destination: string
  destinationAt: Cell | null
  routes: RouteView[]
  chosen: number | null
  arrived: boolean
  progress: [number, number] | null
  events: { day: number; portion: string; title: string; text: string }[]
  check: CheckView | null
  watch: WatchView | null
}

export interface TravelView {
  mapName: string
  day: number
  portion: string | null
  night: boolean
  portions: string[]
  supplies: { name: string; value: number; perDay: number }
  journey: JourneyView | null
}

export type PlayerCommand = { kind: 'vote'; route: number } | { kind: 'roll' } | { kind: 'wake' }

export function fetchTravel(campaignId: string): Promise<TravelView | null> {
  return apiRequest<TravelView | null>('GET', `${play(campaignId)}/travel`)
}

export function travelCommand(campaignId: string, cmd: PlayerCommand): Promise<TravelView | null> {
  return apiRequest<TravelView | null>('POST', `${play(campaignId)}/travel`, cmd)
}

// ——— The GM's side (`GET /api/campaigns/{id}/travel`) ———

interface Route {
  hexes: Cell[]
  steps: number
  portions: number
  days: number
  terrains: [string, number][]
}

interface Journey {
  destination: { name: string; at: Cell }
  routes: { name: string; description: string; route: Route }[]
  /** Player id → route index. */
  votes: Record<string, number>
  chosen: number | null
  arrived: boolean
  events: { day: number; portion: string; event: string; title: string; text: string }[]
  check: {
    label: string
    ability: string
    abilityName: string
    difficulty: number
    rolls: { character: string; player: string; name: string; roll: RollBreakdown | null }[]
    success: boolean | null
    successes: number
    needed: number
  } | null
  watch: {
    slots: { name: string; character: string | null; player: string | null; who: string | null }[]
    current: number | null
    message: string
    woken: boolean
  } | null
}

interface TravelEvent {
  id: string
  title: string
  text: string
  gm_notes?: string
}

export interface GmTravel {
  mapId: string
  mapName: string
  travel: {
    mapId: string
    party: {
      at: Cell
      revealed: Cell[]
      clock: { day: number; portion: number; night: boolean }
      supplies: number
      way?: { hexes: Cell[]; step: number; bank: number } | null
    }
    journey: Journey | null
    proposal: { day: number; portion: string; terrain: string; events: TravelEvent[] } | null
    version: number
  } | null
  places: { name: string; at: Cell; map: string | null; scene: string | null; secret: boolean }[]
  members: { character: string; player: string; name: string }[]
  guide: { portions: string[]; watches: string[]; supplies: string; perPersonPerDay: number; stepsPerPortion: number }
  abilities: { id: string; name: string }[] | null
  difficulties: { id: string; name: string; value: number }[] | null
}

export type GmTravelCommand =
  | { kind: 'plan'; to: Cell }
  | { kind: 'route'; index: number; name: string; description: string }
  | { kind: 'choose'; index: number }
  | { kind: 'advance'; hold?: boolean }
  | { kind: 'keep'; event: string; text?: string }
  | { kind: 'skip' }
  | { kind: 'check'; ability: string; difficulty: number; label: string }
  | { kind: 'closeCheck' }
  | { kind: 'watch'; slots: (string | null)[] }
  | { kind: 'watchTurn'; slot: number; message: string }
  | { kind: 'supplies'; value: number }
  | { kind: 'place'; at: Cell }
  | { kind: 'enter' }
  | { kind: 'abandon' }

export function fetchGmTravel(campaignId: string): Promise<GmTravel | null> {
  return apiRequest<GmTravel | null>('GET', `${gm(campaignId)}/travel`)
}

export function gmTravelCommand(campaignId: string, cmd: GmTravelCommand): Promise<GmTravel | null> {
  return apiRequest<GmTravel | null>('POST', `${gm(campaignId)}/travel`, cmd)
}
