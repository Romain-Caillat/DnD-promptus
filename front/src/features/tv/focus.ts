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
