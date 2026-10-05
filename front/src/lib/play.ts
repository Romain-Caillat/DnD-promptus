import { ApiError, apiRequest } from './api'

/** What an invitation link opens, as the server's projection gives it. */
export interface Invitation {
  campaignId: string
  title: string
  world: string
  playerHook: string
  gmName: string
}

export type Role = 'player' | 'spectator'
export type CharacterStatus = 'draft' | 'submitted' | 'validated' | 'returned'

/** What the player wrote about their character (`players::CharacterSheet`). */
interface CharacterSheet {
  name?: string
  classId?: string
  abilities?: Record<string, number>
  appearance?: string
}

/** A player's home in one campaign (`projection::PlayerHomeView`). */
export interface PlayerHome {
  me: { id: string; nickname: string; role: Role }
  campaign: Invitation
  character: {
    id: string
    status: CharacterStatus
    sheet: CharacterSheet
    gmNote: string | null
    updatedAt: string
  } | null
}

/** The campaign an invitation code opens, or `null` for a dead link. */
export async function fetchInvitation(code: string): Promise<Invitation | null> {
  try {
    return await apiRequest<Invitation>('GET', `/join/${encodeURIComponent(code)}`)
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) return null
    throw err
  }
}

/**
 * Take a seat. The device token comes back in an HttpOnly cookie scoped
 * to the campaign: no script ever reads it.
 */
export function joinCampaign(code: string, nickname: string, role: Role): Promise<PlayerHome> {
  return apiRequest<PlayerHome>('POST', `/join/${encodeURIComponent(code)}`, { nickname, role })
}

/** This device's seat in `campaignId`, or `null` when it has none. */
export async function fetchPlayerHome(campaignId: string): Promise<PlayerHome | null> {
  try {
    return await apiRequest<PlayerHome>('GET', `/play/${encodeURIComponent(campaignId)}/me`)
  } catch (err) {
    if (err instanceof ApiError && err.status === 401) return null
    throw err
  }
}

/** Where a player's home in `campaignId` lives in the app. */
export function playPath(campaignId: string): string {
  return `/partie/${encodeURIComponent(campaignId)}`
}
