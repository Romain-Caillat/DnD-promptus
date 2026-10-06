import { apiRequest } from './api'
import type { RulesRef } from './campaigns'

/** A finding of the campaign validator (`story::validate`). */
export interface StoryIssue {
  severity: 'error' | 'warning' | 'info'
  code: string
  path: string
  detail: string
}

/** Any entity of the story: an id, a name of some sort, the rest of its fields. */
export interface Entity {
  id: string
  [field: string]: unknown
}

interface StoryNode extends Entity {
  act: string
  title: string
  optional?: boolean
  summary?: string
  read_aloud?: string
  flow?: string
}

interface StoryClue extends Entity {
  revelation: string
  node: string
  text: string
  discovery?: string
}

export interface Story {
  id: string
  title: string
  world?: string
  rules: RulesRef
  bible: Record<string, unknown>
  party?: Entity[]
  acts?: (Entity & { title: string })[]
  fronts?: Entity[]
  nodes?: StoryNode[]
  revelations?: (Entity & { statement: string; importance: 'critical' | 'optional' })[]
  clues?: StoryClue[]
  npcs?: Entity[]
  adversaries?: Entity[]
  locations?: Entity[]
  items?: Entity[]
}

/** The campaign as the review screen reads it (`GET /api/campaigns/{id}`). */
export interface ReviewCampaign {
  id: string
  story: Story
  issues: StoryIssue[]
  validatedAt: string | null
  updatedAt: string
}

/** One change by id (`story::edit::Edit`). */
export type Edit =
  | { op: 'set'; target: string; field: string; value: unknown }
  | { op: 'add'; kind: string; value: Entity }
  | { op: 'remove'; target: string }

/** What an edit changed (`story::edit::Change`). */
export interface Change {
  op: 'add' | 'set' | 'remove'
  kind: string
  id: string
  name: string
  field?: string
  before?: unknown
  after?: unknown
  place?: string
}

export interface Proposal {
  id: string
  prompt: string
  reply: string
  changes: Change[]
  stale: boolean
  dropped: number
  status: 'pending' | 'accepted' | 'rejected'
  createdAt: string
}

export interface WorkshopAsk {
  prompt?: string
  node?: string
  issue?: { code: string; path: string }
}

const base = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`

export function fetchReview(campaignId: string): Promise<ReviewCampaign> {
  return apiRequest<ReviewCampaign>('GET', base(campaignId))
}

export function applyEdits(
  campaignId: string,
  edits: Edit[],
): Promise<{ campaign: ReviewCampaign; changes: Change[] }> {
  return apiRequest('POST', `${base(campaignId)}/story/edits`, { edits })
}

export function validateCampaign(campaignId: string): Promise<ReviewCampaign> {
  return apiRequest<ReviewCampaign>('POST', `${base(campaignId)}/story/validate`)
}

export function listProposals(campaignId: string): Promise<Proposal[]> {
  return apiRequest<Proposal[]>('GET', `${base(campaignId)}/workshop`)
}

export function askWorkshop(campaignId: string, ask: WorkshopAsk): Promise<Proposal> {
  return apiRequest<Proposal>('POST', `${base(campaignId)}/workshop`, ask)
}

export function decideProposal(
  campaignId: string,
  proposalId: string,
  accept: boolean,
): Promise<{ campaign: ReviewCampaign; proposal: Proposal }> {
  return apiRequest('POST', `${base(campaignId)}/workshop/${proposalId}/${accept ? 'accept' : 'reject'}`)
}

/** The value at a dotted `path` of `entity`. */
export function valueAt(entity: Record<string, unknown>, path: string): unknown {
  let node: unknown = entity
  for (const key of path.split('.')) {
    if (node === null || typeof node !== 'object') return undefined
    node = (node as Record<string, unknown>)[key]
  }
  return node
}

/** The display name of an entity: its name, title, statement or text. */
export function nameOf(entity: Entity | undefined): string {
  if (!entity) return ''
  for (const key of ['name', 'title', 'statement', 'label', 'text']) {
    const v = entity[key]
    if (typeof v === 'string' && v) return v
  }
  return entity.id
}

const LISTS = [
  'party',
  'acts',
  'fronts',
  'nodes',
  'revelations',
  'clues',
  'npcs',
  'adversaries',
  'locations',
  'items',
] as const

/** Every entity of the story, by id. */
export function entities(story: Story): Map<string, Entity> {
  const out = new Map<string, Entity>()
  for (const list of LISTS) for (const e of story[list] ?? []) out.set(e.id, e)
  return out
}

/** The entity a validator path points at (`nodes[3].requires[0]` → node 3). */
export function entityAtPath(story: Story, path: string): Entity | undefined {
  const m = /^([a-z_]+)\[(\d+)\]/.exec(path)
  if (!m) return undefined
  const list = story[m[1] as (typeof LISTS)[number]]
  return Array.isArray(list) ? list[Number(m[2])] : undefined
}

/** A fresh id from a name: `cl_` + lowercase ASCII, never one the story has. */
export function freshId(prefix: string, name: string, story: Story): string {
  const stem =
    prefix +
    (name
      .normalize('NFD')
      .replace(/[̀-ͯ]/g, '')
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '_')
      .replace(/^_+|_+$/g, '')
      .slice(0, 32) || 'nouveau')
  const taken = entities(story)
  let id = stem
  for (let n = 2; taken.has(id); n++) id = `${stem}_${n}`
  return id
}
