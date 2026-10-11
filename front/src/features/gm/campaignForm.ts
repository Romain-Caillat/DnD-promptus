import { formatDollars, parseDollars, type CampaignDetail, type CampaignSettings } from '@/lib/campaigns'
import type { Edit } from '@/lib/prep'

/** « L'idée » as typed: numbers stay text until sent, themes one per line. */
export interface CampaignForm {
  title: string
  world: string
  pitch: string
  playerHook: string
  tone: string
  /** One theme per line. */
  themes: string
  playerCount: string
  /** Dollars, as typed (`10`, `2,50`). */
  budget: string
}

/** The non-blank lines of `text`, trimmed. */
export function linesOf(text: string): string[] {
  return text
    .split('\n')
    .map((l) => l.trim())
    .filter(Boolean)
}

export function formOf(detail: CampaignDetail): CampaignForm {
  return {
    title: detail.story.title,
    world: detail.story.world ?? '',
    pitch: detail.story.bible.pitch ?? '',
    playerHook: detail.story.bible.player_hook ?? '',
    tone: detail.story.bible.tone ?? '',
    themes: (detail.story.bible.themes ?? []).join('\n'),
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

/** Whether `settings` change anything the campaign holds. */
export function settingsChanged(settings: CampaignSettings, detail: CampaignDetail): boolean {
  return (
    settings.title !== detail.story.title ||
    settings.world !== (detail.story.world ?? '') ||
    settings.pitch !== (detail.story.bible.pitch ?? '') ||
    settings.playerHook !== (detail.story.bible.player_hook ?? '') ||
    settings.playerCount !== detail.settings.playerCount ||
    settings.aiBudgetCents !== detail.settings.aiBudgetCents
  )
}

/**
 * The bible's tone and themes, which the settings route does not carry:
 * story edits of the changed ones (an empty value clears it).
 */
export function bibleEditsOf(form: CampaignForm, detail: CampaignDetail): Edit[] {
  const edits: Edit[] = []
  const tone = form.tone.trim()
  if (tone !== (detail.story.bible.tone ?? '')) {
    edits.push({ op: 'set', target: 'bible', field: 'tone', value: tone || null })
  }
  const themes = linesOf(form.themes)
  if (themes.join('\n') !== (detail.story.bible.themes ?? []).join('\n')) {
    edits.push({ op: 'set', target: 'bible', field: 'themes', value: themes.length ? themes : null })
  }
  return edits
}
