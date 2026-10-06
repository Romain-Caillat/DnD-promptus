import { formatDollars, parseDollars, type CampaignDetail, type CampaignSettings } from '@/lib/campaigns'

/** The campaign's settings as typed: numbers stay text until sent. */
export interface CampaignForm {
  title: string
  world: string
  pitch: string
  playerHook: string
  playerCount: string
  /** Dollars, as typed (`10`, `2,50`). */
  budget: string
}

/** A new campaign: Romain's table of six, no AI spending yet. */
export const EMPTY_FORM: CampaignForm = {
  title: '',
  world: '',
  pitch: '',
  playerHook: '',
  playerCount: '6',
  budget: '0',
}

export function formOf(detail: CampaignDetail): CampaignForm {
  return {
    title: detail.story.title,
    world: detail.story.world ?? '',
    pitch: detail.story.bible.pitch ?? '',
    playerHook: detail.story.bible.player_hook ?? '',
    playerCount: String(detail.settings.playerCount),
    budget: formatDollars(detail.settings.aiBudgetCents),
  }
}

/** Why a form is refused: the server's own codes. */
export type FormError = 'TITLE_REQUIRED' | 'INVALID_PLAYER_COUNT' | 'INVALID_AI_BUDGET'

/** The largest budget the server takes, in cents (10 000 $). */
const MAX_BUDGET_CENTS = 1_000_000

/** The settings to send, or what the server would refuse them for. */
export function settingsOf(form: CampaignForm): CampaignSettings | FormError {
  if (!form.title.trim()) return 'TITLE_REQUIRED'
  const players = /^\s*\d{1,2}\s*$/.test(form.playerCount) ? Number(form.playerCount) : NaN
  if (!(players >= 1 && players <= 12)) return 'INVALID_PLAYER_COUNT'
  const cents = parseDollars(form.budget)
  if (cents === null || cents > MAX_BUDGET_CENTS) return 'INVALID_AI_BUDGET'
  return {
    title: form.title.trim(),
    world: form.world.trim(),
    pitch: form.pitch.trim(),
    playerHook: form.playerHook.trim(),
    playerCount: players,
    aiBudgetCents: cents,
  }
}
