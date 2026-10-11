import { apiRequest } from './api'
import type { Cell, CellKind, MapData } from './board'

/** One of the campaign's own maps (`campaign_maps::CampaignMap`). */
export interface CampaignMap {
  map: MapData
  source: 'editor' | 'generated' | 'imported'
  node: string | null
  /** An imported image is behind its grid. */
  backdrop: boolean
  validatedAt: string | null
  updatedAt: string
  /** The campaign's preparation revision it was read at: the one a batch of operations is sent on. */
  revision: number
}

/** What a `place` operation puts down (`maps::edit::Thing`). */
type MapThing =
  | { kind: 'prop'; prop: string; label: string }
  | { kind: 'object'; label: string }
  | { kind: 'light' }
  | { kind: 'start'; side: 'party' | 'foes' | 'neutral'; entity?: string }

/** One operation on a map (`maps::edit::MapOp`); the server applies a batch all or nothing. */
export type MapOp =
  | { op: 'paint'; cells: Cell[]; kind: CellKind }
  | { op: 'place'; at: Cell; thing: MapThing; layer: string }
  | { op: 'move'; id: string; to: Cell }
  | { op: 'remove'; id: string }
  | { op: 'erase'; at: Cell }
  | { op: 'door'; at: Cell; layer: string }
  | { op: 'set'; name?: string; notes?: string; ambience?: MapData['ambience'] }

export interface MapListing {
  maps: CampaignMap[]
  /** The world's maps, to copy from. */
  world: { id: string; name: string }[]
}

/** One map of the atlas (`campaign_maps::AtlasMap`): whole, with where it hangs. */
export interface AtlasMap {
  map: MapData
  /** `world`: compiled with the world; `campaign`: the GM's own. */
  origin: 'world' | 'campaign'
  /** The map and exit it hangs on; `null` for a root. */
  parent: { map: string; exit: string } | null
  /** In the order of this map's exits. */
  children: string[]
  /** Exits to maps of the same scale, or back up. */
  links: { to: string; exit: string }[]
  backdrop: boolean
  validated: boolean
  /** The scenes standing on this map, by place (campaign/pin-scenes-on-maps). */
  pins: Pin[]
  /** The NPCs, clues and fights placed on it (campaign/place-cast-and-clues). */
  cast: Cast
  /** The scenes' exits between its pins (campaign/draw-story-thread). */
  thread: Thread
}

/**
 * The thread on one map (campaign/draw-story-thread): one segment per
 * pair of pins an exit joins, in each direction; the scenes standing on
 * the map, each a start or an end of its act or neither.
 */
interface Thread {
  segments: { from: ThreadEnd; to: ThreadEnd; steps: { from: string; to: string; act: string }[] }[]
  stops: ThreadStop[]
}

interface ThreadEnd {
  place: string
  at: Cell
}

export interface ThreadStop {
  scene: string
  /** The pin it stands on. */
  place: string
  act: string
  start: boolean
  end: boolean
}

/** Where a story entity stands on a map: a map and a cell. */
interface Anchor {
  map: string
  at: Cell
}

/** An NPC's spot (`story::Spot`): for one scene, or every scene on the map. */
export interface Spot extends Anchor {
  scene?: string
}

/** What stands on one map (`story::cast::Cast`). */
export interface Cast {
  npcs: { npc: string; name: string; index: number; at: Cell; scene?: string }[]
  clues: ClueMark[]
  fights: FightMark[]
}

/** A clue's spot: what the players learn, and its scene. */
export interface ClueMark {
  clue: string
  text: string
  at: Cell
  scene: string
}

/** Where a scene's fight breaks out, and its battle plan. */
export interface FightMark {
  scene: string
  title: string
  number: number
  at: Cell
  /** The battle plan: the scene's own map. */
  map?: string
  opponents: number
  /** How many opponents the battle plan already places, when it is known. */
  placed?: number
}

/** A scene on a pin (`story::pins::PinScene`): numbered as in the campaign. */
interface PinScene {
  id: string
  title: string
  number: number
  /** The grid map it is played on. */
  map?: string
}

/** A place anchored on a map and the scenes standing there (`story::pins::Pin`). */
export interface Pin {
  location: string
  name: string
  at: Cell
  scenes: PinScene[]
}

/** A place of the story and where it is pinned (`campaign_maps::AtlasPlace`). */
interface AtlasPlace {
  id: string
  name: string
  parent: string | null
  anchor: Anchor | null
}

/** maps/browse-atlas — every map of the world and the campaign, roots first then depth first. */
export interface Atlas {
  revision: number
  roots: string[]
  maps: AtlasMap[]
  /** Every place of the story, for the pin tool. */
  places: AtlasPlace[]
  /** Scenes no place anchors on a map. */
  unplaced: PinScene[]
  /** Pins and placements standing nowhere, or in a wall. */
  issues: { severity: string; code: string; path: string; detail: string }[]
  /** What the GM can place (campaign/place-cast-and-clues). */
  castable: {
    npcs: { id: string; name: string; spots: Spot[] }[]
    clues: { id: string; text: string; scene: string; anchor: Anchor | null }[]
    fights: { scene: string; title: string; number: number; map: string | null; anchor: Anchor | null }[]
    scenes: PinScene[]
  }
  /** The acts in order, for the thread's filter. */
  acts: { id: string; title: string }[]
}

export function fetchAtlas(campaignId: string): Promise<Atlas> {
  return apiRequest<Atlas>('GET', `/campaigns/${encodeURIComponent(campaignId)}/atlas`)
}

export type MapImport =
  | { kind: 'uvtt'; name: string; file: string }
  | {
      kind: 'image'
      name: string
      image: string
      cellPx: number
      offsetX: number
      offsetY: number
      columns: number
      rows: number
    }

const base = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}/maps`
const one = (campaignId: string, map: string) => `${base(campaignId)}/${encodeURIComponent(map)}`

export function fetchMaps(campaignId: string): Promise<MapListing> {
  return apiRequest<MapListing>('GET', base(campaignId))
}

export function fetchMap(campaignId: string, map: string): Promise<CampaignMap> {
  return apiRequest<CampaignMap>('GET', one(campaignId, map))
}

export function createMap(
  campaignId: string,
  body: { name: string; copy?: string; width?: number; height?: number },
): Promise<CampaignMap> {
  return apiRequest<CampaignMap>('POST', base(campaignId), body)
}

export function importMap(campaignId: string, body: MapImport): Promise<CampaignMap> {
  return apiRequest<CampaignMap>('POST', `${base(campaignId)}/import`, body)
}

export function generateMap(campaignId: string, node: string): Promise<CampaignMap & { dropped: number }> {
  return apiRequest('POST', `${base(campaignId)}/generate`, { node })
}

/**
 * maps/edit-maps-by-ops — one batch of operations on the revision the map
 * was read at. Refused with 409 `MAP_STALE` (and the map as it is now in
 * `ApiError.fresh`) when a later batch touched the same cells or things.
 */
export function editMap(campaignId: string, map: string, revision: number, ops: MapOp[]): Promise<CampaignMap> {
  return apiRequest<CampaignMap>('POST', `${one(campaignId, map)}/ops`, { revision, ops })
}

export function validateMap(campaignId: string, map: string): Promise<CampaignMap> {
  return apiRequest<CampaignMap>('POST', `${one(campaignId, map)}/validate`)
}

export function deleteMap(campaignId: string, map: string): Promise<void> {
  return apiRequest<void>('DELETE', one(campaignId, map))
}

export function gmBackdropUrl(campaignId: string, map: string): string {
  return `/api${one(campaignId, map)}/backdrop`
}

/** The image behind the map shown at the table; `map` only keeps one map's image from standing in for another's in the cache. */
export function playerBackdropUrl(campaignId: string, map: string): string {
  return `/api/play/${encodeURIComponent(campaignId)}/board/backdrop?map=${encodeURIComponent(map)}`
}
