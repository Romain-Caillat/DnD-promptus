import { apiRequest } from './api'

const play = (campaignId: string) => `/play/${encodeURIComponent(campaignId)}`
const gm = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`

// ——— The table (`campaigns::projection::schedule::ScheduleView`) ———

export interface PlayerSchedule {
  number: number
  next: { id: string; startsAt: string; minutes: number; lobbyOpensAt: string } | null
  proposed: { id: string; startsAt: string; minutes: number; mine: boolean | null; yes: string[] }[]
  canAnswer: boolean
}

export function fetchPlayerSchedule(campaignId: string): Promise<PlayerSchedule> {
  return apiRequest<PlayerSchedule>('GET', `${play(campaignId)}/schedule`)
}

export function answerDate(campaignId: string, date: string, available: boolean): Promise<PlayerSchedule> {
  return apiRequest<PlayerSchedule>('PUT', `${play(campaignId)}/schedule/${encodeURIComponent(date)}`, {
    available,
  })
}

/** The fixed date as a calendar file, with the day-before and hour-before alarms. */
export function calendarUrl(campaignId: string): string {
  return `/api${play(campaignId)}/schedule.ics`
}

// ——— The GM ———

type DateStatus = 'proposed' | 'chosen'

export interface GmDate {
  id: string
  startsAt: string
  minutes: number
  status: DateStatus
  lobbyOpensAt: string
  answers: { playerId: string; nickname: string; available: boolean }[]
  log: { kind: 'chosen' | 'eve' | 'hour' | 'lobby'; ok: boolean; detail: string; at: string }[]
}

export interface GmSchedule {
  number: number
  dates: GmDate[]
  players: { id: string; nickname: string }[]
  discord: { configured: boolean; hint: string | null }
  lobbyBeforeMinutes: number
}

export function fetchSchedule(campaignId: string): Promise<GmSchedule> {
  return apiRequest<GmSchedule>('GET', `${gm(campaignId)}/schedule`)
}

export function proposeDate(campaignId: string, startsAt: string, minutes: number): Promise<unknown> {
  return apiRequest('POST', `${gm(campaignId)}/schedule/dates`, { startsAt, minutes })
}

export function chooseDate(campaignId: string, date: string): Promise<unknown> {
  return apiRequest('POST', `${gm(campaignId)}/schedule/dates/${encodeURIComponent(date)}/choose`)
}

export function dropDate(campaignId: string, date: string): Promise<void> {
  return apiRequest<void>('DELETE', `${gm(campaignId)}/schedule/dates/${encodeURIComponent(date)}`)
}

export function setDiscord(campaignId: string, webhook: string | null): Promise<void> {
  return apiRequest<void>('PUT', `${gm(campaignId)}/schedule/discord`, { webhook })
}
