import type { CharacterLook } from '@/features/sprites/look'
import { apiRequest } from './api'
import type { Backstory, CharacterStatus, Role } from './play'

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

/** A seat's character, as the GM's table lists it. */
interface SeatCharacter {
  id: string
  status: CharacterStatus
  /** Empty until the player names it. */
  name: string
  classId: string | null
  /** The class's name in the campaign's rules. */
  className: string | null
  /** What the sprite draws, once the player chose it. */
  look: CharacterLook | null
  /** Sent again after the GM returned it. */
  resubmitted: boolean
  updatedAt: string
}

/** One seat at the table, as the GM sees it. */
export interface Seat {
  id: string
  nickname: string
  role: Role
  joinedAt: string
  lastSeenAt: string
  character: SeatCharacter | null
}

/** What a rule check reports (`promptus_shared::issue::Issue`). Never blocking. */
interface Issue {
  severity: 'error' | 'warning' | 'info'
  code: string
  /** Where in the sheet: `abilities.FOR`, `classId`. */
  path: string
  /** For the GM, in French. */
  message?: string
}

/** One field that moved since the GM's last decision. */
export interface Change {
  path: string
  before: unknown
  after: unknown
}

/** A sheet as the player stored it; the creator may add fields. */
interface ReviewSheet {
  name?: string
  classId?: string
  abilities?: Record<string, number>
  appearance?: string
  backstory?: Backstory
  look?: CharacterLook
}

/** What the GM reads to decide on one character (`players::review::Review`). */
export interface CharacterReview {
  id: string
  playerId: string
  nickname: string
  status: CharacterStatus
  sheet: ReviewSheet
  gmNote: string | null
  updatedAt: string
  reviewedSheet: ReviewSheet | null
  reviewedAt: string | null
  changes: Change[]
  checks: Issue[]
  rulesName: string | null
  className: string | null
  abilities: { id: string; name: string }[]
}

/** A hook the GM draws from a backstory. Never shown to players. */
export interface SecretHook {
  id: string
  characterId: string
  title: string
  body: string
  /** Ids of the story's nodes and fronts it ties into. */
  links: string[]
}

/** A node or front of the story a hook can tie into. */
export interface HookTarget {
  id: string
  kind: 'node' | 'front'
  title: string
}

export interface HookInput {
  title: string
  body: string
  links: string[]
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

const campaignPath = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`

export function fetchReview(campaignId: string, characterId: string): Promise<CharacterReview> {
  return apiRequest<CharacterReview>(
    'GET',
    `${campaignPath(campaignId)}/characters/${encodeURIComponent(characterId)}`,
  )
}

/** Validate the sheet the GM read (`seen` is its `updatedAt`). */
export function validateCharacter(campaignId: string, characterId: string, seen: string): Promise<void> {
  return apiRequest<void>(
    'POST',
    `${campaignPath(campaignId)}/characters/${encodeURIComponent(characterId)}/validate`,
    { seen },
  )
}

/** Return the sheet the GM read to its player, with a word. */
export function returnCharacter(
  campaignId: string,
  characterId: string,
  seen: string,
  note: string,
): Promise<void> {
  return apiRequest<void>(
    'POST',
    `${campaignPath(campaignId)}/characters/${encodeURIComponent(characterId)}/return`,
    { seen, note },
  )
}

export function fetchHooks(campaignId: string): Promise<{ hooks: SecretHook[]; targets: HookTarget[] }> {
  return apiRequest('GET', `${campaignPath(campaignId)}/hooks`)
}

export function addHook(campaignId: string, characterId: string, hook: HookInput): Promise<SecretHook> {
  return apiRequest<SecretHook>('POST', `${campaignPath(campaignId)}/hooks`, { characterId, ...hook })
}

/** The co-GM's word to the player about their sent sheet, for the GM to edit (nothing is sent). */
export async function draftNote(campaignId: string, characterId: string): Promise<string> {
  const r = await apiRequest<{ note: string }>(
    'POST',
    `${campaignPath(campaignId)}/characters/${encodeURIComponent(characterId)}/note-draft`,
  )
  return r.note
}

/** Hooks the co-GM draws from a backstory: nothing is kept until the GM adds one. */
export function proposeHooks(campaignId: string, characterId: string): Promise<{ hooks: HookInput[]; dropped: number }> {
  return apiRequest('POST', `${campaignPath(campaignId)}/characters/${encodeURIComponent(characterId)}/hooks/propose`)
}

export function editHook(campaignId: string, hookId: string, hook: HookInput): Promise<SecretHook> {
  return apiRequest<SecretHook>('PUT', `${campaignPath(campaignId)}/hooks/${encodeURIComponent(hookId)}`, hook)
}

export function deleteHook(campaignId: string, hookId: string): Promise<void> {
  return apiRequest<void>('DELETE', `${campaignPath(campaignId)}/hooks/${encodeURIComponent(hookId)}`)
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
