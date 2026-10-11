import type { Cell, PinView } from '@/lib/board'
import type { Pin } from '@/lib/maps'

/**
 * A pin as the two maps draw it (campaign/pin-scenes-on-maps): the hex
 * map and the grid map take the same marks, so the atlas, the GM's board
 * and the player's map cannot drift apart. What it says is decided by
 * the caller: scene numbers for the GM, a place's name for players.
 */
export interface PinMark {
  /** The place's id (GM) or its name (players): what a tap answers. */
  key: string
  at: Cell
  /** Written in the pin: the scene numbers, or nothing. */
  label: string
  /** Its accessible name. */
  title: string
  /** Written under the pin (a player's place name). */
  caption?: string
  /** The table is in one of its scenes. */
  current?: boolean
  /** Chosen in the side panel. */
  picked?: boolean
}

/** The numbers of a pin's scenes: « 1·2·6 », « 1…9 » when there are many. */
function numbers(pin: Pin): string {
  const n = pin.scenes.map((s) => s.number)
  if (n.length <= 3) return n.join('·')
  return `${n[0]}…${n[n.length - 1]}`
}

/** The GM's pins: numbered, named with their scenes. */
export function gmMarks(
  pins: readonly Pin[],
  title: (pin: Pin) => string,
  opts: { picked?: string | null; current?: string | null } = {},
): PinMark[] {
  return pins.map((p) => ({
    key: p.location,
    at: p.at,
    label: numbers(p),
    title: title(p),
    picked: opts.picked === p.location,
    current: Boolean(opts.current && p.scenes.some((s) => s.id === opts.current)),
  }))
}

/** A player's pins: where the table went, by name only. */
export function playerMarks(pins: readonly PinView[] | undefined): PinMark[] {
  return (pins ?? []).map((p) => ({ key: p.name, at: p.at, label: '', title: p.name, caption: p.name, current: p.current }))
}
