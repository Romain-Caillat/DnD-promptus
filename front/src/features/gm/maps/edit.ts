import type { Cell, CellKind, MapData } from '@/lib/board'

/** Glyphs a new kind of cell may take, in order. */
const GLYPHS = Array.from('.#,~_=:;+*%&@!?abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789')

const same = (a: CellKind, b: CellKind) =>
  a.terrain === b.terrain &&
  Boolean(a.wall) === Boolean(b.wall) &&
  Boolean(a.void) === Boolean(b.void) &&
  (a.water ?? 'none') === (b.water ?? 'none') &&
  Boolean(a.difficult) === Boolean(b.difficult) &&
  (a.elevation ?? 0) === (b.elevation ?? 0)

const inside = (map: MapData, [x, y]: Cell) =>
  y >= 0 && y < map.grid.rows.length && x >= 0 && x < Array.from(map.grid.rows[0] ?? '').length

const at = (c: Cell, d: Cell) => c[0] === d[0] && c[1] === d[1]

/** The kind of cell `c`, if it is on the map. */
function kindAt(map: MapData, [x, y]: Cell): CellKind | null {
  const glyph = Array.from(map.grid.rows[y] ?? '')[x]
  return glyph === undefined ? null : (map.grid.legend[glyph] ?? null)
}

/**
 * `cells` painted with `kind`: the legend keeps one glyph per kind of
 * cell, a new kind takes the first free glyph, and a glyph no cell uses
 * any more leaves the legend.
 */
export function paint(map: MapData, cells: Cell[], kind: CellKind): MapData {
  const legend = { ...map.grid.legend }
  let glyph = Object.keys(legend).find((g) => same(legend[g], kind))
  if (!glyph) {
    glyph = GLYPHS.find((g) => !(g in legend))
    if (!glyph) return map
    legend[glyph] = kind
  }
  const rows = map.grid.rows.map((r) => Array.from(r))
  for (const c of cells) if (inside(map, c)) rows[c[1]][c[0]] = glyph
  const used = new Set(rows.flat())
  for (const g of Object.keys(legend)) if (!used.has(g)) delete legend[g]
  // A door needs its wall: one painted over leaves with it.
  const doors = (map.doors ?? []).filter((d) => {
    const k = legend[rows[d.at[1]]?.[d.at[0]] ?? '']
    return Boolean(k?.wall)
  })
  return { ...map, grid: { legend, rows: rows.map((r) => r.join('')) }, doors }
}

const THINGS = ['doors', 'props', 'objects', 'lights', 'exits', 'starts'] as const

/** An id no door, prop, object, light, exit or start of `map` has yet. */
function freshId(map: MapData, prefix: string): string {
  const taken = new Set(THINGS.flatMap((k) => (map[k] ?? []).map((t) => t.id)))
  let n = 1
  while (taken.has(`${prefix}-${n}`)) n++
  return `${prefix}-${n}`
}

const DOOR_CYCLE = { closed: 'open', open: 'locked', locked: null } as const

/**
 * A door on `c`: a new one is closed (its cell becomes a wall of the
 * same ground); tapping one goes closed → open → locked → gone.
 */
export function toggleDoor(map: MapData, c: Cell, layer: string): MapData {
  if (!inside(map, c)) return map
  const doors = map.doors ?? []
  const there = doors.find((d) => at(d.at, c))
  if (there) {
    const next = DOOR_CYCLE[there.state]
    return {
      ...map,
      doors: next ? doors.map((d) => (d === there ? { ...d, state: next } : d)) : doors.filter((d) => d !== there),
    }
  }
  const ground = kindAt(map, c)
  const walled = ground && !ground.wall ? paint(map, [c], { ...ground, wall: true, void: false }) : map
  const door = {
    id: freshId(walled, 'porte'),
    at: c,
    state: 'closed' as const,
    layer,
  }
  return { ...walled, doors: [...(walled.doors ?? []), door] }
}

export type Placed =
  | { kind: 'prop'; prop: string; label: string }
  | { kind: 'object'; label: string }
  | { kind: 'light' }
  | { kind: 'start'; side: 'party' | 'foes' }

/** Put a thing on `c`, on `layer`. */
export function place(map: MapData, c: Cell, what: Placed, layer: string): MapData {
  if (!inside(map, c)) return map
  switch (what.kind) {
    case 'prop':
      return {
        ...map,
        props: [
          ...(map.props ?? []),
          {
            id: freshId(map, 'decor'),
            kind: what.prop,
            label: what.label,
            at: c,
            cover: 'half',
            blocks_movement: true,
            layer,
          },
        ],
      }
    case 'object':
      return {
        ...map,
        objects: [
          ...(map.objects ?? []),
          {
            id: freshId(map, 'objet'),
            kind: 'cache',
            label: what.label || null,
            at: c,
            layer,
          },
        ],
      }
    case 'light':
      return {
        ...map,
        lights: [
          ...(map.lights ?? []),
          {
            id: freshId(map, 'lumiere'),
            at: c,
            bright: 1,
            dim: 4,
            color: '#ffb35c',
            layer,
          },
        ],
      }
    case 'start':
      return {
        ...map,
        starts: [...(map.starts ?? []), { id: freshId(map, 'depart'), side: what.side, at: c, layer }],
      }
  }
}

const covers = (p: NonNullable<MapData['props']>[number], c: Cell) => {
  const [w, h] = p.size ?? [1, 1]
  return c[0] >= p.at[0] && c[0] < p.at[0] + w && c[1] >= p.at[1] && c[1] < p.at[1] + h
}

/** Everything placed on `c` removed (the ground stays). */
export function erase(map: MapData, c: Cell): MapData {
  return {
    ...map,
    doors: (map.doors ?? []).filter((d) => !at(d.at, c)),
    props: (map.props ?? []).filter((p) => !covers(p, c)),
    objects: (map.objects ?? []).filter((o) => !at(o.at, c)),
    lights: (map.lights ?? []).filter((l) => !at(l.at, c)),
    starts: (map.starts ?? []).filter((s) => !at(s.at, c)),
  }
}
