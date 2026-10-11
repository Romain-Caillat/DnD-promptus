import { gmMarks, type PinMark, type ThreadMark } from '@/features/map/pins'
import type { Cell } from '@/lib/board'
import type { Pin, Proposed } from '@/lib/maps'
import { castMarks, type StoryLayer } from './cast'

/**
 * platform/propose-edits-by-claude — what Claude's waiting batches would
 * add on one map, as marks the two maps draw dotted (`proposed`): pins,
 * NPCs, clues and fights, thread arrows, and the cells its map
 * operations touch. Pure: the page draws them, and a tap on one opens
 * its batch in the side panel.
 */

const PREFIX = 'claude:'

/** The batch a mark's key belongs to, or none for a mark of the campaign as it is. */
export function batchOf(key: string): string | null {
  if (!key.startsWith(PREFIX)) return null
  const rest = key.slice(PREFIX.length)
  const end = rest.indexOf('|')
  return end > 0 ? rest.slice(0, end) : null
}

const tagged = (proposal: string, key: string) => `${PREFIX}${proposal}|${key}`

export interface ProposedMarks {
  pins: PinMark[]
  thread: ThreadMark[]
  cells: Cell[]
}

/** Every waiting batch's marks on a map, for the story layers lit. */
export function proposedMarks(
  proposed: readonly Proposed[],
  lit: ReadonlySet<StoryLayer>,
  titles: {
    pin: (pin: Pin) => string
    npc: (name: string, scene?: string) => string
    clue: (text: string) => string
    fight: (number: number, title: string) => string
    segment: (from: string, to: string) => string
  },
  picked: string | null,
): ProposedMarks {
  const out: ProposedMarks = { pins: [], thread: [], cells: [] }
  for (const p of proposed) {
    const mine = (m: PinMark): PinMark => ({
      ...m,
      key: tagged(p.proposal, m.key),
      proposed: true,
      picked: picked === p.proposal,
      faint: false,
    })
    if (lit.has('story')) {
      out.pins.push(...gmMarks(p.pins, titles.pin).map(mine))
      for (const sg of p.thread) {
        out.thread.push({
          key: tagged(p.proposal, `${sg.from.place}>${sg.to.place}`),
          from: sg.from.at,
          to: sg.to.at,
          title: titles.segment(sg.from.place, sg.to.place),
          proposed: true,
          lit: picked === p.proposal,
        })
      }
    }
    out.pins.push(...castMarks(p.cast, lit, titles, null).map(mine))
    out.cells.push(...p.cells)
  }
  return out
}
