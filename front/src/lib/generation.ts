import { apiRequest } from './api'
import type { Story, StoryIssue } from './prep'

export type GenerationLength = 'one_shot' | 'short' | 'long'

/** What the GM asks for (`prep::generation::Input`). */
export interface GenerationInput {
  pitch: string
  tone: string
  themes: string
  constraints: string
  length: GenerationLength
}

type StepId = 'cast' | 'scenes' | 'check'

export interface GenerationStep {
  id: StepId
  status: 'pending' | 'running' | 'done' | 'failed'
  counts: Record<string, number>
}

/** A generation job, followed live (`ai/generate-campaign`). */
export interface GenerationJob {
  id: string
  status: 'running' | 'succeeded' | 'failed' | 'applied'
  input: GenerationInput
  steps: GenerationStep[]
  draft: Story | null
  issues: StoryIssue[]
  removed: { code: string; path: string; detail: string }[]
  repairs: number
  dropped: number
  costMicros: number
  error: string | null
  detail: string | null
  createdAt: string
}

export interface GenerationDesk {
  jobs: GenerationJob[]
  /** The most one more job costs. */
  estimateMicros: number
  spending: { budgetMicros: number; spentMicros: number }
  validated: boolean
}

const base = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}/generation`

export function fetchGenerations(campaignId: string): Promise<GenerationDesk> {
  return apiRequest<GenerationDesk>('GET', base(campaignId))
}

export function startGeneration(campaignId: string, input: GenerationInput): Promise<GenerationJob> {
  return apiRequest<GenerationJob>('POST', base(campaignId), input)
}

export function applyGeneration(campaignId: string, jobId: string): Promise<{ job: GenerationJob }> {
  return apiRequest('POST', `${base(campaignId)}/${encodeURIComponent(jobId)}/apply`)
}

/** Dollars, two decimals, French style (`0,42`). */
export function dollars(micros: number): string {
  return (micros / 1_000_000).toLocaleString('fr-FR', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}
