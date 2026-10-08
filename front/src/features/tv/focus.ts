import type { FightEvent } from '@/lib/board'
import type { Moment, ScreenRoll, ScreenView } from '@/lib/screens'

/**
 * The one focal point the TV shows (planche « Écran TV »: one subject at
 * a time, readable from the couch), chosen from what the screen may see
 * now. The GM never drives the TV: the evening does.
 */
export type Focus =
  /** No session open: the campaign, and what the last evening left. */
  | 'between'
  /** The lobby: who is here. */
  | 'lobby'
  /** Live, before the first scene: « Précédemment… ». */
  | 'previously'
  | 'scene'
  | 'map'
  | 'fight'
  /** Live, with nothing to show yet (or the GM hid it all). */
  | 'waiting'

export function pickFocus(view: ScreenView): Focus {
  const session = view.session
  if (!session || session.status === 'ended') return 'between'
  if (session.status === 'lobby') return 'lobby'
  if (view.board?.fight?.live) return 'fight'
  if (view.board) return 'map'
  if (view.scene) return 'scene'
  if (view.previously) return 'previously'
  return 'waiting'
}

/** A big moment the TV plays over its focal point, then lets go. */
export type Highlight =
  | { id: string; at: number; kind: 'roll'; roll: ScreenRoll }
  | { id: string; at: number; kind: 'clue' | 'loot' | 'item' | 'npc'; text: string }
  /** A blow landed in a fight: who took it, and how hard. */
  | { id: string; at: number; kind: 'hit'; target: string; amount: number; down: boolean }

/** Journal kinds that make a big moment (a roll comes with its dice instead). */
const BIG: ReadonlySet<Moment['kind']> = new Set(['clue', 'loot', 'item', 'npc'])

/**
 * How old a moment may be and still be played: a TV that reconnects, or
 * opens mid-evening, does not replay the whole evening.
 */
const FRESH_MS = 30_000

/**
 * The big moments not played yet, oldest first: dice rolled, clues
 * found, loot, a goal reached, someone met. `seen` holds the ids already
 * played (or skipped); a moment older than `FRESH_MS` is never played.
 */
export function freshHighlights(view: ScreenView, seen: ReadonlySet<string>, now: number = Date.now()): Highlight[] {
  const fresh = (at: string) => now - Date.parse(at) <= FRESH_MS
  const out: Highlight[] = []
  for (const r of view.rolls) {
    if (!seen.has(r.id) && fresh(r.at)) out.push({ id: r.id, kind: 'roll', roll: r, at: Date.parse(r.at) })
  }
  for (const m of view.moments) {
    if (BIG.has(m.kind) && !seen.has(m.id) && fresh(m.at)) {
      out.push({ id: m.id, kind: m.kind as 'clue' | 'loot' | 'item' | 'npc', text: m.text, at: Date.parse(m.at) })
    }
  }
  out.sort((a, b) => a.at - b.at)
  return out
}

/** The one line at the bottom: the latest thing the table learned. */
export function feedLine(view: ScreenView): string | null {
  return view.moments.at(-1)?.text ?? null
}

/**
 * Where the TV is in the fight's log. Fight events carry no time, so the
 * TV remembers how many it has read: the first view it gets only sets
 * the mark (a TV opening mid-fight replays nothing), and a log shorter
 * than the mark is a new fight, read from its start.
 */
export type FightMark = { read: number } | null

/**
 * The blows landed since `mark`, as big moments stamped `now` (each is
 * shown once, in the order they fell), and the new mark. An opponent's
 * hit points are hidden from the table, so its blow reads the damage
 * rolled; a party member's reads what it actually lost.
 */
export function newHits(view: ScreenView, mark: FightMark, now: number = Date.now()): { hits: Highlight[]; mark: FightMark } {
  const fight = view.board?.fight ?? null
  const events: FightEvent[] = fight?.events ?? []
  if (mark === null) return { hits: [], mark: { read: events.length } }
  const from = events.length < mark.read ? 0 : mark.read
  const name = (id: string) => fight?.order.find((f) => f.id === id)?.name ?? view.board?.tokens.find((k) => k.id === id)?.name ?? id
  const hits: Highlight[] = []
  events.slice(from).forEach((e, i) => {
    if (e.kind !== 'rules' || e.event.event !== 'damaged') return
    const r = e.event
    const lost = r.hp_before - r.hp_after
    const amount = lost > 0 ? lost : Math.max(0, r.breakdown.total)
    if (amount <= 0) return
    const down = events.slice(from + i + 1).some((n) => n.kind === 'rules' && n.event.event === 'knocked_out' && n.event.target === r.target)
    hits.push({ id: `hit:${now}:${from + i}`, at: now, kind: 'hit', target: name(r.target), amount, down })
  })
  return { hits, mark: { read: events.length } }
}
