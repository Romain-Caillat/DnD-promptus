import { apiRequest } from './api'
import type { MapData } from './board'

/** One of the campaign's own maps (`campaign_maps::CampaignMap`). */
export interface CampaignMap {
  map: MapData
  source: 'editor' | 'generated' | 'imported'
  node: string | null
  /** An imported image is behind its grid. */
  backdrop: boolean
  validatedAt: string | null
  updatedAt: string
}

export interface MapListing {
  maps: CampaignMap[]
  /** The world's maps, to copy from. */
  world: { id: string; name: string }[]
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

export function saveMap(campaignId: string, map: MapData): Promise<CampaignMap> {
  return apiRequest<CampaignMap>('PUT', one(campaignId, map.id), map)
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
