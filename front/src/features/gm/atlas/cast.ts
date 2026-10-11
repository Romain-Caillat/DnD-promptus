import type { PinMark } from '@/features/map/pins'
import type { Cell } from '@/lib/board'
import type { Atlas, Cast, Spot } from '@/lib/maps'
import type { Edit } from '@/lib/prep'

/**
 * campaign/place-cast-and-clues — what the atlas places on a map besides
 * the scenes' pins: NPCs, clues and fights, each kept in the story next
 * to the entity (an NPC's `spots`, a clue's `anchor`, a fight's
 * `encounter.anchor`), so placing one is a story edit, a line of the
 * journal. Pure: the page draws the marks and sends the edits.
 */

/** The four story layers the GM lights or dims on the atlas. */
export const STORY_LAYERS = ['story', 'cast', 'fights', 'clues'] as const
export type StoryLayer = (typeof STORY_LAYERS)[number]

/** What the placement tool sets down: a place (its pin), an NPC, a clue or a scene's fight. */
export type Target =
  | { kind: 'place'; id: string }
  | { kind: 'npc'; id: string; scene?: string; index?: number }
  | { kind: 'clue'; id: string }
  | { kind: 'fight'; id: string }

/** A mark of the cast, as a tap names it. */
export type Picked =
  | { kind: 'npc'; id: string; index: number }
  | { kind: 'clue'; id: string }
  | { kind: 'fight'; id: string }

/** The key of a mark: ids hold no colon, so a place's pin never reads as one. */
function keyOf(p: Picked): string {
  return p.kind === 'npc' ? `npc:${p.id}:${p.index}` : `${p.kind}:${p.id}`
}

/** The mark a key names, or none for a place's pin. */
export function readKey(key: string): Picked | null {
  const [kind, id, index] = key.split(':')
  if (!id) return null
  if (kind === 'npc') return { kind, id, index: Number(index) }
  if (kind === 'clue' || kind === 'fight') return { kind, id }
  return null
}

/** The cast's marks on one map, for the layers lit. */
export function castMarks(
  cast: Cast,
  lit: ReadonlySet<StoryLayer>,
  titles: {
    npc: (name: string, scene?: string) => string
    clue: (text: string) => string
    fight: (number: number, title: string) => string
  },
  picked: string | null,
): PinMark[] {
  const marks: PinMark[] = []
  if (lit.has('cast')) {
    for (const n of cast.npcs) {
      const key = keyOf({ kind: 'npc', id: n.npc, index: n.index })
      marks.push({
        key,
        at: n.at,
        label: n.name.slice(0, 1).toUpperCase(),
        title: titles.npc(n.name, n.scene),
        shape: 'npc',
        faint: Boolean(n.scene),
        picked: picked === key,
      })
    }
  }
  if (lit.has('clues')) {
    for (const c of cast.clues) {
      const key = keyOf({ kind: 'clue', id: c.clue })
      marks.push({ key, at: c.at, label: '', title: titles.clue(c.text), shape: 'clue', picked: picked === key })
    }
  }
  if (lit.has('fights')) {
    for (const f of cast.fights) {
      const key = keyOf({ kind: 'fight', id: f.scene })
      marks.push({
        key,
        at: f.at,
        label: '',
        title: titles.fight(f.number, f.title),
        shape: 'fight',
        picked: picked === key,
      })
    }
  }
  return marks
}

/** The story edit that sets `target` down on `map` at `at` (a move, for one already placed). */
export function placeEdit(castable: Atlas['castable'], target: Target, map: string, at: Cell): Edit {
  const anchor = { map, at }
  switch (target.kind) {
    case 'place':
      return { op: 'set', target: target.id, field: 'anchor', value: anchor }
    case 'clue':
      return { op: 'set', target: target.id, field: 'anchor', value: anchor }
    case 'fight':
      return { op: 'set', target: target.id, field: 'encounter.anchor', value: anchor }
    case 'npc': {
      const spots: Spot[] = castable.npcs.find((n) => n.id === target.id)?.spots ?? []
      const spot: Spot = target.scene ? { map, at, scene: target.scene } : { map, at }
      const next =
        target.index !== undefined && target.index < spots.length
          ? spots.map((s, i) => (i === target.index ? { ...spot, scene: s.scene } : s))
          : [...spots, spot]
      return { op: 'set', target: target.id, field: 'spots', value: next.map(clean) }
    }
  }
}

/** The story edit that takes a mark off the map. */
export function removeEdit(castable: Atlas['castable'], p: Picked): Edit {
  switch (p.kind) {
    case 'clue':
      return { op: 'set', target: p.id, field: 'anchor', value: null }
    case 'fight':
      return { op: 'set', target: p.id, field: 'encounter.anchor', value: null }
    case 'npc': {
      const spots = castable.npcs.find((n) => n.id === p.id)?.spots ?? []
      return { op: 'set', target: p.id, field: 'spots', value: spots.filter((_, i) => i !== p.index).map(clean) }
    }
  }
}

/** A spot as the format writes it: no `scene` key when it is for every scene. */
function clean(s: Spot): Spot {
  return s.scene ? { map: s.map, at: s.at, scene: s.scene } : { map: s.map, at: s.at }
}
