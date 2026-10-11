import { alphaAt, createBitmap, rgbAt, setPixel, type Bitmap, type Rgb } from './image'
import { labDistance, nearestIndex, toOklab, type Lab } from './palette'

/**
 * A diffusion model draws "pixel art" as blocks of roughly 8 screen
 * pixels, unevenly sized, with noise inside and hundreds of colours.
 * This module turns that into real pixel art: one value per cell, a
 * shared palette, a transparent background.
 */

/** SDXL's latent cell: one art pixel is this many image pixels. */
export const LATENT_CELL = 8

export interface NormalizeOptions {
  palette: readonly Rgb[]
  /** Upper bound on distinct colours in the result. */
  maxColors?: number
  /** Forces the sprite to this many pixels tall instead of using the grid. */
  height?: number
  /**
   * Art-pixel size in source pixels. SDXL works on a latent eight times
   * smaller than the image, so its blocks are 8 px by construction;
   * `'detect'` searches for it, for images from elsewhere.
   */
  cell?: number | 'detect'
  /**
   * Islands smaller than this share of the largest are dropped. 1 keeps
   * only the figure, for panels where a neighbour's elbow leaks in.
   */
  minIslandShare?: number
  /** OKLab distance under which a pixel touching the border counts as background. */
  backgroundTolerance?: number
  /** Draws a 1 px outline around the silhouette in this colour. */
  outline?: Rgb
}

export interface NormalizeResult {
  sprite: Bitmap
  /** Size of one art pixel in source pixels. */
  cell: number
  colors: number
}

export interface Box {
  x0: number
  y0: number
  x1: number
  y1: number
}

export function normalize(source: Bitmap, options: NormalizeOptions): NormalizeResult {
  const { cells, cell } = extractCells(source, options)
  let sprite = cells
  const paletteLab = options.palette.map(toOklab)
  sprite = quantize(sprite, options.palette, paletteLab, options.maxColors ?? options.palette.length)
  sprite = despeckle(sprite)
  sprite = crop(sprite)
  if (options.outline) sprite = outline(sprite, options.outline)
  return { sprite, cell, colors: countColors(sprite) }
}

/** Background removed and one pixel per grid cell, colours still unconstrained. */
export function extractCells(
  source: Bitmap,
  options: Pick<NormalizeOptions, 'height' | 'backgroundTolerance' | 'cell' | 'minIslandShare'> = {},
): { cells: Bitmap; cell: number } {
  const background = removeBackground(source, options.backgroundTolerance ?? 0.1)
  const cut = removeShadow(keepMainShapes(background, options.minIslandShare))
  const box = opaqueBox(cut)
  if (box === null) throw new Error('nothing left once the background is removed')
  const cell = options.cell ?? LATENT_CELL
  const grid =
    options.height !== undefined
      ? { cell: (box.y1 - box.y0) / options.height, x: box.x0, y: box.y0 }
      : cell === 'detect'
        ? detectGrid(cut, box)
        : detectGrid(cut, box, cell, cell)
  return { cells: sampleCells(cut, box, grid), cell: grid.cell }
}

/** Flood-fills from the border every pixel close to the border's dominant colour. */
export function removeBackground(source: Bitmap, tolerance: number): Bitmap {
  const { width, height } = source
  const out = { width, height, data: source.data.slice() }
  const border: Rgb[] = []
  for (let x = 0; x < width; x++) border.push(rgbAt(source, x, 0), rgbAt(source, x, height - 1))
  for (let y = 0; y < height; y++) border.push(rgbAt(source, 0, y), rgbAt(source, width - 1, y))
  const background = toOklab(medianRgb(border))

  const seen = new Uint8Array(width * height)
  const stack: number[] = []
  const push = (x: number, y: number) => {
    const i = y * width + x
    if (seen[i]) return
    seen[i] = 1
    if (alphaAt(source, x, y) === 0) {
      stack.push(i)
      return
    }
    if (labDistance(toOklab(rgbAt(source, x, y)), background) <= tolerance) stack.push(i)
  }
  for (let x = 0; x < width; x++) {
    push(x, 0)
    push(x, height - 1)
  }
  for (let y = 0; y < height; y++) {
    push(0, y)
    push(width - 1, y)
  }
  while (stack.length > 0) {
    const i = stack.pop()!
    const x = i % width
    const y = (i - x) / width
    out.data[i * 4 + 3] = 0
    if (x > 0) push(x - 1, y)
    if (x < width - 1) push(x + 1, y)
    if (y > 0) push(x, y - 1)
    if (y < height - 1) push(x, y + 1)
  }
  // Background showing through a gap, between the legs or under an arm:
  // the flood cannot reach it. Large enough patches of the exact
  // background colour go too; small ones are highlights.
  for (const region of regions(out, (x, y) => {
    return alphaAt(out, x, y) > 0 && labDistance(toOklab(rgbAt(out, x, y)), background) <= tolerance / 2
  })) {
    if (region.length >= ENCLOSED_MIN_AREA) for (const i of region) out.data[i * 4 + 3] = 0
  }
  return out
}

const ENCLOSED_MIN_AREA = 96

/**
 * Drops specks the flood left behind (vignetting in a corner, a stray
 * blot): every opaque island much smaller than the figure goes.
 */
export function keepMainShapes(bitmap: Bitmap, minShare = 0.02): Bitmap {
  const islands = regions(bitmap, (x, y) => alphaAt(bitmap, x, y) > 0)
  const largest = Math.max(0, ...islands.map((r) => r.length))
  const out = { width: bitmap.width, height: bitmap.height, data: bitmap.data.slice() }
  for (const island of islands) {
    if (island.length < largest * minShare) for (const i of island) out.data[i * 4 + 3] = 0
  }
  return out
}

/**
 * The model drops a grey ellipse under the feet. It is neutral, touches
 * the background and sits in the bottom of the figure: armour can be
 * just as grey, hence the band.
 */
export function removeShadow(bitmap: Bitmap, band = 0.1, maxChroma = 0.03): Bitmap {
  const box = opaqueBox(bitmap)
  if (box === null) return bitmap
  const out = { width: bitmap.width, height: bitmap.height, data: bitmap.data.slice() }
  const top = Math.floor(box.y1 - (box.y1 - box.y0) * band)
  const neutral = (x: number, y: number) => {
    if (y < top || alphaAt(out, x, y) === 0) return false
    const [l, a, b] = toOklab(rgbAt(out, x, y))
    return l > 0.25 && Math.hypot(a, b) <= maxChroma
  }
  for (const region of regions(out, neutral)) {
    const touchesBackground = region.some((i) => {
      const x = i % out.width
      const y = (i - x) / out.width
      return [[x - 1, y], [x + 1, y], [x, y - 1], [x, y + 1]].some(
        ([nx, ny]) => nx! >= 0 && ny! >= 0 && nx! < out.width && ny! < out.height && alphaAt(out, nx!, ny!) === 0,
      )
    })
    if (touchesBackground) for (const i of region) out.data[i * 4 + 3] = 0
  }
  return out
}

/** 4-connected regions of pixels matching `inside`, as pixel indices. */
function regions(bitmap: Bitmap, inside: (x: number, y: number) => boolean): number[][] {
  const { width, height } = bitmap
  const seen = new Uint8Array(width * height)
  const found: number[][] = []
  for (let start = 0; start < width * height; start++) {
    if (seen[start]) continue
    const sx = start % width
    if (!inside(sx, (start - sx) / width)) continue
    const region: number[] = []
    const stack = [start]
    seen[start] = 1
    while (stack.length > 0) {
      const i = stack.pop()!
      region.push(i)
      const x = i % width
      const y = (i - x) / width
      for (const [nx, ny] of [[x - 1, y], [x + 1, y], [x, y - 1], [x, y + 1]] as const) {
        if (nx < 0 || ny < 0 || nx >= width || ny >= height) continue
        const n = ny * width + nx
        if (seen[n] || !inside(nx, ny)) continue
        seen[n] = 1
        stack.push(n)
      }
    }
    found.push(region)
  }
  return found
}

export function opaqueBox(bitmap: Bitmap): Box | null {
  let x0 = Infinity
  let y0 = Infinity
  let x1 = -1
  let y1 = -1
  for (let y = 0; y < bitmap.height; y++) {
    for (let x = 0; x < bitmap.width; x++) {
      if (alphaAt(bitmap, x, y) < 128) continue
      x0 = Math.min(x0, x)
      y0 = Math.min(y0, y)
      x1 = Math.max(x1, x + 1)
      y1 = Math.max(y1, y + 1)
    }
  }
  return x1 < 0 ? null : { x0, y0, x1, y1 }
}

export interface Grid {
  cell: number
  /** Where one grid line falls, in source pixels. */
  x: number
  y: number
}

/**
 * Finds the art-pixel size from where colour edges line up. Edges pile
 * up on grid lines, so the right period scores high; its multiples
 * score as high, hence the smallest period close to the best wins.
 */
export function detectGrid(bitmap: Bitmap, box: Box, minCell = 3, maxCell = 32): Grid {
  const columns = edgeProfile(bitmap, box, 'x')
  const rows = edgeProfile(bitmap, box, 'y')
  const candidates: { cell: number; score: number; x: number; y: number }[] = []
  for (let cell = minCell; cell <= maxCell; cell += 0.05) {
    const cx = bestPhase(columns, box.x0, box.x1, cell)
    const cy = bestPhase(rows, box.y0, box.y1, cell)
    candidates.push({ cell, score: cx.score + cy.score, x: cx.phase, y: cy.phase })
  }
  const best = Math.max(...candidates.map((c) => c.score))
  const chosen = candidates.find((c) => c.score >= best * 0.85)!
  return { cell: chosen.cell, x: chosen.x, y: chosen.y }
}

function edgeProfile(bitmap: Bitmap, box: Box, axis: 'x' | 'y'): Float64Array {
  const size = axis === 'x' ? bitmap.width : bitmap.height
  const profile = new Float64Array(size)
  for (let y = box.y0; y < box.y1; y++) {
    for (let x = box.x0; x < box.x1; x++) {
      const px = axis === 'x' ? x - 1 : x
      const py = axis === 'x' ? y : y - 1
      if (px < 0 || py < 0) continue
      if (alphaAt(bitmap, x, y) < 128 || alphaAt(bitmap, px, py) < 128) continue
      const a = rgbAt(bitmap, x, y)
      const b = rgbAt(bitmap, px, py)
      profile[axis === 'x' ? x : y]! += Math.abs(a[0] - b[0]) + Math.abs(a[1] - b[1]) + Math.abs(a[2] - b[2])
    }
  }
  return profile
}

function bestPhase(profile: Float64Array, from: number, to: number, cell: number) {
  let mean = 0
  for (let i = from; i < to; i++) mean += profile[i]!
  mean /= Math.max(1, to - from)
  let best = { phase: from, score: 0 }
  for (let phase = 0; phase < cell; phase += 0.5) {
    let sum = 0
    let lines = 0
    for (let at = from + phase; at < to; at += cell) {
      sum += profile[Math.round(at)] ?? 0
      lines++
    }
    const score = lines === 0 || mean === 0 ? 0 : sum / lines / mean
    if (score > best.score) best = { phase: from + phase, score }
  }
  return best
}

/**
 * One output pixel per grid cell: the median of the cell's centre, so
 * that the blurry seams between blocks never decide the colour.
 */
export function sampleCells(bitmap: Bitmap, box: Box, grid: Grid): Bitmap {
  const startX = grid.x - Math.ceil((grid.x - box.x0) / grid.cell) * grid.cell
  const startY = grid.y - Math.ceil((grid.y - box.y0) / grid.cell) * grid.cell
  const columns = Math.ceil((box.x1 - startX) / grid.cell)
  const rows = Math.ceil((box.y1 - startY) / grid.cell)
  const out = createBitmap(columns, rows)
  const inset = grid.cell * 0.25
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < columns; col++) {
      const left = startX + col * grid.cell
      const top = startY + row * grid.cell
      const pixels: Rgb[] = []
      let total = 0
      for (let y = Math.floor(top + inset); y < Math.ceil(top + grid.cell - inset); y++) {
        for (let x = Math.floor(left + inset); x < Math.ceil(left + grid.cell - inset); x++) {
          if (x < 0 || y < 0 || x >= bitmap.width || y >= bitmap.height) continue
          total++
          if (alphaAt(bitmap, x, y) >= 128) pixels.push(rgbAt(bitmap, x, y))
        }
      }
      if (total > 0 && pixels.length * 2 >= total) setPixel(out, col, row, medianRgb(pixels))
    }
  }
  return out
}

/** Snaps every pixel to the palette, then merges the rarest colours away. */
export function quantize(
  bitmap: Bitmap,
  palette: readonly Rgb[],
  paletteLab: readonly Lab[],
  maxColors: number,
): Bitmap {
  const out = createBitmap(bitmap.width, bitmap.height)
  const index = new Int16Array(bitmap.width * bitmap.height).fill(-1)
  const usage = new Map<number, number>()
  for (let i = 0; i < index.length; i++) {
    if (bitmap.data[i * 4 + 3]! < 128) continue
    const x = i % bitmap.width
    const p = nearestIndex(toOklab(rgbAt(bitmap, x, (i - x) / bitmap.width)), paletteLab)
    index[i] = p
    usage.set(p, (usage.get(p) ?? 0) + 1)
  }
  const remap = new Map<number, number>()
  while (usage.size > maxColors) {
    const [rarest] = [...usage.entries()].sort((a, b) => a[1] - b[1])[0]!
    usage.delete(rarest)
    const kept = [...usage.keys()]
    const target = kept[nearestIndex(paletteLab[rarest]!, kept.map((k) => paletteLab[k]!))]!
    remap.set(rarest, target)
    usage.set(target, usage.get(target)! + 1)
  }
  for (let i = 0; i < index.length; i++) {
    let p = index[i]!
    if (p < 0) continue
    while (remap.has(p)) p = remap.get(p)!
    const x = i % bitmap.width
    setPixel(out, x, (i - x) / bitmap.width, palette[p]!)
  }
  return out
}

/** Replaces lone pixels that share their colour with no neighbour. */
export function despeckle(bitmap: Bitmap): Bitmap {
  const out = { width: bitmap.width, height: bitmap.height, data: bitmap.data.slice() }
  for (let y = 0; y < bitmap.height; y++) {
    for (let x = 0; x < bitmap.width; x++) {
      if (alphaAt(bitmap, x, y) === 0) continue
      const own = rgbAt(bitmap, x, y).join()
      const counts = new Map<string, { rgb: Rgb; count: number }>()
      let alone = true
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          if (dx === 0 && dy === 0) continue
          const nx = x + dx
          const ny = y + dy
          if (nx < 0 || ny < 0 || nx >= bitmap.width || ny >= bitmap.height) continue
          if (alphaAt(bitmap, nx, ny) === 0) continue
          const rgb = rgbAt(bitmap, nx, ny)
          const key = rgb.join()
          if (key === own) alone = false
          counts.set(key, { rgb, count: (counts.get(key)?.count ?? 0) + 1 })
        }
      }
      if (!alone) continue
      const major = [...counts.values()].sort((a, b) => b.count - a.count)[0]
      if (major && major.count >= 5) setPixel(out, x, y, major.rgb)
    }
  }
  return out
}

export function crop(bitmap: Bitmap): Bitmap {
  const box = opaqueBox(bitmap)
  if (box === null) return createBitmap(0, 0)
  const out = createBitmap(box.x1 - box.x0, box.y1 - box.y0)
  for (let y = 0; y < out.height; y++) {
    const from = ((box.y0 + y) * bitmap.width + box.x0) * 4
    out.data.set(bitmap.data.subarray(from, from + out.width * 4), y * out.width * 4)
  }
  return out
}

export function outline(bitmap: Bitmap, colour: Rgb): Bitmap {
  const out = createBitmap(bitmap.width + 2, bitmap.height + 2)
  for (let y = 0; y < bitmap.height; y++) {
    const from = y * bitmap.width * 4
    out.data.set(bitmap.data.subarray(from, from + bitmap.width * 4), ((y + 1) * out.width + 1) * 4)
  }
  const solid = (x: number, y: number) =>
    x >= 0 && y >= 0 && x < out.width && y < out.height && out.data[(y * out.width + x) * 4 + 3]! > 0
  const edge: [number, number][] = []
  for (let y = 0; y < out.height; y++) {
    for (let x = 0; x < out.width; x++) {
      if (solid(x, y)) continue
      if (solid(x - 1, y) || solid(x + 1, y) || solid(x, y - 1) || solid(x, y + 1)) edge.push([x, y])
    }
  }
  for (const [x, y] of edge) setPixel(out, x, y, colour)
  return out
}

export function countColors(bitmap: Bitmap): number {
  const seen = new Set<string>()
  for (let y = 0; y < bitmap.height; y++) {
    for (let x = 0; x < bitmap.width; x++) {
      if (alphaAt(bitmap, x, y) > 0) seen.add(rgbAt(bitmap, x, y).join())
    }
  }
  return seen.size
}

function medianRgb(pixels: readonly Rgb[]): Rgb {
  const channel = (c: 0 | 1 | 2) => {
    const values = pixels.map((p) => p[c]).sort((a, b) => a - b)
    return values[values.length >> 1] ?? 0
  }
  return [channel(0), channel(1), channel(2)]
}
