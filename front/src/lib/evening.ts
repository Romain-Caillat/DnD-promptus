import { apiRequest } from './api'
import type { PlayerView } from './campaigns'
import type { RollBreakdown } from './rules'

const play = (campaignId: string) => `/play/${encodeURIComponent(campaignId)}`
const gm = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`

type SessionStatus = 'lobby' | 'live' | 'ended'
type MusicMood = 'calm' | 'exploration' | 'tension' | 'mystery' | 'combat' | 'epic'

/** The track everyone hears, and when it started (`session::Music`). */
export interface Music {
  url: string
  title: string
  mood: MusicMood
  startedAt: string
}

export type JournalKind =
  | 'scene'
  | 'clue'
  | 'npc'
  | 'promise'
  | 'debt'
  | 'item'
  | 'note'
  | 'roll'
  | 'narration'
  | 'fight'
  | 'loot'

type RequestStatus = 'pending' | 'accepted' | 'refused' | 'check' | 'rolled' | 'withdrawn'

/** A card played with a request. */
export type Card = { kind: 'ability'; ability: string } | { kind: 'action'; action: string } | { kind: 'other' }

/** A player's own request, as the projection gives it. */
export interface RequestView {
  id: string
  card: { kind: Card['kind']; id: string | null; name: string | null }
  text: string
  status: RequestStatus
  gmReason: string | null
  check: { ability: string; abilityName: string; difficulty: number; label: string | null } | null
  roll: RollBreakdown | null
  outcome: string | null
  contested: boolean
  createdAt: string
}

export interface SceneCard {
  kind: 'ability' | 'action'
  id: string
  name: string
  description: string
  modifier: number | null
}

/** The evening on a player's phone (`projection::evening::EveningView`). */
export interface EveningView {
  session: { number: number; status: SessionStatus; startedAt: string | null } | null
  previously: string | null
  /** At the launch, the GM reading « Précédemment… » aloud: how many lines are up. */
  reading: { lines: string[]; shown: number } | null
  music: Music | null
  lobby: { playerId: string; nickname: string; soundOk: boolean; remote: boolean }[]
  campaign: PlayerView
  journal: { kind: JournalKind; text: string; createdAt: string }[]
  requests: RequestView[]
  cards: SceneCard[]
  feedback: { number: number; answered: boolean } | null
}

export type Answer = 'yes' | 'partly' | 'no'

export interface FeedbackAnswers {
  rulesClear: Answer
  hadMoment: Answer
  knowsNext: Answer
  comment: string
}

export function fetchEvening(campaignId: string): Promise<EveningView> {
  return apiRequest<EveningView>('GET', `${play(campaignId)}/evening`)
}

export function arrive(campaignId: string, soundOk: boolean, remote: boolean): Promise<EveningView> {
  return apiRequest<EveningView>('POST', `${play(campaignId)}/lobby`, { soundOk, remote })
}

export function askGm(campaignId: string, card: Card, text: string): Promise<EveningView> {
  return apiRequest<EveningView>('POST', `${play(campaignId)}/requests`, { card, text })
}

export function rollRequest(campaignId: string, id: string): Promise<EveningView> {
  return apiRequest<EveningView>('POST', `${play(campaignId)}/requests/${encodeURIComponent(id)}/roll`)
}

export function followRequest(campaignId: string, id: string, follow: 'withdraw' | 'contest'): Promise<EveningView> {
  return apiRequest<EveningView>('POST', `${play(campaignId)}/requests/${encodeURIComponent(id)}/${follow}`)
}

export function answerFeedback(campaignId: string, answers: FeedbackAnswers): Promise<EveningView> {
  return apiRequest<EveningView>('POST', `${play(campaignId)}/feedback`, answers)
}

/** Seconds into the track a phone opening now starts at, so everyone hears the same bar. */
export function musicOffset(music: Music, now: number = Date.now()): number {
  return Math.max(0, Math.floor((now - Date.parse(music.startedAt)) / 1000))
}

/** The YouTube id of a link (`watch?v=`, `youtu.be/`, `embed/`), or `null`. */
export function youtubeId(url: string): string | null {
  const m = /(?:youtu\.be\/|[?&]v=|embed\/|shorts\/)([\w-]{11})/.exec(url)
  return m ? m[1] : null
}

// ——— The GM's evening ———

export interface SessionInfo {
  id: string
  number: number
  status: SessionStatus
  openedAt: string
  startedAt: string | null
  endedAt: string | null
  recap: string
  previously: string
  gmChanges: string
  music: Music | null
  /** The chronicle's entry: a title and two lines. */
  title: string
  chronicle: string
  /** Players read « Précédemment… » and the entry once published. */
  published: boolean
  /** At the launch, how many lines of « Précédemment… » the table sees. */
  readingLine: number | null
}

interface MusicTrack {
  mood: MusicMood
  title: string
  url?: string
  search?: string
}

/** The story node in full, as the GM runs it (only what the screen reads). */
interface GmNode {
  id: string
  title: string
  act: string
  summary?: string
  read_aloud?: string
  flow?: string
  key_points?: string[]
  transition?: string
  gm_notes?: string
  map?: string
  location?: string
  encounter?: unknown
  ambience?: { mood?: string; sounds?: string; music?: MusicTrack[] }
}

export interface GmRequest {
  id: string
  playerId: string
  card: Card
  text: string
  status: RequestStatus
  gmReason: string | null
  check: { ability: string; difficulty: number; label: string | null } | null
  roll: RollBreakdown | null
  contested: boolean
  createdAt: string
  nickname: string
  characterName: string
  cardName: string | null
  rulings: { id: string; situation: string; ability: string; difficulty: number; createdAt: string }[]
}

interface Spot {
  playerId: string
  nickname: string
  characterId: string
  characterName: string
  idleMinutes: number
  pendingRequests: number
  hooks: { id: string; title: string }[]
  alert: boolean
}

/** A gesture the co-GM suggests, on ids the campaign has. */
export type CopilotAction =
  | { type: 'reveal_clue'; clue: string }
  | { type: 'advance_front'; front: string }
  | { type: 'enter_scene'; node: string }
  | { type: 'reveal_npc'; npc: string }

interface CopilotAnswer {
  narration: string
  npcLines: { npc: string | null; speaker: string; text: string }[]
  suggestions: { label: string; why: string | null; action: CopilotAction | null }[]
  gmNote: string | null
}

export interface Draft {
  id: string
  kind: CopilotKind
  prompt: string
  answer: CopilotAnswer
  status: 'draft' | 'shown' | 'dismissed'
  createdAt: string
}

export type CopilotKind = 'describe' | 'npc' | 'consequence' | 'next' | 'free'

interface GmJournalLine {
  id: string
  kind: JournalKind
  text: string
  shared: boolean
  ref: string | null
  createdAt: string
}

/** The GM's live screen (`GET /api/campaigns/{id}/session`). */
/** What a scene ahead needs the table to know, and where it can still be learned. */
interface KnowledgeGap {
  node: string
  nodeTitle: string
  revelation: string
  statement: string
  clues: { clue: string; node: string; nodeTitle: string }[]
}

export interface LiveScreen {
  session: SessionInfo | null
  /** The published « Précédemment… », cut into the lines the GM reads. */
  readingLines: string[]
  lastEnded: SessionInfo | null
  lobby: {
    playerId: string
    nickname: string
    role: 'player' | 'spectator'
    online: boolean
    here: boolean
    soundOk: boolean
    remote: boolean
    characterName: string | null
  }[]
  scene: {
    node: GmNode
    clues: { id: string; text: string; discovery: string; found: boolean; revelation: string }[]
    npcs: { id: string; name: string; title: string; role: string; roleplay: string; wants: string; hides: string; met: boolean }[]
    exits: { to: string; label: string; title: string; visited: boolean }[]
  } | null
  nodes: { id: string; title: string; act: string; visited: boolean; current: boolean }[]
  startNode: string | null
  /** What the next scenes need that the table does not know yet. */
  gaps: KnowledgeGap[]
  fronts: { id: string; name: string; goal: string; steps: string[]; progress: number }[]
  requests: GmRequest[]
  journal: GmJournalLine[]
  spotlight: Spot[]
  drafts: Draft[]
  alertAfterMinutes: number
  rules: { abilities: [string, string][]; difficulties: [string, string, number][] } | null
  ai: { configured: boolean; spending: { budgetMicros: number; spentMicros: number } }
}

export type Reveal =
  | { kind: 'scene'; node: string }
  | { kind: 'clue'; clue: string }
  | { kind: 'npc'; npc: string }
  | { kind: 'front'; front: string; delta: number }
  | { kind: 'resolve'; node: string }

export type Decision =
  | { kind: 'accept'; reason: string }
  | { kind: 'refuse'; reason: string }
  | { kind: 'check'; ability: string; difficulty?: string; value?: number }

export function fetchLiveScreen(campaignId: string): Promise<LiveScreen> {
  return apiRequest<LiveScreen>('GET', `${gm(campaignId)}/session`)
}

export function openSession(campaignId: string): Promise<SessionInfo> {
  return apiRequest<SessionInfo>('POST', `${gm(campaignId)}/session`)
}

export function startSession(campaignId: string): Promise<SessionInfo> {
  return apiRequest<SessionInfo>('POST', `${gm(campaignId)}/session/start`)
}

export function endSession(campaignId: string, recap: string, previously: string): Promise<SessionInfo> {
  return apiRequest<SessionInfo>('POST', `${gm(campaignId)}/session/end`, { recap, previously })
}

export function reveal(campaignId: string, r: Reveal): Promise<void> {
  return apiRequest<void>('POST', `${gm(campaignId)}/session/reveal`, r)
}

export function playTrack(campaignId: string, track: number | null): Promise<Music | null> {
  return apiRequest<Music | null>('PUT', `${gm(campaignId)}/session/music`, { track })
}

export function writeNote(campaignId: string, kind: JournalKind, text: string, shared: boolean): Promise<void> {
  return apiRequest<void>('POST', `${gm(campaignId)}/session/journal`, { kind, text, shared })
}

export function decide(campaignId: string, request: string, d: Decision): Promise<unknown> {
  return apiRequest('POST', `${gm(campaignId)}/session/requests/${encodeURIComponent(request)}`, d)
}

export function giveSpotlight(campaignId: string, player: string): Promise<void> {
  return apiRequest<void>('POST', `${gm(campaignId)}/session/spotlight/${encodeURIComponent(player)}`)
}

export function askCopilot(campaignId: string, kind: CopilotKind, prompt: string, npc?: string): Promise<Draft> {
  return apiRequest<Draft>('POST', `${gm(campaignId)}/session/copilot`, { kind, prompt, ...(npc ? { npc } : {}) })
}

export function showDraft(
  campaignId: string,
  draft: string,
  narration: string,
  npcLines: { speaker: string; text: string }[],
): Promise<void> {
  return apiRequest<void>('POST', `${gm(campaignId)}/session/copilot/${encodeURIComponent(draft)}/show`, {
    narration,
    npcLines,
  })
}

export function dismissDraft(campaignId: string, draft: string): Promise<void> {
  return apiRequest<void>('POST', `${gm(campaignId)}/session/copilot/${encodeURIComponent(draft)}/dismiss`)
}

/** What the GM rereads after the evening. */
export interface RecapText {
  recap: string
  previously: string
  title: string
  chronicle: string
}

/** The co-GM's draft of the recaps and the chronicle entry (a counted AI call; nothing saved). */
export async function draftRecap(campaignId: string, session: string): Promise<RecapText> {
  const d = await apiRequest<{ players: string; gm: string; title: string; chronicle: string }>(
    'POST',
    `${gm(campaignId)}/sessions/${encodeURIComponent(session)}/recap-draft`,
  )
  return { recap: d.gm, previously: d.players, title: d.title, chronicle: d.chronicle }
}

/** Save the reread recap of an ended session. */
export function editRecap(campaignId: string, session: string, text: RecapText): Promise<SessionInfo> {
  return apiRequest<SessionInfo>('PUT', `${gm(campaignId)}/sessions/${encodeURIComponent(session)}/recap`, text)
}

/** « Précédemment… » and the chronicle entry reach the players. */
export function publishRecap(campaignId: string, session: string): Promise<SessionInfo> {
  return apiRequest<SessionInfo>('POST', `${gm(campaignId)}/sessions/${encodeURIComponent(session)}/publish`)
}

export interface FeedbackReport {
  sessionId: string
  number: number
  players: {
    playerId: string
    nickname: string
    characterName: string
    answers: FeedbackAnswers | null
    longestIdleMinutes: number
    requests: number
    refused: number
    contested: number
  }[]
  gaps: { node: string; nodeTitle: string; revelation: string; statement: string }[]
  gmChanges: string
}

export function fetchFeedback(campaignId: string, session: string): Promise<FeedbackReport> {
  return apiRequest<FeedbackReport>('GET', `${gm(campaignId)}/sessions/${encodeURIComponent(session)}/feedback`)
}

export function saveChanges(campaignId: string, session: string, text: string): Promise<unknown> {
  return apiRequest('PUT', `${gm(campaignId)}/sessions/${encodeURIComponent(session)}/changes`, { text })
}
