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
  /** « Précédemment… » once the GM published it; `null` during the launch reading. */
  previously: string | null
  /** gm/launch-session: the sentences of « Précédemment… » the GM has shown so far. */
  launch: { number: number; lines: string[]; total: number } | null
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
  chronicleTitle: string
  chronicle: string
  /** When « Précédemment… » and the chronicle entry reached the players; `null` while drafts. */
  publishedAt: string | null
  gmChanges: string
  music: Music | null
  /** Sentences of « Précédemment… » shown while launching; `null` outside the reading. */
  previouslyShown: number | null
}

/** A name a player text should not hold: a front, or someone the table has not met. */
export interface RecapWarning {
  name: string
  kind: 'front' | 'unmet'
}

/** A session in the GM's chronicle, with the names flagged in its player texts. */
export type ChronicleSession = SessionInfo & { warnings: RecapWarning[] }

/** The recaps of a session, as the GM edits them. */
export interface RecapTexts {
  recap: string
  previously: string
  chronicleTitle: string
  chronicle: string
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
  lastEnded: SessionInfo | null
  /** gm/launch-session: « Précédemment… » whole, how far the table read, the scene to send. */
  launch: {
    number: number
    lines: string[]
    shown: number
    firstScene: { node: string; title: string } | null
  } | null
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
  /** Every faction, its gauge and whether the table knows of it. */
  factions: GmFaction[]
  goals: { id: string; title: string; heldBy: string | null; status: GoalStatus | null }[]
  /** NPCs with the party whatever the scene, played by the co-GM. */
  companions: { id: string; name: string; title: string; roleplay: string }[]
  requests: GmRequest[]
  journal: GmJournalLine[]
  spotlight: Spot[]
  drafts: Draft[]
  alertAfterMinutes: number
  rules: { abilities: [string, string][]; difficulties: [string, string, number][] } | null
  ai: { configured: boolean; spending: { budgetMicros: number; spentMicros: number } }
}

export type GoalStatus = 'known' | 'done'

interface GmFaction {
  id: string
  name: string
  diplomacy: string
  affinity: number
  start: number
  min: number
  max: number
  rivals: string[]
  known: boolean
}

export type Reveal =
  | { kind: 'scene'; node: string }
  | { kind: 'clue'; clue: string }
  | { kind: 'npc'; npc: string }
  | { kind: 'front'; front: string; delta: number }
  | { kind: 'resolve'; node: string }
  | { kind: 'faction'; faction: string }
  | { kind: 'affinity'; faction: string; delta: number }
  | { kind: 'goal'; goal: string; status: GoalStatus | null }

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

/** The co-GM's draft of the recaps (a counted AI call; nothing saved), with the names it should not have written. */
export async function draftRecap(
  campaignId: string,
  session: string,
): Promise<RecapTexts & { warnings: RecapWarning[] }> {
  const d = await apiRequest<{
    players: string
    gm: string
    chronicleTitle: string
    chronicle: string
    warnings: RecapWarning[]
  }>('POST', `${gm(campaignId)}/sessions/${encodeURIComponent(session)}/recap-draft`)
  return { recap: d.gm, previously: d.players, chronicleTitle: d.chronicleTitle, chronicle: d.chronicle, warnings: d.warnings }
}

/** The GM's chronicle: every session, the latest first, drafts included. */
export function fetchSessions(campaignId: string): Promise<ChronicleSession[]> {
  return apiRequest<ChronicleSession[]>('GET', `${gm(campaignId)}/sessions`)
}

/** Save the recaps of an ended session; `publish` gives them to the players. */
export function saveRecap(campaignId: string, session: string, texts: RecapTexts, publish: boolean): Promise<SessionInfo> {
  return apiRequest<SessionInfo>('PUT', `${gm(campaignId)}/sessions/${encodeURIComponent(session)}/recap`, {
    ...texts,
    publish,
  })
}

/** gm/launch-session: the next sentence of « Précédemment… » reaches the table. */
export function readNext(campaignId: string): Promise<SessionInfo> {
  return apiRequest<SessionInfo>('POST', `${gm(campaignId)}/session/previously/next`)
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
