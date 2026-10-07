import { apiRequest } from './api'

const gm = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`
const STORE_KEY = 'promptus.tv.campaign'

/** What a TV shows while it waits for the GM (`screens::Pairing`). */
export interface Pairing {
  code: string
  secret: string
}

export type PairingState = { state: 'waiting'; code: string } | { state: 'paired'; campaign: string }

/** A screen seated at the table, for the GM (`screens::Screen`). */
export interface Screen {
  id: string
  name: string
  pairedAt: string
  lastSeenAt: string
}

export function startPairing(): Promise<Pairing> {
  return apiRequest<Pairing>('POST', '/tv/pairings')
}

/** Paired, the answer also leaves the seat's cookie on this device. */
export function pollPairing(secret: string): Promise<PairingState> {
  return apiRequest<PairingState>('GET', `/tv/pairings/${encodeURIComponent(secret)}`)
}

export function qrUrl(secret: string): string {
  return `/api/tv/pairings/${encodeURIComponent(secret)}/qr.svg`
}

export function fetchScreens(campaignId: string): Promise<Screen[]> {
  return apiRequest<Screen[]>('GET', `${gm(campaignId)}/tv`)
}

export function pairScreen(campaignId: string, code: string): Promise<Screen[]> {
  return apiRequest<Screen[]>('POST', `${gm(campaignId)}/tv`, { code })
}

/** A window to share on Discord: its secret goes in the address it opens. */
export async function openWindow(campaignId: string): Promise<string> {
  return (await apiRequest<{ secret: string }>('POST', `${gm(campaignId)}/tv/window`)).secret
}

export function forgetScreen(campaignId: string, screen: string): Promise<void> {
  return apiRequest<void>('DELETE', `${gm(campaignId)}/tv/${encodeURIComponent(screen)}`)
}

/** How many lines of « Précédemment… » the table sees; `null` ends the reading. */
export async function setReading(campaignId: string, line: number | null): Promise<number | null> {
  return (await apiRequest<{ line: number | null }>('POST', `${gm(campaignId)}/session/reading`, { line })).line
}

/** The table this TV joined, so it finds it again on its own. */
export function rememberTable(campaignId: string): void {
  try {
    localStorage.setItem(STORE_KEY, campaignId)
  } catch {
    // Private mode: the TV pairs again next time.
  }
}

export function rememberedTable(): string | null {
  try {
    return localStorage.getItem(STORE_KEY)
  } catch {
    return null
  }
}

export function forgetTable(): void {
  try {
    localStorage.removeItem(STORE_KEY)
  } catch {
    // Nothing stored.
  }
}
