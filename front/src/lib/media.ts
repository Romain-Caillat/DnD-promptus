import { apiRequest } from './api'

/** `intro` is an act's introduction: a video. */
export type MediaKind = 'scene' | 'npc' | 'adversary' | 'location' | 'item' | 'tileset' | 'intro'

type Pattern =
  | 'plain'
  | 'cobbles'
  | 'planks'
  | 'flagstones'
  | 'grating'
  | 'plates'
  | 'water'
  | 'grass'
  | 'sand'
  | 'dirt'
  | 'rock'
  | 'asphalt'

export interface Material {
  name: string
  base: string
  dark: string
  light: string
  pattern: Pattern
  blend?: boolean
}

interface PropLook {
  name: string
  shape: string
  color: string
  accent: string
}

export interface Tileset {
  id: string
  name: string
  void: string
  walls: { cap: string; face: string; joint: string; pattern: string }
  materials: Record<string, Material>
  props: Record<string, PropLook>
}

/** A world's look (`shared::theme::Theme`). */
interface Theme {
  id: string
  name: string
  rules: string
  sprite_pack: string
  title_font: string
  colors: Record<string, string>
  tilesets: Tileset[]
}

export interface Asset {
  id: string
  kind: MediaKind
  subject: string
  direction?: string
  status?: 'drawing' | 'pending' | 'approved' | 'rejected'
  error?: string | null
  createdAt?: string
}

export interface MediaList {
  assets: Asset[]
  theme: Theme | null
}

interface Subject {
  kind: MediaKind
  subject: string
}

/** What a batch would draw, and at what price (`media::Plan`). */
interface MediaPlan {
  images: Subject[]
  videos: Subject[]
  imageMicros: number
  videoMicros: number
  spending: { budgetMicros: number; spentMicros: number }
  running: boolean
  configured: boolean
}

export interface GmMediaList extends MediaList {
  plan: MediaPlan
}

export function fetchPlayerMedia(campaignId: string): Promise<MediaList> {
  return apiRequest<MediaList>('GET', `/play/${encodeURIComponent(campaignId)}/media`)
}

export function playerImageUrl(campaignId: string, asset: string): string {
  return `/api/play/${encodeURIComponent(campaignId)}/media/${encodeURIComponent(asset)}/image`
}

export function fetchGmMedia(campaignId: string): Promise<GmMediaList> {
  return apiRequest<GmMediaList>('GET', `/campaigns/${encodeURIComponent(campaignId)}/media`)
}

/** Draw every missing image (and the acts' videos) in the background. */
export function drawMissing(campaignId: string, videos: boolean): Promise<{ queued: Asset[] }> {
  return apiRequest('POST', `/campaigns/${encodeURIComponent(campaignId)}/media/batch`, { videos })
}

export function gmImageUrl(campaignId: string, asset: string): string {
  return `/api/campaigns/${encodeURIComponent(campaignId)}/media/${encodeURIComponent(asset)}/image`
}

export function askImage(campaignId: string, kind: MediaKind, subject: string, direction: string): Promise<Asset> {
  return apiRequest<Asset>('POST', `/campaigns/${encodeURIComponent(campaignId)}/media`, {
    kind,
    subject,
    direction,
  })
}

export function decideImage(campaignId: string, asset: string, approve: boolean): Promise<void> {
  return apiRequest<void>(
    'POST',
    `/campaigns/${encodeURIComponent(campaignId)}/media/${encodeURIComponent(asset)}/decision`,
    { approve },
  )
}

/** The approved image of `subject`, if the list has one. */
export function imageOf(list: MediaList | null, kind: MediaKind, subject: string): Asset | undefined {
  return list?.assets.find((a) => a.kind === kind && a.subject === subject && (a.status ?? 'approved') === 'approved')
}
