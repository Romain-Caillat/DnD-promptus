import { apiRequest } from './api'
import type { ActionCardView, CharacterView } from './play'

/** The tabs of the player's page (planche « Jouer »). */
export type PlayerTab = 'jeu' | 'carte' | 'perso' | 'journal'

/** The caller's own last evening (`projection::between::MyEveningView`). */
interface MyEvening {
  xpGained: number
  levelBefore: number
  level: number
  totalXp: number
  xpBar: number
  xpBarMax: number
  nextLevelXp: number | null
  /** Shared lines about the character: loot handed to them, purchases. */
  got: string[]
}

/** The last ended session (`projection::between::LastSessionView`). */
interface LastSession {
  number: number
  endedAt: string | null
  minutes: number | null
  title: string | null
  /** « Précédemment… », once the GM published it. */
  previously: string | null
  learnt: string[]
  /** `null` for a spectator. */
  mine: MyEvening | null
}

/** The level-up moment: points to place, cards the new level opened. */
interface LevelUp {
  level: number
  points: number
  abilities: { id: string; name: string; score: number; modifier: number }[]
  newCards: ActionCardView[]
}

interface ChronicleEntry {
  number: number
  endedAt: string | null
  title: string | null
  text: string | null
}

/** Between two sessions, as the server projects it for this player. */
export interface BetweenView {
  open: { number: number; status: 'lobby' | 'live' | 'ended'; startedAt: string | null } | null
  last: LastSession | null
  levelUp: LevelUp | null
  /** The first session first. */
  chronicle: ChronicleEntry[]
  openThreads: string[]
}

/** `GET /api/play/…/between`. */
export function fetchBetween(campaignId: string): Promise<BetweenView> {
  return apiRequest<BetweenView>('GET', `/play/${encodeURIComponent(campaignId)}/between`)
}

/** Spend one upgrade point: +1 to `ability`. Answers the character. */
export function spendUpgrade(campaignId: string, ability: string): Promise<CharacterView> {
  return apiRequest<CharacterView>('POST', `/play/${encodeURIComponent(campaignId)}/character/upgrade`, {
    ability,
  })
}
