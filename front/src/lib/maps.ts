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
}

/** maps/browse-atlas — every map of the world and the campaign, roots first then depth first. */
export interface Atlas {
  revision: number
  roots: string[]
  maps: AtlasMap[]
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
