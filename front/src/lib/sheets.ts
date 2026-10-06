import type { CharacterLook } from '@/features/sprites/look'
import { apiRequest } from './api'
import type { PlayView } from './play'

/** One character in play on the GM's board (`api::sheets`). */
export interface BoardSheet {
  characterId: string
  playerId: string
  nickname: string
  name: string
  className: string | null
  look: CharacterLook | null
  /** `null` when the sheet names no class of the rules. */
  play: PlayView | null
}

/** An item of the rules the GM can give. */
interface GivableItem {
  id: string
  name: string
  description: string
  consumable: boolean
}

type HistoryKind = 'xp' | 'hit_points' | 'resource' | 'item' | 'equip'

/** One change to a character in play, as the history keeps it. */
export interface HistoryEntry {
  id: string
  characterId: string
  actor: 'gm' | 'player'
  kind: HistoryKind
  /** The resource's or the item's name. */
  label: string | null
  before: number
  after: number
  createdAt: string
}

export interface Board {
  /** `false` when the server does not have the campaign's rules. */
  rulesKnown: boolean
  sheets: BoardSheet[]
  items: GivableItem[]
  resources: { id: string; name: string; abbr: string }[]
  /** Newest first. */
  history: HistoryEntry[]
}

/** One gesture of the GM (`players::play::Adjustment`). */
export type Adjustment =
  | { kind: 'xp'; delta: number }
  | { kind: 'hitPoints'; delta: number }
  | { kind: 'resource'; resource: string; delta: number }
  | { kind: 'giveItem'; item?: string; name?: string; description?: string; qty?: number }
  | { kind: 'takeItem'; entry: string; qty?: number }

const campaignPath = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`

export function fetchBoard(campaignId: string): Promise<Board> {
  return apiRequest<Board>('GET', `${campaignPath(campaignId)}/sheets`)
}

/** Apply one gesture; answers the character as the board shows it. */
export function adjustSheet(campaignId: string, characterId: string, adjustment: Adjustment): Promise<BoardSheet> {
  return apiRequest<BoardSheet>(
    'POST',
    `${campaignPath(campaignId)}/characters/${encodeURIComponent(characterId)}/adjust`,
    adjustment,
  )
}
