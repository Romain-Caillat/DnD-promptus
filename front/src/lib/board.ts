import { apiRequest } from './api'
import type { RollBreakdown } from './rules'

const play = (campaignId: string) => `/play/${encodeURIComponent(campaignId)}`
const gm = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`

/** `[x, y]`, row 0 at the top. */
export type Cell = [number, number]

export interface CellKind {
  terrain: string
  wall?: boolean
  water?: 'none' | 'shallow' | 'deep'
  difficult?: boolean
  void?: boolean
  elevation?: number
}

export type TimeOfDay = 'dawn' | 'day' | 'dusk' | 'night'
export type Weather = 'clear' | 'cloudy' | 'rain' | 'storm' | 'fog' | 'snow' | 'sandstorm'
type LightLevel = 'dark' | 'dim' | 'bright'
type DoorState = 'open' | 'closed' | 'locked'

/** A map as the server sends it (`maps::Map`): only what the screens draw. */
export interface MapData {
  id: string
  name: string
  theme: string
  ambience: {
    time?: TimeOfDay
    weather?: Weather
    light?: LightLevel | null
    sight_limit?: number | null
    mood?: string | null
  }
  gm_notes?: string | null
  layers?: { id: string; name: string; visibility: string }[]
  grid: { legend: Record<string, CellKind>; rows: string[] }
  doors?: { id: string; at: Cell; state: DoorState; label?: string | null; layer?: string }[]
  /** `size` is `[w, h]` from `at`, the top-left cell. */
  props?: {
    id: string
    kind: string
    label?: string | null
    at: Cell
    size?: [number, number]
    cover?: 'none' | 'half' | 'three_quarters' | 'total'
    blocks_movement?: boolean
    layer?: string
  }[]
  objects?: {
    id: string
    kind: string
    label?: string | null
    at: Cell
    layer?: string
    check?: { stat: string; dc: number } | null
    notes?: string | null
  }[]
  lights?: { id: string; at: Cell; bright: number; dim: number; color?: string | null; flicker?: boolean; layer?: string }[]
  exits?: { id: string; cells: Cell[]; to: string; label?: string | null }[]
  labels?: { text: string; at: Cell }[]
  starts?: { id: string; at: Cell; side?: string | null; entity?: string | null; layer?: string }[]
  /** An imported image behind the grid: decor only. */
  backdrop?: { image?: string | null; cell_px?: number | null; offset?: [number, number] | null } | null
}

export interface ReachCell {
  at: Cell
  cost: number
}

/** A token as a player may see it. */
export interface TokenView {
  id: string
  name: string
  at: Cell
  party: boolean
  mine: boolean
  /** Invisible: only its owner sees it. */
  ghost: boolean
}

type Standing = 'in_fight' | 'defeated' | 'out_of_scene' | 'fled'

interface FighterView {
  id: string
  name: string
  party: boolean
  standing: Standing
  at: Cell | null
  hitPoints: number | null
  maxHitPoints: number | null
  down: boolean
  conditions: string[]
  mine: boolean
}

interface FightCardView {
  id: string
  name: string
  description: string
  kind: string
  target: string
  area: unknown
  range: number
  attackBonus: number | null
  locked: boolean
}

/** One event of a fight (`combat::fight::FightEvent`); `rules` wraps an engine event. */
export type FightEvent =
  | { kind: 'initiative_rolled'; roll: { who: string; total: number } }
  | { kind: 'order'; order: string[] }
  | { kind: 'round_started'; round: number }
  | { kind: 'turn_started'; who: string; round: number }
  | { kind: 'moved'; who: string; path: Cell[]; cost: number; budget: number }
  | { kind: 'acted'; who: string; action: string; targets: string[] }
  | { kind: 'rules'; event: RulesEvent }
  | { kind: 'flee_roll'; who: string; roll: RollBreakdown }
  | { kind: 'flee_failed' | 'fled' | 'defeated' | 'left_the_scene' | 'turn_ended'; who: string }
  | { kind: 'ended'; end: { winner: 'party' | 'opposition' | null; rounds: number } }

type RulesEvent =
  | { event: 'roll'; roller: string; against: string | null; purpose: unknown; breakdown: RollBreakdown }
  | { event: 'missed'; target: string }
  | { event: 'damaged'; target: string; hp_before: number; hp_after: number }
  | { event: 'healed'; target: string; amount: number; hp_before: number; hp_after: number }
  | { event: 'condition_applied'; target: string; name: string; turns: number | null }
  | { event: 'condition_resisted' | 'condition_ended'; target: string; name: string }
  | { event: 'knocked_out' | 'revived' | 'out_of_scene'; target: string }
  | { event: 'turn_lost'; who: string; because: string }
  | { event: 'item_used'; who: string; item: string; left: number }
  | { event: 'house_rule'; rule: string; name: string; target: string; shown: boolean }
  | { event: 'cooldown_started' | 'progress' | 'for_the_gm'; [k: string]: unknown }

export interface FightView {
  live: boolean
  round: number
  active: string | null
  myTurn: boolean
  order: FighterView[]
  actionsLeft: number | null
  cards: FightCardView[]
  events: FightEvent[]
  loot: { name: string; toMe: boolean }[]
  won: boolean | null
}

/** The grid as a player may see it (`projection::board::BoardView`). */
export interface BoardView {
  map: MapData
  fog: boolean
  tokens: TokenView[]
  reachable: ReachCell[]
  fight: FightView | null
}

export type Command =
  | { kind: 'move'; path: Cell[] }
  | { kind: 'act'; action: string; targets: string[]; cell?: Cell }
  | { kind: 'item'; item: string; targets: string[] }
  | { kind: 'flee' }
  | { kind: 'endTurn' }

export function fetchBoard(campaignId: string): Promise<BoardView | null> {
  return apiRequest<BoardView | null>('GET', `${play(campaignId)}/board`)
}

export function walk(campaignId: string, path: Cell[]): Promise<BoardView | null> {
  return apiRequest<BoardView | null>('POST', `${play(campaignId)}/board/walk`, { path })
}

export function fightCommand(campaignId: string, cmd: Command): Promise<BoardView | null> {
  return apiRequest<BoardView | null>('POST', `${play(campaignId)}/fight`, cmd)
}

// ——— The GM's grid ———

interface Token {
  id: string
  kind: 'character' | 'npc'
  ref: string
  name: string
  at: Cell
  hidden: boolean
  invisible: boolean
}

interface Board {
  mapId: string
  map: MapData
  fog: boolean
  revealed: Cell[]
  tokens: Token[]
}

interface Combatant {
  id: string
  name: string
  side: 'party' | 'opposition'
  hit_points: number
  conditions: { id: string | null; name: string; remaining: number | null }[]
}

interface Fight {
  scene: { combatants: Record<string, Combatant> }
  positions: Record<string, Cell>
  order: string[]
  standing: Record<string, Standing>
  round: number
  turn: number
  end: unknown
}

type ProposedStep =
  | { kind: 'move'; path: Cell[] }
  | { kind: 'act'; action: string; targets: string[] }
  | { kind: 'flee' }
  | { kind: 'end_turn' }

interface LootLine {
  index: number
  item: string | null
  name: string
  coins: number | null
  found: string
  hidden: boolean
  givenTo: string | null
}

interface GmEncounter {
  id: string
  node: string
  live: boolean
  version: number
  fight: Fight
  maxHitPoints: Record<string, number>
  proposal: { who: string; version: number; steps: ProposedStep[]; events: FightEvent[] } | null
  loot: LootLine[]
  events: FightEvent[]
  reachable: ReachCell[]
}

/** The GM's grid screen (`GET /api/campaigns/{id}/board`). */
export interface GmBoard {
  board: Board | null
  maps: { id: string; name: string; nodes: string[] }[]
  encounters: { node: string; title: string; map: string | null }[]
  encounter: GmEncounter | null
  conditions: { id: string; name: string }[] | null
}

export type Edit =
  | { kind: 'revealCells'; cells: Cell[] }
  | { kind: 'hideCells'; cells: Cell[] }
  | { kind: 'fog'; enabled: boolean }
  | { kind: 'revealLayer'; layer: string }
  | { kind: 'revealThing'; id: string }
  | { kind: 'door'; door: string; state: DoorState }
  | { kind: 'ambience'; time?: TimeOfDay; weather?: Weather; light?: LightLevel | null; sightLimit?: number | null }
  | { kind: 'moveToken'; token: string; at: Cell }
  | { kind: 'placeNpc'; npc: string; at: Cell; hidden: boolean }
  | { kind: 'removeToken'; token: string }
  | { kind: 'tokenState'; token: string; hidden?: boolean; invisible?: boolean }

export type GmCommand =
  | { kind: 'adversary'; command: Command }
  | { kind: 'propose' }
  | { kind: 'accept' }
  | { kind: 'condition'; who: string; condition: string; turns?: number; remove: boolean }
  | { kind: 'stop' }

export function fetchGmBoard(campaignId: string): Promise<GmBoard> {
  return apiRequest<GmBoard>('GET', `${gm(campaignId)}/board`)
}

export function showMap(campaignId: string, map: string): Promise<GmBoard> {
  return apiRequest<GmBoard>('POST', `${gm(campaignId)}/board`, { map })
}

export function editBoard(campaignId: string, edit: Edit): Promise<GmBoard> {
  return apiRequest<GmBoard>('POST', `${gm(campaignId)}/board/edit`, edit)
}

export function startFight(campaignId: string, node: string): Promise<GmBoard> {
  return apiRequest<GmBoard>('POST', `${gm(campaignId)}/fight`, { node })
}

export function gmFightCommand(campaignId: string, cmd: GmCommand): Promise<GmBoard> {
  return apiRequest<GmBoard>('POST', `${gm(campaignId)}/fight/command`, cmd)
}

export function giveLoot(campaignId: string, gives: { index: number; character: string }[]): Promise<GmBoard> {
  return apiRequest<GmBoard>('POST', `${gm(campaignId)}/fight/loot`, { gives })
}

const key = ([x, y]: Cell) => `${x},${y}`

/**
 * The cells to walk from `from` to `to` through `reach` (the server's
 * reachable cells), one 8-neighbour step at a time; `null` when `to` is
 * not reachable. The server checks the path again: this only draws it.
 */
export function pathTo(from: Cell, to: Cell, reach: ReachCell[]): Cell[] | null {
  const open = new Set(reach.map((r) => key(r.at)))
  if (!open.has(key(to))) return null
  const prev = new Map<string, Cell | null>([[key(from), null]])
  const queue: Cell[] = [from]
  while (queue.length > 0) {
    const cur = queue.shift()!
    if (key(cur) === key(to)) break
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        if (dx === 0 && dy === 0) continue
        const next: Cell = [cur[0] + dx, cur[1] + dy]
        if (!open.has(key(next)) || prev.has(key(next))) continue
        prev.set(key(next), cur)
        queue.push(next)
      }
    }
  }
  if (!prev.has(key(to))) return null
  const path: Cell[] = []
  for (let c: Cell | null = to; c && key(c) !== key(from); c = prev.get(key(c)) ?? null) path.unshift(c)
  return path
}
