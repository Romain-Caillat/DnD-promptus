import type { Cell, CellKind, MapData, ReachCell, TokenView } from '@/lib/board'
import type { Material, Tileset } from '@/lib/media'
import { SPRITE_HEIGHT, SPRITE_WIDTH, sheetUrl } from '@/features/sprites/look'
import type { Pose } from './motion'

/**
 * The grid renderer (maps/render-three-quarter-tiles,
 * maps/blend-outdoor-terrain): a map drawn on a canvas from its world's
 * tileset, in three-quarter view — walls show their top one half-tile up
 * and their face below when the cell under them is open.
 *
 * Each material is drawn in sixteen autotile variants: the variant is
 * the 4-bit mask of which sides (N, E, S, W) touch the same material.
 * Hard materials get a shaded border on their open sides; outdoor ground
 * (`blend`) melts into its neighbour by a fixed noise instead. When the
 * GM approved an atlas for a material (a 4×4 grid of its 16 variants,
 * `media_assets` kind `tileset`), its tile replaces the drawn one.
 *
 * Everything here is pure drawing from server data: the server decided
 * what the caller may see (fog, hidden tokens) before it was sent.
 */

/** Pixels per cell at zoom 1. */
export const TILE = 32

export const N = 1
export const E = 2
export const S = 4
export const W = 8

/** A map's cells, read once. */
export interface Grid {
  width: number
  height: number
  kind: (x: number, y: number) => CellKind | null
}

export function readGrid(map: MapData): Grid {
  const rows = map.grid.rows.map((r) => Array.from(r))
  const width = Math.max(0, ...rows.map((r) => r.length))
  return {
    width,
    height: rows.length,
    kind: (x, y) => {
      const glyph = rows[y]?.[x]
      return glyph === undefined ? null : (map.grid.legend[glyph] ?? null)
    },
  }
}

/** Which sides of `(x, y)` touch the same material: the autotile variant, 0–15. */
export function edgeMask(grid: Grid, x: number, y: number): number {
  const here = grid.kind(x, y)
  if (!here) return 0
  const same = (dx: number, dy: number) => {
    const k = grid.kind(x + dx, y + dy)
    return k !== null && k.terrain === here.terrain && Boolean(k.wall) === Boolean(here.wall)
  }
  return (same(0, -1) ? N : 0) | (same(1, 0) ? E : 0) | (same(0, 1) ? S : 0) | (same(-1, 0) ? W : 0)
}

/** The cell under a point of the canvas, in canvas pixels. */
export function cellAt(px: number, py: number, tile: number = TILE): Cell {
  return [Math.floor(px / tile), Math.floor(py / tile)]
}

/** A fixed noise in [0, 1) per cell and salt: the same picture on every screen. */
export function noise(x: number, y: number, salt = 0): number {
  let h = (x * 374761393 + y * 668265263 + salt * 2147483647) | 0
  h = Math.imul(h ^ (h >>> 13), 1274126177)
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296
}

const FALLBACK: Tileset = {
  id: 'fallback',
  name: '',
  void: '#07090c',
  walls: { cap: '#8c8577', face: '#5b5448', joint: '#2f2b25', pattern: 'stones' },
  materials: {},
  props: {},
}

const FALLBACK_MATERIAL: Material = { name: '', base: '#55524c', dark: '#3a3834', light: '#6f6b63', pattern: 'plain' }

export interface Scene {
  map: MapData
  tileset: Tileset | null
  /** Approved atlases, by material id. */
  atlases?: Record<string, CanvasImageSource>
  tokens: TokenView[]
  reachable?: ReachCell[]
  /** The path being drawn, or the cells being painted. */
  highlight?: Cell[]
  /** GM: the cells still under the fog (drawn veiled, not hidden). */
  veiled?: Set<string>
  selected?: string | null
  /** Milliseconds, for water and weather; 0 draws them still. */
  time: number
  tile?: number
  /** The imported image behind the grid, once loaded: it replaces the drawn floor and walls. */
  backdrop?: HTMLImageElement | null
  /** Editor: walls shown over the backdrop, to trace them. */
  showWalls?: boolean
  /** Loaded character sheets, by `sheetUrl`: a token whose sheet is here is drawn as its sprite. */
  sprites?: Record<string, CanvasImageSource>
  /** Where each token is and which frame it shows now (`Motion.pose`), by token id. */
  poses?: Record<string, Pose>
}

export const cellKey = ([x, y]: Cell) => `${x},${y}`

function drawPattern(ctx: CanvasRenderingContext2D, m: Material, x: number, y: number, t: number, time: number) {
  const px = x * t
  const py = y * t
  const u = t / 8
  ctx.fillStyle = m.base
  ctx.fillRect(px, py, t, t)
  switch (m.pattern) {
    case 'cobbles':
      for (let i = 0; i < 4; i++) {
        const ox = (i % 2) * 4 * u + (Math.floor(i / 2) % 2) * 2 * u
        const oy = Math.floor(i / 2) * 4 * u
        ctx.fillStyle = noise(x, y, i) > 0.5 ? m.light : m.base
        ctx.fillRect(px + (ox % t) + u * 0.5, py + oy + u * 0.5, 3 * u, 3 * u)
        ctx.fillStyle = m.dark
        ctx.fillRect(px + (ox % t) + u * 0.5, py + oy + 3.5 * u - 1, 3 * u, 1)
      }
      break
    case 'planks':
      ctx.fillStyle = m.dark
      for (let i = 1; i < 4; i++) ctx.fillRect(px, py + i * 2 * u, t, 1)
      for (let i = 0; i < 4; i++) ctx.fillRect(px + ((noise(x, y, i) * 8) | 0) * u, py + i * 2 * u, 1, 2 * u)
      ctx.fillStyle = m.light
      ctx.fillRect(px, py, t, 1)
      break
    case 'flagstones':
    case 'plates':
      ctx.fillStyle = m.dark
      ctx.fillRect(px, py + t / 2, t, 1)
      ctx.fillRect(px + (y % 2 ? t / 2 : t / 4), py, 1, t / 2)
      ctx.fillRect(px + (y % 2 ? t / 4 : (3 * t) / 4), py + t / 2, 1, t / 2)
      if (m.pattern === 'plates') {
        ctx.fillStyle = m.light
        ctx.fillRect(px + u, py + u, 1, 1)
        ctx.fillRect(px + t - u, py + t - u, 1, 1)
      }
      break
    case 'grating':
      ctx.fillStyle = m.dark
      for (let i = 1; i < 8; i += 2) {
        ctx.fillRect(px + i * u, py, 1, t)
        ctx.fillRect(px, py + i * u, t, 1)
      }
      break
    case 'water': {
      const shift = time ? Math.floor(time / 250) % 8 : 0
      ctx.fillStyle = m.light
      for (let i = 0; i < 3; i++) {
        const wx = ((noise(x, y, i) * 8 + shift) % 8) * u
        ctx.fillRect(px + wx, py + (i * 3 + 1) * u, 2 * u, 1)
      }
      ctx.fillStyle = m.dark
      ctx.fillRect(px + ((noise(x, y, 9) * 6) | 0) * u, py + 6 * u, 2 * u, 1)
      break
    }
    case 'grass':
    case 'sand':
    case 'dirt':
    case 'rock':
    case 'asphalt':
      for (let i = 0; i < 6; i++) {
        ctx.fillStyle = i % 2 ? m.dark : m.light
        ctx.fillRect(px + ((noise(x, y, i) * 8) | 0) * u, py + ((noise(x, y, i + 7) * 8) | 0) * u, m.pattern === 'grass' ? 1 : u, m.pattern === 'grass' ? u : u)
      }
      break
    case 'plain':
      break
  }
}

/** The open sides of a hard material: a darker line, lighter on top. */
function drawEdges(ctx: CanvasRenderingContext2D, m: Material, mask: number, x: number, y: number, t: number) {
  const px = x * t
  const py = y * t
  ctx.fillStyle = m.dark
  if (!(mask & S)) ctx.fillRect(px, py + t - 2, t, 2)
  if (!(mask & E)) ctx.fillRect(px + t - 2, py, 2, t)
  if (!(mask & W)) ctx.fillRect(px, py, 1, t)
  ctx.fillStyle = m.light
  if (!(mask & N)) ctx.fillRect(px, py, t, 1)
}

/** Outdoor ground: speckles of the neighbour's colour along the open sides. */
function drawBlend(ctx: CanvasRenderingContext2D, grid: Grid, ts: Tileset, mask: number, x: number, y: number, t: number) {
  const u = t / 8
  const sides: [number, number, number][] = [
    [N, 0, -1],
    [E, 1, 0],
    [S, 0, 1],
    [W, -1, 0],
  ]
  for (const [bit, dx, dy] of sides) {
    if (mask & bit) continue
    const other = grid.kind(x + dx, y + dy)
    const om = other && !other.wall && !other.void ? ts.materials[other.terrain] : null
    if (!om) continue
    ctx.fillStyle = om.base
    for (let i = 0; i < 8; i++) {
      const depth = (noise(x, y, bit * 10 + i) * 3) | 0
      for (let d = 0; d <= depth; d++) {
        const cx = dx === 0 ? i : dx > 0 ? 7 - d : d
        const cy = dy === 0 ? i : dy > 0 ? 7 - d : d
        ctx.fillRect(x * t + cx * u, y * t + cy * u, u, u)
      }
    }
  }
}

function drawWall(ctx: CanvasRenderingContext2D, grid: Grid, ts: Tileset, x: number, y: number, t: number) {
  const px = x * t
  const py = y * t
  const below = grid.kind(x, y + 1)
  const openBelow = below !== null && !below.wall
  // The face: the lower half when the cell below is open.
  ctx.fillStyle = ts.walls.cap
  ctx.fillRect(px, py, t, openBelow ? t / 2 : t)
  if (openBelow) {
    ctx.fillStyle = ts.walls.face
    ctx.fillRect(px, py + t / 2, t, t / 2)
    ctx.fillStyle = ts.walls.joint
    ctx.fillRect(px, py + t / 2, t, 1)
    const courses = ts.walls.pattern === 'plates' ? 1 : 2
    for (let c = 0; c < courses; c++) {
      const cy = py + t / 2 + ((c + 1) * t) / 2 / (courses + 1)
      ctx.fillRect(px, cy, t, 1)
      ctx.fillRect(px + ((c + y) % 2 ? t / 3 : (2 * t) / 3), cy - t / 6, 1, t / 6)
    }
    // The shadow the wall casts on the floor below.
    ctx.fillStyle = 'rgba(0,0,0,0.35)'
    ctx.fillRect(px, py + t, t, t / 6)
  } else {
    ctx.fillStyle = ts.walls.joint
    ctx.fillRect(px, py + t - 1, t, 1)
  }
}

function drawProp(
  ctx: CanvasRenderingContext2D,
  look: { shape: string; color: string; accent: string } | undefined,
  at: Cell,
  size: { w: number; h: number },
  t: number,
) {
  const px = at[0] * t
  const py = at[1] * t
  const w = size.w * t
  const h = size.h * t
  const color = look?.color ?? '#8a6239'
  const accent = look?.accent ?? '#4f3720'
  ctx.fillStyle = 'rgba(0,0,0,0.3)'
  ctx.fillRect(px + 3, py + h - 4, w - 2, 5)
  switch (look?.shape) {
    case 'barrel':
    case 'post':
    case 'reactor':
    case 'beacon':
    case 'tree':
    case 'rock': {
      ctx.fillStyle = color
      ctx.beginPath()
      ctx.ellipse(px + w / 2, py + h / 2, w / 2.6, h / 2.6, 0, 0, Math.PI * 2)
      ctx.fill()
      ctx.strokeStyle = accent
      ctx.lineWidth = 2
      ctx.stroke()
      break
    }
    case 'mast':
      ctx.fillStyle = color
      ctx.fillRect(px + w / 2 - t / 6, py - t, t / 3, h + t)
      ctx.fillStyle = accent
      ctx.fillRect(px, py - t / 2, w, t / 6)
      break
    case 'rail':
    case 'pipes':
      ctx.fillStyle = color
      ctx.fillRect(px, py + h / 2 - t / 8, w, t / 4)
      ctx.fillStyle = accent
      for (let i = 0; i < size.w; i++) ctx.fillRect(px + i * t + t / 2 - 1, py + h / 4, 2, h / 2)
      break
    case 'hatch':
    case 'ladder':
      ctx.fillStyle = color
      ctx.fillRect(px + 3, py + 3, w - 6, h - 6)
      ctx.fillStyle = accent
      for (let i = 1; i < 4; i++) ctx.fillRect(px + 3, py + (i * h) / 4, w - 6, 2)
      break
    case 'rope':
      ctx.strokeStyle = color
      ctx.lineWidth = 3
      ctx.beginPath()
      ctx.arc(px + w / 2, py + h / 2, Math.min(w, h) / 3, 0, Math.PI * 2)
      ctx.stroke()
      ctx.strokeStyle = accent
      ctx.lineWidth = 1
      ctx.stroke()
      break
    default: {
      // Crates, consoles, lockers, cars, barricades: a box seen from above-front.
      ctx.fillStyle = accent
      ctx.fillRect(px + 2, py + h * 0.25, w - 4, h * 0.75 - 2)
      ctx.fillStyle = color
      ctx.fillRect(px + 2, py + 2, w - 4, h * 0.6)
      ctx.strokeStyle = accent
      ctx.lineWidth = 1
      ctx.strokeRect(px + 2.5, py + 2.5, w - 5, h * 0.6 - 1)
      ctx.beginPath()
      ctx.moveTo(px + 2, py + 2)
      ctx.lineTo(px + w - 2, py + h * 0.6)
      ctx.stroke()
    }
  }
}

function darkness(map: MapData): number {
  const light = map.ambience.light ?? (map.ambience.time === 'night' ? 'dark' : map.ambience.time === 'dusk' || map.ambience.time === 'dawn' ? 'dim' : 'bright')
  return light === 'dark' ? 0.62 : light === 'dim' ? 0.32 : 0
}

function drawLights(ctx: CanvasRenderingContext2D, scene: Scene, t: number) {
  const level = darkness(scene.map)
  if (level === 0) return
  const { width, height } = ctx.canvas
  const shade = typeof OffscreenCanvas === 'undefined' ? null : new OffscreenCanvas(width, height)
  const sctx = shade?.getContext('2d')
  if (!shade || !sctx) {
    ctx.fillStyle = `rgba(4,8,20,${level})`
    ctx.fillRect(0, 0, width, height)
    return
  }
  sctx.fillStyle = `rgba(4,8,20,${level})`
  sctx.fillRect(0, 0, width, height)
  sctx.globalCompositeOperation = 'destination-out'
  const sources = [
    ...(scene.map.lights ?? []).map((l) => ({ at: l.at, r: l.dim || l.bright, flicker: l.flicker })),
    // Each character carries a lantern's worth of sight.
    ...scene.tokens.filter((tk) => tk.party).map((tk) => ({ at: tk.at, r: 2, flicker: false })),
  ]
  for (const s of sources) {
    const cx = (s.at[0] + 0.5) * t
    const cy = (s.at[1] + 0.5) * t
    const wobble = s.flicker && scene.time ? 1 + Math.sin(scene.time / 90) * 0.04 : 1
    const r = Math.max(1, s.r) * t * wobble
    const g = sctx.createRadialGradient(cx, cy, 0, cx, cy, r)
    g.addColorStop(0, 'rgba(0,0,0,1)')
    g.addColorStop(1, 'rgba(0,0,0,0)')
    sctx.fillStyle = g
    sctx.fillRect(cx - r, cy - r, 2 * r, 2 * r)
  }
  ctx.drawImage(shade, 0, 0)
  // The lights' own colour, warm by default.
  ctx.globalCompositeOperation = 'lighter'
  for (const l of scene.map.lights ?? []) {
    const cx = (l.at[0] + 0.5) * t
    const cy = (l.at[1] + 0.5) * t
    const r = Math.max(1, l.bright) * t
    const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, r)
    g.addColorStop(0, (l.color ?? '#e0a650') + '55')
    g.addColorStop(1, (l.color ?? '#e0a650') + '00')
    ctx.fillStyle = g
    ctx.fillRect(cx - r, cy - r, 2 * r, 2 * r)
  }
  ctx.globalCompositeOperation = 'source-over'
}

function drawWeather(ctx: CanvasRenderingContext2D, scene: Scene) {
  const { width, height } = ctx.canvas
  const weather = scene.map.ambience.weather ?? 'clear'
  const time = scene.time
  if (weather === 'fog') {
    for (let i = 0; i < 6; i++) {
      const x = ((noise(i, 1) * width + time / 40) % (width + 200)) - 100
      const y = noise(i, 2) * height
      const g = ctx.createRadialGradient(x, y, 0, x, y, 160)
      g.addColorStop(0, 'rgba(200,210,220,0.18)')
      g.addColorStop(1, 'rgba(200,210,220,0)')
      ctx.fillStyle = g
      ctx.fillRect(x - 160, y - 160, 320, 320)
    }
  }
  if (weather === 'rain' || weather === 'storm') {
    ctx.strokeStyle = 'rgba(170,190,220,0.35)'
    ctx.lineWidth = 1
    ctx.beginPath()
    const drops = Math.floor((width * height) / 3000)
    for (let i = 0; i < drops; i++) {
      const x = (noise(i, 3) * width + time / 6) % width
      const y = (noise(i, 4) * height + time / 2) % height
      ctx.moveTo(x, y)
      ctx.lineTo(x - 3, y + 9)
    }
    ctx.stroke()
    if (weather === 'storm' && time && Math.floor(time / 100) % 70 === 0) {
      ctx.fillStyle = 'rgba(255,255,255,0.25)'
      ctx.fillRect(0, 0, width, height)
    }
  }
  if (weather === 'snow' || weather === 'sandstorm') {
    ctx.fillStyle = weather === 'snow' ? 'rgba(240,240,250,0.8)' : 'rgba(210,170,110,0.5)'
    const flakes = Math.floor((width * height) / 4000)
    for (let i = 0; i < flakes; i++) {
      const x = (noise(i, 5) * width + time / (weather === 'snow' ? 30 : 4)) % width
      const y = (noise(i, 6) * height + time / (weather === 'snow' ? 12 : 40)) % height
      ctx.fillRect(x, y, 2, 2)
    }
  }
}

/** Sprite pixels per screen pixel on a map of `t`-pixel cells: about a cell and a third tall, whole numbers only. */
function spriteScale(t: number): number {
  return Math.max(1, Math.round((t * 1.3) / SPRITE_HEIGHT))
}

function drawToken(ctx: CanvasRenderingContext2D, tk: TokenView, t: number, selected: boolean, scene: Scene) {
  const pose = scene.poses?.[tk.id]
  const x = pose?.x ?? tk.at[0]
  const y = pose?.y ?? tk.at[1]
  const cx = (x + 0.5) * t
  const cy = (y + 0.5) * t
  const sheet = tk.look ? scene.sprites?.[sheetUrl(tk.look, pose?.facing ?? tk.facing)] : undefined
  ctx.globalAlpha = tk.ghost ? 0.45 : pose?.blink ? 0.3 : 1
  if (sheet) {
    // The ring under the feet says whose it is and who is picked.
    ctx.lineWidth = Math.max(2, t / 16)
    ctx.strokeStyle = selected ? '#e0a650' : tk.mine ? '#3aa0ff' : tk.party ? 'rgba(239,230,210,0.7)' : 'rgba(194,69,58,0.85)'
    ctx.beginPath()
    ctx.ellipse(cx, cy + t * 0.3, t * 0.36, t * 0.14, 0, 0, Math.PI * 2)
    ctx.stroke()
    const k = spriteScale(t)
    const w = SPRITE_WIDTH * k
    const h = SPRITE_HEIGHT * k
    const frame = pose?.frame ?? 0
    // Feet on the lower part of the cell; the head rises over the row above (three-quarter view).
    ctx.drawImage(sheet, frame * SPRITE_WIDTH, 0, SPRITE_WIDTH, SPRITE_HEIGHT, Math.round(cx - w / 2), Math.round((y + 1) * t - h + k * 2), w, h)
    ctx.globalAlpha = 1
    return
  }
  ctx.fillStyle = 'rgba(0,0,0,0.4)'
  ctx.beginPath()
  ctx.ellipse(cx, cy + t * 0.32, t * 0.32, t * 0.12, 0, 0, Math.PI * 2)
  ctx.fill()
  ctx.fillStyle = tk.party ? '#efe6d2' : '#c2453a'
  ctx.beginPath()
  ctx.arc(cx, cy, t * 0.36, 0, Math.PI * 2)
  ctx.fill()
  ctx.lineWidth = tk.mine || selected ? 3 : 1.5
  ctx.strokeStyle = selected ? '#e0a650' : tk.mine ? '#3aa0ff' : '#14110d'
  ctx.stroke()
  ctx.fillStyle = tk.party ? '#14110d' : '#fff'
  ctx.font = `bold ${Math.round(t * 0.42)}px sans-serif`
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.fillText(Array.from(tk.name)[0]?.toUpperCase() ?? '', cx, cy + 1)
  ctx.globalAlpha = 1
}

/**
 * An imported image behind the grid, scaled so one of its cells covers
 * one tile; with `walls`, the grid's walls veiled over it (the editor,
 * to trace them).
 */
function drawBackdrop(
  ctx: CanvasRenderingContext2D,
  img: HTMLImageElement,
  b: NonNullable<MapData['backdrop']>,
  t: number,
  walls: Grid | null,
) {
  const scale = t / (b.cell_px ?? t)
  const [ox, oy] = b.offset ?? [0, 0]
  ctx.drawImage(img, -ox * scale, -oy * scale, img.naturalWidth * scale, img.naturalHeight * scale)
  if (!walls) return
  ctx.fillStyle = 'rgba(224,80,60,0.45)'
  for (let y = 0; y < walls.height; y++) {
    for (let x = 0; x < walls.width; x++) {
      if (walls.kind(x, y)?.wall) ctx.fillRect(x * t, y * t, t, t)
    }
  }
}

/** Draw `scene` on `ctx`, whose canvas is already sized to the map. */
export function drawScene(ctx: CanvasRenderingContext2D, scene: Scene) {
  const t = scene.tile ?? TILE
  const grid = readGrid(scene.map)
  const ts = scene.tileset ?? FALLBACK
  ctx.imageSmoothingEnabled = false
  ctx.fillStyle = ts.void
  ctx.fillRect(0, 0, ctx.canvas.width, ctx.canvas.height)

  const backdrop = scene.map.backdrop && scene.backdrop ? scene.backdrop : null
  if (backdrop) {
    drawBackdrop(ctx, backdrop, scene.map.backdrop ?? {}, t, scene.showWalls ? grid : null)
  } else {
    for (let y = 0; y < grid.height; y++) {
      for (let x = 0; x < grid.width; x++) {
        const k = grid.kind(x, y)
        if (!k || k.void || k.wall) continue
        const m = ts.materials[k.terrain] ?? FALLBACK_MATERIAL
        const mask = edgeMask(grid, x, y)
        const atlas = scene.atlases?.[k.terrain]
        if (atlas) {
          const size = ('width' in atlas ? Number(atlas.width) : 4 * t) / 4
          ctx.drawImage(atlas, (mask % 4) * size, Math.floor(mask / 4) * size, size, size, x * t, y * t, t, t)
        } else {
          drawPattern(ctx, m, x, y, t, scene.time)
          if (m.blend) drawBlend(ctx, grid, ts, mask, x, y, t)
          else drawEdges(ctx, m, mask, x, y, t)
        }
        if ((k.elevation ?? 0) > 0 && grid.kind(x, y + 1)?.elevation !== k.elevation) {
          ctx.fillStyle = m.dark
          ctx.fillRect(x * t, y * t + t - t / 4, t, t / 4)
        }
      }
    }
    for (let y = 0; y < grid.height; y++) {
      for (let x = 0; x < grid.width; x++) {
        if (grid.kind(x, y)?.wall) drawWall(ctx, grid, ts, x, y, t)
      }
    }
  }
  for (const d of scene.map.doors ?? []) {
    const px = d.at[0] * t
    const py = d.at[1] * t
    ctx.fillStyle = d.state === 'open' ? 'rgba(0,0,0,0.5)' : '#6b4a2e'
    ctx.fillRect(px + t / 6, py + t / 6, (2 * t) / 3, (5 * t) / 6)
    if (d.state === 'locked') {
      ctx.fillStyle = '#e0a650'
      ctx.fillRect(px + t / 2 - 2, py + t / 2, 4, 4)
    }
  }
  for (const p of scene.map.props ?? []) {
    const [w, h] = p.size ?? [1, 1]
    drawProp(ctx, ts.props[p.kind], p.at, { w, h }, t)
  }
  for (const o of scene.map.objects ?? []) drawProp(ctx, ts.props[o.kind], o.at, { w: 1, h: 1 }, t)

  for (const r of scene.reachable ?? []) {
    ctx.fillStyle = 'rgba(58,160,255,0.18)'
    ctx.fillRect(r.at[0] * t + 1, r.at[1] * t + 1, t - 2, t - 2)
  }
  for (const c of scene.highlight ?? []) {
    ctx.fillStyle = 'rgba(224,166,80,0.45)'
    ctx.fillRect(c[0] * t + 2, c[1] * t + 2, t - 4, t - 4)
  }
  for (const ex of scene.map.exits ?? []) {
    for (const c of ex.cells) {
      ctx.strokeStyle = 'rgba(224,166,80,0.8)'
      ctx.setLineDash([3, 3])
      ctx.strokeRect(c[0] * t + 2, c[1] * t + 2, t - 4, t - 4)
      ctx.setLineDash([])
    }
  }

  drawLights(ctx, scene, t)
  // Later rows paint over earlier ones: a character south of another stands in front of it.
  const rowOf = (tk: TokenView) => scene.poses?.[tk.id]?.y ?? tk.at[1]
  for (const tk of [...scene.tokens].sort((a, b) => rowOf(a) - rowOf(b))) drawToken(ctx, tk, t, tk.id === scene.selected, scene)
  drawWeather(ctx, scene)

  if (scene.veiled && scene.veiled.size > 0) {
    ctx.fillStyle = 'rgba(7,9,12,0.55)'
    for (const k of scene.veiled) {
      const [x, y] = k.split(',').map(Number)
      ctx.fillRect(x * t, y * t, t, t)
    }
  }
  ctx.fillStyle = '#efe6d2'
  ctx.font = `${Math.round(t * 0.34)}px sans-serif`
  ctx.textAlign = 'left'
  ctx.textBaseline = 'top'
  for (const l of scene.map.labels ?? []) ctx.fillText(l.text, l.at[0] * t + 2, l.at[1] * t + 2)
}

/** Whether a scene moves on its own (water, weather, flickering lights). */
export function animated(map: MapData, tileset: Tileset | null): boolean {
  const weather = map.ambience.weather ?? 'clear'
  if (weather !== 'clear' && weather !== 'cloudy') return true
  if ((map.lights ?? []).some((l) => l.flicker)) return true
  return Object.values(map.grid.legend).some((k) => tileset?.materials[k.terrain]?.pattern === 'water')
}
