export type Music = { chosen: number; toFind: number }

export type Scene = {
  id: string
  act: string
  title: string
  optional: boolean
  summary: string
  location: string
  hook: string
  exits: string[]
  mood: string
  invented: boolean
  missing: string[]
  map: { id: string; isFound: boolean } | null
  music: Music
  hasArt: boolean
  portraits: { described: number; total: number }
  opponents: number
  hash: string
}

export type Act = { id: string; title: string; scenes: Scene[] }

export type CampaignRef = { key: string; title: string; path: string; isExample: boolean }

export type Scan = {
  root: string
  campaigns: CampaignRef[]
  key: string
  title: string
  path: string
  mtimeMs: number
  acts: Act[]
  error: string
}

export type Change = 'new' | 'changed'

declare module 'claude-code' {
  interface PluginState {
    'promptus-scenes': {
      scan: Scan | null
      act: string
      scene: string
      changes: Record<string, Change>
    }
  }
}
