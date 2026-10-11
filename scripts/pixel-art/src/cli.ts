#!/usr/bin/env bun
/**
 * normalize <image.png...> --out <dir> [--height 48] [--max-colors 16]
 *           [--outline] [--palette file.hex] [--preview 8]
 * turnaround <sheet.png...> --gabarit moyen --out <dir> [--palette file.hex]
 * animate <cast.json> --out <dir>
 * palette <out.hex> <image.png...> [--colors 32]
 * sheet <out.png> <image.png...> [--scale 6] [--columns 4]
 */
import { basename, join } from 'node:path'
import { mkdir } from 'node:fs/promises'
import { parseArgs } from 'node:util'
import { blit, createBitmap, readPng, scaleUp, setPixel, writePng, type Bitmap } from './image'
import { ANIMATION_NAMES, FRAME, keyFrame, renderSheet, type Item, type KeyFrame } from './animate'
import { alphaAt, type Rgb } from './image'
import { learnPalette, toHexFile } from './learn'
import { extractCells, normalize } from './normalize'
import { HOUSE_32, parsePaletteFile } from './palette'
import { GABARITS, KEY_POSES, type Gabarit, type KeyPose } from './pose'
import { cropColumn, splitTurnaround } from './turnaround'

const [command, ...rest] = Bun.argv.slice(2)
const { values, positionals } = parseArgs({
  args: rest,
  allowPositionals: true,
  options: {
    out: { type: 'string', default: 'out' },
    height: { type: 'string' },
    'max-colors': { type: 'string', default: '16' },
    outline: { type: 'boolean', default: false },
    palette: { type: 'string' },
    preview: { type: 'string', default: '0' },
    scale: { type: 'string', default: '6' },
    columns: { type: 'string', default: '4' },
    colors: { type: 'string', default: '32' },
    gabarit: { type: 'string', default: 'moyen' },
  },
})

/** Screen directions: the side view, mirrored, faces left. */
const DIRECTIONS = [
  ['bas', 'front', false],
  ['haut', 'back', false],
  ['droite', 'side', false],
  ['gauche', 'side', true],
] as const

if (command === 'normalize') {
  const palette = values.palette ? parsePaletteFile(await Bun.file(values.palette).text()) : HOUSE_32
  await mkdir(values.out, { recursive: true })
  for (const path of positionals) {
    const { sprite, cell, colors } = normalize(await readPng(path), {
      palette,
      maxColors: Number(values['max-colors']),
      height: values.height === undefined ? undefined : Number(values.height),
      outline: values.outline ? palette.reduce(darkest) : undefined,
    })
    const name = basename(path, '.png')
    await writePng(join(values.out, `${name}.png`), sprite)
    const preview = Number(values.preview)
    if (preview > 0) await writePng(join(values.out, `${name}@${preview}x.png`), scaleUp(sprite, preview))
    console.log(`${name}: ${sprite.width}×${sprite.height}, cell ${cell.toFixed(2)} px, ${colors} colours`)
  }
} else if (command === 'turnaround') {
  const gabarit = values.gabarit as Gabarit
  if (!(gabarit in GABARITS)) throw new Error(`unknown gabarit: ${gabarit}`)
  const palette = values.palette ? parsePaletteFile(await Bun.file(values.palette).text()) : HOUSE_32
  await mkdir(values.out, { recursive: true })
  for (const path of positionals) {
    const name = basename(path, '.png')
    for (const [view, image] of Object.entries(splitTurnaround(await readPng(path), gabarit))) {
      const { sprite, colors } = normalize(image, { palette, maxColors: Number(values['max-colors']) })
      await writePng(join(values.out, `${name}_${view}.png`), sprite)
      console.log(`${name}_${view}: ${sprite.width}×${sprite.height}, ${colors} colours`)
    }
  }
} else if (command === 'animate') {
  const [castPath] = positionals
  if (!castPath) throw new Error('usage: animate <cast.json> --out <dir>')
  const cast = (await Bun.file(castPath).json()) as CastEntry[]
  const root = castPath.replace(/[^/]*$/, '')
  await mkdir(values.out, { recursive: true })
  for (const entry of cast) {
    const raw = splitTurnaround(await readPng(join(root, entry.sheet)), entry.gabarit)
    const views = {
      front: normalize(raw.front, { palette: HOUSE_32, maxColors: 24 }).sprite,
      side: normalize(raw.side, { palette: HOUSE_32, maxColors: 24 }).sprite,
      back: normalize(raw.back, { palette: HOUSE_32, maxColors: 24 }).sprite,
    }
    const character = {
      views,
      weapon: entry.weapon && (await loadItem(join(root, entry.weapon.image), entry.weapon)),
      shield: entry.shield && (await loadItem(join(root, entry.shield.image), entry.shield)),
      keys: entry.keys && (await loadKeys(root, entry.keys, entry.gabarit)),
    }
    for (const [direction, view, mirror] of DIRECTIONS) {
      const { sheet } = renderSheet(character, view, mirror)
      await writePng(join(values.out, `${entry.name}_${direction}.png`), sheet)
    }
    console.log(`${entry.name}: ${views.front.width}×${views.front.height} front, 4 directions`)
  }
  await Bun.write(
    join(values.out, 'animations.json'),
    JSON.stringify({ frame: FRAME, directions: DIRECTIONS.map(([d]) => d), animations: ANIMATION_NAMES }, null, 2),
  )
} else if (command === 'palette') {
  const [target, ...inputs] = positionals
  if (!target || inputs.length === 0) throw new Error('usage: palette <out.hex> <image.png...>')
  const sprites = await Promise.all(inputs.map(async (path) => extractCells(await readPng(path)).cells))
  await Bun.write(target, toHexFile(learnPalette(sprites, Number(values.colors))))
  console.log(`${target}: ${values.colors} colours from ${inputs.length} images`)
} else if (command === 'sheet') {
  const [target, ...inputs] = positionals
  if (!target || inputs.length === 0) throw new Error('usage: sheet <out.png> <image.png...>')
  const images = await Promise.all(inputs.map(readPng))
  await writePng(target, sheet(images, Number(values.scale), Number(values.columns)))
} else {
  console.error('usage: cli.ts normalize|turnaround|palette|sheet …')
  process.exit(1)
}

interface ItemEntry {
  image: string
  /** Height in art pixels once normalized. */
  height: number
  /** Where the hand closes, as a share of the height from the top. */
  grip: number
}

interface CastEntry {
  name: string
  gabarit: Gabarit
  sheet: string
  weapon?: ItemEntry
  shield?: ItemEntry
  /** One drawing per key pose: the rest pose in the first panel, the key pose in the second. */
  keys?: Record<KeyPose, string>
}

async function loadKeys(root: string, keys: Record<KeyPose, string>, gabarit: Gabarit): Promise<Record<KeyPose, KeyFrame>> {
  const out = {} as Record<KeyPose, KeyFrame>
  for (const pose of Object.keys(KEY_POSES) as KeyPose[]) {
    if (!keys[pose]) throw new Error(`key pose ${pose} missing`)
    // Wider than a view: the strike flings the arm out.
    const panel = cropColumn(await readPng(join(root, keys[pose])), gabarit, 1, 1.4)
    out[pose] = keyFrame(normalize(panel, { palette: HOUSE_32, maxColors: 24, minIslandShare: 1 }).sprite, pose, gabarit)
  }
  return out
}

async function loadItem(path: string, entry: ItemEntry): Promise<Item> {
  const sprite = normalize(await readPng(path), { palette: HOUSE_32, height: entry.height, maxColors: 12 }).sprite
  const y = Math.min(sprite.height - 1, Math.round(sprite.height * entry.grip))
  // The grip sits on the middle of the item's opaque span at that row.
  const xs: number[] = []
  for (let x = 0; x < sprite.width; x++) if (alphaAt(sprite, x, y) > 0) xs.push(x)
  const x = xs.length > 0 ? Math.round((xs[0]! + xs[xs.length - 1]!) / 2) : sprite.width >> 1
  return { sprite, grip: [x, y] }
}

function darkest(a: Rgb, b: Rgb) {
  return a[0] + a[1] + a[2] <= b[0] + b[1] + b[2] ? a : b
}

/**
 * Lays images out on a neutral grid. Small sprites are scaled up by
 * `scale`, large ones fitted down to the same cell, to compare a raw
 * generation with its normalized sprite side by side.
 */
function sheet(images: Bitmap[], scale: number, columns: number): Bitmap {
  const cell = Math.max(...images.map((i) => (i.width > 256 ? 0 : Math.max(i.width, i.height) * scale)), 256)
  const gap = 8
  const rows = Math.ceil(images.length / columns)
  const out = createBitmap(columns * (cell + gap) + gap, rows * (cell + gap) + gap)
  for (let y = 0; y < out.height; y++) for (let x = 0; x < out.width; x++) setPixel(out, x, y, [58, 68, 102])
  images.forEach((image, i) => {
    const fitted = image.width > 256 ? fitDown(image, cell) : scaleUp(image, scale)
    const x = gap + (i % columns) * (cell + gap) + ((cell - fitted.width) >> 1)
    const y = gap + Math.floor(i / columns) * (cell + gap) + ((cell - fitted.height) >> 1)
    blit(out, fitted, x, y)
  })
  return out
}

function fitDown(image: Bitmap, size: number): Bitmap {
  const factor = Math.max(image.width, image.height) / size
  const out = createBitmap(Math.round(image.width / factor), Math.round(image.height / factor))
  for (let y = 0; y < out.height; y++) {
    for (let x = 0; x < out.width; x++) {
      const src = (Math.floor(y * factor) * image.width + Math.floor(x * factor)) * 4
      out.data.set(image.data.subarray(src, src + 4), (y * out.width + x) * 4)
    }
  }
  return out
}
