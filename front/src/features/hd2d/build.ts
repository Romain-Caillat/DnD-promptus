import type { PinMark } from '@/features/map/pins'
import type { Cell, CellKind, MapData } from '@/lib/board'
import type { Needs } from '@/lib/elements'
import type { Asset, Material, Tileset } from '@/lib/media'

/**
 * campaign/watch-prep-in-hd2d — what the HD-2D scene draws, worked out
 * from the map, its tileset, the PixelLab elements and the marks of the
 * Atlas, before any WebGL. Pure: `Hd2dView` turns it into a Three.js
 * scene; the tests read it as data.
 *
 * The shape rule of `MEMORY.md` §2: what has volume (walls, crates,
 * barrels) is geometry with textures seen from the front; what is thin
 * (a rope, a lantern) and every character is a flat image turned to the
 * camera. An element still missing is drawn from the theme's colours
 * (a procedural texture) and listed as missing, never hidden; one still
 * waiting for the GM is drawn and marked pending.
 */

/** An image the scene paints: a PixelLab element, or the theme's colours. */
export type Paint =
  | { kind: 'image'; url: string; pending: boolean; subject: string }
  | { kind: 'theme'; material: Material | null; pattern: string; base: string; dark: string; light: string }

interface Floor {
  /** The terrain's material id. */
  material: string
  cells: Cell[]
  paint: Paint
  /** Height of the floor, in cells (relief). */
  heights: number[]
  water: 'none' | 'shallow' | 'deep'
}

interface Volume {
  id: string
  name: string
  at: Cell
  size: [number, number]
  side: Paint
  top: Paint
}

interface Flat {
  id: string
  name: string
  at: Cell
  paint: Paint | null
  color: string
}

/** A mark of the Atlas stood up in the scene: a figure, a clue, a fight, a pin. */
export interface Figure {
  key: string
  at: Cell
  title: string
  label: string
  shape: 'npc' | 'clue' | 'fight' | 'pin'
  /** Claude's batch it belongs to: drawn translucent, marked ▶. */
  proposal: string | null
  faint: boolean
  picked: boolean
  /** An NPC's PixelLab figure. */
  image: { url: string; pending: boolean } | null
}

interface Missing {
  subject: string
  name: string
  kind: 'element' | 'character'
  generations: number
}

export interface Hd2dScene {
  width: number
  height: number
  void: string
  floors: Floor[]
  walls: Cell[]
  wallPaint: Paint
  volumes: Volume[]
  flats: Flat[]
  lights: { at: Cell; color: string; bright: number; flicker: boolean }[]
  figures: Figure[]
  /** Cells Claude's map operations would change. */
  proposedCells: Cell[]
  time: 'dawn' | 'day' | 'dusk' | 'night'
  fog: boolean
  /** What the scene still lacks from PixelLab, or an NPC without a figure. */
  missing: Missing[]
  /** PixelLab images drawn, waiting for the GM's word. */
  pending: { asset: string; subject: string; name: string }[]
}

export interface BuildInput {
  map: MapData
  tileset: Tileset | null
  needs: Needs | null
  /** The campaign's images: an NPC's figure is a `character` asset. */
  assets: readonly Asset[]
  marks: readonly PinMark[]
  /** Cells Claude's waiting batches touch. */
  proposedCells: readonly Cell[]
  imageUrl: (asset: string) => string
  /** The NPC a mark's key names (`cast.readKey`), if it is one. */
  npcOf: (key: string) => string | null
  /** Claude's batch a mark's key belongs to (`claude.batchOf`). */
  batchOf: (key: string) => string | null
}

const GREY: Material = { name: '', base: '#5a5a5a', dark: '#3a3a3a', light: '#7a7a7a', pattern: 'plain' }

function kindAt(map: MapData, [x, y]: Cell): CellKind | null {
  const glyph = map.grid.rows[y]?.[x]
  return glyph === undefined ? null : (map.grid.legend[glyph] ?? null)
}

function themePaint(material: Material | null | undefined): Paint {
  const m = material ?? GREY
  return { kind: 'theme', material: material ?? null, pattern: m.pattern, base: m.base, dark: m.dark, light: m.light }
}

const live = (status?: Asset['status']) => (status ?? 'approved') === 'approved' || status === 'pending'

export function buildScene(input: BuildInput): Hd2dScene {
  const { map, tileset, needs, imageUrl } = input
  const ts = tileset?.id ?? map.theme
  const rows = map.grid.rows
  const height = rows.length
  const width = Math.max(0, ...rows.map((r) => [...r].length))
  const need = (subject: string) => needs?.needs.find((n) => n.subject === subject) ?? null
  const element = (subject: string): Paint | null => {
    const n = need(subject)
    if (!n || n.status === 'missing' || !n.asset) return null
    return { kind: 'image', url: imageUrl(n.asset), pending: n.status === 'pending', subject }
  }

  const floors = new Map<string, Floor>()
  const walls: Cell[] = []
  for (let y = 0; y < height; y++) {
    const row = [...(rows[y] ?? '')]
    for (let x = 0; x < row.length; x++) {
      const k = kindAt(map, [x, y])
      if (!k || k.void) continue
      if (k.wall) {
        walls.push([x, y])
        continue
      }
      const water = k.water ?? 'none'
      const key = `${k.terrain}|${water}`
      let floor = floors.get(key)
      if (!floor) {
        floor = {
          material: k.terrain,
          cells: [],
          heights: [],
          water,
          paint: element(`${ts}/material/${k.terrain}`) ?? themePaint(tileset?.materials[k.terrain]),
        }
        floors.set(key, floor)
      }
      floor.cells.push([x, y])
      floor.heights.push(k.elevation ?? 0)
    }
  }
  const w = tileset?.walls
  const wallPaint =
    element(`${ts}/wall`) ??
    themePaint(w ? { name: '', base: w.face, dark: w.joint, light: w.cap, pattern: w.pattern as Material['pattern'] } : null)

  const volumes: Volume[] = []
  const flats: Flat[] = []
  const things = [
    ...(map.props ?? []).map((p) => ({ id: p.id, kind: p.kind, label: p.label, at: p.at, size: p.size ?? ([1, 1] as [number, number]) })),
    ...(map.objects ?? []).map((o) => ({ id: o.id, kind: o.kind, label: o.label, at: o.at, size: [1, 1] as [number, number] })),
  ]
  for (const t of things) {
    const look = tileset?.props[t.kind]
    const name = t.label || look?.name || t.kind
    const flat = need(`${ts}/prop/${t.kind}`) !== null && need(`${ts}/prop/${t.kind}/side`) === null
    const colours: Material = look
      ? { name: look.name, base: look.color, dark: look.accent, light: look.color, pattern: 'planks' }
      : GREY
    if (flat) {
      flats.push({ id: t.id, name, at: t.at, paint: element(`${ts}/prop/${t.kind}`), color: look?.color ?? GREY.base })
    } else {
      volumes.push({
        id: t.id,
        name,
        at: t.at,
        size: t.size,
        side: element(`${ts}/prop/${t.kind}/side`) ?? themePaint(colours),
        top: element(`${ts}/prop/${t.kind}/top`) ?? themePaint(colours),
      })
    }
  }

  const figureOf = (npc: string) => {
    const a = input.assets.find((x) => x.kind === 'character' && x.subject === npc && live(x.status))
    return a ? { url: imageUrl(a.id), pending: a.status === 'pending' } : null
  }
  const missing: Missing[] = (needs?.needs ?? [])
    .filter((n) => n.status === 'missing' && n.kind === 'element')
    .map((n) => ({ subject: n.subject, name: n.name, kind: n.kind, generations: n.generations }))
  const figures: Figure[] = input.marks.map((m) => {
    const proposal = input.batchOf(m.key)
    const inner = proposal ? m.key.slice(m.key.indexOf('|') + 1) : m.key
    const npc = m.shape === 'npc' ? input.npcOf(inner) : null
    const image = npc ? figureOf(npc) : null
    if (npc && !image && !missing.some((x) => x.subject === npc)) {
      const named = needs?.needs.find((n) => n.kind === 'character' && n.subject === npc)
      missing.push({ subject: npc, name: named?.name ?? m.title, kind: 'character', generations: named?.generations ?? 6 })
    }
    return {
      key: m.key,
      at: m.at,
      title: m.title,
      label: m.label,
      shape: m.shape ?? 'pin',
      proposal,
      faint: Boolean(m.faint),
      picked: Boolean(m.picked),
      image,
    }
  })

  const pending = [
    ...(needs?.needs ?? []).filter((n) => n.status === 'pending' && n.asset && n.kind === 'element'),
  ].map((n) => ({ asset: n.asset!, subject: n.subject, name: n.name }))
  for (const a of input.assets) {
    if (a.kind === 'character' && a.status === 'pending' && figures.some((f) => f.image?.url === imageUrl(a.id))) {
      pending.push({ asset: a.id, subject: a.subject, name: a.meta?.name ?? a.subject })
    }
  }

  return {
    width,
    height,
    void: tileset?.void ?? '#07090c',
    floors: [...floors.values()],
    walls,
    wallPaint,
    volumes,
    flats,
    lights: (map.lights ?? []).map((l) => ({ at: l.at, color: l.color ?? '#ffb060', bright: l.bright, flicker: Boolean(l.flicker) })),
    figures,
    proposedCells: [...input.proposedCells],
    time: map.ambience.time ?? 'day',
    fog: map.ambience.weather === 'fog' || map.ambience.weather === 'rain',
    missing,
    pending,
  }
}
