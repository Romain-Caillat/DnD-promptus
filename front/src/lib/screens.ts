/**
 * The shared screen (session/pair-shared-screen, tv/show-evening): a TV
 * in the living room or a window the GM shares on Discord. The screen's
 * own calls go to `/api/tv`, where its cookie lives; the GM's to the
 * campaign.
 */
import { apiRequest } from './api'
import type { BoardView } from './board'
import type { PlayerView } from './campaigns'
import type { JournalKind, Music } from './evening'
import type { MediaList } from './media'
import type { RollBreakdown } from './rules'
import type { CharacterLook } from '@/features/sprites/look'

/** What the GM lets the screens show; each switch only takes away. */
export interface Shows {
  scene: boolean
  map: boolean
  party: boolean
  moments: boolean
}

/** Where a screen stands when it says hello. */
export interface Hello {
  paired: boolean
  /** The code to type, while waiting for a GM. */
  code: string | null
  expiresAt: string | null
}

export interface ScreenSeat {
  name: string
  nickname: string
  look: CharacterLook | null
  here: boolean
  hitPoints: number | null
  maxHitPoints: number | null
}

/** A shared line of this session's journal. */
export interface Moment {
  id: string
  kind: JournalKind
  text: string
  at: string
}

/** A check rolled at the table, as the die shows it. */
export interface ScreenRoll {
  id: string
  character: string
  ability: string
  difficulty: string
  roll: RollBreakdown
  outcome: string | null
  at: string
}

/** The evening as the screen may show it (`projection::screen::ScreenView`). */
export interface ScreenView {
  title: string
  shows: Shows
  session: { number: number; status: 'lobby' | 'live' | 'ended'; startedAt: string | null } | null
  previously: string | null
  music: Music | null
  party: ScreenSeat[]
  scene: PlayerView['scene']
  board: BoardView | null
  moments: Moment[]
  rolls: ScreenRoll[]
  tonight: string[]
}

export function sayHello(): Promise<Hello> {
  return apiRequest<Hello>('POST', '/tv')
}

export function fetchScreenView(): Promise<ScreenView> {
  return apiRequest<ScreenView>('GET', '/tv/show')
}

export function fetchScreenMedia(): Promise<MediaList> {
  return apiRequest<MediaList>('GET', '/tv/media')
}

export function screenImageUrl(asset: string): string {
  return `/api/tv/media/${encodeURIComponent(asset)}/image`
}

export const SCREEN_BACKDROP_URL = '/api/tv/board/backdrop'

/** The QR of the page a GM opens to pair this screen. */
export function pairingQrUrl(origin: string = window.location.origin): string {
  return `/api/tv/qr.svg?origin=${encodeURIComponent(origin)}`
}

// ——— The GM's side ———

export interface GmScreens {
  screens: { id: string; kind: 'tv' | 'window'; pairedAt: string | null; online: boolean }[]
  shows: Shows
}

const gm = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}/screens`

export function fetchScreens(campaignId: string): Promise<GmScreens> {
  return apiRequest<GmScreens>('GET', gm(campaignId))
}

export function pairScreen(campaignId: string, code: string): Promise<GmScreens> {
  return apiRequest<GmScreens>('POST', gm(campaignId), { code })
}

/** A window in this browser, paired at birth: open `/tv` next. */
export function openScreenWindow(campaignId: string): Promise<GmScreens> {
  return apiRequest<GmScreens>('POST', `${gm(campaignId)}/window`)
}

export function forgetScreen(campaignId: string, screen: string): Promise<GmScreens> {
  return apiRequest<GmScreens>('DELETE', `${gm(campaignId)}/${encodeURIComponent(screen)}`)
}

export function setShows(campaignId: string, shows: Shows): Promise<GmScreens> {
  return apiRequest<GmScreens>('PUT', `${gm(campaignId)}/shows`, shows)
}
