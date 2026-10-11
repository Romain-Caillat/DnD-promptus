import { alphaAt, createBitmap, rgbAt, setPixel, type Bitmap, type Rgb } from './image'
import { LATENT_CELL } from './normalize'
import { HOUSE_32 } from './palette'
import { GABARITS, KEY_POSES, type Gabarit, type KeyPose, type View } from './pose'

/**
 * Cut-out animation. Every character stands on the same mannequin, so
 * its body parts sit at known heights: the sprite is sliced into head,
 * torso, arms and legs by those proportions, and each animation moves
 * the slices by whole pixels. The animations are written once, here,
 * and every character gets all of them, consistent by construction.
 *
 * Weapons are separate sprites held at the hand: generated with the
 * body they never matched from one view to the next, and separate they
 * can swing.
 *
 * A slice cannot raise an arm overhead, so the big gestures (the swing,
 * the cast) can come from key poses the model drew, front view only:
 * the whole figure is swapped for the drawing during those frames.
 */

export const FRAME = 80
/** Row the feet stand on, inside a frame. */
const FLOOR = 74

type Part = 'head' | 'torso' | 'armL' | 'armR' | 'legL' | 'legR'
const PARTS: readonly Part[] = ['head', 'torso', 'armL', 'armR', 'legL', 'legR']
type Offset = readonly [number, number]

export interface Item {
  sprite: Bitmap
  /** Where the hand closes on it, in item pixels. */
  grip: readonly [number, number]
}

interface Rig {
  view: View
  parts: Record<Part, Bitmap>
  /** Body axis and sprite height, in sprite pixels. */
  axis: number
  height: number
  /** Hands in sprite pixels, by screen side. */
  hands: { L: Offset; R: Offset }
}

export interface KeyFrame {
  sprite: Bitmap
  axis: number
  /** Hands in sprite pixels, by screen side. */
  hands: { L: Offset; R: Offset }
}

/** The body's axis: the mean column of the legs, which stay put whatever the arms do. */
function feetAxis(sprite: Bitmap): number {
  const { width, height } = sprite
  let sum = 0
  let count = 0
  for (let y = Math.floor(height * 0.75); y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (alphaAt(sprite, x, y) === 0) continue
      sum += x + 0.5
      count++
    }
  }
  return count === 0 ? width / 2 : sum / count
}

/**
 * A key pose drawn by the model on the mannequin. Its hands are found
 * where the skeleton put the wrists: the feet stand on the sprite's
 * bottom row, as they stood on the generation's baseline.
 */
export function keyFrame(sprite: Bitmap, pose: KeyPose, gabarit: Gabarit): KeyFrame {
  const axis = feetAxis(sprite)
  const { height, width } = GABARITS[gabarit]
  const at = (p: readonly [number, number] | null): Offset => {
    if (!p) throw new Error(`key pose ${pose} has no wrist`)
    return [Math.round(axis + (p[0] * height * width) / LATENT_CELL), Math.round(sprite.height - ((1 - p[1]) * height) / LATENT_CELL)]
  }
  // OpenPose wrists: 4 is the right one, on the screen's left from the front.
  return { sprite, axis, hands: { L: at(KEY_POSES[pose][4]!), R: at(KEY_POSES[pose][7]!) } }
}

/** Slices a normalized sprite by the mannequin's proportions. */
export function rig(sprite: Bitmap, view: View): Rig {
  const { width, height } = sprite
  const axis = feetAxis(sprite)
  const parts = Object.fromEntries(PARTS.map((p) => [p, createBitmap(width, height)])) as Record<Part, Bitmap>
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (alphaAt(sprite, x, y) === 0) continue
      const u = (x + 0.5 - axis) / height
      const v = (y + 0.5) / height
      let part: Part
      if (v < 0.35) part = 'head'
      else if (v > 0.68) part = u < 0 ? 'legL' : 'legR'
      else if (view !== 'side' && Math.abs(u) > 0.145) part = u < 0 ? 'armL' : 'armR'
      else part = 'torso'
      setPixel(parts[part], x, y, rgbAt(sprite, x, y), alphaAt(sprite, x, y))
    }
  }
  const hand = (part: Bitmap, fallback: Offset): Offset => {
    // The hand is the arm's lowest row.
    for (let y = part.height - 1; y >= 0; y--) {
      let xs = 0
      let n = 0
      for (let x = 0; x < part.width; x++) {
        if (alphaAt(part, x, y) > 0) {
          xs += x
          n++
        }
      }
      if (n > 0) return [Math.round(xs / n), y - 1]
    }
    return fallback
  }
  const side: Offset = [Math.round(axis + 0.05 * height), Math.round(0.6 * height)]
  return {
    view,
    parts,
    axis,
    height,
    hands:
      view === 'side'
        ? { L: side, R: side }
        : {
            L: hand(parts.armL, [Math.round(axis - 0.2 * height), Math.round(0.6 * height)]),
            R: hand(parts.armR, [Math.round(axis + 0.2 * height), Math.round(0.6 * height)]),
          },
  }
}

/** One frame's worth of motion. Every field is optional: absent means at rest. */
interface Pose {
  /** Moves the whole figure. */
  body?: Offset
  /** Moves everything above the legs. */
  upper?: Offset
  parts?: Partial<Record<Part, Offset>>
  /** Weapon rotation in degrees, clockwise, around the grip. */
  weapon?: number
  tint?: 'white' | 'red'
  /** Draws the swept arc between these two weapon angles. */
  slash?: readonly [number, number]
  /** Healing sparkles, by animation phase. */
  sparkles?: number
  halo?: boolean
  /** Lies down: the frame is turned a quarter round on the feet. */
  lying?: boolean
  /** Drops every other pixel, for fading out. */
  dither?: boolean
  /** Draws this key pose instead of the rig, when the character has key poses. */
  key?: KeyPose
}

export interface Animation {
  name: string
  /** Milliseconds per frame. */
  ms: number
  frames: Pose[]
}

/**
 * The animations, for a figure seen from the front. `forward` is the
 * screen direction the character faces, used for lunges and recoils.
 * `keyed` swaps the rig's swing and cast for the drawn key poses.
 */
function animations(view: View, keyed = false): Animation[] {
  const forward: Offset = view === 'front' ? [0, 1] : view === 'back' ? [0, -1] : [1, 0]
  const fwd = (n: number): Offset => [forward[0] * n, forward[1] * n]
  const swing = view === 'back' ? -1 : 1
  const stride: Pose[] =
    view === 'side'
      ? [
          { upper: [0, -1], parts: { legL: [-2, 0], legR: [2, 0] } },
          {},
          { upper: [0, -1], parts: { legL: [2, 0], legR: [-2, 0] } },
          {},
        ]
      : [
          { upper: [0, -1], parts: { legL: [0, -1], armL: [0, 1], armR: [0, -1] } },
          {},
          { upper: [0, -1], parts: { legR: [0, -1], armL: [0, -1], armR: [0, 1] } },
          {},
        ]
  const raise = (n: number): Pose['parts'] => ({ armL: [0, -n], armR: [0, -n] })
  return [
    {
      name: 'repos',
      ms: 220,
      frames: [{}, {}, { parts: { head: [0, 1], armL: [0, 1], armR: [0, 1] } }, { parts: { head: [0, 1], armL: [0, 1], armR: [0, 1] } }],
    },
    { name: 'marche', ms: 150, frames: stride },
    {
      name: 'brandir',
      ms: 170,
      frames: [{ weapon: -14 * swing }, { weapon: 0 }, { weapon: 14 * swing }, { weapon: 0 }],
    },
    {
      name: 'attaque',
      ms: 110,
      // Keyed, the arm comes down from overhead and ends flung out on
      // the screen's left: the weapon turns counter-clockwise.
      frames: keyed
        ? [
            { key: 'windup', weapon: 10 },
            { key: 'windup', weapon: 25, body: fwd(-1) },
            { key: 'strike', weapon: -60, body: fwd(2) },
            { key: 'strike', weapon: -115, body: fwd(2), slash: [25, -115] },
            { key: 'strike', weapon: -100, body: fwd(1) },
            { weapon: 15 },
          ]
        : [
            { weapon: -35 * swing, parts: raise(1) },
            { weapon: -80 * swing, parts: raise(3), body: fwd(-1) },
            { weapon: 30 * swing, body: fwd(2) },
            { weapon: 95 * swing, body: fwd(2), slash: [-80 * swing, 95 * swing], parts: { armL: [0, 1], armR: [0, 1] } },
            { weapon: 80 * swing, body: fwd(1) },
            { weapon: 15 * swing },
          ],
    },
    {
      name: 'soin',
      ms: 120,
      frames: [0, 1, 2, 3, 3, 3, 2, 1].map((n, i) => ({
        parts: raise(n),
        key: keyed && n >= 2 ? ('cast' as const) : undefined,
        sparkles: i,
        halo: i >= 3 && i <= 5,
      })),
    },
    {
      name: 'touché',
      ms: 90,
      frames: [{ tint: 'white' }, { tint: 'red', body: fwd(-2) }, { body: fwd(-1) }, {}],
    },
    {
      name: 'mort',
      ms: 170,
      frames: [
        { tint: 'white' },
        { tint: 'red', body: fwd(-2) },
        { upper: [0, 3] },
        { upper: [0, 6] },
        { lying: true },
        { lying: true, dither: true },
      ],
    },
  ]
}

export const ANIMATION_NAMES = animations('front').map((a) => ({ name: a.name, ms: a.ms, frames: a.frames.length }))

export interface Character {
  views: Record<View, Bitmap>
  weapon?: Item
  shield?: Item
  /** Drawn key poses, front view. */
  keys?: Record<KeyPose, KeyFrame>
}

/**
 * Renders one view as a sheet: one row per animation, one 80 px frame
 * per column. `mirror` flips the side view to face left.
 */
export function renderSheet(character: Character, view: View, mirror = false): { sheet: Bitmap; animations: Animation[] } {
  const r = rig(character.views[view], view)
  const list = animations(view, view === 'front' && character.keys !== undefined)
  const columns = Math.max(...list.map((a) => a.frames.length))
  const sheet = createBitmap(columns * FRAME, list.length * FRAME)
  list.forEach((animation, row) => {
    animation.frames.forEach((pose, column) => {
      let frame = compose(r, pose, character)
      if (mirror) frame = flip(frame)
      paste(sheet, frame, column * FRAME, row * FRAME)
    })
  })
  return { sheet, animations: list }
}

const SPARKLES = HOUSE_32.filter((_, i) => i === 24 || i === 23 || i === 17)
const SLASH: readonly Rgb[] = [HOUSE_32[6]!, HOUSE_32[5]!]
const HALO = HOUSE_32[24]!
const WHITE = HOUSE_32[6]!
const REDS = [HOUSE_32[30]!, HOUSE_32[31]!]

function compose(r: Rig, pose: Pose, character: Character): Bitmap {
  const key = pose.key && character.keys?.[pose.key]
  const { frame, weaponHand } = key ? drawKey(key, pose, character) : drawRig(r, pose, character)
  if (pose.tint) tint(frame, pose.tint)
  if (pose.halo) halo(frame)
  if (pose.slash && character.weapon) slash(frame, weaponHand, character.weapon, pose.slash)
  if (pose.sparkles !== undefined) sparkles(frame, pose.sparkles, r.height)
  let out = frame
  if (pose.lying) out = lieDown(out)
  if (pose.dither) dither(out)
  return out
}

/** A key pose, front view: the weapon in the hand on the screen's left, the shield in the other. */
function drawKey(key: KeyFrame, pose: Pose, character: Character): { frame: Bitmap; weaponHand: Offset } {
  const frame = createBitmap(FRAME, FRAME)
  const ox = Math.round(FRAME / 2 - key.axis) + (pose.body?.[0] ?? 0)
  const oy = FLOOR - key.sprite.height + (pose.body?.[1] ?? 0)
  paste(frame, key.sprite, ox, oy)
  const weaponHand: Offset = [ox + key.hands.L[0], oy + key.hands.L[1]]
  if (character.weapon) drawItem(frame, character.weapon, weaponHand, pose.weapon ?? 0)
  if (character.shield) drawItem(frame, character.shield, [ox + key.hands.R[0], oy + key.hands.R[1]], 0)
  return { frame, weaponHand }
}

function drawRig(r: Rig, pose: Pose, character: Character): { frame: Bitmap; weaponHand: Offset } {
  const frame = createBitmap(FRAME, FRAME)
  const ox = Math.round(FRAME / 2 - r.axis) + (pose.body?.[0] ?? 0)
  const oy = FLOOR - r.height + (pose.body?.[1] ?? 0)
  const offset = (part: Part): Offset => {
    const own = pose.parts?.[part] ?? [0, 0]
    const upper = part === 'legL' || part === 'legR' ? [0, 0] : (pose.upper ?? [0, 0])
    return [ox + own[0] + upper[0]!, oy + own[1] + upper[1]!]
  }
  // Which screen-side hand holds what: the right hand is on the
  // screen's left from the front, on its right from behind.
  const weaponArm: Part = r.view === 'back' ? 'armR' : 'armL'
  const shieldArm: Part = r.view === 'back' ? 'armL' : 'armR'
  const at = (arm: Part, hand: Offset): Offset => {
    const [dx, dy] = r.view === 'side' ? offset('torso') : offset(arm)
    return [hand[0] + dx, hand[1] + dy]
  }
  const weaponHand = at(weaponArm, weaponArm === 'armL' ? r.hands.L : r.hands.R)
  const shieldHand = at(shieldArm, shieldArm === 'armL' ? r.hands.L : r.hands.R)

  const drawWeapon = () => {
    if (character.weapon) drawItem(frame, character.weapon, weaponHand, pose.weapon ?? 0)
  }
  const drawShield = () => {
    if (character.shield && r.view !== 'side') drawItem(frame, character.shield, shieldHand, 0)
  }
  const order: Part[] = r.view === 'back' ? ['legL', 'legR', 'torso', 'armL', 'armR', 'head'] : ['legL', 'legR', 'torso', 'head', 'armL', 'armR']
  if (r.view === 'back') drawWeapon()
  for (const part of order) paste(frame, r.parts[part], ...offset(part))
  if (r.view !== 'back') drawWeapon()
  drawShield()
  return { frame, weaponHand }
}

/** Draws an item rotated around its grip, the grip on `hand`. */
function drawItem(frame: Bitmap, item: Item, hand: Offset, degrees: number) {
  const a = (degrees * Math.PI) / 180
  const cos = Math.cos(a)
  const sin = Math.sin(a)
  const reach = Math.ceil(Math.hypot(item.sprite.width, item.sprite.height))
  for (let y = hand[1] - reach; y <= hand[1] + reach; y++) {
    for (let x = hand[0] - reach; x <= hand[0] + reach; x++) {
      if (x < 0 || y < 0 || x >= FRAME || y >= FRAME) continue
      // Inverse rotation: which item pixel lands here.
      const dx = x - hand[0]
      const dy = y - hand[1]
      const sx = Math.round(item.grip[0] + dx * cos + dy * sin)
      const sy = Math.round(item.grip[1] - dx * sin + dy * cos)
      if (sx < 0 || sy < 0 || sx >= item.sprite.width || sy >= item.sprite.height) continue
      if (alphaAt(item.sprite, sx, sy) === 0) continue
      setPixel(frame, x, y, rgbAt(item.sprite, sx, sy))
    }
  }
}

function slash(frame: Bitmap, hand: Offset, weapon: Item, [from, to]: readonly [number, number]) {
  const length = weapon.grip[1] + 2
  const step = Math.sign(to - from) * 2
  for (let deg = from; Math.sign(to - deg) === Math.sign(step); deg += step) {
    const a = (deg * Math.PI) / 180
    for (const [k, colour] of SLASH.entries()) {
      const x = Math.round(hand[0] + Math.sin(a) * (length + 1 - k))
      const y = Math.round(hand[1] - Math.cos(a) * (length + 1 - k))
      if (x >= 0 && y >= 0 && x < FRAME && y < FRAME && alphaAt(frame, x, y) === 0) setPixel(frame, x, y, colour)
    }
  }
}

function sparkles(frame: Bitmap, phase: number, height: number) {
  const seeds = [
    [-14, 0], [10, 2], [-6, 5], [15, 7], [-17, 9], [4, 11], [-9, 13], [12, 15],
  ] as const
  seeds.forEach(([dx, start], i) => {
    const age = phase - (i % 4)
    if (age < 0) return
    const x = FRAME / 2 + dx
    const y = FLOOR - Math.round(height * 0.3) - start - age * 3
    const colour = SPARKLES[i % SPARKLES.length]!
    const plus = i % 2 === 0 && age < 4
    const dots: Offset[] = plus ? [[0, 0], [1, 0], [-1, 0], [0, 1], [0, -1]] : [[0, 0]]
    for (const [px, py] of dots) {
      const tx = x + px
      const ty = y + py
      if (tx >= 0 && ty >= 0 && tx < FRAME && ty < FRAME) setPixel(frame, tx, ty, colour)
    }
  })
}

function halo(frame: Bitmap) {
  const edge: Offset[] = []
  for (let y = 0; y < FRAME; y++) {
    for (let x = 0; x < FRAME; x++) {
      if (alphaAt(frame, x, y) > 0) continue
      const solid = [[1, 0], [-1, 0], [0, 1], [0, -1]].some(([dx, dy]) => {
        const nx = x + dx!
        const ny = y + dy!
        return nx >= 0 && ny >= 0 && nx < FRAME && ny < FRAME && alphaAt(frame, nx, ny) > 0
      })
      if (solid) edge.push([x, y])
    }
  }
  for (const [x, y] of edge) setPixel(frame, x, y, HALO)
}

/** White flash, or red by luminance so it stays on the palette. */
function tint(frame: Bitmap, kind: 'white' | 'red') {
  for (let y = 0; y < FRAME; y++) {
    for (let x = 0; x < FRAME; x++) {
      if (alphaAt(frame, x, y) === 0) continue
      const [r, g, b] = rgbAt(frame, x, y)
      if (kind === 'white') setPixel(frame, x, y, WHITE)
      else setPixel(frame, x, y, REDS[0.3 * r + 0.59 * g + 0.11 * b > 90 ? 1 : 0]!)
    }
  }
}

/**
 * A quarter turn clockwise: the head goes right, the figure lies on the
 * floor, centred so that even the tallest one stays inside the frame.
 */
function lieDown(frame: Bitmap): Bitmap {
  let x0 = FRAME
  let y0 = FRAME
  let x1 = -1
  let y1 = -1
  for (let y = 0; y < FRAME; y++) {
    for (let x = 0; x < FRAME; x++) {
      if (alphaAt(frame, x, y) === 0) continue
      x0 = Math.min(x0, x)
      y0 = Math.min(y0, y)
      x1 = Math.max(x1, x)
      y1 = Math.max(y1, y)
    }
  }
  const out = createBitmap(FRAME, FRAME)
  if (x1 < 0) return out
  const left = Math.round(FRAME / 2 - (y1 - y0) / 2)
  const top = FLOOR - 1 - (x1 - x0)
  for (let y = 0; y < FRAME; y++) {
    for (let x = 0; x < FRAME; x++) {
      // Inverse of the turn: feet on the left, the figure's left side up.
      const sy = y1 - (x - left)
      const sx = x0 + (y - top)
      if (sx < x0 || sy < y0 || sx > x1 || sy > y1) continue
      if (alphaAt(frame, sx, sy) === 0) continue
      setPixel(out, x, y, rgbAt(frame, sx, sy))
    }
  }
  return out
}

function dither(frame: Bitmap) {
  for (let y = 0; y < FRAME; y++) {
    for (let x = 0; x < FRAME; x++) if ((x + y) % 2 === 0) frame.data[(y * FRAME + x) * 4 + 3] = 0
  }
}

function flip(frame: Bitmap): Bitmap {
  const out = createBitmap(frame.width, frame.height)
  for (let y = 0; y < frame.height; y++) {
    for (let x = 0; x < frame.width; x++) {
      if (alphaAt(frame, x, y) > 0) setPixel(out, frame.width - 1 - x, y, rgbAt(frame, x, y))
    }
  }
  return out
}

/** Copies opaque pixels of `src` onto `dst` at (dx, dy). */
function paste(dst: Bitmap, src: Bitmap, dx: number, dy: number) {
  for (let y = 0; y < src.height; y++) {
    for (let x = 0; x < src.width; x++) {
      const tx = x + dx
      const ty = y + dy
      if (tx < 0 || ty < 0 || tx >= dst.width || ty >= dst.height) continue
      if (alphaAt(src, x, y) === 0) continue
      setPixel(dst, tx, ty, rgbAt(src, x, y))
    }
  }
}
