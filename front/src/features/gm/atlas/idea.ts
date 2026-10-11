import type { StoryFront } from '@/lib/campaigns'
import type { Change, Proposal, ProposalItem } from '@/lib/prep'
import { linesOf, type CampaignForm } from '../campaignForm'

/**
 * campaign/write-story-by-map-scale — what « L'idée » reads of Claude's
 * waiting batches (the atlas's `claude`): the items about the campaign,
 * the bible, an act or a front, each shown dotted beside what it would
 * change until the GM keeps or discards it. Pure.
 */

/** One of Claude's items still waiting, and the batch it belongs to. */
export interface Waiting {
  proposal: Proposal
  item: ProposalItem & { change: Change }
}

/** Every item still waiting in `batches` whose change `match` picks. */
export function waiting(batches: readonly Proposal[], match: (c: Change) => boolean): Waiting[] {
  return batches.flatMap((proposal) =>
    (proposal.items ?? []).flatMap((item) =>
      item.status === 'pending' && item.change && match(item.change)
        ? [{ proposal, item: { ...item, change: item.change } }]
        : [],
    ),
  )
}

/** A change setting `field` of `target` (an id, `bible` or `campaign`). */
export const setting =
  (target: string, field: string) =>
  (c: Change): boolean =>
    c.op === 'set' && c.id === target && c.field === field

/** A change adding an entity of `kind` (`act`, `front`). */
export const adding =
  (kind: string) =>
  (c: Change): boolean =>
    c.op === 'add' && c.kind === kind

/** Any change touching the entity `id`. */
export const touching =
  (id: string) =>
  (c: Change): boolean =>
    c.id === id

/** Where each field of « L'idée » lives in the story; the table's settings are not story. */
export const IDEA_TARGETS: Partial<Record<keyof CampaignForm, [target: string, field: string]>> = {
  title: ['campaign', 'title'],
  world: ['campaign', 'world'],
  pitch: ['bible', 'pitch'],
  playerHook: ['bible', 'player_hook'],
  tone: ['bible', 'tone'],
  themes: ['bible', 'themes'],
}

/** A proposed value as the GM reads it: text, a list, or an entity's name. */
export function shownValue(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  if (Array.isArray(value)) return value.map(shownValue).filter(Boolean).join(', ')
  if (typeof value === 'object') {
    const o = value as Record<string, unknown>
    for (const key of ['name', 'title', 'label', 'text']) if (typeof o[key] === 'string') return o[key]
  }
  return JSON.stringify(value)
}

/**
 * A front's clock typed one step per line: each step keeps the
 * description of the step it replaces at the same rank.
 */
export function stepsOf(text: string, before: StoryFront['steps']): StoryFront['steps'] {
  return linesOf(text).map((label, i) => {
    const description = before[i]?.description
    return description ? { label, description } : { label }
  })
}

/** The first `<prefix><n>` no id of `taken` uses. */
export function nextId(prefix: string, taken: readonly { id: string }[]): string {
  const ids = new Set(taken.map((x) => x.id))
  let n = taken.length + 1
  while (ids.has(`${prefix}${n}`)) n++
  return `${prefix}${n}`
}
