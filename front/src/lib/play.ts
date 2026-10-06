import type { CharacterLook } from '@/features/sprites/look'
import { ApiError, apiRequest } from './api'
import type { PlayerView } from './campaigns'

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

/** Three short answers and the paragraph made of them (`players::Backstory`). */
export interface Backstory {
  origin?: string
  loss?: string
  quest?: string
  text?: string
}

/** What the player wrote about their character (`players::CharacterSheet`). */
export interface CharacterSheet {
  name?: string
  peopleId?: string
  classId?: string
  abilities?: Record<string, number>
  appearance?: string
  /** The description the server draws, never an image. */
  look?: CharacterLook
  backstory?: Backstory
}

/** An action card as the rules deal it (`projection::ActionCardView`). */
export interface ActionCardView {
  id: string
  name: string
  description: string
  /** The action kind it spends (« Attaque »). */
  kind: string
  /** Unlocks at this level; 1 for a starting card. */
  level: number
  attackBonus: number | null
  damage: string | null
  heal: string | null
  cooldown: number
  range: number
}

/** A sheet's numbers at level 1, computed by the server. */
export interface SheetStats {
  hitPoints: number
  armorClass: number
  initiative: number
  modifiers: Record<string, number>
  cards: ActionCardView[]
}

/** An ability's score in play, with the modifier the rules give it. */
interface AbilityScore {
  id: string
  name: string
  score: number
  modifier: number
}

/** A resource of the rules (the Corsaires' gold), as held now. */
interface ResourceAmount {
  id: string
  name: string
  abbr: string
  amount: number
}

/** A bag line (`projection::ItemView`). */
export interface BagItem {
  /** Stable within the bag: what `equipItem` names. */
  key: string
  itemId: string | null
  name: string
  description: string
  qty: number
  consumable: boolean
  /** On the character rather than in the bag. */
  equipped: boolean
}

/**
 * A character in play (`projection::PlayView`): what the server holds
 * and what the rules derive from it. The GM's board reads the same.
 */
export interface PlayView {
  level: number
  totalXp: number
  /** Fills up to `xpBarMax`, then turns into an upgrade point. */
  xpBar: number
  xpBarMax: number
  upgradePoints: number
  nextLevelXp: number | null
  hitPoints: number
  maxHitPoints: number
  armorClass: number
  initiative: number
  abilities: AbilityScore[]
  /** Every class card; a `level` above the character's is still locked. */
  cards: ActionCardView[]
  resources: ResourceAmount[]
  inventory: BagItem[]
  /** What a level adds to hit points, when the rules make them grow. */
  levelHitPoints: LevelHitPoints | null
  /** Levels reached whose hit points are still to take. */
  levelsToChoose: number[]
}

/** A level's hit points as the level-up screen offers them. */
interface LevelHitPoints {
  /** The die (« 1d10 »). */
  dice: string
  /** Its average, rounded up. */
  average: number
  /** Added to either (the Constitution modifier). */
  bonus: number
  bonusFormula: string
}

/** What taking a level's hit points gave. */
export interface LevelTaken {
  level: number
  die: number
  /** The faces rolled, `null` for the average. */
  faces: number[] | null
  maxBefore: number
  maxAfter: number
}

/** A player's own character (`projection::CharacterView`). */
export interface CharacterView {
  id: string
  status: CharacterStatus
  sheet: CharacterSheet
  gmNote: string | null
  updatedAt: string
  /** The sheet's people and class as the rules name them. */
  peopleName: string | null
  className: string | null
  /** `null` until the sheet names a class of the rules. */
  stats: SheetStats | null
  /** Once validated, the character in play. */
  play?: PlayView | null
}

/** A player's home in one campaign (`projection::PlayerHomeView`). */
export interface PlayerHome {
  me: { id: string; nickname: string; role: Role }
  campaign: Invitation
  character: CharacterView | null
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

/** Carry bag line `entry` on the character, or put it back in the bag. */
export function equipItem(campaignId: string, entry: string, equipped: boolean): Promise<CharacterView> {
  return apiRequest<CharacterView>('POST', `/play/${encodeURIComponent(campaignId)}/character/equip`, {
    entry,
    equipped,
  })
}

/** The campaign as players see it now (`GET /api/play/…/view`). */
export function fetchCampaignView(campaignId: string): Promise<PlayerView> {
  return apiRequest<PlayerView>('GET', `/play/${encodeURIComponent(campaignId)}/view`)
}

/** Take a reached level's hit points: the server rolls, or the average. */
export function takeLevel(
  campaignId: string,
  level: number,
  choice: 'roll' | 'average',
): Promise<{ taken: LevelTaken; character: CharacterView }> {
  return apiRequest('POST', `/play/${encodeURIComponent(campaignId)}/character/level-up`, { level, choice })
}

/** Spend an upgrade point: +1 to an ability. */
export function spendUpgrade(campaignId: string, ability: string): Promise<CharacterView> {
  return apiRequest<CharacterView>('POST', `/play/${encodeURIComponent(campaignId)}/character/upgrade`, { ability })
}
