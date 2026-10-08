import { apiRequest } from './api'
import type { Cell, GmBoard, MapData } from './board'
import type { RollBreakdown } from './rules'

const play = (campaignId: string) => `/play/${encodeURIComponent(campaignId)}`
const gm = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`

/** A ship's bow (`vehicle::Facing`), north being row 0. */
export type Facing = 'n' | 'e' | 's' | 'w'
export const FACINGS: Facing[] = ['n', 'e', 's', 'w']
type Direction = 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w' | 'nw'
export type ShipStanding = 'afloat' | 'destroyed' | 'struck' | 'captured' | 'fled'
type EndReason = 'victory' | 'defeat' | 'disengaged' | 'stopped_by_gm' | 'stalemate'

/** One event of a ship battle (`vehicle::BattleEvent`). */
export type BattleEvent =
  | { kind: 'initiative_rolled'; unit: string; roll: { total: number } }
  | { kind: 'round_started'; round: number }
  | { kind: 'turn_started'; unit: string; round: number }
  | { kind: 'seated'; crew: string; station: string | null }
  | { kind: 'maneuvered'; ship: string; from: Cell; to: Cell; facing: Facing }
  | {
      kind: 'fired'
      ship: string
      weapon: string
      by?: string
      target: string
      roll: RollBreakdown
      hit: boolean
      damage: number
      screen_after: number | null
      hull_after: number | null
    }
  | { kind: 'evading'; ship: string; armor: number }
  | { kind: 'braced'; ship: string; damage_taken: number }
  | { kind: 'recharged'; ship: string; screen: number }
  | { kind: 'repaired'; ship: string; by: string; amount: number; screen: boolean }
  | { kind: 'rerouted'; ship: string; power: Record<string, number> }
  | { kind: 'locked' | 'scanned'; by: string; target: string }
  | { kind: 'jammed'; by: string; target: string; malus: number }
  | { kind: 'morale_roll'; by: string; target: string; roll: RollBreakdown; loss: number; morale_after: number | null }
  | { kind: 'rallied'; by: string; bonus: number }
  | { kind: 'check'; by: string; action: string; roll: RollBreakdown }
  | { kind: 'for_the_gm'; by: string; action: string }
  | { kind: 'damage_rolled'; ship: string; face: number; damage: string; name: string; station?: string; crew?: string }
  | { kind: 'damage_fixed'; ship: string; damage: string; by: string }
  | { kind: 'burned'; ship: string; damage: string; amount: number; hull_after: number }
  | { kind: 'power_short'; ship: string; points: number }
  | { kind: 'ship_out'; ship: string; standing: ShipStanding }
  | { kind: 'adjusted'; ship: string }
  | { kind: 'current_changed'; from: Direction | null }
  | { kind: 'boarding_started'; attacker: string; defender: string }
  | { kind: 'boarding_ended'; attacker: string; defender: string; attacker_won: boolean }
  | { kind: 'turn_ended'; unit: string }
  | { kind: 'ended'; end: { winner: 'party' | 'opposition' | null; reason: EndReason } }

export interface ShipView {
  id: string
  name: string
  party: boolean
  at: Cell
  facing: Facing
  standing: ShipStanding
  /** Its numbers are known to the crew. */
  known: boolean
  hull: number | null
  maxHull: number | null
  screen: number | null
  maxScreen: number | null
  armor: number | null
  morale: number | null
  maxMorale: number | null
  damages: string[]
}

export interface ActionView {
  id: string
  name: string
  description: string
  cost: number
  attack: boolean
  check: number | null
  aim: 'none' | 'ship' | 'cell' | 'power' | 'damage'
  usable: boolean
  targets: { ship: string; weapon: string | null }[]
  reach: Cell[]
  turns: number
  damages: { index: number; name: string }[]
}

interface PowerView {
  name: string
  points: number
  channels: { id: string; name: string; level: number; max: number; note: string }[]
}

/** The ship battle as a player may see it (`projection::battle::BattleView`). */
export interface BattleView {
  live: boolean
  boarding: boolean
  round: number
  crewTurn: boolean
  active: string | null
  map: MapData
  current: Direction | null
  gauges: { hull: string; screen: string | null; armor: string; morale: string; power: string | null }
  ships: ShipView[]
  crew: { id: string; name: string; station: string | null; npc: boolean; done: boolean; mine: boolean }[]
  stations: { id: string; name: string; description: string; holder: string | null; down: boolean }[]
  power: PowerView | null
  me: {
    id: string
    station: string | null
    actions: number
    attacks: number
    done: boolean
    myTurn: boolean
    stationCost: number
    options: ActionView[]
  } | null
  events: BattleEvent[]
  won: boolean | null
  reason: EndReason | null
}

/** What a crew member's action is aimed at (`vehicle::Aim`). */
export interface Aim {
  target?: string
  weapon?: string
  to?: Cell
  facing?: Facing
  power?: Record<string, number>
  damage?: number
}

export type CrewCommand =
  | { kind: 'station'; station: string | null }
  | { kind: 'act'; action: string; aim: Aim }
  | { kind: 'pass' }

export function fetchBattle(campaignId: string): Promise<BattleView | null> {
  return apiRequest<BattleView | null>('GET', `${play(campaignId)}/battle`)
}

export function crewCommand(campaignId: string, cmd: CrewCommand): Promise<BattleView | null> {
  return apiRequest<BattleView | null>('POST', `${play(campaignId)}/battle`, cmd)
}

// ——— The GM's side ———

interface Ship {
  id: string
  name: string
  side: 'party' | 'opposition'
  at: Cell
  facing: Facing
  hull: number
  max_hull: number
  screen: number
  max_screen: number
  armor: number
  morale: number | null
  max_morale: number | null
  standing: ShipStanding
  scanned: boolean
  moved: boolean
  fired: boolean
  damages: { id: string; name: string; station?: string }[]
}

/** The battle from the GM's side, in the GM board (`GET /api/campaigns/{id}/board`). */
export interface GmBattle {
  id: string
  node: string
  status: 'live' | 'boarding' | 'ended'
  version: number
  battle: { ships: Ship[]; round: number; boarding: { attacker: string; defender: string } | null }
  view: BattleView
  proposal: { unit: string; version: number; events: BattleEvent[] } | null
  events: BattleEvent[]
  crewOptions: Record<string, ActionView[]>
  enemyReach: Record<string, Cell[]>
  enemyFire: { ship: string; weapon: string; name: string; target: string }[] | null
}

export type BattleCommand =
  | { kind: 'seat'; crew: string; station: string | null }
  | { kind: 'crew'; crew: string; command: CrewCommand }
  | { kind: 'maneuver'; ship: string; to: Cell; facing: Facing }
  | { kind: 'fire'; ship: string; weapon: string; target: string }
  | { kind: 'endTurn' }
  | { kind: 'propose' }
  | { kind: 'accept' }
  | { kind: 'adjust'; ship: string; hull?: number; screen?: number; morale?: number }
  | { kind: 'strike'; ship: string; standing: ShipStanding }
  | { kind: 'board'; attacker: string; defender: string }
  | { kind: 'stop' }

export function startBattle(campaignId: string, node: string): Promise<GmBoard> {
  return apiRequest<GmBoard>('POST', `${gm(campaignId)}/battle`, { node })
}

export function gmBattleCommand(campaignId: string, cmd: BattleCommand): Promise<GmBoard> {
  return apiRequest<GmBoard>('POST', `${gm(campaignId)}/battle/command`, cmd)
}
