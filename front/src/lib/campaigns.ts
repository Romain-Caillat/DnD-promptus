import { ApiError, apiRequest } from './api'

/** Which rule system, at which locked version, a campaign plays. */
export interface RulesRef {
  id: string
  version: number
}

/** A stat as a rule system names it (`FOR` Force, `PV` Points de vie). */
interface NamedStat {
  abbr: string
  name: string
}

/** A rule system a new campaign can start from, and the stats it brings. */
export interface RulePreset extends RulesRef {
  name: string
  description: string
  abilities: NamedStat[]
  hitPoints: NamedStat
  armorClass: NamedStat
}

/** One card of the GM's campaign list. */
export interface CampaignSummary {
  id: string
  title: string
  world: string
  rules: RulesRef
  playerCount: number
  /** Players who joined to play; spectators are not counted. */
  playersSeated: number
  archivedAt: string | null
  lastActivityAt: string
}

/** The GM's planning: never shown to players. */
interface CampaignPlan {
  playerCount: number
  /** The most AI may spend on the campaign, in US cents. */
  aiBudgetCents: number
}

/** A campaign as its page reads it (the story is much larger; only what the page shows is typed). */
export interface CampaignDetail {
  id: string
  story: {
    title: string
    world?: string
    rules: RulesRef
    bible: { pitch?: string; player_hook?: string }
  }
  settings: CampaignPlan
  archivedAt: string | null
  /** When the GM declared it playable; `null` while it is being prepared. */
  validatedAt?: string | null
}

/** What the GM edits on the campaign page. */
export interface CampaignSettings extends CampaignPlan {
  title: string
  world: string
  pitch: string
  playerHook: string
}

export type NewCampaign = CampaignSettings & { rules: RulesRef }

/** A campaign's rule system and the stat names it brings, when the server has it. */
export function presetOf(presets: RulePreset[], rules: RulesRef): RulePreset | undefined {
  return presets.find((p) => p.id === rules.id && p.version === rules.version)
}

export function listCampaigns(): Promise<CampaignSummary[]> {
  return apiRequest<CampaignSummary[]>('GET', '/campaigns')
}

export function listRulePresets(): Promise<RulePreset[]> {
  return apiRequest<RulePreset[]>('GET', '/rule-systems')
}

export function createCampaign(input: NewCampaign): Promise<CampaignDetail> {
  return apiRequest<CampaignDetail>('POST', '/campaigns', input)
}

export function importCampaign(yaml: string): Promise<{ id: string }> {
  return apiRequest<{ id: string }>('POST', '/campaigns/import', { yaml })
}

export function fetchCampaign(campaignId: string): Promise<CampaignDetail> {
  return apiRequest<CampaignDetail>('GET', `/campaigns/${encodeURIComponent(campaignId)}`)
}

export function saveCampaignSettings(campaignId: string, settings: CampaignSettings): Promise<CampaignDetail> {
  return apiRequest<CampaignDetail>('PUT', `/campaigns/${encodeURIComponent(campaignId)}/settings`, settings)
}

export function archiveCampaign(campaignId: string, archived: boolean): Promise<CampaignDetail> {
  return apiRequest<CampaignDetail>('PUT', `/campaigns/${encodeURIComponent(campaignId)}/archive`, {
    archived,
  })
}

/**
 * A budget typed in dollars, in cents: `10`, `2,50` and `2.5` all read;
 * anything else (a negative amount, a third decimal, text) is `null`.
 */
export function parseDollars(text: string): number | null {
  const match = /^\s*(\d{1,5})(?:[.,](\d{1,2}))?\s*$/.exec(text)
  if (!match) return null
  const [, whole, fraction = ''] = match
  return Number(whole) * 100 + Number(fraction.padEnd(2, '0'))
}

/** Cents as the GM types them back: `1000` → `10`, `250` → `2,50`. */
export function formatDollars(cents: number): string {
  const whole = Math.floor(cents / 100)
  const rest = cents % 100
  return rest === 0 ? String(whole) : `${whole},${String(rest).padStart(2, '0')}`
}

/** An NPC the players have met. */
interface PlayerNpc {
  id: string
  name: string
  title: string
  appearance: string
}

/** What players see of a campaign now: the server's single projection. */
export interface PlayerView {
  title: string
  world: string
  playerHook: string
  scene: {
    id: string
    /** Its act: the act's introduction video plays above the scene. */
    act: string
    title: string
    readAloud: string
    place: { name: string; description: string } | null
    npcs: PlayerNpc[]
    opponents: { label: string; count: number }[]
  } | null
  clues: string[]
  npcs: PlayerNpc[]
}

export type PlayerViewResult =
  | { kind: 'ready'; view: PlayerView }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }

/** `GET /api/campaigns/{id}/player-view`: the GM's preview of it. */
export async function fetchPlayerView(campaignId: string): Promise<PlayerViewResult> {
  try {
    const view = await apiRequest<PlayerView>(
      'GET',
      `/campaigns/${encodeURIComponent(campaignId)}/player-view`,
    )
    return { kind: 'ready', view }
  } catch (err) {
    if (err instanceof ApiError && err.status === 401) return { kind: 'signed-out' }
    if (err instanceof ApiError && err.status === 404) return { kind: 'not-found' }
    throw err
  }
}
