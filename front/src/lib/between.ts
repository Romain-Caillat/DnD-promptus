import { apiRequest } from './api'

const play = (campaignId: string) => `/play/${encodeURIComponent(campaignId)}`
const gm = (campaignId: string) => `/campaigns/${encodeURIComponent(campaignId)}`

/** Between two sessions, on a player's phone (`projection::between::BetweenView`). */
export interface BetweenView {
  lastSession: {
    number: number
    endedAt: string | null
    durationMinutes: number
    title: string | null
    published: boolean
    xpGained: number
    gains: { kind: 'item' | 'resource'; label: string; delta: number }[]
  } | null
  previously: {
    number: number
    text: string
    clues: string[]
    revelations: string[]
    openThreads: string[]
  } | null
  chronicle: { number: number; title: string; text: string; date: string | null }[]
  next: {
    number: number
    options: string[]
    chosenAt: string | null
    lobbyAt: string | null
    mine: string[] | null
    answered: number
  } | null
}

export function fetchBetween(campaignId: string): Promise<BetweenView> {
  return apiRequest<BetweenView>('GET', `${play(campaignId)}/between`)
}

/** Which proposed times suit me (all at once). */
export function answerDates(campaignId: string, available: string[]): Promise<BetweenView> {
  return apiRequest<BetweenView>('PUT', `${play(campaignId)}/availability`, { available })
}

/** The calendar reminder of the chosen date (opened, not fetched). */
export function calendarUrl(campaignId: string): string {
  return `/api${play(campaignId)}/next-session.ics`
}

/** The next session as the GM plans it (`evening::schedule::Plan`). */
export interface Plan {
  options: string[]
  chosenAt: string | null
  lobbyMinutes: number
  answers: { playerId: string; available: string[] }[]
}

export function fetchPlan(campaignId: string): Promise<Plan | null> {
  return apiRequest<Plan | null>('GET', `${gm(campaignId)}/plan`)
}

export function proposeDates(campaignId: string, options: string[], lobbyMinutes: number): Promise<Plan> {
  return apiRequest<Plan>('PUT', `${gm(campaignId)}/plan`, { options, lobbyMinutes })
}

export function chooseDate(campaignId: string, at: string | null): Promise<Plan> {
  return apiRequest<Plan>('PUT', `${gm(campaignId)}/plan/choice`, { at })
}

/** « jeu. 10 oct. · 20 h 30 », in French, in the reader's time zone. */
export function formatWhen(iso: string): string {
  const d = new Date(iso)
  const day = new Intl.DateTimeFormat('fr-FR', { weekday: 'short', day: 'numeric', month: 'short' }).format(d)
  const time = new Intl.DateTimeFormat('fr-FR', { hour: 'numeric', minute: '2-digit' }).format(d).replace(':', ' h ')
  return `${day} · ${time}`
}

/** « 3 octobre », in French. */
export function formatDay(iso: string): string {
  return new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'long' }).format(new Date(iso))
}
