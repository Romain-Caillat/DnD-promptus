import { apiRequest } from './api'
import type { CharacterStatus, Role } from './play'

/**
 * The part of the players' view the invite panel quotes: read from the
 * GM's preview of the projection, so the Discord message can only repeat
 * what players may see.
 */
export interface PlayerPreview {
  title: string
  world: string
  playerHook: string
}

/** The campaign's active link. The code is only in a minted answer. */
export interface InviteStatus {
  createdAt: string
  expiresAt: string
}

/** One seat at the table, as the GM sees it. */
export interface Seat {
  id: string
  nickname: string
  role: Role
  joinedAt: string
  lastSeenAt: string
  character: { id: string; status: CharacterStatus; name: string; updatedAt: string } | null
}

export function fetchPlayerPreview(campaignId: string): Promise<PlayerPreview> {
  return apiRequest<PlayerPreview>('GET', `/campaigns/${encodeURIComponent(campaignId)}/player-view`)
}

export function fetchInvite(campaignId: string): Promise<InviteStatus | null> {
  return apiRequest<InviteStatus | null>('GET', `/campaigns/${encodeURIComponent(campaignId)}/invite`)
}

export function mintInvite(campaignId: string): Promise<InviteStatus & { code: string }> {
  return apiRequest<InviteStatus & { code: string }>(
    'POST',
    `/campaigns/${encodeURIComponent(campaignId)}/invite`,
  )
}

export function closeInvite(campaignId: string): Promise<void> {
  return apiRequest<void>('DELETE', `/campaigns/${encodeURIComponent(campaignId)}/invite`)
}

export function listSeats(campaignId: string): Promise<Seat[]> {
  return apiRequest<Seat[]>('GET', `/campaigns/${encodeURIComponent(campaignId)}/players`)
}

export function removeSeat(campaignId: string, playerId: string): Promise<void> {
  return apiRequest<void>(
    'DELETE',
    `/campaigns/${encodeURIComponent(campaignId)}/players/${encodeURIComponent(playerId)}`,
  )
}

/** The link a player opens to join. */
export function joinLink(code: string): string {
  return `${window.location.origin}/rejoindre/${encodeURIComponent(code)}`
}

/*
 * The server keeps only the hash of an invitation code, so the code is
 * shown once. This device remembers the code it minted, tied to the
 * link's creation date: when the server's active link is that one, the
 * GM can copy it again from here; on another device, or once the link
 * changed, they mint a new one.
 */
const STORE_PREFIX = 'promptus.invite.'

interface RememberedCode {
  code: string
  createdAt: string
}

export function rememberCode(campaignId: string, code: string, createdAt: string): void {
  try {
    localStorage.setItem(STORE_PREFIX + campaignId, JSON.stringify({ code, createdAt }))
  } catch {
    // Private mode: the link is shown until the page closes.
  }
}

export function forgetCode(campaignId: string): void {
  try {
    localStorage.removeItem(STORE_PREFIX + campaignId)
  } catch {
    // Nothing stored.
  }
}

/** The code this device minted for the active link, if it is still it. */
export function rememberedCode(campaignId: string, active: InviteStatus | null): string | null {
  if (!active) return null
  try {
    const raw = localStorage.getItem(STORE_PREFIX + campaignId)
    if (!raw) return null
    const saved = JSON.parse(raw) as RememberedCode
    return saved.createdAt === active.createdAt ? saved.code : null
  } catch {
    return null
  }
}
