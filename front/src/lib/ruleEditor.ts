import { apiRequest } from './api'
import type { RuleChange } from './rules'

/** A finding of a check (`issue::Issue`). */
interface Issue {
  severity: 'error' | 'warning' | 'info'
  code: string
  path: string
  detail: string
  message?: string
}

interface FightSummary {
  fights: number
  partyWinRate: number
  rounds: number
  minutes: number
}

/** One fight played on the current version and on the draft. */
interface FightCheck {
  id: string
  name: string
  source: 'scenario' | 'encounter'
  current: FightSummary | null
  draft: FightSummary | null
  error: string | null
}

/** What a draft touches (`rules::report::Report`). */
export interface RuleReport {
  lint: Issue[]
  lintAdded: Issue[]
  lintRemoved: Issue[]
  changes: RuleChange[]
  story: Issue[]
  fights: FightCheck[]
}

interface VersionInfo {
  version: number
  note: string
  lockedAt: string | null
  createdAt: string
  updatedAt: string
  preset: boolean
}

/** The rule system as a JSON tree (`docs/rules-format.md`), edited in place. */
export type RuleDocument = Record<string, unknown>

interface RuleDraft {
  version: number
  note: string
  yaml: string
  document: RuleDocument
  report: RuleReport
}

export interface RuleEditor {
  rulesId: string
  name: string
  current: number
  next: number | null
  versions: VersionInfo[]
  draft: RuleDraft | null
}

interface DiffLine {
  kind: 'same' | 'removed' | 'added'
  text: string
  skipped?: number
}

export interface Comparison {
  from: number
  to: number
  changes: RuleChange[]
  lines: DiffLine[]
}

const base = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}/rules`

export function fetchRuleEditor(campaignId: string): Promise<RuleEditor> {
  return apiRequest<RuleEditor>('GET', base(campaignId))
}

export function startRuleDraft(campaignId: string): Promise<RuleEditor> {
  return apiRequest<RuleEditor>('POST', `${base(campaignId)}/draft`)
}

export function saveRuleDraft(
  campaignId: string,
  input: { yaml: string; note: string } | { document: RuleDocument; note: string },
): Promise<RuleEditor> {
  return apiRequest<RuleEditor>('PUT', `${base(campaignId)}/draft`, input)
}

export function discardRuleDraft(campaignId: string): Promise<RuleEditor> {
  return apiRequest<RuleEditor>('DELETE', `${base(campaignId)}/draft`)
}

export function lockRuleDraft(campaignId: string): Promise<RuleEditor> {
  return apiRequest<RuleEditor>('POST', `${base(campaignId)}/draft/lock`)
}

export function compareRules(campaignId: string, from: number, to: number): Promise<Comparison> {
  return apiRequest<Comparison>('GET', `${base(campaignId)}/compare?from=${from}&to=${to}`)
}

/** A copy of `doc` with `value` set at `path` (keys and indexes). */
export function setAt(doc: RuleDocument, path: (string | number)[], value: unknown): RuleDocument {
  const root: unknown = structuredClone(doc)
  let node = root as Record<string | number, unknown>
  for (const key of path.slice(0, -1)) {
    node = node[key] as Record<string | number, unknown>
  }
  node[path[path.length - 1]] = value
  return root as RuleDocument
}

/** The list at `key` of `doc` (an empty one when absent). */
export function listOf<T>(doc: RuleDocument, key: string): T[] {
  const value = doc[key]
  return Array.isArray(value) ? (value as T[]) : []
}

/** A stable id from a name: lowercase ASCII and `_` (`Le feu effraie` → `le_feu_effraie`). */
export function slugId(name: string, taken: string[]): string {
  const ascii = name
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')
  const stem = ascii || 'regle'
  let id = stem
  for (let n = 2; taken.includes(id); n++) id = `${stem}_${n}`
  return id
}
