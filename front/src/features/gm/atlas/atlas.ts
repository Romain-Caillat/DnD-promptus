import type { Cell, MapData } from '@/lib/board'
import type { Atlas, AtlasMap } from '@/lib/maps'

/** The maps of `atlas` by id. */
export function index(atlas: Atlas): Map<string, AtlasMap> {
  return new Map(atlas.maps.map((m) => [m.map.id, m]))
}

/** From the root down to `id`: what the breadcrumb shows. */
export function trail(maps: Map<string, AtlasMap>, id: string): AtlasMap[] {
  const out: AtlasMap[] = []
  const seen = new Set<string>()
  let at = maps.get(id)
  while (at && !seen.has(at.map.id)) {
    seen.add(at.map.id)
    out.unshift(at)
    at = at.parent ? maps.get(at.parent.map) : undefined
  }
  return out
}

/** How deep `id` sits in the tree (a root is 0). */
export function depth(maps: Map<string, AtlasMap>, id: string): number {
  return Math.max(0, trail(maps, id).length - 1)
}

/** The exits of `entry` that lead to a map of the atlas: its children, then its links. */
function doors(entry: AtlasMap): NonNullable<MapData['exits']> {
  const known = new Set([...entry.children, ...entry.links.map((l) => l.to)])
  return (entry.map.exits ?? []).filter((e) => known.has(e.to))
}

/** The map an exit of `entry` opens on `cell`, if any. */
export function doorAt(entry: AtlasMap, [x, y]: Cell): string | null {
  return doors(entry).find((e) => e.cells.some(([cx, cy]) => cx === x && cy === y))?.to ?? null
}

/** Every cell of `entry` that opens another map of the atlas. */
export function doorCells(entry: AtlasMap): Cell[] {
  return doors(entry).flatMap((e) => e.cells)
}

/** The layers a GM can switch: every one but the base layer (always shown). */
export function switchable(map: MapData): { id: string; name: string }[] {
  return (map.layers ?? []).filter((l) => l.id !== 'base')
}

/**
 * `map` without what sits on the `hidden` layers — doors, decor,
 * objects, lights, exits, labels, starts. The ground stays: a layer
 * holds things, not cells. A display choice of the GM, never a
 * projection for players.
 */
export function withoutLayers(map: MapData, hidden: ReadonlySet<string>): MapData {
  if (hidden.size === 0) return map
  const shown = <T extends { layer?: string }>(items: T[] | undefined) =>
    items?.filter((i) => !hidden.has(i.layer ?? 'base'))
  return {
    ...map,
    doors: shown(map.doors),
    props: shown(map.props),
    objects: shown(map.objects),
    lights: shown(map.lights),
    exits: shown(map.exits),
    labels: shown(map.labels),
    starts: shown(map.starts),
  }
}

/**
 * Where the GM has been: the last is the map shown. Zooming out goes
 * back where they came from; with nowhere to go back to, up to the
 * map's parent.
 */
export function enter(walk: readonly string[], id: string): string[] {
  return walk[walk.length - 1] === id ? [...walk] : [...walk, id]
}

/** Back up to `id`, an ancestor of the map shown: the walk cut there, or its line from the root. */
export function climb(walk: readonly string[], maps: Map<string, AtlasMap>, id: string): string[] {
  const at = walk.lastIndexOf(id)
  return at >= 0 ? walk.slice(0, at + 1) : trail(maps, id).map((m) => m.map.id)
}

export function leave(walk: readonly string[], maps: Map<string, AtlasMap>): string[] {
  if (walk.length > 1) return walk.slice(0, -1)
  const parent = maps.get(walk[0] ?? '')?.parent?.map
  return parent ? [parent] : [...walk]
}

/** Whether zooming out goes anywhere. */
export function canLeave(walk: readonly string[], maps: Map<string, AtlasMap>): boolean {
  return walk.length > 1 || Boolean(maps.get(walk[0] ?? '')?.parent)
}

/** World, place, encounter: the lower, the wider. */
const WIDTH = { world: 0, place: 1, encounter: 2 } as const
const width = (m: AtlasMap) => WIDTH[m.map.scale ?? 'encounter']

/**
 * The map a campaign opens on (campaign/write-story-by-map-scale): the
 * first root at the widest scale the atlas holds — the region for the
 * Corsaires. `null` when there is no map at all.
 */
export function widestRoot(atlas: Atlas, maps: Map<string, AtlasMap>): string | null {
  const roots = atlas.roots.flatMap((id) => maps.get(id) ?? [])
  if (roots.length === 0) return null
  return roots.reduce((best, m) => (width(m) < width(best) ? m : best)).map.id
}

/** Whether `entry` stands at the widest scale of the atlas: there, the story is the campaign's idea. */
export function atWidest(atlas: Atlas, entry: AtlasMap): boolean {
  return atlas.maps.every((m) => width(entry) <= width(m))
}
